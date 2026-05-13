//! High-level Knowledge Base Service.
//!
//! Orchestrates the embedding, chunking, reranking, and storage components
//! to provide a unified API for knowledge base management and RAG retrieval.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::chunker::{Chunker, ChunkerConfig};
use super::embedding::EmbeddingProvider;
use super::rerank::RerankProvider;
use super::store::{
    AddDocumentRequest, CreateKnowledgeBaseRequest, KbChunk, KnowledgeBaseStore, SearchResult,
    UpdateKnowledgeBaseRequest,
};

/// High-level service for managing knowledge bases, documents, and retrieval.
pub struct KnowledgeBaseService {
    store: Arc<KnowledgeBaseStore>,
}

/// Batch size for embedding API calls.
const EMBEDDING_BATCH_SIZE: usize = 32;

impl KnowledgeBaseService {
    pub fn new(store: Arc<KnowledgeBaseStore>) -> Self {
        Self { store }
    }

    // ─── Knowledge Base CRUD ──────────────────────────────────────

    /// Create a new knowledge base.
    pub async fn create_knowledge_base(
        &self,
        req: CreateKnowledgeBaseRequest,
    ) -> Result<super::store::KnowledgeBase> {
        tracing::info!(name = %req.name, "Creating knowledge base");
        // Store layer already provides descriptive context; no need to double-wrap.
        self.store.create_knowledge_base(req).await
    }

    /// List all knowledge bases.
    pub async fn list_knowledge_bases(&self) -> Result<Vec<super::store::KnowledgeBase>> {
        self.store.list_knowledge_bases().await
    }

    /// Get a knowledge base by ID.
    pub async fn get_knowledge_base(&self, id: &str) -> Result<super::store::KnowledgeBase> {
        self.store
            .get_knowledge_base(id)
            .await
            .with_context(|| format!("Failed to get knowledge base '{}'", id))
    }

    /// Update a knowledge base.
    pub async fn update_knowledge_base(
        &self,
        id: &str,
        req: UpdateKnowledgeBaseRequest,
    ) -> Result<super::store::KnowledgeBase> {
        tracing::info!(id, "Updating knowledge base");
        self.store
            .update_knowledge_base(id, req)
            .await
            .with_context(|| format!("Failed to update knowledge base '{}'", id))
    }

    /// Delete a knowledge base and all its documents/chunks.
    pub async fn delete_knowledge_base(&self, id: &str) -> Result<()> {
        tracing::info!(id, "Deleting knowledge base");
        self.store
            .delete_knowledge_base(id)
            .await
            .with_context(|| format!("Failed to delete knowledge base '{}'", id))
    }

    // ─── Document operations ──────────────────────────────────────

    /// Upload a document to a knowledge base.
    ///
    /// This will:
    /// 1. Create a document record with status "processing"
    /// 2. Chunk the text using the knowledge base's chunk_size and chunk_overlap settings
    /// 3. Store chunks in the database
    /// 4. Generate embeddings for all chunks (in batches of 32)
    /// 5. Update each chunk's embedding
    /// 6. Update document status to "completed"
    ///
    /// If any step fails, the document status is set to "failed" with the error message.
    pub async fn upload_document(
        &self,
        knowledge_base_id: &str,
        filename: &str,
        content: &str,
    ) -> Result<super::store::KbDocument> {
        tracing::info!(
            knowledge_base_id,
            filename,
            content_len = content.len(),
            "Uploading document"
        );

        // Load knowledge base to get chunking/embedding configuration
        let kb = self
            .store
            .get_knowledge_base(knowledge_base_id)
            .await
            .context("Failed to load knowledge base for document upload")?;

        // 1. Compute content hash
        let content_hash = {
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        };

        // 2. Create document record with status "processing"
        let file_type = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt")
            .to_string();

        let doc_req = AddDocumentRequest {
            knowledge_base_id: knowledge_base_id.to_string(),
            filename: filename.to_string(),
            file_size: content.len() as i64,
            file_type: file_type.clone(),
            content_hash: content_hash.clone(),
        };

        let document = self
            .store
            .add_document(doc_req)
            .await
            .context("Failed to create document record")?;

        // Update status to "processing"
        self.store
            .update_document_status(&document.id, "processing", None)
            .await?;

        // Execute the processing pipeline; on failure, mark document as failed
        if let Err(e) = self
            .process_document(&kb, &document.id, knowledge_base_id, content)
            .await
        {
            tracing::error!(
                document_id = %document.id,
                error = %e,
                "Document processing failed"
            );
            let _ = self
                .store
                .update_document_status(&document.id, "failed", Some(&e.to_string()))
                .await;
        }

        // Return the (possibly updated) document
        self.store
            .get_document(&document.id)
            .await
            .context("Failed to retrieve document after upload")
    }

    /// Internal: run the chunking + embedding pipeline for a document.
    async fn process_document(
        &self,
        kb: &super::store::KnowledgeBase,
        document_id: &str,
        knowledge_base_id: &str,
        content: &str,
    ) -> Result<()> {
        // 3. Chunk the text
        let chunker = Chunker::new(ChunkerConfig {
            chunk_size: kb.chunk_size,
            chunk_overlap: kb.chunk_overlap,
            separator: "\n\n".to_string(),
        });

        let raw_chunks = chunker.chunk_with_metadata(content);
        tracing::info!(
            document_id,
            chunk_count = raw_chunks.len(),
            "Chunking complete"
        );

        // 4. Store chunks in the database (without embeddings initially)
        let now = chrono::Utc::now().to_rfc3339();
        let chunks: Vec<KbChunk> = raw_chunks
            .into_iter()
            .map(|c| KbChunk {
                id: uuid::Uuid::new_v4().to_string(),
                document_id: document_id.to_string(),
                knowledge_base_id: knowledge_base_id.to_string(),
                content: c.content,
                chunk_index: c.index,
                start_char: c.start_char,
                end_char: c.end_char,
                embedding: None,
                created_at: now.clone(),
            })
            .collect();

        self.store
            .add_chunks(chunks.clone())
            .await
            .context("Failed to store chunks")?;

        // 5. Generate embeddings for all chunks in batches
        let embedding_provider = EmbeddingProvider::new(kb.embedding_provider_config.clone());
        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let chunk_ids: Vec<String> = chunks.iter().map(|c| c.id.clone()).collect();

        for batch_start in (0..texts.len()).step_by(EMBEDDING_BATCH_SIZE) {
            let batch_end = std::cmp::min(batch_start + EMBEDDING_BATCH_SIZE, texts.len());
            let batch_texts = texts[batch_start..batch_end].to_vec();
            let batch_ids = &chunk_ids[batch_start..batch_end];

            tracing::debug!(batch_start, batch_end, "Embedding batch");

            let response = embedding_provider
                .embed(batch_texts)
                .await
                .context("Embedding API call failed")?;

            // 6. Update each chunk's embedding
            for embedding_data in &response.data {
                let idx = embedding_data.index;
                if idx < batch_ids.len() {
                    self.store
                        .update_chunk_embedding(&batch_ids[idx], embedding_data.vector.clone())
                        .await?;
                }
            }
        }

        // 7. Update document status to "completed"
        self.store
            .update_document_status(document_id, "completed", None)
            .await?;

        tracing::info!(document_id, "Document processing completed");
        Ok(())
    }

    /// Delete a document.
    pub async fn delete_document(&self, document_id: &str) -> Result<()> {
        tracing::info!(document_id, "Deleting document");
        self.store
            .delete_document(document_id)
            .await
            .with_context(|| format!("Failed to delete document '{}'", document_id))
    }

    /// List documents in a knowledge base.
    pub async fn list_documents(
        &self,
        knowledge_base_id: &str,
    ) -> Result<Vec<super::store::KbDocument>> {
        self.store
            .list_documents(knowledge_base_id)
            .await
            .with_context(|| {
                format!(
                    "Failed to list documents for knowledge base '{}'",
                    knowledge_base_id
                )
            })
    }

    // ─── Search & Retrieval ───────────────────────────────────────

    /// Search a knowledge base with a query.
    ///
    /// This will:
    /// 1. Embed the query using the knowledge base's embedding provider
    /// 2. Perform vector search against stored chunk embeddings
    /// 3. If rerank is configured, rerank the results for better relevance
    /// 4. Return top_k results
    pub async fn search(
        &self,
        knowledge_base_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        tracing::info!(
            knowledge_base_id,
            query_len = query.len(),
            top_k,
            "Searching knowledge base"
        );

        // 1. Load knowledge base config
        let kb = self
            .store
            .get_knowledge_base(knowledge_base_id)
            .await
            .context("Failed to load knowledge base for search")?;

        // 2. Embed the query
        let embedding_provider = EmbeddingProvider::new(kb.embedding_provider_config.clone());
        let query_embedding = embedding_provider
            .embed_single(query.to_string())
            .await
            .context("Failed to embed query")?;

        // 3. If rerank is configured, fetch wider results then rerank
        if let Some(ref rerank_config) = kb.rerank_provider_config {
            let wider_top_k = top_k * 3;
            let results = self
                .store
                .search(knowledge_base_id, query_embedding, wider_top_k)
                .await
                .context("Vector search failed")?;

            // 4. Rerank
            let rerank_provider = RerankProvider::new(rerank_config.clone());
            let documents: Vec<String> = results.iter().map(|r| r.chunk.content.clone()).collect();

            if documents.is_empty() {
                return Ok(vec![]);
            }

            let rerank_response = rerank_provider
                .rerank(query, documents, Some(top_k))
                .await
                .context("Rerank API call failed")?;

            // Reorder results according to rerank response
            let mut reranked = Vec::new();
            for rerank_result in &rerank_response.results {
                let idx = rerank_result.index;
                if idx < results.len() {
                    let mut sr = results[idx].clone();
                    // Update score to rerank relevance score
                    sr.score = rerank_result.relevance_score as f32;
                    reranked.push(sr);
                }
            }

            tracing::info!(
                knowledge_base_id,
                result_count = reranked.len(),
                "Search with rerank completed"
            );
            Ok(reranked)
        } else {
            // No rerank — just return top_k directly
            let results = self
                .store
                .search(knowledge_base_id, query_embedding, top_k)
                .await
                .context("Vector search failed")?;

            tracing::info!(
                knowledge_base_id,
                result_count = results.len(),
                "Search completed"
            );
            Ok(results)
        }
    }

    /// Retrieve relevant context for a query across specified knowledge bases.
    ///
    /// Used by the RAG skill to inject context into the agent's conversation.
    /// Searches each knowledge base, deduplicates, and formats results as a
    /// context string.
    pub async fn retrieve_context(
        &self,
        knowledge_base_ids: &[String],
        query: &str,
        top_k: usize,
    ) -> Result<String> {
        tracing::info!(
            kb_count = knowledge_base_ids.len(),
            query_len = query.len(),
            top_k,
            "Retrieving context across knowledge bases"
        );

        let mut all_results: Vec<SearchResult> = Vec::new();
        let mut seen_chunk_ids: HashSet<String> = HashSet::new();

        // Fetch more results per knowledge base to ensure coverage across all KBs
        // Use top_k * 2 per KB so we don't lose relevant content when merging
        let per_kb_limit = top_k * 2;

        for kb_id in knowledge_base_ids {
            match self.search(kb_id, query, per_kb_limit).await {
                Ok(results) => {
                    for result in results {
                        if seen_chunk_ids.insert(result.chunk.id.clone()) {
                            all_results.push(result);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        knowledge_base_id = %kb_id,
                        error = %e,
                        "Failed to search knowledge base, skipping"
                    );
                }
            }
        }

        // Sort by score descending
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Truncate to a reasonable overall limit (top_k * 2) to provide more context
        // while still avoiding overwhelming the model with too much content
        all_results.truncate(top_k * 2);

        if all_results.is_empty() {
            return Ok(String::new());
        }

        // Format as context string
        let mut context = String::from("--- Relevant Knowledge ---\n");
        for result in &all_results {
            context.push_str(&format!("[Source: {}]\n", result.document_filename));
            context.push_str(&result.chunk.content);
            context.push_str("\n\n");
        }
        context.push_str("--- End of Knowledge ---");

        Ok(context)
    }
}

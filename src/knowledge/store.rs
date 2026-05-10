//! Knowledge base database layer and vector store.
//!
//! Stores knowledge bases, documents, and chunks (with embeddings) in SQLite.
//! Cosine-similarity search is performed in memory against stored embedding blobs.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tracing::{debug, info};

use super::embedding::EmbeddingProviderConfig;
use super::rerank::RerankProviderConfig;

// ─── Models ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub embedding_provider_config: EmbeddingProviderConfig,
    pub rerank_provider_config: Option<RerankProviderConfig>,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub document_count: usize,
    pub chunk_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbDocument {
    pub id: String,
    pub knowledge_base_id: String,
    pub filename: String,
    pub file_size: i64,
    pub file_type: String,
    pub content_hash: String,
    pub chunk_count: usize,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbChunk {
    pub id: String,
    pub document_id: String,
    pub knowledge_base_id: String,
    pub content: String,
    pub chunk_index: usize,
    pub start_char: usize,
    pub end_char: usize,
    pub embedding: Option<Vec<f32>>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: KbChunk,
    pub score: f32,
    pub document_filename: String,
}

// ─── Request types ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreateKnowledgeBaseRequest {
    pub name: String,
    pub description: String,
    pub embedding_provider_config: EmbeddingProviderConfig,
    pub rerank_provider_config: Option<RerankProviderConfig>,
    pub chunk_size: Option<usize>,
    pub chunk_overlap: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateKnowledgeBaseRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    /// `Some(Some(config))` to set a new rerank config,
    /// `Some(None)` to clear the rerank config,
    /// `None` to leave it unchanged.
    pub rerank_provider_config: Option<Option<RerankProviderConfig>>,
    pub chunk_size: Option<usize>,
    pub chunk_overlap: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct AddDocumentRequest {
    pub knowledge_base_id: String,
    pub filename: String,
    pub file_size: i64,
    pub file_type: String,
    pub content_hash: String,
}

// ─── Embedding serialization helpers ───────────────────────────────────

/// Serialize a `Vec<f32>` into little-endian bytes for BLOB storage.
fn embedding_to_bytes(vec: &[f32]) -> Vec<u8> {
    vec.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialize little-endian bytes from a BLOB into `Vec<f32>`.
fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Compute cosine similarity between two vectors.
///
/// Returns a value in `[-1.0, 1.0]`. Returns `0.0` for zero vectors
/// or when the result would be NaN (e.g. due to floating-point
/// imprecision with very small norms).
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    let sim = dot / (norm_a * norm_b);
    // Clamp to valid range and guard against NaN from floating-point
    // imprecision (e.g. sim slightly > 1.0 or NaN due to sub-normals).
    if sim.is_nan() {
        0.0
    } else {
        sim.clamp(-1.0, 1.0)
    }
}

// ─── KnowledgeBaseStore ───────────────────────────────────────────────

pub struct KnowledgeBaseStore {
    pool: SqlitePool,
}

impl KnowledgeBaseStore {
    /// Create a new `KnowledgeBaseStore` using a **shared** `SqlitePool`.
    ///
    /// Schema creation is *not* performed here — it is handled centrally by
    /// `crate::db::init_schema()`. This constructor only stores the pool.
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let store = Self { pool };

        // Quick sanity-check: ensure the knowledge_bases table is reachable.
        sqlx::query("SELECT 1 FROM knowledge_bases LIMIT 1")
            .execute(&store.pool)
            .await
            .context("knowledge_bases table not found – did db::init() run?")?;

        info!("KnowledgeBaseStore ready (shared pool)");
        Ok(store)
    }

    // ─── Knowledge Base CRUD ──────────────────────────────────────

    pub async fn create_knowledge_base(
        &self,
        req: CreateKnowledgeBaseRequest,
    ) -> Result<KnowledgeBase> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let embedding_config_json = serde_json::to_string(&req.embedding_provider_config)
            .context("Failed to serialize embedding_provider_config")?;
        let rerank_config_json = req
            .rerank_provider_config
            .as_ref()
            .map(|c| serde_json::to_string(c))
            .transpose()
            .context("Failed to serialize rerank_provider_config")?;
        let chunk_size = req.chunk_size.unwrap_or(512) as i64;
        let chunk_overlap = req.chunk_overlap.unwrap_or(64) as i64;

        sqlx::query(
            r#"
            INSERT INTO knowledge_bases
                (id, name, description, embedding_provider_config, rerank_provider_config,
                 chunk_size, chunk_overlap, document_count, chunk_count, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0, ?8, ?8)
            "#,
        )
        .bind(&id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&embedding_config_json)
        .bind(&rerank_config_json)
        .bind(chunk_size)
        .bind(chunk_overlap)
        .bind(&now)
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to insert knowledge base '{}' into database",
                req.name
            )
        })?;

        debug!("Created knowledge base with id: {}", id);
        self.get_knowledge_base(&id).await
    }

    pub async fn list_knowledge_bases(&self) -> Result<Vec<KnowledgeBase>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, embedding_provider_config, rerank_provider_config,
                   chunk_size, chunk_overlap, document_count, chunk_count, created_at, updated_at
            FROM knowledge_bases
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to query knowledge bases from database")?;

        rows.iter()
            .enumerate()
            .map(|(i, row)| {
                self.row_to_knowledge_base(row)
                    .with_context(|| format!("Failed to parse knowledge base row {}", i))
            })
            .collect()
    }

    pub async fn get_knowledge_base(&self, id: &str) -> Result<KnowledgeBase> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, embedding_provider_config, rerank_provider_config,
                   chunk_size, chunk_overlap, document_count, chunk_count, created_at, updated_at
            FROM knowledge_bases
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Knowledge base '{}' not found", id))?;

        self.row_to_knowledge_base(&row)
    }

    pub async fn update_knowledge_base(
        &self,
        id: &str,
        req: UpdateKnowledgeBaseRequest,
    ) -> Result<KnowledgeBase> {
        let now = Utc::now().to_rfc3339();

        // Build dynamic SET clause
        let mut set_clauses = vec!["updated_at = ?1".to_string()];
        let mut bind_index = 2u32;

        // We'll collect the values to bind in order
        let name_val = req.name.clone();
        let desc_val = req.description.clone();
        let rerank_val: Option<Option<String>> = req.rerank_provider_config.map(|opt_config| {
            opt_config
                .map(|c| serde_json::to_string(&c).expect("Failed to serialize rerank config"))
        });
        let chunk_size_val = req.chunk_size.map(|v| v as i64);
        let chunk_overlap_val = req.chunk_overlap.map(|v| v as i64);

        if name_val.is_some() {
            set_clauses.push(format!("name = ?{}", bind_index));
            bind_index += 1;
        }
        if desc_val.is_some() {
            set_clauses.push(format!("description = ?{}", bind_index));
            bind_index += 1;
        }
        if rerank_val.is_some() {
            set_clauses.push(format!("rerank_provider_config = ?{}", bind_index));
            bind_index += 1;
        }
        if chunk_size_val.is_some() {
            set_clauses.push(format!("chunk_size = ?{}", bind_index));
            bind_index += 1;
        }
        if chunk_overlap_val.is_some() {
            set_clauses.push(format!("chunk_overlap = ?{}", bind_index));
            bind_index += 1;
        }

        let sql = format!(
            "UPDATE knowledge_bases SET {} WHERE id = ?{}",
            set_clauses.join(", "),
            bind_index
        );

        let mut query = sqlx::query(&sql).bind(&now);

        if let Some(ref v) = name_val {
            query = query.bind(v);
        }
        if let Some(ref v) = desc_val {
            query = query.bind(v);
        }
        if let Some(ref opt_str) = rerank_val {
            // Some(Some(json_string)) → bind the JSON string
            // Some(None) → bind SQL NULL to clear the field
            match opt_str {
                Some(json_str) => query = query.bind(json_str),
                None => query = query.bind(Option::<String>::None),
            }
        }
        if let Some(v) = chunk_size_val {
            query = query.bind(v);
        }
        if let Some(v) = chunk_overlap_val {
            query = query.bind(v);
        }

        query = query.bind(id);

        query
            .execute(&self.pool)
            .await
            .with_context(|| format!("Failed to update knowledge base '{}' in database", id))?;

        debug!("Updated knowledge base with id: {}", id);
        self.get_knowledge_base(id).await
    }

    pub async fn delete_knowledge_base(&self, id: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM knowledge_bases
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to delete knowledge base '{}' from database", id))?;

        debug!("Deleted knowledge base with id: {}", id);
        Ok(())
    }

    // ─── Document CRUD ────────────────────────────────────────────

    pub async fn add_document(&self, req: AddDocumentRequest) -> Result<KbDocument> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO kb_documents
                (id, knowledge_base_id, filename, file_size, file_type, content_hash,
                 chunk_count, status, error_message, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 'pending', NULL, ?7, ?7)
            "#,
        )
        .bind(&id)
        .bind(&req.knowledge_base_id)
        .bind(&req.filename)
        .bind(req.file_size)
        .bind(&req.file_type)
        .bind(&req.content_hash)
        .bind(&now)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to add document '{}' to database", req.filename))?;

        // Update knowledge base document_count
        self.refresh_knowledge_base_counts(&req.knowledge_base_id)
            .await?;

        debug!(
            "Added document with id: {} to knowledge base: {}",
            id, req.knowledge_base_id
        );
        self.get_document(&id).await
    }

    pub async fn list_documents(&self, knowledge_base_id: &str) -> Result<Vec<KbDocument>> {
        let rows = sqlx::query(
            r#"
            SELECT id, knowledge_base_id, filename, file_size, file_type, content_hash,
                   chunk_count, status, error_message, created_at, updated_at
            FROM kb_documents
            WHERE knowledge_base_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(knowledge_base_id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to list documents for knowledge base '{}' from database",
                knowledge_base_id
            )
        })?;

        rows.iter().map(|row| self.row_to_document(row)).collect()
    }

    pub async fn get_document(&self, id: &str) -> Result<KbDocument> {
        let row = sqlx::query(
            r#"
            SELECT id, knowledge_base_id, filename, file_size, file_type, content_hash,
                   chunk_count, status, error_message, created_at, updated_at
            FROM kb_documents
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Document '{}' not found", id))?;

        self.row_to_document(&row)
    }

    pub async fn delete_document(&self, id: &str) -> Result<()> {
        // Get the knowledge_base_id before deleting so we can update counts
        let doc = self.get_document(id).await?;

        sqlx::query(
            r#"
            DELETE FROM kb_documents
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to delete document '{}' from database", id))?;

        // Refresh counts (CASCADE already deleted chunks)
        self.refresh_knowledge_base_counts(&doc.knowledge_base_id)
            .await?;

        debug!("Deleted document with id: {}", id);
        Ok(())
    }

    pub async fn update_document_status(
        &self,
        id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE kb_documents
            SET status = ?1, error_message = ?2, updated_at = ?3
            WHERE id = ?4
            "#,
        )
        .bind(status)
        .bind(error)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to update document '{}' status", id))?;

        debug!("Updated document {} status to: {}", id, status);
        Ok(())
    }

    // ─── Chunk operations ─────────────────────────────────────────

    pub async fn add_chunks(&self, chunks: Vec<KbChunk>) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }

        let knowledge_base_id = chunks[0].knowledge_base_id.clone();
        let document_id = chunks[0].document_id.clone();

        for chunk in &chunks {
            let embedding_blob: Option<Vec<u8>> =
                chunk.embedding.as_ref().map(|e| embedding_to_bytes(e));

            sqlx::query(
                r#"
                INSERT INTO kb_chunks
                    (id, document_id, knowledge_base_id, content, chunk_index,
                     start_char, end_char, embedding, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
            )
            .bind(&chunk.id)
            .bind(&chunk.document_id)
            .bind(&chunk.knowledge_base_id)
            .bind(&chunk.content)
            .bind(chunk.chunk_index as i64)
            .bind(chunk.start_char as i64)
            .bind(chunk.end_char as i64)
            .bind(&embedding_blob)
            .bind(&chunk.created_at)
            .execute(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to insert chunk {} for document '{}'",
                    chunk.id, chunk.document_id
                )
            })?;
        }

        // Update document chunk_count
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE kb_documents
            SET chunk_count = (SELECT COUNT(*) FROM kb_chunks WHERE document_id = ?1),
                updated_at = ?2
            WHERE id = ?1
            "#,
        )
        .bind(&document_id)
        .bind(&now)
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to update chunk count for document '{}'",
                document_id
            )
        })?;

        // Update knowledge base counts
        self.refresh_knowledge_base_counts(&knowledge_base_id)
            .await?;

        debug!("Added {} chunks for document {}", chunks.len(), document_id);
        Ok(())
    }

    pub async fn get_chunks_by_document(&self, document_id: &str) -> Result<Vec<KbChunk>> {
        let rows = sqlx::query(
            r#"
            SELECT id, document_id, knowledge_base_id, content, chunk_index,
                   start_char, end_char, embedding, created_at
            FROM kb_chunks
            WHERE document_id = ?1
            ORDER BY chunk_index ASC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to get chunks by document")?;

        rows.iter().map(|row| self.row_to_chunk(row)).collect()
    }

    pub async fn get_all_chunks(&self, knowledge_base_id: &str) -> Result<Vec<KbChunk>> {
        let rows = sqlx::query(
            r#"
            SELECT id, document_id, knowledge_base_id, content, chunk_index,
                   start_char, end_char, embedding, created_at
            FROM kb_chunks
            WHERE knowledge_base_id = ?1
            ORDER BY document_id, chunk_index ASC
            "#,
        )
        .bind(knowledge_base_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to get all chunks")?;

        rows.iter().map(|row| self.row_to_chunk(row)).collect()
    }

    pub async fn update_chunk_embedding(&self, chunk_id: &str, embedding: Vec<f32>) -> Result<()> {
        let blob = embedding_to_bytes(&embedding);

        sqlx::query(
            r#"
            UPDATE kb_chunks
            SET embedding = ?1
            WHERE id = ?2
            "#,
        )
        .bind(&blob)
        .bind(chunk_id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to update embedding for chunk '{}'", chunk_id))?;

        Ok(())
    }

    // ─── Vector search ────────────────────────────────────────────

    pub async fn search(
        &self,
        knowledge_base_id: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        // Query all chunks with embeddings from the knowledge base, joined with document filename
        let rows = sqlx::query(
            r#"
            SELECT c.id, c.document_id, c.knowledge_base_id, c.content, c.chunk_index,
                   c.start_char, c.end_char, c.embedding, c.created_at,
                   d.filename AS document_filename
            FROM kb_chunks c
            JOIN kb_documents d ON c.document_id = d.id
            WHERE c.knowledge_base_id = ?1 AND c.embedding IS NOT NULL
            "#,
        )
        .bind(knowledge_base_id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to search chunks in knowledge base '{}' from database",
                knowledge_base_id
            )
        })?;

        let mut results: Vec<SearchResult> = rows
            .iter()
            .filter_map(|row| {
                let embedding_blob: Vec<u8> = row.try_get("embedding").ok()?;
                let embedding = bytes_to_embedding(&embedding_blob);
                let score = cosine_similarity(&query_embedding, &embedding);

                let chunk = self.row_to_chunk(row).ok()?;
                let document_filename: String = row.try_get("document_filename").ok()?;

                Some(SearchResult {
                    chunk,
                    score,
                    document_filename,
                })
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top_k
        results.truncate(top_k);

        Ok(results)
    }

    // ─── Internal helpers ─────────────────────────────────────────

    /// Refresh `document_count` and `chunk_count` on a knowledge base row
    /// by counting from the actual tables.
    async fn refresh_knowledge_base_counts(&self, knowledge_base_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE knowledge_bases
            SET document_count = (SELECT COUNT(*) FROM kb_documents WHERE knowledge_base_id = ?1),
                chunk_count = (SELECT COUNT(*) FROM kb_chunks WHERE knowledge_base_id = ?1),
                updated_at = ?2
            WHERE id = ?1
            "#,
        )
        .bind(knowledge_base_id)
        .bind(&now)
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to refresh counts for knowledge base '{}'",
                knowledge_base_id
            )
        })?;

        Ok(())
    }

    fn row_to_knowledge_base(&self, row: &sqlx::sqlite::SqliteRow) -> Result<KnowledgeBase> {
        let embedding_config_str: String = row.try_get("embedding_provider_config")?;
        let embedding_provider_config: EmbeddingProviderConfig =
            serde_json::from_str(&embedding_config_str)
                .context("Failed to deserialize embedding_provider_config")?;

        let rerank_config_str: Option<String> = row.try_get("rerank_provider_config").ok();
        let rerank_provider_config: Option<RerankProviderConfig> = rerank_config_str
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(serde_json::from_str)
            .transpose()
            .context("Failed to deserialize rerank_provider_config")?;

        Ok(KnowledgeBase {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            embedding_provider_config,
            rerank_provider_config,
            chunk_size: row.try_get::<i64, _>("chunk_size")? as usize,
            chunk_overlap: row.try_get::<i64, _>("chunk_overlap")? as usize,
            document_count: row.try_get::<i64, _>("document_count")? as usize,
            chunk_count: row.try_get::<i64, _>("chunk_count")? as usize,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    fn row_to_document(&self, row: &sqlx::sqlite::SqliteRow) -> Result<KbDocument> {
        Ok(KbDocument {
            id: row.try_get("id")?,
            knowledge_base_id: row.try_get("knowledge_base_id")?,
            filename: row.try_get("filename")?,
            file_size: row.try_get("file_size")?,
            file_type: row.try_get("file_type")?,
            content_hash: row.try_get("content_hash")?,
            chunk_count: row.try_get::<i64, _>("chunk_count")? as usize,
            status: row.try_get("status")?,
            error_message: row.try_get("error_message").ok(),
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    fn row_to_chunk(&self, row: &sqlx::sqlite::SqliteRow) -> Result<KbChunk> {
        let embedding_blob: Option<Vec<u8>> = row.try_get("embedding").ok();
        let embedding = embedding_blob.as_deref().map(bytes_to_embedding);

        Ok(KbChunk {
            id: row.try_get("id")?,
            document_id: row.try_get("document_id")?,
            knowledge_base_id: row.try_get("knowledge_base_id")?,
            content: row.try_get("content")?,
            chunk_index: row.try_get::<i64, _>("chunk_index")? as usize,
            start_char: row.try_get::<i64, _>("start_char")? as usize,
            end_char: row.try_get::<i64, _>("end_char")? as usize,
            embedding,
            created_at: row.try_get("created_at")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_roundtrip() {
        let original = vec![1.0f32, 2.0, 3.0, -1.5, 0.0];
        let bytes = embedding_to_bytes(&original);
        let recovered = bytes_to_embedding(&bytes);
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0f32, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0f32, 0.0];
        let b = vec![0.0f32, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![1.0f32, 2.0];
        let b = vec![0.0f32, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6);
    }
}

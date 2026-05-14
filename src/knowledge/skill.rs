//! Knowledge Base RAG Skill and Search Tool for the Agent.
//!
//! The `KnowledgeBaseSkill` informs the model that a knowledge base search
//! tool is available, so it can decide when to retrieve context on demand.
//!
//! The `KnowledgeBaseSearchTool` is the actual tool implementation that the
//! model calls when it determines that knowledge base context is needed.
//!
//! Two retrieval modes are supported:
//! - **Tool-based** (default): The model decides when to call the search tool.
//! - **Auto**: Context is automatically retrieved and injected into each user
//!   message before the model sees it.

use crate::agent::skill::Skill;
use crate::agent::tool_executor::{Tool, ToolError};
use crate::types::message::{ChatMessage, MessageContent};
use crate::types::tool::*;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::service::KnowledgeBaseService;

/// Default number of results to retrieve from knowledge base search.
pub const DEFAULT_KB_SEARCH_TOP_K: usize = 10;

/// How knowledge base context is injected into the conversation.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub enum KnowledgeBaseRetrievalMode {
    /// The model decides when to call the `knowledge_base_search` tool.
    #[default]
    ToolBased,
    /// Context is automatically retrieved and injected into each user message.
    Auto,
}

// ─── KnowledgeBaseSearchTool ──────────────────────────────────────

/// A tool that the model can call to search knowledge bases on demand.
///
/// Rather than automatically injecting KB context on every message, this
/// tool lets the model decide when retrieval is necessary.
pub struct KnowledgeBaseSearchTool {
    service: Arc<RwLock<Option<KnowledgeBaseService>>>,
    knowledge_base_ids: Vec<String>,
    top_k: usize,
}

impl KnowledgeBaseSearchTool {
    pub fn new(
        service: Arc<RwLock<Option<KnowledgeBaseService>>>,
        knowledge_base_ids: Vec<String>,
        top_k: usize,
    ) -> Self {
        Self {
            service,
            knowledge_base_ids,
            top_k,
        }
    }
}

/// Parse a JSON args string into a serde_json::Value.
fn parse_args(args: &str) -> Result<Value, ToolError> {
    serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))
}

#[async_trait]
impl Tool for KnowledgeBaseSearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("knowledge_base_search")
            .description(
                "Search the knowledge base for relevant information. \
                 Use this tool when the user's question might benefit from \
                 information stored in the configured knowledge bases. \
                 Do NOT use it for casual conversation or when you can answer \
                 from your own knowledge. Always cite the source document when \
                 using knowledge base information in your response.",
            )
            .parameter_with_description(
                "query",
                ParameterType::String,
                true,
                Some("The search query to find relevant information in the knowledge base."),
            )
            .parameter_with_description(
                "tags",
                ParameterType::Array,
                false,
                Some("Optional list of tags to filter search results. Only documents matching these tags will be searched."),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed = parse_args(args)?;
        let query = parsed["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query' parameter".into()))?;

        if query.is_empty() {
            return Err(ToolError::InvalidArguments(
                "Query parameter must not be empty".into(),
            ));
        }

        // Parse optional tags parameter
        let tag_filter: Option<Vec<String>> = parsed
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .filter(|tags: &Vec<String>| !tags.is_empty());

        let guard = self.service.read().await;
        match guard.as_ref() {
            Some(service) => {
                match service
                    .retrieve_context(
                        &self.knowledge_base_ids,
                        query,
                        self.top_k,
                        1,
                        4096,
                        tag_filter,
                    )
                    .await
                {
                    Ok(context) if !context.is_empty() => Ok(context),
                    Ok(_) => Ok("No relevant information found in the knowledge base.".to_string()),
                    Err(e) => {
                        tracing::warn!(error = %e, "Knowledge base search failed");
                        Err(ToolError::ExecutionError(format!(
                            "Knowledge base search failed: {}",
                            e
                        )))
                    }
                }
            }
            None => {
                Ok("Knowledge base service is not available. Please try again later.".to_string())
            }
        }
    }
}

// ─── KnowledgeBaseSkill ───────────────────────────────────────────

/// A `Skill` implementation that informs the model about the knowledge base
/// search tool availability.
///
/// When active, it:
/// - Injects a system message on attach telling the model about the KB search tool
/// - In Auto mode, automatically retrieves and injects relevant context into
///   each user message before the model sees it
pub struct KnowledgeBaseSkill {
    /// IDs of the knowledge bases to search when retrieving context.
    knowledge_base_ids: Vec<String>,
    /// How knowledge base context is injected into the conversation.
    retrieval_mode: KnowledgeBaseRetrievalMode,
    /// Shared knowledge base service handle.
    service: Arc<RwLock<Option<KnowledgeBaseService>>>,
}

impl KnowledgeBaseSkill {
    pub fn new(
        knowledge_base_ids: Vec<String>,
        retrieval_mode: KnowledgeBaseRetrievalMode,
        service: Arc<RwLock<Option<KnowledgeBaseService>>>,
    ) -> Self {
        Self {
            knowledge_base_ids,
            retrieval_mode,
            service,
        }
    }
}

#[async_trait]
impl Skill for KnowledgeBaseSkill {
    fn name(&self) -> &str {
        "knowledge_base"
    }

    async fn on_attach(&self) -> Vec<ChatMessage> {
        if self.knowledge_base_ids.is_empty() {
            return Vec::new();
        }
        let prompt = match self.retrieval_mode {
            KnowledgeBaseRetrievalMode::ToolBased => {
                "You have access to a knowledge base search tool. When the user's question might benefit \
                 from information in the knowledge base, call the `knowledge_base_search` tool with a \
                 search query to retrieve relevant context. Do NOT call it for casual conversation or \
                 when you can answer from your own knowledge. Always cite the source document when \
                 using knowledge base information."
            }
            KnowledgeBaseRetrievalMode::Auto => {
                "Knowledge base context is automatically injected before your responses. When you see \
                 content prefixed with '--- Relevant Knowledge ---', it has been retrieved from the \
                 knowledge base and is relevant to the user's question. Use this context to enhance \
                 your response and always cite the source document when using knowledge base information. \
                 You do NOT need to call the `knowledge_base_search` tool as context is already provided."
            }
        };
        vec![ChatMessage::system(prompt)]
    }

    async fn on_user_message(&self, messages: &mut Vec<ChatMessage>) {
        if matches!(self.retrieval_mode, KnowledgeBaseRetrievalMode::ToolBased) {
            return; // Tool-based mode: no auto-injection
        }

        // Auto mode: retrieve context based on user message
        if let Some(last) = messages.last_mut() {
            if last.role == crate::types::MessageRole::User {
                if let Some(ref content) = last.content {
                    let text = content.as_text_full().unwrap_or_default();
                    if !text.is_empty() {
                        let guard = self.service.read().await;
                        if let Some(service) = guard.as_ref() {
                            match service
                                .retrieve_context(
                                    &self.knowledge_base_ids,
                                    &text,
                                    DEFAULT_KB_SEARCH_TOP_K,
                                    1,
                                    4096,
                                    None,
                                )
                                .await
                            {
                                Ok(context) if !context.is_empty() => {
                                    let new_content = format!("{}\n\n{}", context, text);
                                    last.content = Some(MessageContent::Text(new_content));
                                }
                                _ => {} // No relevant context found, proceed without it
                            }
                        }
                    }
                }
            }
        }
    }

    fn is_active(&self) -> bool {
        !self.knowledge_base_ids.is_empty()
    }
}

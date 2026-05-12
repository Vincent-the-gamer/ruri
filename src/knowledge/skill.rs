//! Knowledge Base RAG Skill for the Agent.
//!
//! This skill intercepts user messages, retrieves relevant context from
//! configured knowledge bases, and injects that context into the
//! conversation so the model can use it when formulating responses.

use crate::agent::skill::Skill;
use crate::types::message::{ChatMessage, MessageContent};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::service::KnowledgeBaseService;

/// A `Skill` implementation that augments user messages with knowledge base context.
///
/// When active, it:
/// - Injects a system message on attach explaining the KB capability
/// - On each user message, retrieves relevant context and prepends it
pub struct KnowledgeBaseSkill {
    /// The knowledge base service, wrapped in `RwLock<Option<...>>` so it can
    /// be set up lazily or replaced at runtime (e.g., when the DB becomes available).
    service: Arc<RwLock<Option<KnowledgeBaseService>>>,
    /// IDs of the knowledge bases to search when retrieving context.
    knowledge_base_ids: Vec<String>,
    /// Maximum number of results to retrieve across all knowledge bases.
    top_k: usize,
}

impl KnowledgeBaseSkill {
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

#[async_trait]
impl Skill for KnowledgeBaseSkill {
    fn name(&self) -> &str {
        "knowledge_base"
    }

    async fn on_attach(&self) -> Vec<ChatMessage> {
        vec![ChatMessage::system(
            "You have access to a knowledge base. When responding to user queries, \
             relevant information from the knowledge base will be automatically provided \
             as context. Use this context to enhance your responses, but also rely on your \
             general knowledge when the provided context is not sufficient. \
             Always cite the source document when using knowledge base information.",
        )]
    }

    async fn on_user_message(&self, messages: &mut Vec<ChatMessage>) {
        // Find the last user message and extract its text content.
        let last_user_msg = messages
            .iter()
            .rev()
            .find(|m| m.role == crate::types::MessageRole::User);

        let user_text = match last_user_msg.and_then(|m| m.content.as_ref()) {
            Some(MessageContent::Text(text)) => text.clone(),
            Some(MessageContent::Parts(parts)) => parts
                .iter()
                .filter_map(|p| {
                    if p.part_type == crate::types::message::ContentPartType::Text {
                        p.text.as_deref()
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
            None => return, // No user text to retrieve context for
        };

        if user_text.is_empty() {
            return;
        }

        // Try to retrieve context; if the service or retrieval fails, just skip.
        let context = {
            let guard = self.service.read().await;
            match guard.as_ref() {
                Some(service) => {
                    match service
                        .retrieve_context(&self.knowledge_base_ids, &user_text, self.top_k)
                        .await
                    {
                        Ok(ctx) if !ctx.is_empty() => Some(ctx),
                        Ok(_) => None,
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "Knowledge base context retrieval failed, skipping"
                            );
                            None
                        }
                    }
                }
                None => {
                    tracing::debug!("Knowledge base service not available, skipping retrieval");
                    None
                }
            }
        };

        // If we got context, prepend it to the last user message.
        if let Some(context) = context {
            // Find the last user message again, this time mutably
            if let Some(last) = messages
                .iter_mut()
                .rev()
                .find(|m| m.role == crate::types::MessageRole::User)
            {
                if let Some(ref content) = last.content {
                    let original = content.as_text_full().unwrap_or_default();
                    let new_content = format!("{}\n\nUser query: {}", context, original);
                    last.content = Some(MessageContent::Text(new_content));
                }
            }
        }
    }

    fn is_active(&self) -> bool {
        !self.knowledge_base_ids.is_empty()
    }
}

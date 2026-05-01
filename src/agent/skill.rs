use crate::types::ChatMessage;
use async_trait::async_trait;

/// A Skill is a modular capability that can be attached to an Agent.
///
/// Skills can:
/// - Inject system prompts to guide the model's behavior
/// - Pre-process user messages before they reach the model
/// - Post-process the model's responses
/// - Maintain state across conversation turns
///
/// Examples of skills:
/// - "Memory" — stores and retrieves conversation context
/// - "WebSearch" — augments responses with web search results
/// - "CodeExecution" — runs code and feeds results back
/// - "RAG" — retrieves relevant documents to augment prompts
#[async_trait]
pub trait Skill: Send + Sync {
    /// The name of this skill.
    fn name(&self) -> &str;

    /// Optional description of what this skill does.
    fn description(&self) -> &str {
        ""
    }

    /// Called once when the skill is attached to an Agent.
    /// Return a list of system messages that should be injected.
    async fn on_attach(&self) -> Vec<ChatMessage> {
        Vec::new()
    }

    /// Pre-process a user message before it's sent to the model.
    /// Returns the (possibly modified) list of messages.
    async fn on_user_message(&self, messages: &mut Vec<ChatMessage>) {
        let _ = messages; // no-op by default
    }

    /// Post-process a model response after it's received.
    /// Returns the (possibly modified) response message.
    async fn on_response(&self, response: &mut ChatMessage) {
        let _ = response; // no-op by default
    }

    /// Called when a tool call result is available.
    /// Returns whether the agent should continue processing.
    async fn on_tool_result(&self, tool_name: &str, result: &str) -> bool {
        let _ = (tool_name, result);
        true // continue by default
    }

    /// Whether this skill should be active for the current conversation turn.
    fn is_active(&self) -> bool {
        true
    }
}

// ─── Built-in Skills ─────────────────────────────────────────────────

/// A simple system prompt skill that injects a fixed system message.
pub struct SystemPromptSkill {
    prompt: String,
}

impl SystemPromptSkill {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
        }
    }
}

#[async_trait]
impl Skill for SystemPromptSkill {
    fn name(&self) -> &str {
        "system_prompt"
    }

    fn description(&self) -> &str {
        "Injects a system prompt to guide the model's behavior"
    }

    async fn on_attach(&self) -> Vec<ChatMessage> {
        vec![ChatMessage::system(&self.prompt)]
    }
}

/// A skill that adds conversation memory by tracking message history.
pub struct MemorySkill {
    max_messages: usize,
}

impl MemorySkill {
    pub fn new(max_messages: usize) -> Self {
        Self { max_messages }
    }
}

#[async_trait]
impl Skill for MemorySkill {
    fn name(&self) -> &str {
        "memory"
    }

    fn description(&self) -> &str {
        "Manages conversation memory with a configurable message limit"
    }

    async fn on_user_message(&self, messages: &mut Vec<ChatMessage>) {
        // Keep system messages and the most recent messages
        if messages.len() > self.max_messages {
            let system_count = messages
                .iter()
                .take_while(|m| m.role == crate::types::MessageRole::System)
                .count();

            let available = self.max_messages.saturating_sub(system_count);
            if messages.len() - system_count > available {
                let drain_count = messages.len() - system_count - available;
                messages.drain(system_count..system_count + drain_count);
            }
        }
    }
}

/// A skill that prefixes user messages with context information.
pub struct ContextPrefixSkill {
    prefix: String,
}

impl ContextPrefixSkill {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

#[async_trait]
impl Skill for ContextPrefixSkill {
    fn name(&self) -> &str {
        "context_prefix"
    }

    fn description(&self) -> &str {
        "Prefixes user messages with additional context"
    }

    async fn on_user_message(&self, messages: &mut Vec<ChatMessage>) {
        if let Some(last) = messages.last_mut() {
            if last.role == crate::types::MessageRole::User {
                if let crate::types::MessageContent::Text(ref text) = last.content {
                    let new_content = format!("{}\n\n{}", self.prefix, text);
                    last.content = crate::types::MessageContent::Text(new_content);
                }
            }
        }
    }
}

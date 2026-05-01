use crate::agent::runner::{Agent, AgentConfig};
use crate::provider::Provider;
use crate::types::ChatMessage;
use chrono::{DateTime, Utc};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Information about a stored provider configuration.
#[derive(Debug, Clone)]
pub struct StoredProvider {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub config_json: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Information about a stored skill.
#[derive(Debug, Clone)]
pub struct StoredSkill {
    pub name: String,
    pub description: String,
    pub skill_type: String,
    pub config: serde_json::Value,
    pub is_active: bool,
}

/// Application state shared across all API handlers.
pub struct AppState {
    /// Stored provider configurations.
    pub providers: RwLock<HashMap<String, StoredProvider>>,
    /// ID of the currently active provider.
    pub active_provider_id: RwLock<Option<String>>,
    /// Stored skill configurations.
    pub skills: RwLock<HashMap<String, StoredSkill>>,
    /// Tool definitions (read-only, set at startup).
    pub tool_definitions: Vec<crate::types::ToolDefinition>,
    /// Chat history.
    pub chat_history: RwLock<Vec<ChatMessage>>,
    /// Server start time.
    pub start_time: DateTime<Utc>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            active_provider_id: RwLock::new(None),
            skills: RwLock::new(HashMap::new()),
            tool_definitions: Vec::new(),
            chat_history: RwLock::new(Vec::new()),
            start_time: Utc::now(),
        }
    }

    /// Build a Provider instance from a stored provider configuration.
    pub fn build_provider(&self, stored: &StoredProvider) -> Result<Box<dyn Provider>, String> {
        let config = &stored.config_json;

        match stored.provider_type.as_str() {
            "openai" => {
                let base_url = config["base_url"].as_str().unwrap_or("").to_string();
                let api_key = config["api_key"].as_str().map(|s| s.to_string());
                let default_model = config["default_model"]
                    .as_str()
                    .unwrap_or("gpt-4o")
                    .to_string();

                Ok(Box::new(crate::provider::openai::OpenAIProvider::new(
                    base_url,
                    api_key,
                    default_model,
                )))
            }
            "anthropic" => {
                let base_url = config["base_url"].as_str().unwrap_or("").to_string();
                let api_key = config["api_key"].as_str().unwrap_or("").to_string();
                let default_model = config["default_model"]
                    .as_str()
                    .unwrap_or("claude-sonnet-4-20250514")
                    .to_string();

                Ok(Box::new(
                    crate::provider::anthropic::AnthropicProvider::new(api_key, default_model)
                        .with_base_url(base_url),
                ))
            }
            "custom" => {
                // Extract api_key from the DTO before deserializing to CustomProviderConfig
                let api_key = config
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Remove api_key from the value before deserializing (CustomProviderConfig doesn't have this field)
                let mut config_value = config.clone();
                if let Some(obj) = config_value.as_object_mut() {
                    obj.remove("api_key");
                    obj.remove("type"); // Also remove the serde tag
                }

                let custom_config: crate::provider::custom::CustomProviderConfig =
                    serde_json::from_value(config_value)
                        .map_err(|e| format!("Invalid custom provider config: {}", e))?;

                Ok(Box::new(crate::provider::custom::CustomProvider::new(
                    custom_config,
                    api_key,
                )))
            }
            other => Err(format!("Unknown provider type: {}", other)),
        }
    }

    /// Build a fully configured Agent from the current state.
    pub async fn build_agent(&self) -> Result<Agent, String> {
        let providers = self.providers.read().await;
        let active_id = self.active_provider_id.read().await;

        let active_id = active_id.as_ref().ok_or("No active provider configured")?;

        let stored = providers
            .get(active_id)
            .ok_or("Active provider not found")?;

        let provider = self.build_provider(stored)?;
        drop(providers);
        let _ = active_id;

        let config = AgentConfig::new()
            .with_max_tool_rounds(10)
            .with_auto_execute_tools(true);

        let mut agent = Agent::with_config(provider, config);

        // Re-add skills
        let skills = self.skills.read().await;
        for (_name, skill) in skills.iter() {
            if !skill.is_active {
                continue;
            }
            match skill.skill_type.as_str() {
                "system_prompt" => {
                    let prompt = skill.config["prompt"].as_str().unwrap_or("").to_string();
                    agent.add_skill(Arc::new(crate::agent::skill::SystemPromptSkill::new(
                        prompt,
                    )));
                }
                "memory" => {
                    let max = skill.config["max_messages"].as_u64().unwrap_or(50) as usize;
                    agent.add_skill(Arc::new(crate::agent::skill::MemorySkill::new(max)));
                }
                "context_prefix" => {
                    let prefix = skill.config["prefix"].as_str().unwrap_or("").to_string();
                    agent.add_skill(Arc::new(crate::agent::skill::ContextPrefixSkill::new(
                        prefix,
                    )));
                }
                _ => {}
            }
        }
        drop(skills);

        // Register built-in tools
        agent.register_tool(Arc::new(crate::agent::tool_executor::EchoTool));
        agent.register_tool(Arc::new(crate::agent::tool_executor::CalculatorTool));
        agent.register_tool(Arc::new(crate::agent::tool_executor::DateTimeTool));

        // Initialize skills
        agent.initialize_skills().await;

        // Restore chat history
        let history = self.chat_history.read().await;
        for _msg in history.iter() {
            // We need to set history on the agent; use a workaround via chat
        }
        drop(history);

        Ok(agent)
    }
}

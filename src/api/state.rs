use crate::agent::runner::{Agent, AgentConfig};
use crate::agent::skill::{ContextPrefixSkill, MemorySkill, Skill, SystemPromptSkill};
use crate::provider::Provider;
use crate::types::ChatMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

// ─── Persisted Config Structures (serde-friendly) ────────────────

/// Serializable version of StoredProvider for config file persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedProvider {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub config_json: serde_json::Value,
    pub is_active: bool,
    pub created_at: String,
}

/// Serializable version of StoredSkill for config file persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSkill {
    pub name: String,
    pub description: String,
    pub skill_type: String,
    pub config: serde_json::Value,
    pub is_active: bool,
}

/// ACP-specific configuration stored alongside the main config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcpConfig {
    /// The provider ID to use in ACP mode. If None, falls back to the API-mode active provider.
    pub active_provider_id: Option<String>,
    /// Skill names to enable in ACP mode.
    pub active_skill_names: Vec<String>,
}

/// The top-level persisted config file format.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedConfig {
    #[serde(default)]
    pub providers: HashMap<String, PersistedProvider>,
    #[serde(default)]
    pub active_provider_id: Option<String>,
    #[serde(default)]
    pub skills: HashMap<String, PersistedSkill>,
    #[serde(default)]
    pub acp_config: AcpConfig,
    #[serde(default)]
    pub computer_use_config: crate::computer_use::ComputerUseConfig,
    #[serde(default)]
    pub web_search_config: crate::types::WebSearchConfig,
}

// ─── In-Memory State Types ───────────────────────────────────────

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

// ─── Config File Path ────────────────────────────────────────────

/// Returns the Ruri config directory: `~/.ruri/`
///
/// Uses the `dirs` crate for cross-platform home directory resolution:
/// - **Linux/macOS**: `$HOME/.ruri/`
/// - **Windows**: `C:\Users\<user>\AppData\Roaming\.ruri\` (via `dirs::data_dir` fallback)
///
/// Falls back to `.ruri/` in the current directory if the home directory
/// cannot be determined.
pub fn ruri_config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ruri")
}

/// Returns the default config file path: `<config_dir>/config.json`
pub fn default_config_path() -> PathBuf {
    ruri_config_dir().join("config.json")
}

/// Returns the chat history file path: `<config_dir>/chat_history.json`
pub fn chat_history_path() -> PathBuf {
    ruri_config_dir().join("chat_history.json")
}

/// Ensures the parent directory of the given path exists.
async fn ensure_parent_dir(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    Ok(())
}

// ─── Application State ───────────────────────────────────────────

/// Application state shared across all API handlers.
pub struct AppState {
    /// Stored provider configurations.
    pub providers: RwLock<HashMap<String, StoredProvider>>,
    /// ID of the currently active provider.
    pub active_provider_id: RwLock<Option<String>>,
    /// Stored skill configurations.
    pub skills: RwLock<HashMap<String, StoredSkill>>,
    /// ACP-specific configuration.
    pub acp_config: RwLock<AcpConfig>,
    /// Computer use configuration.
    pub computer_use_config: RwLock<crate::computer_use::ComputerUseConfig>,
    /// Web search configuration.
    pub web_search_config: std::sync::Arc<RwLock<crate::types::WebSearchConfig>>,
    /// Workspace manager for computer use.
    pub workspace_manager: std::sync::Arc<crate::computer_use::WorkspaceManager>,
    /// Tool definitions (read-only, set at startup).
    pub tool_definitions: Vec<crate::types::ToolDefinition>,
    /// Chat history.
    pub chat_history: RwLock<Vec<ChatMessage>>,
    /// Server start time.
    pub start_time: DateTime<Utc>,
    /// Path to the config file.
    pub(crate) config_path: PathBuf,
    /// Path to the chat history file.
    pub(crate) chat_history_path: PathBuf,
    /// Log manager for real-time log broadcasting.
    pub log_manager: std::sync::Arc<crate::logging::LogManager>,
}

impl AppState {
    /// Create a new AppState, attempting to load persisted config from the default path.
    pub fn new() -> Self {
        let config_path = default_config_path();
        Self::with_config_path(&config_path)
    }

    /// Set the log manager.
    pub fn with_log_manager(
        mut self,
        log_manager: std::sync::Arc<crate::logging::LogManager>,
    ) -> Self {
        self.log_manager = log_manager;
        self
    }

    /// Create a new AppState with a specific config file path,
    /// attempting to load persisted config from that path.
    pub fn with_config_path(config_path: &Path) -> Self {
        let (
            providers,
            active_provider_id,
            skills,
            acp_config,
            computer_use_config,
            web_search_config,
        ) = match Self::load_from_file_sync(config_path) {
            Ok(config) => {
                tracing::info!("Loaded config from {}", config_path.display());
                let providers = config
                    .providers
                    .into_iter()
                    .map(|(id, p)| {
                        let created_at = DateTime::parse_from_rfc3339(&p.created_at)
                            .map(|dt| dt.to_utc())
                            .unwrap_or(Utc::now());
                        (
                            id,
                            StoredProvider {
                                id: p.id,
                                name: p.name,
                                provider_type: p.provider_type,
                                config_json: p.config_json,
                                is_active: p.is_active,
                                created_at,
                            },
                        )
                    })
                    .collect();

                let skills = config
                    .skills
                    .into_iter()
                    .map(|(name, s)| {
                        (
                            name,
                            StoredSkill {
                                name: s.name,
                                description: s.description,
                                skill_type: s.skill_type,
                                config: s.config,
                                is_active: s.is_active,
                            },
                        )
                    })
                    .collect();

                (
                    providers,
                    config.active_provider_id,
                    skills,
                    config.acp_config,
                    config.computer_use_config,
                    config.web_search_config,
                )
            }
            Err(e) => {
                tracing::info!(
                    "Could not load config from {}: {}",
                    config_path.display(),
                    e
                );
                (
                    HashMap::new(),
                    None,
                    HashMap::new(),
                    AcpConfig::default(),
                    crate::computer_use::ComputerUseConfig::default(),
                    crate::types::WebSearchConfig::default(),
                )
            }
        };

        // Load chat history from the default chat history file path
        let chat_history_file_path = chat_history_path();
        let chat_history = match Self::load_chat_history_sync(&chat_history_file_path) {
            Ok(history) => {
                tracing::info!(
                    "Loaded chat history from {} ({} messages)",
                    chat_history_file_path.display(),
                    history.len()
                );
                history
            }
            Err(e) => {
                tracing::info!(
                    "Could not load chat history from {}: {}",
                    chat_history_file_path.display(),
                    e
                );
                Vec::new()
            }
        };

        // Create workspace manager with data directory
        let data_dir = crate::computer_use::workspace::default_data_dir();
        let workspace_manager =
            std::sync::Arc::new(crate::computer_use::WorkspaceManager::new(data_dir));

        Self {
            providers: RwLock::new(providers),
            active_provider_id: RwLock::new(active_provider_id),
            skills: RwLock::new(skills),
            acp_config: RwLock::new(acp_config),
            computer_use_config: RwLock::new(computer_use_config),
            web_search_config: std::sync::Arc::new(RwLock::new(web_search_config)),
            workspace_manager,
            tool_definitions: Vec::new(),
            chat_history: RwLock::new(chat_history),
            start_time: Utc::now(),
            config_path: config_path.to_path_buf(),
            chat_history_path: chat_history_file_path,
            log_manager: std::sync::Arc::new(crate::logging::LogManager::new(1000)), // Placeholder, will be replaced
        }
    }

    /// Get the config file path.
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    // ─── Persistence ──────────────────────────────────────────────

    /// Load a PersistedConfig from a file (sync, used during construction).
    fn load_from_file_sync(path: &Path) -> anyhow::Result<PersistedConfig> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;
        let config: PersistedConfig = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?;
        Ok(config)
    }

    /// Load a PersistedConfig from a file (async).
    pub async fn load_from_file(path: &Path) -> anyhow::Result<PersistedConfig> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;
        let config: PersistedConfig = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?;
        Ok(config)
    }

    /// Save the current state to a config file (async).
    pub async fn save_to_file(&self, path: &Path) -> anyhow::Result<()> {
        ensure_parent_dir(path).await?;

        let config = self.to_persisted_config().await;
        let content = serde_json::to_string_pretty(&config)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

        tokio::fs::write(path, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;

        tracing::debug!("Config saved to {}", path.display());
        Ok(())
    }

    /// Auto-save to the default config path.
    pub async fn auto_save(&self) {
        if let Err(e) = self.save_to_file(&self.config_path).await {
            tracing::warn!("Failed to auto-save config: {}", e);
        }
        if let Err(e) = self.save_chat_history().await {
            tracing::warn!("Failed to auto-save chat history: {}", e);
        }
    }

    // ─── Chat History Persistence ─────────────────────────────────

    /// Load chat history from a file (sync, used during construction).
    fn load_chat_history_sync(path: &Path) -> anyhow::Result<Vec<ChatMessage>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read chat history file: {}", e))?;
        let history: Vec<ChatMessage> = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse chat history file: {}", e))?;
        Ok(history)
    }

    /// Save the current chat history to the default chat history path.
    pub async fn save_chat_history(&self) -> anyhow::Result<()> {
        ensure_parent_dir(&self.chat_history_path).await?;

        let history = self.chat_history.read().await;
        let content = serde_json::to_string_pretty(&*history)
            .map_err(|e| anyhow::anyhow!("Failed to serialize chat history: {}", e))?;
        drop(history);

        tokio::fs::write(&self.chat_history_path, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write chat history file: {}", e))?;

        tracing::debug!("Chat history saved to {}", self.chat_history_path.display());
        Ok(())
    }

    /// Build a PersistedConfig from the current in-memory state.
    async fn to_persisted_config(&self) -> PersistedConfig {
        let providers = self.providers.read().await;
        let active_provider_id = self.active_provider_id.read().await;
        let skills = self.skills.read().await;
        let acp_config = self.acp_config.read().await;
        let computer_use_config = self.computer_use_config.read().await;
        let web_search_config = self.web_search_config.read().await;

        let persisted_providers: HashMap<String, PersistedProvider> = providers
            .iter()
            .map(|(id, p)| {
                (
                    id.clone(),
                    PersistedProvider {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        provider_type: p.provider_type.clone(),
                        config_json: p.config_json.clone(),
                        is_active: p.is_active,
                        created_at: p.created_at.to_rfc3339(),
                    },
                )
            })
            .collect();

        let persisted_skills: HashMap<String, PersistedSkill> = skills
            .iter()
            .map(|(name, s)| {
                (
                    name.clone(),
                    PersistedSkill {
                        name: s.name.clone(),
                        description: s.description.clone(),
                        skill_type: s.skill_type.clone(),
                        config: s.config.clone(),
                        is_active: s.is_active,
                    },
                )
            })
            .collect();

        PersistedConfig {
            providers: persisted_providers,
            active_provider_id: active_provider_id.clone(),
            skills: persisted_skills,
            acp_config: acp_config.clone(),
            computer_use_config: computer_use_config.clone(),
            web_search_config: web_search_config.clone(),
        }
    }

    // ─── Provider & Skill Building ────────────────────────────────

    /// Build a Provider instance from a stored provider configuration.
    pub fn build_provider(stored: &StoredProvider) -> Result<Box<dyn Provider>, String> {
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
            "lm_studio" => {
                let host = config["host"].as_str().unwrap_or("localhost").to_string();
                let port = config["port"].as_u64().unwrap_or(1234) as u16;
                let api_key = config["api_key"].as_str().map(|s| s.to_string());
                let default_model = config["default_model"]
                    .as_str()
                    .unwrap_or("local-model")
                    .to_string();

                let mut provider = crate::provider::lm_studio::LmStudioProvider::builder()
                    .host(host)
                    .port(port)
                    .default_model(default_model);

                if let Some(key) = api_key {
                    provider = provider.api_key(key);
                }

                Ok(Box::new(provider.build()))
            }
            other => Err(format!("Unknown provider type: {}", other)),
        }
    }

    /// Build skill instances from a list of stored skills, filtered by active status and optional names.
    pub fn build_skills(
        skills: &HashMap<String, StoredSkill>,
        filter_names: Option<&[String]>,
    ) -> Vec<Arc<dyn Skill>> {
        let mut result = Vec::new();
        for (_name, skill) in skills.iter() {
            if !skill.is_active {
                continue;
            }
            // If filter_names is provided, only include skills whose name is in the list
            if let Some(names) = filter_names
                && !names.contains(&skill.name)
            {
                continue;
            }
            match skill.skill_type.as_str() {
                "system_prompt" => {
                    let prompt = skill.config["prompt"].as_str().unwrap_or("").to_string();
                    result.push(Arc::new(SystemPromptSkill::new(prompt)) as Arc<dyn Skill>);
                }
                "memory" => {
                    let max = skill.config["max_messages"].as_u64().unwrap_or(50) as usize;
                    result.push(Arc::new(MemorySkill::new(max)) as Arc<dyn Skill>);
                }
                "context_prefix" => {
                    let prefix = skill.config["prefix"].as_str().unwrap_or("").to_string();
                    result.push(Arc::new(ContextPrefixSkill::new(prefix)) as Arc<dyn Skill>);
                }
                "skill" => {
                    // Custom skill package uploaded via upload_skill_package
                    // Create a GenericSkill that injects the skill content as system prompt
                    let name = skill.name.clone();
                    let description = skill.description.clone();
                    let content = skill.config["content"].as_str().unwrap_or("").to_string();

                    if content.is_empty() {
                        tracing::warn!(
                            skill_name = %skill.name,
                            "Skill has no content, skipping"
                        );
                    } else {
                        result.push(Arc::new(SystemPromptSkill::new(format!(
                            "# {}\n\n{}",
                            description, content
                        ))) as Arc<dyn Skill>);
                        tracing::info!(
                            skill_name = %name,
                            "Loaded generic skill as system prompt"
                        );
                    }
                }
                _ => {
                    tracing::warn!(
                        skill_name = %skill.name,
                        skill_type = %skill.skill_type,
                        "Unknown skill type, ignoring"
                    );
                }
            }
        }
        result
    }

    /// Build a fully configured Agent from the current state.
    pub async fn build_agent(&self) -> Result<Agent, String> {
        self.build_agent_with_context(None, None).await
    }

    /// Build a fully configured Agent with user context for computer use capabilities.
    pub async fn build_agent_with_context(
        &self,
        user_id: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<Agent, String> {
        let providers = self.providers.read().await;
        let active_id = self.active_provider_id.read().await;

        let active_id = active_id.as_ref().ok_or("No active provider configured")?;

        let stored = providers
            .get(active_id)
            .ok_or("Active provider not found")?;

        let provider = Self::build_provider(stored)?;
        drop(providers);

        let config = AgentConfig::new()
            .with_max_tool_rounds(10)
            .with_auto_execute_tools(true);

        let mut agent = Agent::with_config(provider, config);

        // Re-add skills
        let skills = self.skills.read().await;
        let skill_instances = Self::build_skills(&skills, None);
        let num_skills = skill_instances.len();
        drop(skills);
        for skill in skill_instances {
            tracing::info!("Adding skill: {}", skill.name());
            agent.add_skill(skill);
        }
        tracing::info!("Successfully added {} skills to agent", num_skills);

        // Check computer use configuration and register tools accordingly
        let computer_use_config = self.computer_use_config.read().await;
        let user_id = user_id.unwrap_or("default_user");
        let session_id = session_id.unwrap_or("default_session");

        match computer_use_config.runtime {
            crate::computer_use::ComputerUseRuntime::None => {
                // Computer use is disabled - register only basic tools
                tracing::info!("Computer use is disabled, registering basic tools only");
                agent.register_tool(Arc::new(crate::agent::builtin_tools::ReadFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::WriteFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::ListDirectoryTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::BashTool));
            }
            crate::computer_use::ComputerUseRuntime::Local => {
                // Computer use is enabled in local mode
                tracing::info!("Computer use enabled in local mode for user '{}'", user_id);

                // Create permission checker and workspace manager
                let data_dir = crate::computer_use::workspace::default_data_dir();
                let temp_dir = std::env::temp_dir();
                let permission_checker = Arc::new(crate::computer_use::PermissionChecker::new(
                    computer_use_config.clone(),
                    data_dir,
                    temp_dir,
                ));
                let workspace_manager = self.workspace_manager.clone();

                // Create tool context
                let tool_context = Arc::new(crate::computer_use::ComputerUseContext {
                    user_id: user_id.to_string(),
                    session_id: session_id.to_string(),
                    permission_checker,
                    workspace_manager,
                    working_dir: None,
                });

                // Check if user can use power tools
                let can_use_power_tools = computer_use_config.can_use_power_tools(user_id);

                // Register wrapped file tools with permission checking
                agent.register_tool(Arc::new(crate::computer_use::WrappedReadFileTool::new(
                    tool_context.clone(),
                )));
                agent.register_tool(Arc::new(crate::computer_use::WrappedWriteFileTool::new(
                    tool_context.clone(),
                )));
                agent.register_tool(Arc::new(
                    crate::computer_use::WrappedListDirectoryTool::new(tool_context.clone()),
                ));

                // Register Shell and Python tools only if user has permission
                if can_use_power_tools {
                    tracing::info!("User '{}' has permission to use power tools", user_id);
                    agent.register_tool(Arc::new(crate::computer_use::ShellTool::new(
                        tool_context.clone(),
                    )));
                    agent.register_tool(Arc::new(crate::computer_use::PythonTool::new(
                        tool_context.clone(),
                    )));
                } else {
                    tracing::info!(
                        "User '{}' does not have permission to use power tools",
                        user_id
                    );
                }

                // Register other basic tools
                agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
            }
            crate::computer_use::ComputerUseRuntime::Sandbox => {
                // Sandbox mode is not yet implemented
                tracing::warn!("Sandbox mode is not yet implemented, falling back to basic tools");
                agent.register_tool(Arc::new(crate::agent::builtin_tools::ReadFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::WriteFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::ListDirectoryTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
            }
        }

        // Register WebSearchTool only if properly configured (enabled and has API key if needed)
        let web_search_config = self.web_search_config.read().await;
        let web_search_available = web_search_config.enabled && {
            match web_search_config.search_engine {
                crate::types::SearchEngine::DuckDuckGo => {
                    // DuckDuckGo doesn't require an API key
                    true
                }
                _ => {
                    // Other engines require an API key
                    web_search_config.api_key.is_some()
                }
            }
        };
        drop(web_search_config);

        if web_search_available {
            agent.register_tool(Arc::new(crate::agent::builtin_tools::WebSearchTool::new(
                self.web_search_config.clone(),
            )));
            tracing::info!("WebSearchTool registered with current configuration");
        } else {
            tracing::info!(
                "WebSearchTool not registered: web search is disabled or not properly configured"
            );
        }

        // Restore chat history first
        let history = self.chat_history.read().await;
        let has_history = !history.is_empty();
        if has_history {
            agent.set_history(history.clone());
        }
        drop(history);

        // Initialize skills only if no history exists (new conversation)
        // For existing conversations, system messages from skills are already in history
        if !has_history {
            agent.initialize_skills().await;
        }

        Ok(agent)
    }
}

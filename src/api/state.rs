use crate::agent::runner::{Agent, AgentConfig};
use crate::agent::skill::{ContextPrefixSkill, MemorySkill, Skill, SystemPromptSkill};
use crate::platform::types::PlatformStatus;
use crate::provider::Provider;
use crate::types::ChatMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

fn default_true() -> bool {
    true
}

fn default_command_prefix() -> String {
    "/".to_string()
}

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

/// Serializable version of StoredPersona for config file persistence.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedPersona {
    /// The display name of the persona (e.g., "Assistant", "Coder", "Teacher")
    pub name: String,
    /// A short description of the persona's role.
    pub description: String,
    /// The full system prompt that defines the persona's behavior.
    pub prompt: String,
}

/// Config profile persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedConfigProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default = "default_true")]
    pub enable: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    pub web_search_enabled: bool,
    pub computer_use_enabled: bool,
    pub acp_enabled: bool,
    #[serde(default)]
    pub active_skill_names: Vec<String>,
    #[serde(default)]
    pub active_platform_ids: Vec<String>,
    #[serde(default)]
    pub proxy_config: crate::types::ProxyConfig,
    /// Built-in command prefix for this profile (default: "/").
    #[serde(default = "default_command_prefix")]
    pub command_prefix: String,
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
    #[serde(default)]
    pub personas: HashMap<String, PersistedPersona>,
    #[serde(default)]
    pub config_profiles: HashMap<String, PersistedConfigProfile>,
}

// ─── In-Memory State Types ───────────────────────────────────────

/// Information about a stored config profile.
#[derive(Debug, Clone)]
pub struct StoredConfigProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enable: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub provider_id: Option<String>,
    pub persona_id: Option<String>,
    pub web_search_enabled: bool,
    pub computer_use_enabled: bool,
    pub acp_enabled: bool,
    pub active_skill_names: Vec<String>,
    pub active_platform_ids: Vec<String>,
    pub proxy_config: crate::types::ProxyConfig,
    pub command_prefix: String,
}

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

/// Information about a stored persona.
#[derive(Debug, Clone)]
pub struct StoredPersona {
    /// Unique identifier for this persona.
    pub id: String,
    /// The display name of the persona.
    pub name: String,
    /// A short description of the persona's role.
    pub description: String,
    /// The full system prompt that defines the persona's behavior.
    pub prompt: String,
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
    /// All configured personas, keyed by ID.
    pub personas: RwLock<HashMap<String, StoredPersona>>,
    /// All configured config profiles, keyed by ID.
    pub config_profiles: RwLock<HashMap<String, StoredConfigProfile>>,
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
    /// ID of the current active conversation for chat messages.
    pub chat_conversation_id: RwLock<Option<String>>,
    /// Server start time.
    pub start_time: DateTime<Utc>,
    /// Path to the config file.
    pub(crate) config_path: PathBuf,

    /// Log manager for real-time log broadcasting.
    pub log_manager: std::sync::Arc<crate::logging::LogManager>,
    /// Shared database pool for the unified `ruri.db`.
    /// Initialized once in `main()` and shared across all sub-modules.
    pub db_pool: std::sync::Arc<tokio::sync::RwLock<Option<sqlx::SqlitePool>>>,
    /// Conversation database (initialized after AppState creation).
    pub conversation_db: std::sync::Arc<
        tokio::sync::RwLock<Option<std::sync::Arc<crate::conversation::ConversationDatabase>>>,
    >,
    /// MCP configuration manager (initialized after AppState creation).
    pub mcp_config:
        std::sync::Arc<tokio::sync::RwLock<Option<crate::mcp::config::McpConfigManager>>>,
    /// Platform instance configurations.
    pub platform_configs: RwLock<Vec<crate::platform::manager::PlatformInstanceConfig>>,
    /// Path to the platforms config file.
    pub(crate) platforms_config_path: PathBuf,
    /// Shared platform manager for runtime control of adapters.
    pub platform_manager: std::sync::Arc<tokio::sync::RwLock<crate::platform::PlatformManager>>,
    /// Command dispatcher for built-in commands.
    pub command_dispatcher: std::sync::Arc<tokio::sync::RwLock<crate::command::CommandDispatcher>>,
    /// Session variables for `/set` and `/unset` commands.
    /// Keyed by session_id, each value is a map of variable name -> value.
    pub session_variables: std::sync::Arc<
        tokio::sync::RwLock<
            std::collections::HashMap<String, std::collections::HashMap<String, String>>,
        >,
    >,
    /// Running agent tasks, keyed by session_id.
    /// Used by `/stop` to cancel in-progress tasks.
    pub running_agent_tasks: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
    >,
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
            personas,
            config_profiles,
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

                let personas = config
                    .personas
                    .into_iter()
                    .map(|(id, p)| {
                        (
                            id.clone(),
                            StoredPersona {
                                id,
                                name: p.name,
                                description: p.description,
                                prompt: p.prompt,
                            },
                        )
                    })
                    .collect();

                let config_profiles = config
                    .config_profiles
                    .into_iter()
                    .map(|(id, p)| {
                        let created_at = DateTime::parse_from_rfc3339(&p.created_at)
                            .map(|dt| dt.to_utc())
                            .unwrap_or_else(|_| Utc::now());
                        let updated_at = DateTime::parse_from_rfc3339(&p.updated_at)
                            .map(|dt| dt.to_utc())
                            .unwrap_or_else(|_| Utc::now());
                        (
                            id.clone(),
                            StoredConfigProfile {
                                id,
                                name: p.name,
                                description: p.description,
                                enable: p.enable,
                                is_active: p.is_active,
                                created_at,
                                updated_at,
                                provider_id: p.provider_id,
                                persona_id: p.persona_id,
                                web_search_enabled: p.web_search_enabled,
                                computer_use_enabled: p.computer_use_enabled,
                                acp_enabled: p.acp_enabled,
                                active_skill_names: p.active_skill_names,
                                active_platform_ids: p.active_platform_ids,
                                proxy_config: p.proxy_config,
                                command_prefix: p.command_prefix,
                            },
                        )
                    })
                    .collect();

                (
                    providers,
                    config.active_provider_id,
                    skills,
                    personas,
                    config_profiles,
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
                    HashMap::new(),
                    HashMap::new(),
                    AcpConfig::default(),
                    crate::computer_use::ComputerUseConfig::default(),
                    crate::types::WebSearchConfig::default(),
                )
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
            personas: RwLock::new(personas),
            config_profiles: RwLock::new(config_profiles),
            acp_config: RwLock::new(acp_config),
            computer_use_config: RwLock::new(computer_use_config),
            web_search_config: std::sync::Arc::new(RwLock::new(web_search_config)),
            workspace_manager,
            tool_definitions: Vec::new(),
            start_time: Utc::now(),
            config_path: config_path.to_path_buf(),
            log_manager: std::sync::Arc::new(crate::logging::LogManager::new(1000)), // Placeholder, will be replaced
            db_pool: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            conversation_db: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            chat_conversation_id: RwLock::new(None),
            mcp_config: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            platform_configs: RwLock::new(Vec::new()),
            platforms_config_path: ruri_config_dir().join("platforms.yaml"),
            platform_manager: std::sync::Arc::new(tokio::sync::RwLock::new(
                crate::platform::PlatformManager::new(),
            )),
            command_dispatcher: std::sync::Arc::new(tokio::sync::RwLock::new(
                crate::command::create_builtin_dispatcher(),
            )),
            session_variables: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            running_agent_tasks: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
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
    }

    // ─── Platform Config Persistence ────────────────────────────

    /// Load platform instance configurations from the platforms.yaml file.
    pub async fn load_platforms_config(&self) {
        let path = &self.platforms_config_path;
        if path.exists() {
            match tokio::fs::read_to_string(path).await {
                Ok(content) => {
                    match serde_yaml::from_str::<crate::platform::manager::PlatformConfigFile>(
                        &content,
                    ) {
                        Ok(file_config) => {
                            tracing::info!(
                                "Loaded {} platform config(s) from {}",
                                file_config.platforms.len(),
                                path.display()
                            );
                            *self.platform_configs.write().await = file_config.platforms;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse platforms config from {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to read platforms config from {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }
    }

    /// Save platform instance configurations to the platforms.yaml file.
    pub async fn save_platforms_config(&self) -> anyhow::Result<()> {
        let path = &self.platforms_config_path;
        ensure_parent_dir(path).await?;

        let configs = self.platform_configs.read().await;
        let file_config = crate::platform::manager::PlatformConfigFile {
            platforms: configs.clone(),
        };
        let content = serde_yaml::to_string(&file_config)
            .map_err(|e| anyhow::anyhow!("Failed to serialize platforms config: {}", e))?;

        tokio::fs::write(path, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write platforms config file: {}", e))?;

        tracing::debug!("Platforms config saved to {}", path.display());
        Ok(())
    }

    /// Synchronize running platform adapters with the currently active config profile.
    ///
    /// - Stops adapters that are running but **not** in the active profile's
    ///   `active_platform_ids`.
    /// - Starts adapters that are in `active_platform_ids` but not yet running.
    ///
    /// This is the single source of truth for which adapters should be alive
    /// at any given time — used both at startup and during hot-reload.
    pub async fn sync_platforms_with_active_profile(&self) {
        let (active_platform_ids, proxy_config): (Vec<String>, Option<crate::types::ProxyConfig>) = {
            let profiles = self.config_profiles.read().await;
            match profiles.values().find(|p| p.is_active && p.enable) {
                Some(p) => (
                    p.active_platform_ids.clone(),
                    if p.proxy_config.is_configured() {
                        Some(p.proxy_config.clone())
                    } else {
                        None
                    },
                ),
                None => (Vec::new(), None),
            }
        };

        let active_set: std::collections::HashSet<&str> =
            active_platform_ids.iter().map(|s| s.as_str()).collect();

        let configs = self.platform_configs.read().await;
        let mut pm = self.platform_manager.write().await;

        // Stop adapters that are running but not in the active profile
        // (or the active profile is disabled / missing, so stop all)
        let running_ids: Vec<String> = pm.statuses().iter().map(|(id, _)| id.clone()).collect();
        for running_id in &running_ids {
            if !active_set.contains(running_id.as_str()) {
                tracing::info!(platform_id = %running_id, "Stopping platform (not in active profile)");
                if let Err(e) = pm.remove_platform(running_id).await {
                    tracing::error!(platform_id = %running_id, error = %e, "Failed to stop platform");
                }
            }
        }

        // For platforms already running, check if proxy config changed and restart them
        let still_running: Vec<String> = pm.statuses().iter().map(|(id, _)| id.clone()).collect();
        for config in configs.iter() {
            if !still_running.contains(&config.id) {
                continue;
            }
            if !active_set.contains(config.id.as_str()) {
                continue;
            }

            // Build updated config with current proxy settings
            let mut config_with_proxy = config.clone();
            if let Some(ref proxy) = proxy_config {
                let platform_host = match config.platform_type.as_str() {
                    "discord" => "discord.gg",
                    "dingtalk" => "dingtalk.com",
                    other => other,
                };

                if proxy.should_proxy(platform_host) {
                    if let Some(obj) = config_with_proxy.extra.as_object_mut() {
                        obj.insert(
                            "proxy_url".to_string(),
                            serde_json::Value::String(proxy.url.clone()),
                        );
                    }
                } else if let Some(obj) = config_with_proxy.extra.as_object_mut() {
                    obj.remove("proxy_url");
                }
            } else if let Some(obj) = config_with_proxy.extra.as_object_mut() {
                obj.remove("proxy_url");
            }

            // Check if proxy_url changed compared to current config
            let old_proxy = config
                .extra
                .get("proxy_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let new_proxy = config_with_proxy
                .extra
                .get("proxy_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if old_proxy != new_proxy {
                tracing::info!(
                    platform_id = %config.id,
                    old_proxy = %old_proxy,
                    new_proxy = %new_proxy,
                    "Proxy config changed, restarting platform (hot-reload)"
                );
                if let Err(e) = pm.restart_platform(config_with_proxy).await {
                    tracing::error!(platform_id = %config.id, error = %e, "Failed to restart platform");
                }
            }
        }

        // Start adapters that are in the active profile but not yet running
        for config in configs.iter() {
            if !active_set.contains(config.id.as_str()) {
                continue;
            }
            if pm.is_running(&config.id) {
                continue;
            }

            let mut config_with_proxy = config.clone();

            // Inject proxy_url from the active profile into the platform config.
            // In "rules" mode, only inject for platforms whose domains match proxy_domains.
            // We determine the platform's host from its type (Discord → discord.gg, DingTalk → dingtalk.com).
            if let Some(ref proxy) = proxy_config {
                let platform_host = match config.platform_type.as_str() {
                    "discord" => "discord.gg",
                    "dingtalk" => "dingtalk.com",
                    other => other,
                };

                if proxy.should_proxy(platform_host) {
                    if let Some(obj) = config_with_proxy.extra.as_object_mut() {
                        // Set proxy_url — always override from profile config
                        obj.insert(
                            "proxy_url".to_string(),
                            serde_json::Value::String(proxy.url.clone()),
                        );
                    }
                    tracing::info!(
                        platform_id = %config.id,
                        proxy_mode = %proxy.mode,
                        "Injecting proxy for platform"
                    );
                } else {
                    // Ensure no stale proxy_url in config
                    if let Some(obj) = config_with_proxy.extra.as_object_mut() {
                        obj.remove("proxy_url");
                    }
                }
            } else {
                // No proxy configured, ensure no stale proxy_url
                if let Some(obj) = config_with_proxy.extra.as_object_mut() {
                    obj.remove("proxy_url");
                }
            }

            tracing::info!(platform_id = %config.id, "Starting platform (from active profile)");
            if let Err(e) = pm.add_platform(config_with_proxy).await {
                tracing::error!(platform_id = %config.id, error = %e, "Failed to start platform");
            }
        }
    }

    /// Returns a map of platform_id -> status string.
    /// Queries live adapter status from the PlatformManager when possible.
    pub async fn platform_statuses_async(&self) -> HashMap<String, String> {
        let pm = self.platform_manager.read().await;
        let live_statuses: HashMap<String, PlatformStatus> = pm.statuses().into_iter().collect();
        drop(pm);

        let configs = self.platform_configs.read().await;
        configs
            .iter()
            .map(|c| {
                let status = if let Some(ps) = live_statuses.get(&c.id) {
                    match ps {
                        PlatformStatus::Pending => "pending",
                        PlatformStatus::Running => "running",
                        PlatformStatus::Error => "error",
                        PlatformStatus::Stopped => "stopped",
                    }
                } else {
                    "stopped"
                };
                (c.id.clone(), status.to_string())
            })
            .collect()
    }

    /// Ensure there is an active conversation for chat messages.
    /// Returns the conversation ID, creating a new one if necessary.
    pub async fn ensure_chat_conversation(&self) -> anyhow::Result<String> {
        // Check if we already have an active conversation ID
        {
            let conv_id = self.chat_conversation_id.read().await;
            if let Some(id) = conv_id.as_ref() {
                return Ok(id.clone());
            }
        }

        // No active conversation, need to create one
        let conv_db = self.conversation_db.read().await;
        let db = conv_db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Conversation database not initialized"))?;

        // Create or get a default conversation
        let conversation = db
            .get_or_create_conversation(
                "webui".to_string(),
                crate::conversation::models::ChatType::Private,
                "default".to_string(),
            )
            .await?;

        // Save the conversation ID
        let mut conv_id = self.chat_conversation_id.write().await;
        *conv_id = Some(conversation.id.clone());

        tracing::info!(
            "Created/loaded default conversation for chat: {}",
            conversation.id
        );

        Ok(conversation.id)
    }

    /// Build a PersistedConfig from the current in-memory state.
    async fn to_persisted_config(&self) -> PersistedConfig {
        let providers = self.providers.read().await;
        let active_provider_id = self.active_provider_id.read().await;
        let skills = self.skills.read().await;
        let personas = self.personas.read().await;
        let config_profiles = self.config_profiles.read().await;
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

        let persisted_personas: HashMap<String, PersistedPersona> = personas
            .iter()
            .map(|(id, p)| {
                (
                    id.clone(),
                    PersistedPersona {
                        name: p.name.clone(),
                        description: p.description.clone(),
                        prompt: p.prompt.clone(),
                    },
                )
            })
            .collect();

        let persisted_config_profiles: HashMap<String, PersistedConfigProfile> = config_profiles
            .iter()
            .map(|(id, p)| {
                (
                    id.clone(),
                    PersistedConfigProfile {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        description: p.description.clone(),
                        enable: p.enable,
                        is_active: p.is_active,
                        created_at: p.created_at.to_rfc3339().to_string(),
                        updated_at: p.updated_at.to_rfc3339().to_string(),
                        provider_id: p.provider_id.clone(),
                        persona_id: p.persona_id.clone(),
                        web_search_enabled: p.web_search_enabled,
                        computer_use_enabled: p.computer_use_enabled,
                        acp_enabled: p.acp_enabled,
                        active_skill_names: p.active_skill_names.clone(),
                        active_platform_ids: p.active_platform_ids.clone(),
                        proxy_config: p.proxy_config.clone(),
                        command_prefix: p.command_prefix.clone(),
                    },
                )
            })
            .collect();

        PersistedConfig {
            providers: persisted_providers,
            active_provider_id: active_provider_id.clone(),
            skills: persisted_skills,
            personas: persisted_personas,
            config_profiles: persisted_config_profiles,
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
        self.build_agent_with_context(None, None, None).await
    }

    /// Build a fully configured Agent with user context for computer use capabilities.
    pub async fn build_agent_with_context(
        &self,
        user_id: Option<&str>,
        session_id: Option<&str>,
        persona_id: Option<&str>,
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

        // Inject persona system prompt if configured
        {
            // First, try to get persona_id from the request
            let resolved_persona_id = if let Some(pid) = persona_id {
                Some(pid.to_string())
            } else {
                // Fall back to the active config profile's persona_id
                let profiles = self.config_profiles.read().await;
                profiles
                    .values()
                    .find(|p| p.is_active && p.enable)
                    .and_then(|p| p.persona_id.clone())
            };

            let personas = self.personas.read().await;
            let persona_to_use = if let Some(pid) = &resolved_persona_id {
                personas.get(pid)
            } else {
                None
            };

            if let Some(p) = persona_to_use {
                if !p.prompt.is_empty() {
                    tracing::info!(
                        persona_id = %p.id,
                        persona_name = %p.name,
                        "Injecting persona system prompt"
                    );
                    agent.add_skill(Arc::new(SystemPromptSkill::new(&p.prompt)));
                }
            } else if let Some(pid) = resolved_persona_id {
                tracing::warn!(
                    persona_id = %pid,
                    "Requested persona not found"
                );
            }
            drop(personas);
        }

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
        let web_search_available = web_search_config.enabled
            && match web_search_config.search_engine {
                crate::types::SearchEngine::DuckDuckGo => true,
                _ => web_search_config.api_key.is_some(),
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

        // Load chat history from database if conversation exists
        let conversation_id = self.ensure_chat_conversation().await.ok();
        if let Some(ref conv_id) = conversation_id {
            let conv_db = self.conversation_db.read().await;
            if let Some(db) = conv_db.as_ref() {
                if let Ok(messages) = db.get_conversation_messages(conv_id).await {
                    if !messages.is_empty() {
                        let chat_messages: Vec<ChatMessage> = messages
                            .iter()
                            .map(|m| {
                                let role = match m.role.as_str() {
                                    "assistant" => crate::types::MessageRole::Assistant,
                                    "system" => crate::types::MessageRole::System,
                                    _ => crate::types::MessageRole::User,
                                };
                                ChatMessage {
                                    role,
                                    content: Some(crate::types::MessageContent::Text(
                                        m.content.clone(),
                                    )),
                                    name: None,
                                    tool_calls: None,
                                    tool_call_id: None,
                                }
                            })
                            .collect();
                        agent.set_history(chat_messages);
                    } else {
                        agent.initialize_skills().await;
                    }
                } else {
                    agent.initialize_skills().await;
                }
            } else {
                agent.initialize_skills().await;
            }
            drop(conv_db);
        } else {
            agent.initialize_skills().await;
        }

        Ok(agent)
    }
}

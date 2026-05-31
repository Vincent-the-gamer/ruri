use crate::agent::runner::{Agent, AgentConfig};
use crate::agent::skill::{
    ContextPrefixSkill, MemorySkill, Skill, SkillPackageSkill, SystemPromptSkill,
};
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
use tracing::{debug, info, warn};

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

/// Serializable persona for config file persistence.
/// Used in `PersistedConfig.personas` for the persona library.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedPersona {
    /// The display name of the persona (e.g., "Assistant", "Coder", "Teacher")
    pub name: String,
    /// A short description of the persona's role.
    pub description: String,
    /// The full system prompt that defines the persona's behavior.
    pub prompt: String,
    /// Optional guidance for how to narrate tool results in a persona-consistent style.
    /// When set, this is appended to the system prompt to guide the model on how to
    /// present tool execution results in character. When not set, a sensible default
    /// is used to ensure the persona's emotional value carries through to tool responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_response_style: Option<String>,
}

/// Persona data used internally to construct the agent's system prompt.
/// Resolved from the persona library by `persona_id` at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedPersona {
    /// The display name of the persona
    pub name: String,
    /// A short description of the persona's role.
    pub description: String,
    /// The full system prompt that defines the persona's behavior.
    pub prompt: String,
    /// Optional guidance for how to narrate tool results in a persona-consistent style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_response_style: Option<String>,
}

/// Embedded provider configuration that belongs to a specific Config Profile.
/// This is independent from global providers - each profile has its own copy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedProvider {
    /// Unique identifier for this provider
    pub id: String,
    /// Display name
    pub name: String,
    /// Provider type (e.g., "openai", "anthropic", "gemini")
    pub provider_type: String,
    /// Provider configuration as JSON
    pub config_json: serde_json::Value,
}

/// Embedded skill configuration that belongs to a specific Config Profile.
/// This is independent from global skills - each profile has its own copy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedSkill {
    /// Skill name
    pub name: String,
    /// Skill description
    pub description: String,
    /// Skill type (e.g., "system_prompt", "memory", "context_prefix", "skill")
    pub skill_type: String,
    /// Skill configuration as JSON
    pub config: serde_json::Value,
}

fn default_segmented_reply_interval_ms() -> u64 {
    500
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
    /// Legacy field: references a global provider by ID.
    /// New profiles should use `embedded_providers` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    /// Persona ID reference to the persona library.
    /// When set, the persona is loaded dynamically from the library at runtime.
    /// This allows hot-reloading: changes to the persona in the library take effect immediately.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    /// Embedded provider configurations - independent copies for this profile.
    /// Takes priority over provider_id if both are set.
    #[serde(default)]
    pub embedded_providers: Vec<EmbeddedProvider>,
    /// Active embedded provider name within this profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_embedded_provider: Option<String>,
    /// Embedded skill configurations - independent copies for this profile.
    #[serde(default)]
    pub embedded_skills: Vec<EmbeddedSkill>,
    /// Active embedded skill names within this profile.
    #[serde(default)]
    pub active_embedded_skill_names: Vec<String>,
    pub web_search_enabled: bool,
    pub computer_use_enabled: bool,
    /// Whether the LLM's extended thinking (chain-of-thought reasoning) is enabled.
    /// When disabled, the model will not spend extra tokens on internal reasoning.
    #[serde(default = "default_true")]
    pub thinking_enabled: bool,
    #[serde(default)]
    pub active_skill_names: Vec<String>,
    #[serde(default)]
    pub active_knowledge_base_ids: Vec<String>,
    #[serde(default)]
    pub proxy_config: crate::types::ProxyConfig,
    /// Built-in command prefix for this profile (default: "/").
    #[serde(default = "default_command_prefix")]
    pub command_prefix: String,
    /// List of enabled built-in command names. Commands not in this list are disabled for this profile.
    /// Default: empty (all commands disabled).
    #[serde(default)]
    pub enabled_commands: Vec<String>,
    /// Per-command admin requirement overrides for this profile.
    /// Key: command name, Value: true = admin required, false = open to all.
    #[serde(default)]
    pub command_admin_required: HashMap<String, bool>,
    /// Custom error message to show users when a tool call or API request fails.
    /// If not set, the raw error message is returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_error_message: Option<String>,
    /// Platform instance IDs that this profile is associated with.
    /// A platform instance can only belong to one config profile at a time.
    #[serde(default)]
    pub platform_ids: Vec<String>,
    /// Whether segmented (multi-message) reply is enabled for this profile.
    /// When enabled, LLM responses are split into multiple messages with delays.
    #[serde(default)]
    pub segmented_reply_enabled: bool,
    /// Interval in milliseconds between segmented reply messages.
    #[serde(default = "default_segmented_reply_interval_ms")]
    pub segmented_reply_interval_ms: u64,
}

/// ACP-specific configuration stored alongside the main config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcpConfig {
    /// The provider ID to use in ACP mode. If None, falls back to the API-mode active provider.
    pub active_provider_id: Option<String>,
    /// Skill names to enable in ACP mode.
    pub active_skill_names: Vec<String>,
    /// Knowledge base IDs to enable in ACP mode.
    #[serde(default)]
    pub active_knowledge_base_ids: Vec<String>,
    /// Proxy configuration for ACP mode (independent of config profile proxy).
    #[serde(default)]
    pub proxy_config: crate::types::ProxyConfig,
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
    /// Persona library — reusable persona templates that can be selected
    /// and embedded (copied) into config profiles or debug sessions.
    /// This is NOT a "global active persona" — just a library of templates.
    #[serde(default)]
    pub personas: HashMap<String, PersistedPersona>,
    #[serde(default)]
    pub config_profiles: HashMap<String, PersistedConfigProfile>,

    /// Global shell command blacklist — any command containing one of these
    /// substrings (case-insensitive) will be blocked regardless of admin status.
    #[serde(default = "default_shell_blacklist")]
    pub shell_command_blacklist: Vec<String>,
}

fn default_shell_blacklist() -> Vec<String> {
    vec![
        // ── Linux / macOS ──
        "sudo ".to_string(),
        "rm -rf".to_string(),
        "dd if=".to_string(),
        "mkfs.".to_string(),
        ":(){ :|:& };:".to_string(),
        "chmod 777".to_string(),
        "chown -R".to_string(),
        "> /dev/sda".to_string(),
        "mv /* ".to_string(),
        "| sh".to_string(),
        "| bash".to_string(),
        "fdisk".to_string(),
        "parted".to_string(),
        "shutdown".to_string(),
        "reboot".to_string(),
        "halt".to_string(),
        "poweroff".to_string(),
        "init 0".to_string(),
        "init 6".to_string(),
        "kill -9".to_string(),
        "pkill".to_string(),
        "killall".to_string(),
        "iptables -F".to_string(),
        "ufw disable".to_string(),
        "systemctl disable".to_string(),
        "modprobe -r".to_string(),
        "rmmod".to_string(),
        "diskutil eraseDisk".to_string(),
        "diskutil unmount".to_string(),
        "hdiutil".to_string(),
        "launchctl unload".to_string(),
        "csrutil disable".to_string(),
        "fdesetup".to_string(),
        "softwareupdate".to_string(),
        // ── Windows ──
        "format ".to_string(),
        "del /f /s".to_string(),
        "rmdir /s".to_string(),
        "diskpart".to_string(),
        "reg delete".to_string(),
        "reg add".to_string(),
        "bcdedit".to_string(),
        "icacls ".to_string(),
        "takeown".to_string(),
        "cipher /w".to_string(),
        "sc delete".to_string(),
        "sc stop".to_string(),
        "net stop".to_string(),
        "Remove-Item -Force -Recurse".to_string(),
        "Set-ExecutionPolicy".to_string(),
        "Stop-Process -Force".to_string(),
        "Clear-RecycleBin".to_string(),
        "Disable-WindowsOptionalFeature".to_string(),
        "Reset-ComputerMachinePassword".to_string(),
    ]
}

// ─── Debug Session Config (WebUI chat debug settings) ────────────────

/// Debug session configuration - persisted independently for WebUI chat debugging.
/// This is completely separate from Config Profiles.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebugSessionConfig {
    /// Whether segmented (multi-message) reply is enabled for this debugsession.
    #[serde(default)]
    pub segmented_reply_enabled: bool,
    /// Interval in milliseconds between segmented reply messages.
    #[serde(default = "default_segmented_reply_interval_ms")]
    pub segmented_reply_interval_ms: u64,
    /// Embedded provider configurations for debug sessions
    #[serde(default)]
    pub providers: Vec<EmbeddedProvider>,
    /// Active embedded provider name for debug sessions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_provider: Option<String>,
    /// Legacy: Provider ID override for debug sessions (references global provider)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    /// Persona ID reference to the persona library.
    /// When set, the persona is loaded dynamically from the library at runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    /// Whether web search is enabled for this debug session.
    #[serde(default)]
    pub web_search_enabled: bool,
    /// Whether computer use is enabled for this debug session.
    #[serde(default)]
    pub computer_use_enabled: bool,
    /// Whether the LLM's extended thinking (chain-of-thought reasoning) is enabled.
    /// When disabled, the model will not spend extra tokens on internal reasoning.
    #[serde(default = "default_true")]
    pub thinking_enabled: bool,
    /// Embedded skill configurations for debug sessions
    #[serde(default)]
    pub skills: Vec<EmbeddedSkill>,
    /// Active embedded skill names for debug sessions
    #[serde(default)]
    pub active_skill_names: Vec<String>,
    /// Active knowledge base IDs
    #[serde(default)]
    pub knowledge_base_ids: Vec<String>,
    /// Proxy configuration for this debug session.
    #[serde(default)]
    pub proxy_config: crate::types::ProxyConfig,
    /// Built-in command prefix for debug session (default: "/").
    #[serde(default = "default_command_prefix")]
    pub command_prefix: String,
    /// List of enabled built-in command names for debug session.
    /// Empty means all commands enabled.
    #[serde(default)]
    pub enabled_commands: Vec<String>,
    /// Per-command admin requirement overrides for this debug session.
    #[serde(default)]
    pub command_admin_required: HashMap<String, bool>,
    /// Temperature override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Max tokens override
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Custom error message
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_error_message: Option<String>,
}

// ─── Internal Helper Types ───────────────────────────────────────

/// Resolved configuration context for building an agent.
/// This bundles all the embedded configuration needed from a single source
/// (Debug Session, Config Profile, or fallback to global).
#[derive(Debug, Clone)]
struct ResolvedConfigContext {
    /// Source identifier for logging (e.g., "debug_session", "profile_xxx")
    source: String,
    /// Embedded provider configurations
    embedded_providers: Vec<EmbeddedProvider>,
    /// Name or ID of the active embedded provider
    active_embedded_provider: Option<String>,
    /// Legacy provider_id from the profile, used as fallback when no embedded providers.
    /// References a global provider by ID.
    provider_id: Option<String>,
    /// Persona ID reference to the persona library.
    /// When set, the persona is loaded dynamically from the library at runtime.
    persona_id: Option<String>,
    /// Embedded skill configurations
    embedded_skills: Vec<EmbeddedSkill>,
    /// Active embedded skill names
    active_embedded_skill_names: Vec<String>,
    /// Active global skill names (references skills stored in `self.skills`)
    active_skill_names: Vec<String>,
    /// Knowledge base IDs to attach
    knowledge_base_ids: Vec<String>,
    /// Proxy configuration (profile-scoped, no global proxy)
    proxy_config: crate::types::ProxyConfig,
    /// Whether web search is enabled for this profile/session
    web_search_enabled: bool,
    /// Whether computer use is enabled for this profile/session
    computer_use_enabled: bool,
    /// Whether the LLM's extended thinking (chain-of-thought reasoning) is enabled.
    thinking_enabled: bool,
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
    pub segmented_reply_enabled: bool,
    pub segmented_reply_interval_ms: u64,
    /// Legacy field: references a global provider by ID
    pub provider_id: Option<String>,
    /// Persona ID reference to the persona library.
    /// When set, the persona is loaded dynamically from the library at runtime.
    pub persona_id: Option<String>,
    /// Embedded providers - independent copies for this profile
    pub embedded_providers: Vec<EmbeddedProvider>,
    /// Active embedded provider name within this profile
    pub active_embedded_provider: Option<String>,
    /// Embedded skills - independent copies for this profile
    pub embedded_skills: Vec<EmbeddedSkill>,
    /// Active embedded skill names within this profile
    pub active_embedded_skill_names: Vec<String>,
    pub web_search_enabled: bool,
    pub computer_use_enabled: bool,
    pub thinking_enabled: bool,
    pub active_skill_names: Vec<String>,
    pub active_knowledge_base_ids: Vec<String>,
    pub proxy_config: crate::types::ProxyConfig,
    pub command_prefix: String,
    pub enabled_commands: Vec<String>,
    pub command_admin_required: HashMap<String, bool>,
    pub custom_error_message: Option<String>,
    /// Platform instance IDs that this profile is associated with.
    /// A platform instance can only belong to one config profile at a time.
    pub platform_ids: Vec<String>,
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
    /// Whether this skill is stored as a folder (ZIP-extracted) on disk.
    /// Folder-based skills are not auto-saved as .md files.
    pub is_folder: bool,
}

/// Information about a stored persona template in the library.
/// Personas in the library are NOT "global" or "active" — they are
/// just reusable templates that the user can pick and embed into
/// config profiles or debug sessions.
#[derive(Debug, Clone)]
pub struct StoredPersona {
    /// Unique identifier for this persona template.
    pub id: String,
    /// The display name of the persona (e.g., "Assistant", "Coder", "Teacher").
    pub name: String,
    /// A short description of the persona's role.
    pub description: String,
    /// The full system prompt that defines the persona's behavior.
    pub prompt: String,
    /// Optional guidance for how to narrate tool results in a persona-consistent style.
    pub tool_response_style: Option<String>,
}

// ─── Config File Path ────────────────────────────────────────────

/// Returns the Ruri config directory: `~/.ruri/`
///
/// Uses the `dirs` crate for cross-platform home directory resolution:
/// - **Linux/macOS**: `$HOME/.ruri/`
/// - **Windows**: `C:\Users\<user>\.ruri\`
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

/// Returns the skills directory path: `~/.ruri/skills/`
pub fn skills_dir() -> PathBuf {
    ruri_config_dir().join("skills")
}

/// Returns the file path for a specific skill: `~/.ruri/skills/{name}.md`
pub fn skill_file_path(name: &str) -> PathBuf {
    skills_dir().join(format!("{}.md", name))
}

/// Returns the folder path for a specific skill package: `~/.ruri/skills/{name}/`
pub fn skill_folder_path(name: &str) -> PathBuf {
    skills_dir().join(name)
}

/// Ensures the parent directory of the given path exists.
async fn ensure_parent_dir(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    Ok(())
}

/// Ensures the skills directory exists.
async fn ensure_skills_dir() -> std::io::Result<()> {
    tokio::fs::create_dir_all(skills_dir()).await
}

// ─── Skill File Serialization ────────────────────────────────────

/// YAML frontmatter structure for a skill `.md` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFileFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(default = "default_skill_type")]
    pub skill_type: String,
    #[serde(default)]
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_model_invocation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_invocable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
}

fn default_skill_type() -> String {
    "skill".to_string()
}

/// Convert a `StoredSkill` to a `.md` file content (YAML frontmatter + markdown body).
fn stored_skill_to_md(skill: &StoredSkill) -> String {
    let content = skill
        .config
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract optional fields from config JSON
    let when_to_use = skill
        .config
        .get("when_to_use")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let argument_hint = skill
        .config
        .get("argument_hint")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let arguments = skill
        .config
        .get("arguments")
        .and_then(|v| serde_json::from_value::<serde_yaml::Value>(v.clone()).ok());
    let disable_model_invocation = skill
        .config
        .get("disable_model_invocation")
        .and_then(|v| v.as_bool());
    let user_invocable = skill.config.get("user_invocable").and_then(|v| v.as_bool());
    let allowed_tools = skill
        .config
        .get("allowed_tools")
        .and_then(|v| serde_json::from_value::<serde_yaml::Value>(v.clone()).ok());
    let model = skill
        .config
        .get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let effort = skill
        .config
        .get("effort")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let context = skill
        .config
        .get("context")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let agent = skill
        .config
        .get("agent")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let hooks = skill
        .config
        .get("hooks")
        .and_then(|v| serde_json::from_value::<serde_yaml::Value>(v.clone()).ok());
    let paths = skill
        .config
        .get("paths")
        .and_then(|v| serde_json::from_value::<serde_yaml::Value>(v.clone()).ok());
    let shell = skill
        .config
        .get("shell")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let fm = SkillFileFrontmatter {
        name: skill.name.clone(),
        description: skill.description.clone(),
        skill_type: skill.skill_type.clone(),
        is_active: skill.is_active,
        when_to_use,
        argument_hint,
        arguments,
        disable_model_invocation,
        user_invocable,
        allowed_tools,
        model,
        effort,
        context,
        agent,
        hooks,
        paths,
        shell,
    };

    let yaml_str = serde_yaml::to_string(&fm).unwrap_or_default();
    format!("---\n{}---\n{}", yaml_str, content)
}

/// Parse a `.md` skill file back into a `StoredSkill`.
fn md_to_stored_skill(md_content: &str) -> Result<StoredSkill, String> {
    // Normalize line endings (CRLF -> LF) and strip UTF-8 BOM.
    // Windows editors (e.g. Notepad) add a BOM and use CRLF, which breaks
    // the frontmatter parser that expects "---" at the very start.
    let content = md_content
        .strip_prefix('\u{feff}')
        .unwrap_or(md_content)
        .replace("\r\n", "\n");

    if !content.starts_with("---") {
        return Err("Missing opening frontmatter marker".to_string());
    }

    let content_without_opening = &content[3..];
    let closing_pos = content_without_opening
        .find("\n---")
        .or_else(|| content_without_opening.find("---"))
        .ok_or("Missing closing frontmatter marker")?;

    let frontmatter_str = &content_without_opening[..closing_pos];
    let markdown_body = content_without_opening[closing_pos..]
        .trim_start_matches("\n---")
        .trim_start_matches("---")
        .trim()
        .to_string();

    let fm: SkillFileFrontmatter = serde_yaml::from_str(frontmatter_str)
        .map_err(|e| format!("Failed to parse frontmatter: {}", e))?;

    // Build the config JSON from frontmatter fields + content
    let mut config = serde_json::Map::new();

    if let Some(ref v) = fm.when_to_use {
        config.insert(
            "when_to_use".to_string(),
            serde_json::Value::String(v.clone()),
        );
    }
    if let Some(ref v) = fm.argument_hint {
        config.insert(
            "argument_hint".to_string(),
            serde_json::Value::String(v.clone()),
        );
    }
    if let Some(ref v) = fm.arguments {
        config.insert(
            "arguments".to_string(),
            serde_json::to_value(v).map_err(|e| e.to_string())?,
        );
    }
    if let Some(v) = fm.disable_model_invocation {
        config.insert(
            "disable_model_invocation".to_string(),
            serde_json::Value::Bool(v),
        );
    }
    if let Some(v) = fm.user_invocable {
        config.insert("user_invocable".to_string(), serde_json::Value::Bool(v));
    }
    if let Some(ref v) = fm.allowed_tools {
        config.insert(
            "allowed_tools".to_string(),
            serde_json::to_value(v).map_err(|e| e.to_string())?,
        );
    }
    if let Some(ref v) = fm.model {
        config.insert("model".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(ref v) = fm.effort {
        config.insert("effort".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(ref v) = fm.context {
        config.insert("context".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(ref v) = fm.agent {
        config.insert("agent".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(ref v) = fm.hooks {
        config.insert(
            "hooks".to_string(),
            serde_json::to_value(v).map_err(|e| e.to_string())?,
        );
    }
    if let Some(ref v) = fm.paths {
        config.insert(
            "paths".to_string(),
            serde_json::to_value(v).map_err(|e| e.to_string())?,
        );
    }
    if let Some(ref v) = fm.shell {
        config.insert("shell".to_string(), serde_json::Value::String(v.clone()));
    }

    // Add the markdown body as "content"
    config.insert(
        "content".to_string(),
        serde_json::Value::String(markdown_body),
    );

    Ok(StoredSkill {
        name: fm.name,
        description: fm.description,
        skill_type: fm.skill_type,
        config: serde_json::Value::Object(config),
        is_active: fm.is_active,
        is_folder: false, // Default; caller may override for folder-based skills
    })
}

/// Load all skills from `~/.ruri/skills/` into a HashMap.
/// Supports both flat `.md` files and folder-based skills (directory with `SKILL.md`).
/// This is the canonical way to load skills from disk — used by both
/// AppState and the ACP ProviderFactory.
pub fn load_skills_from_disk() -> HashMap<String, StoredSkill> {
    let mut skills = HashMap::new();
    let dir = skills_dir();
    if !dir.exists() {
        return skills;
    }

    // Delegate to the shared sync reader which handles both .md files and folders
    AppState::read_skills_from_dir_sync(&dir, &mut skills);

    if !skills.is_empty() {
        info!("Loaded {} skills from {}", skills.len(), dir.display());
    }
    skills
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
    /// Persona library — reusable persona templates that can be selected
    /// and embedded (copied) into config profiles or debug sessions.
    /// These are NOT "global" or "active" personas; they are just a
    /// convenient library of templates for the user to pick from.
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
    /// Active conversation IDs for chat messages, keyed by context identifier.
    /// Each config profile / debug session gets its own isolated conversation.
    /// Key format: `"debug_session"` or `"profile_{id}"`.
    pub chat_conversation_ids: RwLock<std::collections::HashMap<String, String>>,
    /// Server start time.
    pub start_time: DateTime<Utc>,

    /// Metrics collector for network monitoring and token tracking.
    pub metrics: std::sync::Arc<tokio::sync::RwLock<crate::metrics::MetricsCollector>>,
    /// Broadcast sender for notifying WebSocket clients of metrics updates.
    pub metrics_update_tx: tokio::sync::broadcast::Sender<()>,
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
    /// Knowledge base service (initialized after AppState creation).
    pub knowledge_base_service:
        std::sync::Arc<tokio::sync::RwLock<Option<crate::knowledge::KnowledgeBaseService>>>,
    /// Global shell command blacklist — any command containing one of these
    /// substrings (case-insensitive) will be blocked regardless of admin status.
    pub shell_command_blacklist: RwLock<Vec<String>>,
    /// Debug session configuration (WebUI chat debug settings).
    /// Persisted separately from config profiles.
    pub debug_session: RwLock<DebugSessionConfig>,
    /// Shutdown trigger for the main HTTP server.
    /// Sending `true` via the sender initiates graceful shutdown.
    pub server_shutdown_tx: tokio::sync::watch::Sender<bool>,
    /// Shutdown signal receiver (clone of the watch receiver).
    /// Used by `axum::serve(...).with_graceful_shutdown()`.
    pub server_shutdown_rx: tokio::sync::watch::Receiver<bool>,
    /// Flag set to true when a system restart has been requested.
    /// `main()` checks this after graceful shutdown to re-spawn the process.
    pub restart_requested: std::sync::atomic::AtomicBool,
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
            mut computer_use_config,
            web_search_config,
            shell_command_blacklist,
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

                let mut skills: HashMap<String, StoredSkill> = config
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
                                is_folder: false,
                            },
                        )
                    })
                    .collect();

                // Try to load skills from ~/.ruri/skills/*.md files.
                // Filesystem takes priority over config.json.
                // If filesystem has no skills, config.json skills are kept
                // and will be migrated on first auto_save.
                let disk_count = Self::index_skills_from_disk_sync(&mut skills);
                if disk_count > 0 {
                    info!(
                        "Loaded {} skills from filesystem, overriding config.json",
                        disk_count
                    );
                } else if !skills.is_empty() {
                    info!(
                        "{} skills in config.json will be migrated to filesystem on next save",
                        skills.len()
                    );
                }

                // Load persona library (reusable templates).
                let old_personas: HashMap<String, PersistedPersona> = config.personas;
                let personas: HashMap<String, StoredPersona> = old_personas
                    .iter()
                    .map(|(id, p)| {
                        (
                            id.clone(),
                            StoredPersona {
                                id: id.clone(),
                                name: p.name.clone(),
                                description: p.description.clone(),
                                prompt: p.prompt.clone(),
                                tool_response_style: p.tool_response_style.clone(),
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
                                embedded_providers: p.embedded_providers,
                                active_embedded_provider: p.active_embedded_provider,
                                embedded_skills: p.embedded_skills,
                                active_embedded_skill_names: p.active_embedded_skill_names,
                                web_search_enabled: p.web_search_enabled,
                                computer_use_enabled: p.computer_use_enabled,
                                thinking_enabled: p.thinking_enabled,
                                active_skill_names: p.active_skill_names,
                                active_knowledge_base_ids: p.active_knowledge_base_ids,
                                proxy_config: p.proxy_config,
                                command_prefix: p.command_prefix,
                                enabled_commands: p.enabled_commands,
                                command_admin_required: p.command_admin_required,
                                custom_error_message: p.custom_error_message,
                                platform_ids: p.platform_ids.clone(),
                                segmented_reply_enabled: p.segmented_reply_enabled,
                                segmented_reply_interval_ms: p.segmented_reply_interval_ms,
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
                    config.shell_command_blacklist,
                )
            }
            Err(e) => {
                tracing::info!(
                    "Could not load config from {}: {}",
                    config_path.display(),
                    e
                );

                // Even if config.json is missing or corrupt, try to load
                // skills from ~/.ruri/skills/ so they appear in the WebUI.
                let mut skills = HashMap::new();
                let disk_count = Self::index_skills_from_disk_sync(&mut skills);
                if disk_count > 0 {
                    info!(
                        "Loaded {} skills from filesystem (config.json unavailable)",
                        disk_count
                    );
                }

                (
                    HashMap::new(),
                    None,
                    skills,
                    HashMap::new(),
                    HashMap::new(),
                    AcpConfig::default(),
                    crate::computer_use::ComputerUseConfig::default(),
                    crate::types::WebSearchConfig::default(),
                    default_shell_blacklist(),
                )
            }
        };

        // Create workspace manager with data directory
        let data_dir = crate::computer_use::workspace::default_data_dir();
        let workspace_manager =
            std::sync::Arc::new(crate::computer_use::WorkspaceManager::new(data_dir));

        // Initialize command dispatcher from all active profiles
        let mut command_dispatcher = crate::command::create_builtin_dispatcher();
        {
            let active_profiles: Vec<_> = config_profiles
                .values()
                .filter(|p| p.is_active && p.enable)
                .collect();
            if !active_profiles.is_empty() {
                // Merge: use union of enabled commands from all active profiles
                let mut merged_enabled_commands: Vec<String> = Vec::new();
                // Use the first active profile's prefix as the effective prefix
                let effective_prefix = active_profiles[0].command_prefix.clone();

                for profile in &active_profiles {
                    for cmd in &profile.enabled_commands {
                        if !merged_enabled_commands.contains(cmd) {
                            merged_enabled_commands.push(cmd.clone());
                        }
                    }
                }

                command_dispatcher.set_prefix(effective_prefix);
                command_dispatcher.set_enabled_commands(merged_enabled_commands);
            }
        }
        // Sync command_admin_required from all active profiles to ComputerUseConfig
        {
            let active_profiles: Vec<_> = config_profiles
                .values()
                .filter(|p| p.is_active && p.enable)
                .collect();
            if !active_profiles.is_empty() {
                let mut merged_command_admin_required: std::collections::HashMap<String, bool> =
                    std::collections::HashMap::new();

                for profile in &active_profiles {
                    for (cmd, admin_req) in &profile.command_admin_required {
                        merged_command_admin_required.insert(cmd.clone(), *admin_req);
                    }
                }

                computer_use_config.command_admin_required = merged_command_admin_required;
            }
        }

        // Create shutdown channel for graceful server shutdown
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        // Create metrics broadcast channel for real-time WebSocket updates
        let (metrics_update_tx, _) = tokio::sync::broadcast::channel::<()>(256);
        let mut metrics_collector = crate::metrics::MetricsCollector::new();
        metrics_collector.set_update_channel(metrics_update_tx.clone());

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
            metrics: std::sync::Arc::new(tokio::sync::RwLock::new(metrics_collector)),
            metrics_update_tx,
            config_path: config_path.to_path_buf(),
            log_manager: std::sync::Arc::new(crate::logging::LogManager::new(1000)), // Placeholder, will be replaced
            db_pool: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            conversation_db: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            chat_conversation_ids: RwLock::new(std::collections::HashMap::new()),
            mcp_config: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            platform_configs: RwLock::new(Vec::new()),
            platforms_config_path: ruri_config_dir().join("platforms.yaml"),
            platform_manager: std::sync::Arc::new(tokio::sync::RwLock::new(
                crate::platform::PlatformManager::new(),
            )),
            command_dispatcher: std::sync::Arc::new(tokio::sync::RwLock::new(command_dispatcher)),
            session_variables: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            running_agent_tasks: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            knowledge_base_service: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            shell_command_blacklist: RwLock::new(shell_command_blacklist),
            debug_session: RwLock::new(DebugSessionConfig::default()),
            server_shutdown_tx: shutdown_tx,
            server_shutdown_rx: shutdown_rx,
            restart_requested: std::sync::atomic::AtomicBool::new(false),
        }
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

    /// Auto-save to the default config path and save skills to disk.
    pub async fn auto_save(&self) {
        if let Err(e) = self.save_to_file(&self.config_path).await {
            tracing::warn!("Failed to auto-save config: {}", e);
        }
        // Also persist all skills as individual .md files in ~/.ruri/skills/
        if let Err(e) = self.save_all_skills_to_disk().await {
            tracing::warn!("Failed to save skills to disk: {}", e);
        }
    }

    /// Save all skills as individual `.md` files in `~/.ruri/skills/`.
    /// Folder-based skills (from ZIP packages) are skipped — they already
    /// have their files on disk in their own directory.
    /// Also cleans up stale .md files for skills that no longer exist in memory.
    /// Save all in-memory skills to `~/.ruri/skills/`.
    /// Every skill is a folder containing at least a `SKILL.md`.
    /// Folder-based skills already have their files on disk; this only
    /// updates the `SKILL.md` inside each folder to reflect the current
    /// in-memory state (e.g. `is_active` toggles).
    /// It does NOT delete any files or folders from disk.
    pub async fn save_all_skills_to_disk(&self) -> anyhow::Result<()> {
        ensure_skills_dir().await?;
        let skills = self.skills.read().await;

        // Save all in-memory skills to disk
        for (name, skill) in skills.iter() {
            let md_content = stored_skill_to_md(skill);

            if skill.is_folder {
                // Folder-based skill: update SKILL.md inside its folder
                let folder = skill_folder_path(name);
                // Ensure the folder exists (it may have been deleted externally)
                if !folder.exists() {
                    tokio::fs::create_dir_all(&folder).await.map_err(|e| {
                        anyhow::anyhow!("Failed to create skill folder {}: {}", folder.display(), e)
                    })?;
                }
                let skill_md = folder.join("SKILL.md");
                tokio::fs::write(&skill_md, &md_content)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to write skill file {}: {}", skill_md.display(), e)
                    })?;
                debug!("Updated folder skill '{}' at {}", name, skill_md.display());
            } else {
                // Legacy flat-file skill: migrate to folder-based storage
                // Create a folder and write SKILL.md inside it
                let folder = skill_folder_path(name);
                if !folder.exists() {
                    tokio::fs::create_dir_all(&folder).await.map_err(|e| {
                        anyhow::anyhow!("Failed to create skill folder {}: {}", folder.display(), e)
                    })?;
                }
                let skill_md = folder.join("SKILL.md");
                tokio::fs::write(&skill_md, &md_content)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to write skill file {}: {}", skill_md.display(), e)
                    })?;
                // Also remove the old flat .md file if it exists
                let old_path = skill_file_path(name);
                if old_path.exists() {
                    tokio::fs::remove_file(&old_path).await.map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to remove legacy skill file {}: {}",
                            old_path.display(),
                            e
                        )
                    })?;
                    debug!("Migrated legacy skill '{}' from flat file to folder", name);
                }
                info!("Saved skill '{}' to {}", name, skill_md.display());
            }
        }

        info!(
            "Saved {} skills to {}",
            skills.len(),
            skills_dir().display()
        );
        Ok(())
    }

    /// Save a single skill to `~/.ruri/skills/`.
    /// For folder-based skills, updates the `SKILL.md` inside the folder.
    /// For flat-file skills, writes a `.md` file.
    pub async fn save_skill_to_disk(&self, name: &str) -> anyhow::Result<()> {
        ensure_skills_dir().await?;
        let skills = self.skills.read().await;
        let skill = skills
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", name))?;
        let md_content = stored_skill_to_md(skill);

        if skill.is_folder {
            // Folder-based skill: create or update SKILL.md inside the folder
            let folder = skill_folder_path(name);
            if !folder.exists() {
                tokio::fs::create_dir_all(&folder).await.map_err(|e| {
                    anyhow::anyhow!("Failed to create skill folder {}: {}", folder.display(), e)
                })?;
            }
            let skill_md = folder.join("SKILL.md");
            tokio::fs::write(&skill_md, &md_content)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Failed to write skill file {}: {}", skill_md.display(), e)
                })?;
            info!("Saved folder skill '{}' to {}", name, skill_md.display());
        } else {
            let path = skill_file_path(name);
            tokio::fs::write(&path, &md_content).await.map_err(|e| {
                anyhow::anyhow!("Failed to write skill file {}: {}", path.display(), e)
            })?;
            info!("Saved skill '{}' to {}", name, path.display());
        }
        Ok(())
    }

    /// Delete a skill's files from `~/.ruri/skills/`.
    /// Handles both flat .md files and folder-based skills (ZIP-extracted).
    pub async fn delete_skill_from_disk(&self, name: &str) -> anyhow::Result<()> {
        // Try deleting flat .md file first (legacy format)
        let md_path = skill_file_path(name);
        if md_path.exists() {
            tokio::fs::remove_file(&md_path).await.map_err(|e| {
                anyhow::anyhow!("Failed to delete skill file {}: {}", md_path.display(), e)
            })?;
            info!("Deleted skill file {}", md_path.display());
        }

        // Try deleting folder-based skill directory
        let folder_path = skill_folder_path(name);
        if folder_path.exists() && folder_path.is_dir() {
            tokio::fs::remove_dir_all(&folder_path).await.map_err(|e| {
                anyhow::anyhow!(
                    "Failed to delete skill folder {}: {}",
                    folder_path.display(),
                    e
                )
            })?;
            info!("Deleted skill folder {}", folder_path.display());
        }
        Ok(())
    }

    /// Index all skills from `~/.ruri/skills/` into the in-memory store.
    /// Only adds skills that don't already exist in memory, preserving
    /// any in-memory changes (e.g. `is_active` toggles, config edits).
    /// Returns the number of newly added skills.
    pub async fn index_skills_from_disk(&self) -> anyhow::Result<usize> {
        let dir = skills_dir();
        if !dir.exists() {
            debug!("Skills directory does not exist: {}", dir.display());
            return Ok(0);
        }

        let mut skills = self.skills.write().await;
        // Read disk skills into a temporary map, then merge — only insert
        // skills that don't already exist in the in-memory store.
        let mut disk_skills: HashMap<String, StoredSkill> = HashMap::new();
        let disk_count = Self::read_skills_from_dir(&dir, &mut disk_skills).await?;
        let mut added = 0;
        for (name, skill) in disk_skills {
            if !skills.contains_key(&name) {
                debug!("Refreshed skill '{}' from disk", name);
                skills.insert(name, skill);
                added += 1;
            }
        }
        if added > 0 {
            info!("Added {} new skills from disk", added);
        }
        Ok(disk_count)
    }

    /// Synchronous version, for use during construction.
    fn index_skills_from_disk_sync(skills_map: &mut HashMap<String, StoredSkill>) -> usize {
        let dir = skills_dir();
        if !dir.exists() {
            return 0;
        }
        Self::read_skills_from_dir_sync(&dir, skills_map)
    }

    /// Read all .md skill files AND skill directories from a directory into a skills map (async).
    async fn read_skills_from_dir(
        dir: &Path,
        skills: &mut HashMap<String, StoredSkill>,
    ) -> anyhow::Result<usize> {
        let mut count = 0;
        let mut read_dir = tokio::fs::read_dir(dir).await.map_err(|e| {
            anyhow::anyhow!("Failed to read skills directory {}: {}", dir.display(), e)
        })?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read skills dir entry: {}", e))?
        {
            let path = entry.path();

            // Support flat .md files (legacy)
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                match tokio::fs::read_to_string(&path).await {
                    Ok(content) => match md_to_stored_skill(&content) {
                        Ok(skill) => {
                            let name = skill.name.clone();
                            debug!("Indexed skill '{}' from {}", name, path.display());
                            skills.insert(name, skill);
                            count += 1;
                        }
                        Err(e) => {
                            warn!("Failed to parse skill file {}: {}", path.display(), e);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to read skill file {}: {}", path.display(), e);
                    }
                }
            }
            // Support folder-based skills (directory containing SKILL.md)
            else if path.is_dir() {
                let skill_md_path = path.join("SKILL.md");
                if skill_md_path.exists() {
                    match tokio::fs::read_to_string(&skill_md_path).await {
                        Ok(content) => match md_to_stored_skill(&content) {
                            Ok(mut skill) => {
                                skill.is_folder = true;
                                let name = skill.name.clone();
                                debug!("Indexed skill '{}' from folder {}", name, path.display());
                                skills.insert(name, skill);
                                count += 1;
                            }
                            Err(e) => {
                                warn!("Failed to parse SKILL.md in {}: {}", path.display(), e);
                            }
                        },
                        Err(e) => {
                            warn!("Failed to read SKILL.md in {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        if count > 0 {
            info!("Indexed {} skills from {}", count, dir.display());
        }
        Ok(count)
    }

    /// Read all .md skill files AND skill directories from a directory into a skills map (sync).
    fn read_skills_from_dir_sync(dir: &Path, skills: &mut HashMap<String, StoredSkill>) -> usize {
        let mut count = 0;
        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(e) => {
                warn!("Failed to read skills directory {}: {}", dir.display(), e);
                return 0;
            }
        };

        for entry in read_dir.flatten() {
            let path = entry.path();

            // Support flat .md files (legacy)
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match md_to_stored_skill(&content) {
                        Ok(skill) => {
                            let name = skill.name.clone();
                            debug!("Indexed skill '{}' from {}", name, path.display());
                            skills.insert(name, skill);
                            count += 1;
                        }
                        Err(e) => {
                            warn!("Failed to parse skill file {}: {}", path.display(), e);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to read skill file {}: {}", path.display(), e);
                    }
                }
            }
            // Support folder-based skills (directory containing SKILL.md)
            else if path.is_dir() {
                let skill_md_path = path.join("SKILL.md");
                if skill_md_path.exists() {
                    match std::fs::read_to_string(&skill_md_path) {
                        Ok(content) => match md_to_stored_skill(&content) {
                            Ok(mut skill) => {
                                skill.is_folder = true;
                                let name = skill.name.clone();
                                debug!("Indexed skill '{}' from folder {}", name, path.display());
                                skills.insert(name, skill);
                                count += 1;
                            }
                            Err(e) => {
                                warn!("Failed to parse SKILL.md in {}: {}", path.display(), e);
                            }
                        },
                        Err(e) => {
                            warn!("Failed to read SKILL.md in {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        if count > 0 {
            info!("Indexed {} skills from {}", count, dir.display());
        }
        count
    }

    /// Reload the config file from disk into in-memory state.
    /// Used by the file watcher to hot-reload changes to config.json.
    pub async fn reload_config_from_file(&self) -> anyhow::Result<()> {
        let config = Self::load_from_file_sync(&self.config_path)?;

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

        // Skills are now loaded from ~/.ruri/skills/*.md files (see index_skills_from_disk below)
        // Load persona library (reusable templates).
        let old_personas: HashMap<String, PersistedPersona> = config.personas.clone();
        let personas: HashMap<String, StoredPersona> = old_personas
            .iter()
            .map(|(id, p)| {
                (
                    id.clone(),
                    StoredPersona {
                        id: id.clone(),
                        name: p.name.clone(),
                        description: p.description.clone(),
                        prompt: p.prompt.clone(),
                        tool_response_style: p.tool_response_style.clone(),
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
                        embedded_providers: p.embedded_providers,
                        active_embedded_provider: p.active_embedded_provider,
                        embedded_skills: p.embedded_skills,
                        active_embedded_skill_names: p.active_embedded_skill_names,
                        web_search_enabled: p.web_search_enabled,
                        computer_use_enabled: p.computer_use_enabled,
                        thinking_enabled: p.thinking_enabled,
                        active_skill_names: p.active_skill_names,
                        active_knowledge_base_ids: p.active_knowledge_base_ids,
                        proxy_config: p.proxy_config,
                        command_prefix: p.command_prefix,
                        enabled_commands: p.enabled_commands,
                        command_admin_required: p.command_admin_required,
                        custom_error_message: p.custom_error_message,
                        platform_ids: p.platform_ids.clone(),
                        segmented_reply_enabled: p.segmented_reply_enabled,
                        segmented_reply_interval_ms: p.segmented_reply_interval_ms,
                    },
                )
            })
            .collect();

        // Update all in-memory state
        {
            let mut guard = self.providers.write().await;
            *guard = providers;
        }
        {
            let mut guard = self.active_provider_id.write().await;
            *guard = config.active_provider_id;
        }
        // Skills are loaded from filesystem, not config.json
        {
            let mut guard = self.personas.write().await;
            *guard = personas;
        }
        {
            let mut guard = self.config_profiles.write().await;
            *guard = config_profiles;
        }
        {
            let mut guard = self.acp_config.write().await;
            *guard = config.acp_config;
        }
        {
            let mut guard = self.computer_use_config.write().await;
            *guard = config.computer_use_config;
        }
        {
            let mut guard = self.web_search_config.write().await;
            *guard = config.web_search_config;
        }
        {
            let mut guard = self.shell_command_blacklist.write().await;
            *guard = config.shell_command_blacklist;
        }

        // Reload skills from the filesystem (.ruri/skills/*.md)
        // Clear existing skills first so stale entries (deleted from disk) are removed
        {
            let mut guard = self.skills.write().await;
            guard.clear();
        }
        self.index_skills_from_disk().await?;

        Ok(())
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

    // ─── Debug Session Persistence ────────────────────────────

    /// Path to the debug session config file.
    fn debug_session_path(&self) -> PathBuf {
        self.config_path.with_file_name("debug_session.json")
    }

    /// Load debug session config from file.
    pub async fn load_debug_session(&self) -> DebugSessionConfig {
        let path = self.debug_session_path();
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => match serde_json::from_str::<DebugSessionConfig>(&content) {
                Ok(config) => {
                    tracing::debug!("Loaded debug session config from {}", path.display());
                    config
                }
                Err(e) => {
                    tracing::warn!("Failed to parse debug session config: {}", e);
                    DebugSessionConfig::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => DebugSessionConfig::default(),
            Err(e) => {
                tracing::warn!("Failed to read debug session config: {}", e);
                DebugSessionConfig::default()
            }
        }
    }

    /// Save debug session config to file.
    pub async fn save_debug_session(&self) {
        let config = self.debug_session.read().await;
        let path = self.debug_session_path();
        ensure_parent_dir(&path).await.ok();

        let content = match serde_json::to_string_pretty(&*config) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to serialize debug session config: {}", e);
                return;
            }
        };

        if let Err(e) = tokio::fs::write(&path, content).await {
            tracing::error!("Failed to write debug session config: {}", e);
        } else {
            tracing::debug!("Debug session config saved to {}", path.display());
        }
    }

    /// Synchronize running platform adapters with their `enable` state.
    ///
    /// This is the single source of truth for which adapters should be alive
    /// at any given time — used both at startup and during hot-reload.
    /// Each platform's `enable` field determines whether it should be running.
    /// Proxy config is still read from the active profile.
    pub async fn sync_platforms(&self) {
        let (proxy_config, active_platform_ids): (
            Option<crate::types::ProxyConfig>,
            std::collections::HashSet<String>,
        ) = {
            let profiles = self.config_profiles.read().await;
            // Merge proxy config from all active profiles - use the first configured one
            let proxy = profiles
                .values()
                .filter(|p| p.is_active && p.enable)
                .find_map(|p| {
                    if p.proxy_config.is_configured() {
                        Some(p.proxy_config.clone())
                    } else {
                        None
                    }
                });
            // Collect platform IDs that should be running based on active profiles
            let active_ids: std::collections::HashSet<String> = profiles
                .values()
                .filter(|p| p.is_active && p.enable)
                .flat_map(|p| p.platform_ids.iter().cloned())
                .collect();
            (proxy, active_ids)
        };

        let configs = self.platform_configs.read().await;
        let mut pm = self.platform_manager.write().await;

        // Stop adapters that are running but disabled or not in any active profile's platform_ids
        let running_ids: Vec<String> = pm.statuses().iter().map(|(id, _)| id.clone()).collect();
        for running_id in &running_ids {
            let is_enabled = configs
                .iter()
                .find(|c| c.id == *running_id)
                .map(|c| c.enable)
                .unwrap_or(false);
            let in_active_profile = active_platform_ids.contains(running_id);
            if !is_enabled || !in_active_profile {
                tracing::info!(platform_id = %running_id, "Stopping platform (disabled or not in active profile)");
                if let Err(e) = pm.remove_platform(running_id).await {
                    tracing::error!(platform_id = %running_id, error = %e, "Failed to stop platform");
                }
            }
        }

        // For platforms already running, check if proxy config changed and restart them
        let still_running: Vec<String> = pm.statuses().iter().map(|(id, _)| id.clone()).collect();
        for config in configs.iter() {
            if !config.enable {
                continue;
            }
            if !still_running.contains(&config.id) {
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

        // Start adapters that are enabled but not yet running
        for config in configs.iter() {
            if !config.enable {
                continue;
            }
            if pm.is_running(&config.id) {
                continue;
            }
            // Only start platforms that belong to an active profile
            if !active_platform_ids.contains(&config.id) {
                continue;
            }

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
                    tracing::info!(
                        platform_id = %config.id,
                        proxy_mode = %proxy.mode,
                        "Injecting proxy for platform"
                    );
                } else {
                    if let Some(obj) = config_with_proxy.extra.as_object_mut() {
                        obj.remove("proxy_url");
                    }
                }
            } else {
                if let Some(obj) = config_with_proxy.extra.as_object_mut() {
                    obj.remove("proxy_url");
                }
            }

            tracing::info!(platform_id = %config.id, "Starting enabled platform");
            if let Err(e) = pm.add_platform(config_with_proxy).await {
                tracing::error!(platform_id = %config.id, error = %e, "Failed to start platform");
            }
        }

        // Persist any updated config extras that adapters returned after QR login.
        let config_updates = pm.drain_config_updates();
        drop(pm);
        if !config_updates.is_empty() {
            let mut configs = self.platform_configs.write().await;
            let mut updated_ids = Vec::new();
            for (instance_id, updated_extra) in &config_updates {
                if let Some(cfg) = configs.iter_mut().find(|c| c.id == *instance_id) {
                    cfg.extra = updated_extra.clone();
                    updated_ids.push(instance_id.clone());
                }
            }
            drop(configs);
            if let Err(e) = self.save_platforms_config().await {
                tracing::warn!("Failed to persist updated platform configs: {}", e);
            } else {
                let mut pm = self.platform_manager.write().await;
                for id in updated_ids {
                    if let Some(adapter) = pm.get_mut_adapter(&id) {
                        adapter.mark_config_persisted();
                    }
                }
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

    /// Check all running adapters for updated credentials and persist them.
    ///
    /// This should be called after events that may indicate a credential
    /// change (e.g. a `StatusChanged` event after re-login). It reads
    /// each adapter's `persist_config_hint()` and merges the result into
    /// `platform_configs` before saving to the YAML file.
    ///
    /// Safety guard: If an updated extra would clear a credential that
    /// currently exists in the config (e.g. overwriting a WeChat token
    /// with null), the update is skipped and a warning is logged. This
    /// prevents accidental credential loss during session timeouts or
    /// adapter restarts.
    pub async fn persist_adapter_credentials(&self) {
        // Phase 1: Collect hints (read lock on manager)
        let updates: Vec<(String, serde_json::Value)> = {
            let pm = self.platform_manager.read().await;
            pm.adapters()
                .iter()
                .filter_map(|(id, adapter)| {
                    adapter
                        .persist_config_hint()
                        .map(|extra| (id.clone(), extra))
                })
                .collect()
        };

        if updates.is_empty() {
            return;
        }

        // Phase 2: Update platform_configs and save (no manager lock needed)
        //
        // Defensive check: before replacing a platform's extra config, verify
        // that the update would not clear an existing credential. If the old
        // config has a non-empty token/account_id but the new one would set
        // them to null/empty, skip the update and log a warning.
        let updated_ids: Vec<String> = {
            let mut configs = self.platform_configs.write().await;
            let mut ids = Vec::new();
            for (instance_id, updated_extra) in &updates {
                if let Some(cfg) = configs.iter_mut().find(|c| c.id == *instance_id) {
                    // Defensive: check that we're not about to clear existing credentials
                    let old_has_token = cfg
                        .extra
                        .get("token")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| !s.is_empty());
                    let new_has_token = updated_extra
                        .get("token")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| !s.is_empty());
                    let old_has_account = cfg
                        .extra
                        .get("account_id")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| !s.is_empty());
                    let new_has_account = updated_extra
                        .get("account_id")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| !s.is_empty());

                    if (old_has_token && !new_has_token) || (old_has_account && !new_has_account) {
                        tracing::warn!(
                            platform_id = %instance_id,
                            "Refusing to persist config update that would clear existing credentials (old_token={}, new_token={}, old_account={}, new_account={}). Skipping.",
                            old_has_token, new_has_token, old_has_account, new_has_account
                        );
                        continue;
                    }

                    cfg.extra = updated_extra.clone();
                    ids.push(instance_id.clone());
                    tracing::info!(
                        platform_id = %instance_id,
                        "Persisted updated credentials for platform"
                    );
                }
            }
            ids
        };
        if let Err(e) = self.save_platforms_config().await {
            tracing::warn!("Failed to persist updated platform configs: {}", e);
            // Don't mark as persisted if the save failed
            return;
        }

        // Phase 3: Clear dirty flags on adapters (write lock on manager)
        {
            let mut pm = self.platform_manager.write().await;
            for id in updated_ids {
                if let Some(adapter) = pm.get_mut_adapter(&id) {
                    adapter.mark_config_persisted();
                }
            }
        }
    }

    /// Resolve the context key used to isolate chat conversations per profile.
    /// Returns `"debug_session"` or `"profile_{id}"` depending on which
    /// configuration is active for the WebUI chat.
    pub async fn resolve_chat_context_key(&self) -> String {
        // WebUI chat always uses the debug session
        "debug_session".to_string()
    }

    /// Resolve the context key for a specific profile_id or debug session.
    pub async fn resolve_chat_context_key_for(
        &self,
        use_debug_session: bool,
        profile_id: Option<&str>,
    ) -> String {
        if use_debug_session {
            return "debug_session".to_string();
        }

        if let Some(pid) = profile_id {
            return format!("profile_{}", pid);
        }

        // Fall back to active profile
        let profiles = self.config_profiles.read().await;
        if let Some(profile) = profiles.values().find(|p| p.is_active && p.enable) {
            return format!("profile_{}", profile.id);
        }

        // No active profile — use a generic key
        "default".to_string()
    }

    /// Ensure there is an active conversation for chat messages, isolated
    /// per configuration profile. Returns the conversation ID, creating a
    /// new one if necessary.
    ///
    /// The `context_key` determines conversation isolation: each unique key
    /// gets its own conversation. Use `resolve_chat_context_key()` or
    /// `resolve_chat_context_key_for()` to obtain the key.
    pub async fn ensure_chat_conversation_for(&self, context_key: &str) -> anyhow::Result<String> {
        // Check if we already have an active conversation ID for this context
        {
            let conv_ids = self.chat_conversation_ids.read().await;
            if let Some(id) = conv_ids.get(context_key) {
                return Ok(id.clone());
            }
        }

        // No active conversation for this context, need to create one
        let conv_db = self.conversation_db.read().await;
        let db = conv_db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Conversation database not initialized"))?;

        // Use the context_key as chat_id so each profile gets its own conversation.
        // bot_name is always "webui" for WebUI chat.
        let conversation = db
            .get_or_create_conversation(
                "webui".to_string(),
                crate::conversation::models::ChatType::Private,
                context_key.to_string(),
            )
            .await?;

        // Save the conversation ID for this context
        let mut conv_ids = self.chat_conversation_ids.write().await;
        conv_ids.insert(context_key.to_string(), conversation.id.clone());

        tracing::info!(
            context_key = %context_key,
            conversation_id = %conversation.id,
            "Created/loaded conversation for chat context"
        );

        Ok(conversation.id)
    }

    /// Convenience wrapper that resolves the current WebUI chat context
    /// and ensures a conversation exists.
    pub async fn ensure_chat_conversation(&self) -> anyhow::Result<String> {
        let context_key = self.resolve_chat_context_key().await;
        self.ensure_chat_conversation_for(&context_key).await
    }

    /// Build a PersistedConfig from the current in-memory state.
    /// Note: Skills are NOT persisted to config.json anymore.
    /// They are stored as individual .md files in ~/.ruri/skills/.
    async fn to_persisted_config(&self) -> PersistedConfig {
        let providers = self.providers.read().await;
        let active_provider_id = self.active_provider_id.read().await;
        let personas = self.personas.read().await;
        let config_profiles = self.config_profiles.read().await;
        let acp_config = self.acp_config.read().await;
        let computer_use_config = self.computer_use_config.read().await;
        let web_search_config = self.web_search_config.read().await;
        let shell_command_blacklist = self.shell_command_blacklist.read().await;

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

        // Skills are now stored as files in ~/.ruri/skills/, not in config.json.
        // Keep an empty map for backward compatibility.
        let persisted_skills: HashMap<String, PersistedSkill> = HashMap::new();

        let persisted_personas: HashMap<String, PersistedPersona> = personas
            .iter()
            .map(|(id, p)| {
                (
                    id.clone(),
                    PersistedPersona {
                        name: p.name.clone(),
                        description: p.description.clone(),
                        prompt: p.prompt.clone(),
                        tool_response_style: p.tool_response_style.clone(),
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
                        embedded_providers: p.embedded_providers.clone(),
                        active_embedded_provider: p.active_embedded_provider.clone(),
                        embedded_skills: p.embedded_skills.clone(),
                        active_embedded_skill_names: p.active_embedded_skill_names.clone(),
                        web_search_enabled: p.web_search_enabled,
                        computer_use_enabled: p.computer_use_enabled,
                        thinking_enabled: p.thinking_enabled,
                        active_skill_names: p.active_skill_names.clone(),
                        active_knowledge_base_ids: p.active_knowledge_base_ids.clone(),
                        proxy_config: p.proxy_config.clone(),
                        command_prefix: p.command_prefix.clone(),
                        enabled_commands: p.enabled_commands.clone(),
                        command_admin_required: p.command_admin_required.clone(),
                        custom_error_message: p.custom_error_message.clone(),
                        platform_ids: p.platform_ids.clone(),
                        segmented_reply_enabled: p.segmented_reply_enabled,
                        segmented_reply_interval_ms: p.segmented_reply_interval_ms,
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
            shell_command_blacklist: shell_command_blacklist.clone(),
        }
    }

    // ─── Provider & Skill Building ────────────────────────────────

    /// Build a Provider instance from a stored provider configuration.
    pub fn build_provider(stored: &StoredProvider) -> Result<Box<dyn Provider>, String> {
        let config = &stored.config_json;

        match stored.provider_type.as_str() {
            "openai" | "siliconflow" | "deepseek" => {
                let base_url = config["base_url"].as_str().unwrap_or("").to_string();
                let api_key = config["api_key"].as_str().map(|s| s.to_string());
                let default_model = config["default_model"]
                    .as_str()
                    .unwrap_or("gpt-4o")
                    .to_string();
                let supports_multimodal = config
                    .get("supports_multimodal")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                Ok(Box::new(
                    crate::provider::openai::OpenAIProvider::new(base_url, api_key, default_model)
                        .with_multimodal_support(supports_multimodal),
                ))
            }
            "anthropic" => {
                let base_url = config["base_url"].as_str().unwrap_or("").to_string();
                let api_key = config["api_key"].as_str().unwrap_or("").to_string();
                let default_model = config["default_model"]
                    .as_str()
                    .unwrap_or("claude-sonnet-4-20250514")
                    .to_string();

                // Anthropic cloud always supports multimodal; we still read the
                // flag so that it's persisted back correctly, but ignore it.
                let _supports_multimodal = config
                    .get("supports_multimodal")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                Ok(Box::new(
                    crate::provider::anthropic::AnthropicProvider::new(api_key, default_model)
                        .with_base_url(base_url),
                ))
            }
            "gemini" => {
                let base_url = config["base_url"]
                    .as_str()
                    .unwrap_or("https://generativelanguage.googleapis.com/v1beta")
                    .to_string();
                let api_key = config["api_key"].as_str().unwrap_or("").to_string();
                let default_model = config["default_model"]
                    .as_str()
                    .unwrap_or("gemini-2.0-flash")
                    .to_string();
                let supports_multimodal = config
                    .get("supports_multimodal")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                Ok(Box::new(
                    crate::provider::gemini::GeminiProvider::new(base_url, api_key, default_model)
                        .with_multimodal_support(supports_multimodal),
                ))
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
                    // Use SkillPackageSkill which handles all SKILL.md frontmatter fields
                    let name = skill.name.clone();
                    let description = skill.description.clone();

                    let has_content = skill.config["content"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty());
                    let has_shell = skill.config["shell"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty());
                    let has_hooks = skill.config.get("hooks").is_some();
                    let has_when_to_use = skill.config["when_to_use"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty());
                    let has_arguments = skill.config.get("arguments").is_some()
                        || skill.config["argument_hint"]
                            .as_str()
                            .is_some_and(|s| !s.is_empty());
                    let has_allowed_tools = skill.config.get("allowed_tools").is_some();
                    let has_context = skill.config["context"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty());
                    let has_paths = skill.config.get("paths").is_some();

                    // Load the skill if it has any content or executable features
                    if has_content
                        || has_shell
                        || has_hooks
                        || has_when_to_use
                        || has_arguments
                        || has_allowed_tools
                        || has_context
                        || has_paths
                    {
                        result.push(Arc::new(SkillPackageSkill::from_config(
                            name.clone(),
                            description.clone(),
                            &skill.config,
                        )) as Arc<dyn Skill>);
                        tracing::info!(
                            skill_name = %name,
                            shell = has_shell,
                            hooks = has_hooks,
                            allowed_tools = has_allowed_tools,
                            "Loaded skill package with frontmatter support"
                        );
                    } else {
                        tracing::warn!(
                            skill_name = %skill.name,
                            "Skill has no content or executable features, skipping"
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

    /// Build a Provider instance from an embedded provider configuration.
    /// This is used for Config Profiles and Debug Sessions that have their own provider copies.
    pub fn build_provider_from_embedded(
        embedded: &EmbeddedProvider,
    ) -> Result<Box<dyn Provider>, String> {
        let config = &embedded.config_json;

        match embedded.provider_type.as_str() {
            "openai" | "siliconflow" | "deepseek" => {
                let base_url = config["base_url"].as_str().unwrap_or("").to_string();
                let api_key = config["api_key"].as_str().map(|s| s.to_string());
                let default_model = config["default_model"]
                    .as_str()
                    .unwrap_or("gpt-4o")
                    .to_string();
                let supports_multimodal = config
                    .get("supports_multimodal")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                Ok(Box::new(
                    crate::provider::openai::OpenAIProvider::new(base_url, api_key, default_model)
                        .with_multimodal_support(supports_multimodal),
                ))
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
            "gemini" => {
                let base_url = config["base_url"]
                    .as_str()
                    .unwrap_or("https://generativelanguage.googleapis.com/v1beta")
                    .to_string();
                let api_key = config["api_key"].as_str().unwrap_or("").to_string();
                let default_model = config["default_model"]
                    .as_str()
                    .unwrap_or("gemini-2.0-flash")
                    .to_string();
                let supports_multimodal = config
                    .get("supports_multimodal")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                Ok(Box::new(
                    crate::provider::gemini::GeminiProvider::new(base_url, api_key, default_model)
                        .with_multimodal_support(supports_multimodal),
                ))
            }
            other => Err(format!("Unknown provider type: {}", other)),
        }
    }

    /// Build skill instances from a list of embedded skills.
    /// This is used for Config Profiles and Debug Sessions that have their own skill copies.
    pub fn build_skills_from_embedded(
        embedded_skills: &[EmbeddedSkill],
        filter_names: Option<&[String]>,
    ) -> Vec<Arc<dyn Skill>> {
        let mut result = Vec::new();
        for skill in embedded_skills.iter() {
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
                    let name = skill.name.clone();
                    let description = skill.description.clone();

                    let has_content = skill.config["content"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty());
                    let has_shell = skill.config["shell"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty());
                    let has_hooks = skill.config.get("hooks").is_some();
                    let has_when_to_use = skill.config["when_to_use"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty());
                    let has_arguments = skill.config.get("arguments").is_some()
                        || skill.config["argument_hint"]
                            .as_str()
                            .is_some_and(|s| !s.is_empty());
                    let has_allowed_tools = skill.config.get("allowed_tools").is_some();
                    let has_context = skill.config["context"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty());
                    let has_paths = skill.config.get("paths").is_some();

                    if has_content
                        || has_shell
                        || has_hooks
                        || has_when_to_use
                        || has_arguments
                        || has_allowed_tools
                        || has_context
                        || has_paths
                    {
                        result.push(Arc::new(SkillPackageSkill::from_config(
                            name.clone(),
                            description.clone(),
                            &skill.config,
                        )) as Arc<dyn Skill>);
                        tracing::info!(
                            skill_name = %name,
                            "Loaded embedded skill package"
                        );
                    } else {
                        tracing::warn!(
                            skill_name = %skill.name,
                            "Embedded skill has no content or executable features, skipping"
                        );
                    }
                }
                _ => {
                    tracing::warn!(
                        skill_name = %skill.name,
                        skill_type = %skill.skill_type,
                        "Unknown embedded skill type, ignoring"
                    );
                }
            }
        }
        result
    }

    /// Build a fully configured Agent with user context for computer use capabilities.
    ///
    /// Priority order for configuration:
    /// 1. If `use_debug_session` is true, use the Debug Session's embedded configuration.
    /// 2. If `profile_id` is provided, use that specific Config Profile's embedded configuration.
    /// 3. Otherwise, use the active Config Profile's embedded configuration.
    /// 4. Fall back to global configuration (backward compatibility).
    ///
    /// If `provider_id` is explicitly provided, it overrides the resolved provider.
    /// If `persona_id` is explicitly provided, it overrides the resolved persona.
    /// Helper: Resolve the effective configuration context.
    /// Priority: DebugSession > specific profile_id > active profile > global fallback.
    async fn resolve_config_context(
        &self,
        use_debug_session: bool,
        profile_id: Option<&str>,
    ) -> Option<ResolvedConfigContext> {
        // 1. Debug session takes highest priority when requested.
        // Always return the debug session context when use_debug_session is true,
        // even if it appears "empty". This ensures intentional choices like
        // selecting "No reference" for persona (persona_id = None) are not
        // silently overridden by a config profile's persona via fallback.
        // Other fields (temperature, max_tokens, knowledge_base_ids, etc.)
        // also need this consistent behavior.
        if use_debug_session {
            let debug = self.debug_session.read().await;

            // Determine the effective provider source for the debug session
            let effective_provider_id = if debug.providers.is_empty() {
                // No embedded providers, use the provider_id reference
                debug.provider_id.clone()
            } else {
                None // embedded providers take precedence
            };

            return Some(ResolvedConfigContext {
                source: "debug_session".to_string(),
                embedded_providers: debug.providers.clone(),
                active_embedded_provider: debug.active_provider.clone(),
                provider_id: effective_provider_id,
                persona_id: debug.persona_id.clone(),
                embedded_skills: debug.skills.clone(),
                active_embedded_skill_names: debug.active_skill_names.clone(),
                active_skill_names: Vec::new(), // debug session uses embedded skills only
                knowledge_base_ids: debug.knowledge_base_ids.clone(),
                proxy_config: debug.proxy_config.clone(),
                web_search_enabled: debug.web_search_enabled,
                computer_use_enabled: debug.computer_use_enabled,
                thinking_enabled: debug.thinking_enabled,
            });
        }

        let profiles = self.config_profiles.read().await;

        // 2. Specific profile by ID
        if let Some(pid) = profile_id {
            if let Some(profile) = profiles.get(pid) {
                return Some(ResolvedConfigContext {
                    source: format!("profile_{}", pid),
                    embedded_providers: profile.embedded_providers.clone(),
                    active_embedded_provider: profile.active_embedded_provider.clone(),
                    provider_id: profile.provider_id.clone(),
                    persona_id: profile.persona_id.clone(),
                    embedded_skills: profile.embedded_skills.clone(),
                    active_embedded_skill_names: profile.active_embedded_skill_names.clone(),
                    active_skill_names: profile.active_skill_names.clone(),
                    knowledge_base_ids: profile.active_knowledge_base_ids.clone(),
                    proxy_config: profile.proxy_config.clone(),
                    web_search_enabled: profile.web_search_enabled,
                    computer_use_enabled: profile.computer_use_enabled,
                    thinking_enabled: profile.thinking_enabled,
                });
            }
        }

        // 3. Active config profile(s)
        if let Some(profile) = profiles.values().filter(|p| p.is_active && p.enable).next() {
            return Some(ResolvedConfigContext {
                source: format!("active_profile_{}", profile.id),
                embedded_providers: profile.embedded_providers.clone(),
                active_embedded_provider: profile.active_embedded_provider.clone(),
                provider_id: profile.provider_id.clone(),
                persona_id: profile.persona_id.clone(),
                embedded_skills: profile.embedded_skills.clone(),
                active_embedded_skill_names: profile.active_embedded_skill_names.clone(),
                active_skill_names: profile.active_skill_names.clone(),
                knowledge_base_ids: profile.active_knowledge_base_ids.clone(),
                proxy_config: profile.proxy_config.clone(),
                web_search_enabled: profile.web_search_enabled,
                computer_use_enabled: profile.computer_use_enabled,
                thinking_enabled: profile.thinking_enabled,
            });
        }

        drop(profiles);

        // 4. No active profile found at all — return None
        None
    }

    /// Find the config profile that owns a given platform instance ID.
    /// Returns `None` if no profile claims this platform.
    pub async fn find_profile_by_platform_id(&self, platform_id: &str) -> Option<String> {
        let profiles = self.config_profiles.read().await;
        profiles
            .iter()
            .filter(|(_, p)| p.is_active && p.enable)
            .find(|(_, p)| p.platform_ids.contains(&platform_id.to_string()))
            .map(|(id, _)| id.clone())
    }

    /// Extended version that supports debug session and specific profile selection.
    ///
    /// `existing_conversation_id`: When provided (e.g. by the platform message
    /// handler which already created a conversation), load history from this
    /// conversation instead of the WebUI one. When `None`, the WebUI chat
    /// conversation is used (isolated per config profile).
    pub async fn build_agent_with_context_extended(
        &self,
        user_id: Option<&str>,
        session_id: Option<&str>,
        provider_id: Option<&str>,
        use_debug_session: bool,
        profile_id: Option<&str>,
        existing_conversation_id: Option<&str>,
    ) -> Result<Agent, String> {
        // Try to resolve embedded configuration context
        let context = self
            .resolve_config_context(use_debug_session, profile_id)
            .await;

        tracing::info!(
            provider_id = ?provider_id,
            use_debug_session = use_debug_session,
            has_context = context.is_some(),
            context_source = context.as_ref().map(|c| c.source.as_str()).unwrap_or("none"),
            "build_agent_with_context_extended: resolving provider"
        );

        let (mut provider, provider_config_json, proxy_config, kb_ids) = if let Some(ctx) = &context
        {
            // ── Use profile/debug-session configuration ──
            // Each profile independently manages its provider, persona,
            // skills, proxy, knowledge_base, etc. No global fallback.
            tracing::info!(source = %ctx.source, "Using profile configuration context");

            // Priority for provider resolution:
            // 1. Explicit provider_id argument (from API call)
            // 2. Embedded providers in the profile
            // 3. Profile's provider_id field (references a stored provider)
            let resolved_provider = if let Some(pid) = provider_id {
                if !pid.is_empty() {
                    let providers = self.providers.read().await;
                    if let Some(stored) = providers.get(pid) {
                        tracing::info!(
                            provider_id = %pid,
                            "Using explicitly requested provider"
                        );
                        let built = Self::build_provider(stored).ok();
                        let config_json = stored.config_json.clone();
                        drop(providers);
                        built.map(|b| (b, config_json))
                    } else {
                        drop(providers);
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((p, c)) = resolved_provider {
                (
                    p,
                    c,
                    ctx.proxy_config.clone(),
                    ctx.knowledge_base_ids.clone(),
                )
            } else if let Some(ep) = ctx
                .embedded_providers
                .iter()
                .find(|p| {
                    if let Some(ref name) = ctx.active_embedded_provider {
                        p.name == *name || p.id == *name
                    } else {
                        true // first one if no active specified
                    }
                })
                .or_else(|| ctx.embedded_providers.first())
            {
                // Use embedded provider from the profile
                let built = Self::build_provider_from_embedded(ep)?;
                (
                    built,
                    ep.config_json.clone(),
                    ctx.proxy_config.clone(),
                    ctx.knowledge_base_ids.clone(),
                )
            } else if let Some(ref pid) = ctx.provider_id {
                // Profile has a provider_id referencing a stored provider
                if !pid.is_empty() {
                    let providers = self.providers.read().await;
                    if let Some(stored) = providers.get(pid) {
                        tracing::info!(
                            provider_id = %pid,
                            source = %ctx.source,
                            "Using profile's provider_id to resolve stored provider"
                        );
                        let built = Self::build_provider(stored)?;
                        let config_json = stored.config_json.clone();
                        drop(providers);
                        (
                            built,
                            config_json,
                            ctx.proxy_config.clone(),
                            ctx.knowledge_base_ids.clone(),
                        )
                    } else {
                        drop(providers);
                        return Err(format!(
                            "Profile references provider_id '{}' but it was not found in stored providers",
                            pid
                        ));
                    }
                } else {
                    return Err("Profile has no provider configured (no embedded providers, no valid provider_id)".to_string());
                }
            } else {
                return Err(
                    "Profile has no provider configured (no embedded providers, no provider_id)"
                        .to_string(),
                );
            }
        } else {
            // ── No active profile found ──
            // This should not happen in normal operation — every conversation
            // should be managed by a profile. If we reach here, there is no
            // debug session and no active/enabled profile at all.
            return Err(
                "No active profile found. Please create and enable a configuration profile."
                    .to_string(),
            );
        };

        // Apply proxy config if configured
        if proxy_config.is_configured() {
            let base_url = provider_config_json
                .get("base_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let host = url::Url::parse(base_url)
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_string()))
                .unwrap_or_default();

            if proxy_config.should_proxy(&host) {
                provider.set_proxy(
                    &proxy_config.url,
                    proxy_config.username.as_deref(),
                    proxy_config.password.as_deref(),
                );
                tracing::info!(
                    proxy_url = %proxy_config.url,
                    host = %host,
                    "Applied proxy configuration to provider"
                );
            }
        }

        let mut config = AgentConfig::new()
            .with_max_tool_rounds(10)
            .with_auto_execute_tools(true);

        if let Some(ctx) = &context {
            config.thinking_enabled = ctx.thinking_enabled;
        }

        let mut agent = Agent::with_config(provider, config);

        // ── Build skills from profile context ──
        // Skills come from two sources:
        //   1. Embedded skills (profile-internal copies) filtered by active_embedded_skill_names
        //   2. Global skills (stored in self.skills) referenced by active_skill_names
        //
        // Additionally, ALL skills (including inactive ones) are indexed in
        // the agent's available_skill_index so the model can route to them
        // based on name/description/when_to_use. Skills not in the active list
        // are added as "available" (index-only, no full context injection).
        let skill_instances: Vec<Arc<dyn Skill>> = if let Some(ctx) = &context {
            // Build embedded skills (only active ones get full context)
            let embedded_filter: Option<&[String]> = if !ctx.active_embedded_skill_names.is_empty()
            {
                Some(ctx.active_embedded_skill_names.as_slice())
            } else {
                None
            };
            let mut skills =
                Self::build_skills_from_embedded(&ctx.embedded_skills, embedded_filter);

            // Build global skills referenced by active_skill_names
            if !ctx.active_skill_names.is_empty() {
                let global_skills = self.skills.read().await;
                let global =
                    Self::build_skills(&global_skills, Some(ctx.active_skill_names.as_slice()));
                drop(global_skills);
                skills.extend(global);
            }

            // ── Index ALL skills as available (for skill routing) ──
            // Skills that are already active are listed in the routing index
            // automatically by the Agent. Here we add non-active skills as
            // "available" (index-only, no full context injection).
            //
            // Embedded skills not in active_embedded_skill_names:
            let active_embedded_names: std::collections::HashSet<String> =
                ctx.active_embedded_skill_names.iter().cloned().collect();
            for skill_config in &ctx.embedded_skills {
                if !active_embedded_names.contains(&skill_config.name) {
                    let when_to_use = skill_config.config["when_to_use"]
                        .as_str()
                        .map(|s| s.to_string());
                    agent.add_available_skill_with_config(
                        skill_config.name.clone(),
                        skill_config.description.clone(),
                        when_to_use,
                        serde_json::to_value(&skill_config.config).unwrap_or_default(),
                    );
                }
            }

            // Global skills not in active_skill_names:
            let active_global_names: std::collections::HashSet<String> =
                ctx.active_skill_names.iter().cloned().collect();
            let global_skills = self.skills.read().await;
            for (name, stored) in global_skills.iter() {
                if !active_global_names.contains(name) {
                    let when_to_use = stored.config["when_to_use"].as_str().map(|s| s.to_string());
                    agent.add_available_skill_with_config(
                        stored.name.clone(),
                        stored.description.clone(),
                        when_to_use,
                        stored.config.clone(),
                    );
                }
            }
            drop(global_skills);

            skills
        } else {
            // No profile context — should not happen in normal operation
            tracing::warn!("No profile context available, no skills will be loaded");
            Vec::new()
        };

        let num_skills = skill_instances.len();
        for skill in &skill_instances {
            tracing::info!("Adding skill: {}", skill.name());
            agent.add_skill(skill.clone());
        }
        tracing::info!("Successfully added {} skills to agent", num_skills);

        // Register the invoke_skill tool to allow dynamic loading of on-demand skills.
        // This must be done after all available skills are added so the tool knows
        // which skills can be invoked.
        let blacklist_arc = std::sync::Arc::new(tokio::sync::RwLock::new(
            self.shell_command_blacklist.read().await.clone(),
        ));
        agent.register_invoke_skill_tool(blacklist_arc);

        // ── Persona injection is deferred to the end of this function ──
        // to ensure it is the LAST skill added, so its system prompt
        // appears after all other skill system messages and is not
        // overridden by knowledge base or other skill prompts.
        //
        // Persona is resolved exclusively via persona_id from the library.
        // If persona_id is not set or not found, no persona will be applied.
        let deferred_persona: Option<EmbeddedPersona> = if let Some(ctx) = context.as_ref() {
            // Resolve persona from library by persona_id (hot-reload enabled)
            if let Some(ref pid) = ctx.persona_id {
                let personas = self.personas.read().await;
                let resolved = personas.get(pid).map(|p| EmbeddedPersona {
                    name: p.name.clone(),
                    description: p.description.clone(),
                    prompt: p.prompt.clone(),
                    tool_response_style: p.tool_response_style.clone(),
                });
                drop(personas);
                if let Some(p) = resolved {
                    tracing::info!(
                        persona_id = %pid,
                        persona_name = %p.name,
                        "Resolved persona from library (hot-reload enabled)"
                    );
                    Some(p)
                } else {
                    tracing::warn!(
                        persona_id = %pid,
                        "Persona ID not found in library, no persona will be applied"
                    );
                    None
                }
            } else {
                // No persona_id set — no persona will be applied
                None
            }
        } else {
            None
        };

        // Check computer use configuration and register tools accordingly.
        // The per-profile/session `computer_use_enabled` flag overrides the global runtime.
        let global_computer_use_config = self.computer_use_config.read().await;
        let profile_computer_use_enabled = context
            .as_ref()
            .map(|c| c.computer_use_enabled)
            .unwrap_or(true);
        let computer_use_config = if !profile_computer_use_enabled {
            tracing::info!(
                "Computer use is disabled in the active profile/session, forcing runtime to None"
            );
            // Create a config with runtime set to None, preserving other settings
            let mut config = global_computer_use_config.clone();
            config.runtime = crate::computer_use::ComputerUseRuntime::None;
            config
        } else {
            global_computer_use_config.clone()
        };
        drop(global_computer_use_config);
        // Debug sessions always use admin privileges via the reserved debug_admin ID
        let user_id = if use_debug_session {
            crate::computer_use::ComputerUseConfig::DEBUG_ADMIN_ID
        } else {
            user_id.unwrap_or("default_user")
        };
        let session_id = session_id.unwrap_or("default_session");

        match computer_use_config.runtime {
            crate::computer_use::ComputerUseRuntime::None => {
                // Computer use is disabled - register only basic tools
                tracing::info!("Computer use is disabled, registering basic tools only");
                let global_blacklist = self.shell_command_blacklist.read().await.clone();
                agent.register_tool(Arc::new(crate::agent::builtin_tools::ReadFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::WriteFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::DeleteFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::ListDirectoryTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::BashTool::new(
                    global_blacklist,
                )));
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
                    require_shell_confirmation: false, // Chat mode: blacklist handles dangerous commands, no per-command confirmation
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
                agent.register_tool(Arc::new(crate::agent::builtin_tools::DeleteFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
            }
            crate::computer_use::ComputerUseRuntime::AioSandbox => {
                // AIO Sandbox mode - use sandbox tools via HTTP API
                let aio_config = computer_use_config.aio_sandbox_config.as_ref();
                match aio_config {
                    Some(config) => {
                        tracing::info!("Using AIO Sandbox at endpoint: {}", config.endpoint);
                        let client = Arc::new(crate::computer_use::AioSandboxClient::new(
                            config.endpoint.clone(),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxShellTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxReadFileTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxWriteFileTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxListDirectoryTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxCreateFileTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxEditFileTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxFindFilesTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxSearchInFileTool::new(client),
                        ));
                    }
                    None => {
                        tracing::error!(
                            "AIO Sandbox runtime selected but no sandbox config provided, falling back to basic tools"
                        );
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::ReadFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::WriteFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::DeleteFileTool));
                        agent.register_tool(Arc::new(
                            crate::agent::builtin_tools::ListDirectoryTool,
                        ));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
                    }
                }
            }
        }

        // Register WebSearchTool only if properly configured (enabled and has API key if needed).
        // The per-profile/session `web_search_enabled` flag takes precedence over the global config.
        let profile_web_search_enabled = context
            .as_ref()
            .map(|c| c.web_search_enabled)
            .unwrap_or(true);
        let web_search_available = if !profile_web_search_enabled {
            tracing::info!(
                "WebSearchTool not registered: web search is disabled in the active profile/session"
            );
            false
        } else {
            let web_search_config = self.web_search_config.read().await;
            let available = web_search_config.enabled
                && match web_search_config.search_engine {
                    crate::types::SearchEngine::DuckDuckGo => true,
                    _ => web_search_config.api_key.is_some(),
                };
            drop(web_search_config);
            if !available {
                tracing::info!(
                    "WebSearchTool not registered: global web search is disabled or not properly configured"
                );
            }
            available
        };

        if web_search_available {
            agent.register_tool(Arc::new(crate::agent::builtin_tools::WebSearchTool::new(
                self.web_search_config.clone(),
            )));
            tracing::info!("WebSearchTool registered with current configuration");
        }

        // Add Knowledge Base skill and search tool if configured (from embedded context or resolved config)
        if !kb_ids.is_empty() {
            // Use Hybrid mode: auto-inject knowledge base context AND allow the model
            // to call the search tool for follow-up queries. This ensures reliable
            // knowledge base retrieval while still giving the model flexibility.
            let kb_skill = crate::knowledge::KnowledgeBaseSkill::new(
                kb_ids.clone(),
                crate::knowledge::skill::KnowledgeBaseRetrievalMode::Hybrid,
                self.knowledge_base_service.clone(),
            );
            agent.add_skill(std::sync::Arc::new(kb_skill));

            let kb_search_tool = crate::knowledge::KnowledgeBaseSearchTool::new(
                self.knowledge_base_service.clone(),
                kb_ids,
                crate::knowledge::skill::DEFAULT_KB_SEARCH_TOP_K,
            );
            agent.register_tool(std::sync::Arc::new(kb_search_tool));

            tracing::info!("KnowledgeBaseSkill and KnowledgeBaseSearchTool added to agent");
        }

        // ── Inject persona as the system prompt ──
        // Persona is NOT added as a skill. Instead, it is set as the agent's
        // system prompt, which becomes the sole system message in the
        // conversation. Skills (including KB) dynamically inject their
        // context into user messages via `on_user_message()`, never as
        // system messages. This ensures the persona is never overridden.
        if let Some(ref persona) = deferred_persona {
            if !persona.prompt.is_empty() {
                let full_prompt = build_persona_system_prompt(
                    &persona.prompt,
                    persona.tool_response_style.as_deref(),
                );
                tracing::info!(
                    persona_name = %persona.name,
                    has_tool_response_style = persona.tool_response_style.is_some(),
                    "Setting persona as system prompt (sole system message)"
                );
                agent.set_system_prompt(&full_prompt);
            } else {
                tracing::warn!(
                    persona_name = %persona.name,
                    "Persona has empty prompt, skipping"
                );
            }
        } else {
            tracing::info!("No persona configured, skipping system prompt");
        }

        // Load chat history from database if conversation exists.
        // For platform messages, use the existing conversation ID (already created
        // by the platform handler in main.rs). For WebUI chat, resolve the
        // context key to isolate conversations per config profile.
        let conversation_id = if let Some(conv_id) = existing_conversation_id {
            Some(conv_id.to_string())
        } else {
            let context_key = self
                .resolve_chat_context_key_for(use_debug_session, profile_id)
                .await;
            self.ensure_chat_conversation_for(&context_key).await.ok()
        };
        if let Some(ref conv_id) = conversation_id {
            let conv_db = self.conversation_db.read().await;
            if let Some(db) = conv_db.as_ref() {
                if let Ok(messages) = db.get_conversation_messages(conv_id).await {
                    if !messages.is_empty() {
                        // Load chat history, stripping out system messages.
                        // The persona system prompt is managed separately via
                        // `set_system_prompt()` and injected by `build_request()`.
                        // Skill contexts are dynamically injected per-turn via
                        // `inject_skill_contexts()`. Neither should persist.
                        let chat_messages: Vec<ChatMessage> = messages
                            .iter()
                            .filter(|m| m.role != "system")
                            .map(|m| {
                                let role = match m.role.as_str() {
                                    "assistant" => crate::types::MessageRole::Assistant,
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
                    }
                }
            }
            drop(conv_db);
        }

        // Initialize skills: collects context from on_attach() for dynamic
        // injection into user messages. Does NOT inject system messages.
        agent.initialize_skills().await;

        agent.set_metrics(self.metrics.clone());

        // Set metrics source based on the resolved context
        let metrics_source = context.as_ref().map(|ctx| {
            if ctx.source == "debug_session" {
                crate::metrics::TokenSource::DebugSession
            } else if ctx.source.starts_with("profile_")
                || ctx.source.starts_with("active_profile_")
            {
                // Extract profile name from the source string
                let name = ctx
                    .source
                    .strip_prefix("active_profile_")
                    .or_else(|| ctx.source.strip_prefix("profile_"))
                    .unwrap_or(&ctx.source)
                    .to_string();
                crate::metrics::TokenSource::Profile(name)
            } else {
                crate::metrics::TokenSource::Profile(ctx.source.clone())
            }
        });
        if let Some(source) = metrics_source {
            agent.set_metrics_source(source);
        }

        Ok(agent)
    }
}

/// Build the full system prompt from a persona's base prompt and optional
/// tool response style guidance.
///
/// When `tool_response_style` is provided, it is appended to the system prompt
/// to guide the model on how to present tool results in character. When not
/// provided, a sensible default is used to ensure the persona's emotional value
/// carries through to tool responses.
pub(crate) fn build_persona_system_prompt(
    base_prompt: &str,
    tool_response_style: Option<&str>,
) -> String {
    let default_style = "\n\n---\n\n## 回复风格指引\n\
当你使用工具获取结果后，请务必保持你的人设风格向用户呈现信息。\n\
不要机械地罗列数据或结果，而是要用符合你角色设定的语气、情感和表达方式\n\
来进行叙述。保持角色的一致性和生动性，让用户感受到你的个性魅力。";

    let style = tool_response_style.unwrap_or(default_style);
    format!("{}\n\n{}", base_prompt, style)
}

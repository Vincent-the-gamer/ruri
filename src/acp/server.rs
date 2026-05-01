use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_client_protocol::schema::{
    ContentBlock, ContentChunk, SessionId, SessionNotification, SessionUpdate, StopReason,
    TextContent,
};
use serde_json::{Value, json};

use crate::acp::session::{AcpSession, SessionManager};
use crate::api::state::{
    AppState, PersistedConfig, PersistedProvider, PersistedSkill, StoredProvider, StoredSkill,
    default_config_path,
};
use crate::provider::Provider;

/// Runs the ACP server over stdio.
///
/// The agent reads JSON-RPC messages from stdin and writes to stdout.
/// All logging goes to stderr so it doesn't interfere with the protocol.
pub async fn run_acp_server() -> anyhow::Result<()> {
    run_acp_server_with_config_path(None).await
}

/// Runs the ACP server with an optional config file path.
pub async fn run_acp_server_with_config_path(config_path: Option<PathBuf>) -> anyhow::Result<()> {
    let config_path = config_path.unwrap_or_else(default_config_path);
    tracing::info!(
        "Starting ACP server on stdio, config path: {}",
        config_path.display()
    );

    let session_manager = Arc::new(SessionManager::new());
    let provider_factory = Arc::new(ProviderFactory::from_config_path(&config_path));

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut line = String::new();
    let mut reader = stdin.lock();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line);
        match bytes_read {
            Ok(0) => {
                tracing::info!("ACP client disconnected");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Error reading from stdin: {}", e);
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let message: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Invalid JSON-RPC message: {}", e);
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": "Parse error" }
                });
                write_response(&mut stdout, &error_response)?;
                continue;
            }
        };

        let method = message.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or(json!({}));

        tracing::debug!("Received method: {}", method);

        let response = match method {
            "initialize" => handle_initialize(params).await,
            "authenticate" => handle_authenticate(params).await,
            "session/new" => handle_session_new(params, &session_manager, &provider_factory).await,
            "session/prompt" => handle_session_prompt(params, &session_manager, &mut stdout).await,
            "session/cancel" => handle_session_cancel(params, &session_manager).await,
            "session/load" => {
                handle_session_load(params, &session_manager, &provider_factory).await
            }
            "session/close" => handle_session_close(params, &session_manager).await,
            "session/resume" => {
                handle_session_resume(params, &session_manager, &provider_factory).await
            }
            "session/list" => handle_session_list(params).await,
            "session/set_mode" => handle_session_set_mode(params, &session_manager).await,
            "session/set_config_option" => handle_session_set_config_option(params).await,
            "fs/read_text_file" => handle_fs_read_text_file(params).await,
            "fs/write_text_file" => handle_fs_write_text_file(params).await,
            "terminal/create" => handle_terminal_create(params).await,
            "terminal/output" => handle_terminal_output(params).await,
            "terminal/release" => handle_terminal_release(params).await,
            "terminal/wait_for_exit" => handle_terminal_wait_for_exit(params).await,
            "terminal/kill" => handle_terminal_kill(params).await,
            _ => Ok(json!({
                "error": { "code": -32601, "message": format!("Method not found: {}", method) }
            })),
        };

        match response {
            Ok(resp) => {
                if let Some(ref req_id) = id {
                    let mut response_with_id = resp;
                    if let Some(obj) = response_with_id.as_object_mut() {
                        obj.insert("id".to_string(), req_id.clone());
                        if !obj.contains_key("jsonrpc") {
                            obj.insert("jsonrpc".to_string(), json!("2.0"));
                        }
                    }
                    write_response(&mut stdout, &response_with_id)?;
                }
            }
            Err(e) => {
                tracing::error!("Error handling method '{}': {}", method, e);
                if id.is_some() {
                    let error_response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32603, "message": "Internal error", "data": e.to_string() }
                    });
                    write_response(&mut stdout, &error_response)?;
                }
            }
        }
    }

    Ok(())
}

/// Write a JSON-RPC message to stdout (newline-delimited).
fn write_response(stdout: &mut std::io::Stdout, response: &Value) -> anyhow::Result<()> {
    let mut serialized = serde_json::to_string(response)?;
    serialized.push('\n');
    stdout.write_all(serialized.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

/// Send a session/update notification to the client via stdout.
/// Uses proper ACP types from agent_client_protocol crate.
fn send_session_update(
    stdout: &mut std::io::Stdout,
    session_id: &str,
    update: SessionUpdate,
) -> anyhow::Result<()> {
    let session_id = SessionId::new(session_id.to_string());
    let notification = SessionNotification::new(session_id, update);
    let notification_value = serde_json::to_value(notification)?;
    let rpc_notification = json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": notification_value
    });
    write_response(stdout, &rpc_notification)
}

// ─── Protocol Handlers ────────────────────────────────────────────

/// Handle the `initialize` request.
async fn handle_initialize(_params: Value) -> anyhow::Result<Value> {
    tracing::info!("ACP initialize request received");
    Ok(json!({
        "result": {
            "protocolVersion": 1,
            "agentCapabilities": {
                "loadSession": true,
                "promptCapabilities": {
                    "image": false,
                    "audio": false,
                    "embeddedContext": true
                },
                "mcpCapabilities": {
                    "http": false,
                    "sse": false
                },
                "sessionCapabilities": {
                    "close": {},
                    "list": {},
                    "resume": {}
                }
            },
            "agentInfo": {
                "name": "ruri",
                "title": "Ruri AI Agent",
                "version": env!("CARGO_PKG_VERSION")
            },
            "authMethods": []
        }
    }))
}

/// Handle the `authenticate` request.
async fn handle_authenticate(_params: Value) -> anyhow::Result<Value> {
    tracing::info!("ACP authenticate request received");
    Ok(json!({ "result": {} }))
}

/// Handle the `session/new` request.
async fn handle_session_new(
    params: Value,
    session_manager: &Arc<SessionManager>,
    provider_factory: &Arc<ProviderFactory>,
) -> anyhow::Result<Value> {
    let cwd = params
        .get("cwd")
        .and_then(|v| v.as_str())
        .unwrap_or("/")
        .to_string();

    tracing::info!("Creating new ACP session, cwd={}", cwd);

    let provider = provider_factory.create_provider()?;
    let skills = provider_factory.build_skills();

    let session_id = session_manager
        .create_session_with_skills(provider, cwd.clone(), skills)
        .await;
    let mode_state = AcpSession::mode_state_json();

    Ok(json!({
        "result": {
            "sessionId": session_id,
            "modes": mode_state
        }
    }))
}

/// Handle the `session/prompt` request.
async fn handle_session_prompt(
    params: Value,
    session_manager: &Arc<SessionManager>,
    stdout: &mut std::io::Stdout,
) -> anyhow::Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let prompt = params.get("prompt").cloned().unwrap_or(json!([]));
    let text = AcpSession::extract_text_from_prompt(&prompt);

    tracing::info!(
        "Session prompt: session_id={}, text_len={}",
        session_id,
        text.len()
    );

    // Take the session out for processing
    let mut session = session_manager
        .take_session(&session_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

    if session.cancelled {
        session_manager
            .return_session(session_id.clone(), session)
            .await;
        // ACP protocol: return PromptResponse with StopReason::Cancelled
        let response = agent_client_protocol::schema::PromptResponse::new(StopReason::Cancelled);
        return Ok(serde_json::to_value(json!({ "result": response }))?);
    }

    // Process the prompt through the agent
    let result = session.agent.chat(&text).await;

    match result {
        Ok(response) => {
            let content = response
                .choices
                .first()
                .and_then(|c| c.message.content.as_ref())
                .and_then(|c| c.as_text())
                .unwrap_or("")
                .to_string();

            // Determine stop reason from the model's finish_reason
            let stop_reason = response
                .choices
                .first()
                .and_then(|c| c.finish_reason.as_deref())
                .map(|fr| match fr {
                    "stop" => StopReason::EndTurn,
                    "length" => StopReason::MaxTokens,
                    "content_filter" => StopReason::Refusal,
                    _ => StopReason::EndTurn,
                })
                .unwrap_or(StopReason::EndTurn);

            // Send the agent's response as a session/update notification
            // Using proper ACP types: SessionUpdate::AgentMessageChunk
            let update = SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(content),
            )));
            let _ = send_session_update(stdout, &session_id, update);

            session_manager
                .return_session(session_id.clone(), session)
                .await;

            // ACP protocol: return PromptResponse with StopReason
            let prompt_response = agent_client_protocol::schema::PromptResponse::new(stop_reason);
            Ok(serde_json::to_value(json!({ "result": prompt_response }))?)
        }
        Err(e) => {
            session_manager
                .return_session(session_id.clone(), session)
                .await;
            Err(anyhow::anyhow!("Agent error: {}", e))
        }
    }
}

async fn handle_session_cancel(
    params: Value,
    session_manager: &Arc<SessionManager>,
) -> anyhow::Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    tracing::info!("Cancelling session: {}", session_id);
    session_manager.cancel_session(session_id).await;
    Ok(json!({}))
}

/// Handle the `session/load` request.
async fn handle_session_load(
    params: Value,
    session_manager: &Arc<SessionManager>,
    provider_factory: &Arc<ProviderFactory>,
) -> anyhow::Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing sessionId"))?;
    let cwd = params
        .get("cwd")
        .and_then(|v| v.as_str())
        .unwrap_or("/")
        .to_string();

    tracing::info!("Loading session: {}", session_id);

    let provider = provider_factory.create_provider()?;
    let skills = provider_factory.build_skills();

    session_manager
        .load_session_with_skills(provider, session_id.to_string(), cwd, skills)
        .await;

    Ok(json!({ "result": null }))
}

/// Handle the `session/close` request.
async fn handle_session_close(
    params: Value,
    session_manager: &Arc<SessionManager>,
) -> anyhow::Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    tracing::info!("Closing session: {}", session_id);
    session_manager.close_session(session_id).await;
    Ok(json!({ "result": {} }))
}

/// Handle the `session/resume` request.
async fn handle_session_resume(
    params: Value,
    session_manager: &Arc<SessionManager>,
    provider_factory: &Arc<ProviderFactory>,
) -> anyhow::Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing sessionId"))?;
    let cwd = params
        .get("cwd")
        .and_then(|v| v.as_str())
        .unwrap_or("/")
        .to_string();

    tracing::info!("Resuming session: {}", session_id);

    let provider = provider_factory.create_provider()?;
    let skills = provider_factory.build_skills();

    session_manager
        .load_session_with_skills(provider, session_id.to_string(), cwd, skills)
        .await;

    Ok(json!({ "result": {} }))
}

/// Handle the `session/list` request.
async fn handle_session_list(_params: Value) -> anyhow::Result<Value> {
    Ok(json!({ "result": { "sessions": [] } }))
}

/// Handle the `session/set_mode` request.
async fn handle_session_set_mode(
    params: Value,
    session_manager: &Arc<SessionManager>,
) -> anyhow::Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mode_id = params
        .get("modeId")
        .and_then(|v| v.as_str())
        .unwrap_or("ask");

    tracing::info!("Setting mode: session={}, mode={}", session_id, mode_id);

    if let Some(mut session) = session_manager.take_session(session_id).await {
        session.current_mode = mode_id.to_string();
        session_manager
            .return_session(session_id.to_string(), session)
            .await;
    }

    let mode_state = AcpSession::mode_state_json();
    Ok(json!({ "result": { "modes": mode_state } }))
}

/// Handle the `session/set_config_option` request.
async fn handle_session_set_config_option(_params: Value) -> anyhow::Result<Value> {
    Ok(json!({ "result": { "configOptions": [] } }))
}

async fn handle_fs_read_text_file(params: Value) -> anyhow::Result<Value> {
    let path = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing path"))?;

    let content = tokio::fs::read_to_string(path).await?;
    Ok(json!({ "result": { "content": content } }))
}

async fn handle_fs_write_text_file(params: Value) -> anyhow::Result<Value> {
    let path = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
    let content = params
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing content"))?;

    tokio::fs::write(path, content).await?;
    Ok(json!({ "result": {} }))
}

async fn handle_terminal_create(params: Value) -> anyhow::Result<Value> {
    let command = params
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("sh");
    let args: Vec<String> = params
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let cwd = params.get("cwd").and_then(|v| v.as_str()).unwrap_or(".");

    let terminal_id = uuid::Uuid::new_v4().to_string();

    let mut cmd = tokio::process::Command::new(command);
    cmd.args(&args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    {
        cmd.process_group(0);
    }

    match cmd.spawn() {
        Ok(_child) => {
            TERMINAL_CACHE.insert(terminal_id.clone(), terminal_id.clone());
            // Store the child process handle somewhere accessible
            // For simplicity, we just track the ID and manage output separately
            Ok(json!({
                "result": {
                    "terminalId": terminal_id
                }
            }))
        }
        Err(e) => Err(anyhow::anyhow!("Failed to create terminal: {}", e)),
    }
}

async fn handle_terminal_output(params: Value) -> anyhow::Result<Value> {
    let _terminal_id = params
        .get("terminalId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing terminalId"))?;

    // In a full implementation, we'd read from the child process stdout/stderr
    // For now, return empty output
    Ok(json!({
        "result": {
            "output": [],
            "exitCode": null
        }
    }))
}

async fn handle_terminal_release(params: Value) -> anyhow::Result<Value> {
    let terminal_id = params
        .get("terminalId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing terminalId"))?;

    TERMINAL_CACHE.remove(terminal_id);
    Ok(json!({ "result": {} }))
}

async fn handle_terminal_wait_for_exit(params: Value) -> anyhow::Result<Value> {
    let terminal_id = params
        .get("terminalId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing terminalId"))?;

    let exit_code = EXIT_CODES.get(terminal_id).and_then(|v| *v);
    Ok(json!({
        "result": {
            "exitCode": exit_code
        }
    }))
}

async fn handle_terminal_kill(params: Value) -> anyhow::Result<Value> {
    let terminal_id = params
        .get("terminalId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing terminalId"))?;

    TERMINAL_CACHE.remove(terminal_id);
    Ok(json!({ "result": {} }))
}

// ─── Provider Factory ──────────────────────────────────────────────

/// Creates providers and skills for ACP sessions, using persisted config
/// when available and falling back to environment variables.
pub struct ProviderFactory {
    /// The loaded persisted config (if available).
    config: Option<PersistedConfig>,
}

impl ProviderFactory {
    /// Create a ProviderFactory that reads from the given config file path.
    /// Falls back to environment variables if the config file doesn't exist
    /// or is invalid.
    pub fn from_config_path(config_path: &Path) -> Self {
        let config = match Self::load_config(config_path) {
            Ok(c) => {
                tracing::info!("ACP loaded config from {}", config_path.display());
                Some(c)
            }
            Err(e) => {
                tracing::info!(
                    "ACP could not load config from {}: {}, will use env vars",
                    config_path.display(),
                    e
                );
                None
            }
        };
        Self { config }
    }

    /// Load the persisted config file.
    fn load_config(path: &Path) -> anyhow::Result<PersistedConfig> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config: {}", e))?;
        let config: PersistedConfig = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;
        Ok(config)
    }

    /// Resolve the active provider ID for ACP mode.
    /// Priority: ACP config provider → API mode active provider → None.
    fn resolve_acp_provider_id(&self) -> Option<String> {
        if let Some(ref config) = self.config {
            // ACP-specific provider takes priority
            if let Some(ref acp_provider_id) = config.acp_config.active_provider_id
                && config.providers.contains_key(acp_provider_id)
            {
                return Some(acp_provider_id.clone());
            }
            // Fall back to API mode active provider
            if let Some(ref api_active) = config.active_provider_id
                && config.providers.contains_key(api_active)
            {
                return Some(api_active.clone());
            }
        }
        None
    }

    /// Create a provider from the persisted config or environment variables.
    pub fn create_provider(&self) -> anyhow::Result<Box<dyn Provider>> {
        // Try to create from persisted config first
        if let Some(provider_id) = self.resolve_acp_provider_id()
            && let Some(ref config) = self.config
            && let Some(persisted) = config.providers.get(&provider_id)
        {
            let stored = Self::persisted_to_stored_provider(persisted);
            match AppState::build_provider(&stored) {
                Ok(provider) => {
                    tracing::info!(
                        "ACP using persisted provider: {} ({})",
                        persisted.name,
                        persisted.provider_type
                    );
                    return Ok(provider);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to build provider from config: {}, falling back to env vars",
                        e
                    );
                }
            }
        }

        // Fall back to environment variables
        self.create_provider_from_env()
    }

    /// Build skill instances based on the persisted ACP config.
    /// Only returns skills whose names are listed in `acp_config.active_skill_names`.
    pub fn build_skills(&self) -> Vec<Arc<dyn crate::agent::skill::Skill>> {
        let Some(ref config) = self.config else {
            return Vec::new();
        };

        if config.acp_config.active_skill_names.is_empty() {
            return Vec::new();
        }

        let stored_skills: HashMap<String, StoredSkill> = config
            .skills
            .iter()
            .map(|(name, s)| (name.clone(), Self::persisted_to_stored_skill(s)))
            .collect();

        AppState::build_skills(&stored_skills, Some(&config.acp_config.active_skill_names))
    }

    /// Convert a PersistedProvider to a StoredProvider.
    fn persisted_to_stored_provider(p: &PersistedProvider) -> StoredProvider {
        let created_at = chrono::DateTime::parse_from_rfc3339(&p.created_at)
            .map(|dt| dt.to_utc())
            .unwrap_or(chrono::Utc::now());
        StoredProvider {
            id: p.id.clone(),
            name: p.name.clone(),
            provider_type: p.provider_type.clone(),
            config_json: p.config_json.clone(),
            is_active: p.is_active,
            created_at,
        }
    }

    /// Convert a PersistedSkill to a StoredSkill.
    fn persisted_to_stored_skill(s: &PersistedSkill) -> StoredSkill {
        StoredSkill {
            name: s.name.clone(),
            description: s.description.clone(),
            skill_type: s.skill_type.clone(),
            config: s.config.clone(),
            is_active: s.is_active,
        }
    }

    /// Create a provider from environment variables (original behavior).
    fn create_provider_from_env(&self) -> anyhow::Result<Box<dyn Provider>> {
        let provider_type = if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            "anthropic"
        } else if std::env::var("OPENAI_API_KEY").is_ok() {
            "openai"
        } else if std::env::var("CUSTOM_API_URL").is_ok() {
            "custom"
        } else {
            "openai"
        };

        match provider_type {
            "anthropic" => {
                let api_key = std::env::var("ANTHROPIC_API_KEY")
                    .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;
                let model = std::env::var("ANTHROPIC_MODEL")
                    .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
                Ok(Box::new(
                    crate::provider::anthropic::AnthropicProvider::new(api_key, model),
                ))
            }
            "custom" => {
                let url = std::env::var("CUSTOM_API_URL")
                    .map_err(|_| anyhow::anyhow!("CUSTOM_API_URL not set"))?;
                let api_key = std::env::var("CUSTOM_API_KEY").ok();
                let model = std::env::var("CUSTOM_MODEL").unwrap_or_else(|_| "default".to_string());
                let config = crate::provider::custom::CustomProviderConfig {
                    base_url: url,
                    chat_path: "/v1/chat/completions".to_string(),
                    method: "POST".to_string(),
                    auth_header: Some("Authorization".to_string()),
                    auth_prefix: "Bearer ".to_string(),
                    extra_headers: HashMap::new(),
                    request_template: None,
                    response_content_path: None,
                    response_tool_calls_path: None,
                    response_model_path: None,
                    response_finish_reason_path: None,
                    default_model: model,
                    use_openai_format: true,
                };
                Ok(Box::new(crate::provider::custom::CustomProvider::new(
                    config, api_key,
                )))
            }
            _ => {
                let base_url = std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
                let api_key = std::env::var("OPENAI_API_KEY").ok();
                let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
                Ok(Box::new(crate::provider::openai::OpenAIProvider::new(
                    base_url, api_key, model,
                )))
            }
        }
    }
}

impl Default for ProviderFactory {
    fn default() -> Self {
        Self::from_config_path(&default_config_path())
    }
}

// ─── Terminal Cache ───────────────────────────────────────────────

static TERMINAL_CACHE: std::sync::LazyLock<dashmap::DashMap<String, String>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);
static EXIT_CODES: std::sync::LazyLock<dashmap::DashMap<String, Option<i32>>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);

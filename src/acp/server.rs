use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::Arc;

use serde_json::{Value, json};

use crate::acp::session::{AcpSession, SessionManager};
use crate::provider::Provider;

/// Runs the ACP server over stdio.
///
/// The agent reads JSON-RPC messages from stdin and writes to stdout.
/// All logging goes to stderr so it doesn't interfere with the protocol.
pub async fn run_acp_server() -> anyhow::Result<()> {
    tracing::info!("Starting ACP server on stdio");

    let session_manager = Arc::new(SessionManager::new());
    let provider_factory = Arc::new(ProviderFactory::new());

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
            "session/prompt" => handle_session_prompt(params, &session_manager).await,
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
fn send_session_update(
    stdout: &mut std::io::Stdout,
    session_id: &str,
    update: Value,
) -> anyhow::Result<()> {
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "sessionId": session_id,
            "update": update
        }
    });
    write_response(stdout, &notification)
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
    let session_id = session_manager.create_session(provider, cwd.clone()).await;
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
) -> anyhow::Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing sessionId"))?;

    let prompt = params.get("prompt").cloned().unwrap_or(json!([]));
    let user_text = AcpSession::extract_text_from_prompt(&prompt);

    if user_text.is_empty() {
        return Ok(json!({ "result": { "stopReason": "end_turn" } }));
    }

    tracing::info!(
        "Session prompt: session={}, text_len={}",
        session_id,
        user_text.len()
    );

    let mut session = session_manager
        .take_session(session_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

    session.cancelled = false;

    match session.agent.chat(&user_text).await {
        Ok(response) => {
            let choice = &response.choices[0];
            let assistant_text = choice.message.content.as_text().unwrap_or("").to_string();
            let tool_calls = choice.message.tool_calls.as_ref();

            session_manager
                .return_session(session_id.to_string(), session)
                .await;

            let stop_reason = match choice.finish_reason.as_deref() {
                Some("length") => "max_tokens",
                Some("content_filter") => "refusal",
                _ => "end_turn",
            };

            let mut stdout = std::io::stdout();

            // Send agent_message_chunk notification
            if !assistant_text.is_empty() {
                send_session_update(
                    &mut stdout,
                    session_id,
                    json!({
                        "sessionUpdate": "agent_message_chunk",
                        "content": { "type": "text", "text": assistant_text }
                    }),
                )?;
            }

            // Send tool call notifications
            if let Some(calls) = tool_calls {
                for call in calls {
                    send_session_update(
                        &mut stdout,
                        session_id,
                        json!({
                            "sessionUpdate": "tool_call",
                            "toolCallId": call.id,
                            "title": format!("Executing {}", call.function.name),
                            "kind": "other",
                            "status": "completed"
                        }),
                    )?;
                }
            }

            Ok(json!({ "result": { "stopReason": stop_reason } }))
        }
        Err(e) => {
            session_manager
                .return_session(session_id.to_string(), session)
                .await;
            tracing::error!("Agent chat error: {}", e);
            Ok(json!({ "result": { "stopReason": "end_turn" } }))
        }
    }
}

/// Handle the `session/cancel` notification.
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
    session_manager
        .load_session(provider, session_id.to_string(), cwd)
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
    session_manager
        .load_session(provider, session_id.to_string(), cwd)
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

/// Handle `fs/read_text_file`.
async fn handle_fs_read_text_file(params: Value) -> anyhow::Result<Value> {
    let path = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;
    let line = params.get("line").and_then(|v| v.as_u64());
    let limit = params.get("limit").and_then(|v| v.as_u64());

    tracing::debug!("Reading file: {}", path);

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path, e))?;

    let result_content = if let Some(start_line) = line {
        let start = start_line as usize;
        let lines: Vec<&str> = content.lines().collect();
        let end = if let Some(lim) = limit {
            std::cmp::min(start + lim as usize, lines.len())
        } else {
            lines.len()
        };
        if start > 0 && start <= lines.len() {
            lines[start - 1..end].join("\n")
        } else {
            String::new()
        }
    } else {
        content
    };

    Ok(json!({ "result": { "content": result_content } }))
}

/// Handle `fs/write_text_file`.
async fn handle_fs_write_text_file(params: Value) -> anyhow::Result<Value> {
    let path = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;
    let content = params
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing content parameter"))?;

    tracing::debug!("Writing file: {}", path);

    if let Some(parent) = std::path::Path::new(path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, content).await?;

    Ok(json!({ "result": null }))
}

/// Handle `terminal/create`.
async fn handle_terminal_create(params: Value) -> anyhow::Result<Value> {
    let command = params
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing command parameter"))?;
    let args: Vec<String> = params
        .get("args")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let cwd = params.get("cwd").and_then(|v| v.as_str());
    let terminal_id = uuid::Uuid::new_v4().to_string();

    tracing::info!("Terminal create: {} {:?}", command, args);

    let mut cmd = tokio::process::Command::new(command);
    cmd.args(&args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let output = cmd.output().await;

    match output {
        Ok(out) => {
            let stdout_str = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr_str = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = format!("{}{}", stdout_str, stderr_str);
            let exit_code = out.status.code();

            TERMINAL_CACHE.insert(terminal_id.clone(), combined);
            EXIT_CODES.insert(terminal_id.clone(), exit_code);

            Ok(json!({ "result": { "terminalId": terminal_id } }))
        }
        Err(e) => Ok(json!({
            "error": { "code": -32603, "message": format!("Failed to execute command: {}", e) }
        })),
    }
}

/// Handle `terminal/output`.
async fn handle_terminal_output(params: Value) -> anyhow::Result<Value> {
    let terminal_id = params
        .get("terminalId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let output = TERMINAL_CACHE
        .get(terminal_id)
        .map(|s| s.value().clone())
        .unwrap_or_default();

    let exit_status = EXIT_CODES
        .get(terminal_id)
        .and_then(|e| e.value().map(|c| json!({ "exitCode": c, "signal": null })));

    let mut result = json!({ "output": output, "truncated": false });
    if let Some(status) = exit_status {
        result
            .as_object_mut()
            .map(|o| o.insert("exitStatus".to_string(), status));
    }

    Ok(json!({ "result": result }))
}

/// Handle `terminal/release`.
async fn handle_terminal_release(params: Value) -> anyhow::Result<Value> {
    let terminal_id = params
        .get("terminalId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    TERMINAL_CACHE.remove(terminal_id);
    EXIT_CODES.remove(terminal_id);
    Ok(json!({ "result": {} }))
}

/// Handle `terminal/wait_for_exit`.
async fn handle_terminal_wait_for_exit(params: Value) -> anyhow::Result<Value> {
    let terminal_id = params
        .get("terminalId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let exit_code = EXIT_CODES
        .get(terminal_id)
        .and_then(|e| *e.value())
        .unwrap_or(0);
    Ok(json!({ "result": { "exitCode": exit_code, "signal": null } }))
}

/// Handle `terminal/kill`.
async fn handle_terminal_kill(_params: Value) -> anyhow::Result<Value> {
    Ok(json!({ "result": {} }))
}

// ─── Provider Factory ─────────────────────────────────────────────

/// Creates providers for ACP sessions from environment variables.
pub struct ProviderFactory {
    provider_type: String,
}

impl ProviderFactory {
    pub fn new() -> Self {
        let provider_type = if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            "anthropic".to_string()
        } else if std::env::var("OPENAI_API_KEY").is_ok() {
            "openai".to_string()
        } else if std::env::var("CUSTOM_API_URL").is_ok() {
            "custom".to_string()
        } else {
            "openai".to_string()
        };
        Self { provider_type }
    }

    pub fn create_provider(&self) -> anyhow::Result<Box<dyn Provider>> {
        match self.provider_type.as_str() {
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
        Self::new()
    }
}

// ─── Terminal Cache ───────────────────────────────────────────────

static TERMINAL_CACHE: std::sync::LazyLock<dashmap::DashMap<String, String>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);
static EXIT_CODES: std::sync::LazyLock<dashmap::DashMap<String, Option<i32>>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);

//! Ruri - AI Agent application
mod acp;
mod agent;
mod api;
mod auth;
mod command;
mod computer_use;
mod conversation;
mod db;
mod knowledge;
mod logging;
mod mcp;
mod metrics;
mod platform;
mod provider;
mod transport;
mod types;

use agent::builtin_tools::{
    CreateFileTool, EditFileTool, ListDirectoryTool, ReadFileTool, SearchFilesTool, WebSearchTool,
    WriteFileTool,
};
use agent::tool_executor::ToolExecutor;
use api::AppState;
use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use clap::Parser;
use rust_embed::RustEmbed;
use std::io::IsTerminal;
use std::sync::Arc;
use tokio::net::TcpSocket;

/// CLI arguments parsed by clap.
#[derive(Parser)]
#[command(name = "ruri", version, about = "A customizable AI Agent")]
struct Args {
    /// Start in ACP mode (stdio transport)
    #[arg(long, short = 'a')]
    acp: bool,

    /// Override config file path (used in ACP mode)
    #[arg(long, short = 'c')]
    acp_config: Option<std::path::PathBuf>,

    /// Bind WebUI and API to 0.0.0.0 (accessible from network)
    #[arg(long, short = 'r')]
    remote: bool,

    /// Port to listen on (default: 3000)
    #[arg(long, short = 'p', default_value_t = 3000)]
    port: u16,
}

/// Embedded frontend assets from the compiled Vue build.
#[derive(RustEmbed)]
#[folder = "src/web_dist/"]
struct Assets;

/// Generate a friendly, human-like progress message for a tool being executed.
/// These messages make the assistant feel like it's naturally telling the user
/// what it's doing, rather than a cold "🔨 正在调用工具 xxx" message.
fn friendly_tool_message(tool_name: &str) -> String {
    match tool_name {
        "read_file" => "让我看看这个文件里有什么... 📖".to_string(),
        "write_file" => "正在帮你写入文件... ✍️".to_string(),
        "edit_file" => "正在帮你修改文件... ✏️".to_string(),
        "create_file" => "正在帮你创建新文件... 📄".to_string(),
        "delete_file" => "正在帮你清理文件... 🗑️".to_string(),
        "list_directory" => "让我看看这个目录里有什么... 📂".to_string(),
        "search_files" => "正在帮你搜索文件... 🔍".to_string(),
        "grep" => "正在搜索代码... 🔎".to_string(),
        "find_path" => "正在查找文件路径... 🔍".to_string(),
        "bash" => "正在执行命令，稍等一下... ⚙️".to_string(),
        "web_search" => "正在帮你搜索相关资料... 🌐".to_string(),
        "web_fetch" => "正在获取网页内容... 🌍".to_string(),
        "invoke_skill" => "正在调用技能... 🎯".to_string(),
        "fetch" => "正在获取内容... 📥".to_string(),
        "copy_path" => "正在复制文件... 📋".to_string(),
        "move_path" => "正在移动文件... 📦".to_string(),
        "create_directory" => "正在创建目录... 📁".to_string(),
        _ => format!("正在处理，请稍候... 💭"),
    }
}

/// Handler for serving embedded static files.
/// Falls back to index.html for SPA routing.
async fn static_handler(req: Request) -> Response {
    let path = req.uri().path().trim_start_matches('/');

    // Try to find the exact file first
    match Assets::get(path) {
        Some(content) => {
            let mime_type = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(
                    header::CONTENT_TYPE,
                    HeaderValue::from_str(mime_type.as_ref())
                        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
                )
                .body(Body::from(content.data.to_vec()))
                .unwrap()
        }
        None => {
            // SPA fallback: serve index.html for all non-API, non-asset routes
            match Assets::get("index.html") {
                Some(content) => {
                    Html(String::from_utf8_lossy(&content.data).to_string()).into_response()
                }
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Not found"))
                    .unwrap(),
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Parse CLI arguments ──────────────────────────────────────
    let args = Args::parse();

    // ── Check for ACP mode ──────────────────────────────────────
    let acp_mode = args.acp;
    let acp_config_path = args.acp_config.clone();

    // Initialize logging
    if acp_mode {
        // ACP mode: logging goes to stderr so it doesn't interfere with JSON-RPC on stdout
        // Check if stderr is a terminal for ANSI color support
        logging::set_color_enabled(std::io::stderr().is_terminal());

        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .with_writer(std::io::stderr)
            .event_format(logging::RuriFormat)
            .init();

        tracing::info!("Starting Ruri in ACP mode (stdio)");
        // Clippy claims the `Into::into` is useless, but removing it breaks type
        // inference through the `return` expression.
        #[allow(clippy::useless_conversion)]
        return acp::run_acp_server_with_config_path(acp_config_path)
            .await
            .map_err(Into::into);
    }

    // Normal mode: use our logging system with LogManager
    let log_manager = logging::init_logging(5000);

    tracing::info!("══════════════════════════════════════");
    tracing::info!("         🤖 Ruri AI Agent             ");
    tracing::info!("══════════════════════════════════════");

    // ── Create shared application state ──────────────────────────

    // Create AppState to load configuration (including web_search_config)
    let temp_state = AppState::new().with_log_manager(log_manager);

    // Clone the web_search_config before moving state into Arc
    let web_search_config = temp_state.web_search_config.clone();

    // Register built-in tools
    let mut tool_executor = ToolExecutor::new();
    tool_executor.register(Arc::new(ReadFileTool));
    tool_executor.register(Arc::new(WriteFileTool));
    tool_executor.register(Arc::new(CreateFileTool));
    tool_executor.register(Arc::new(EditFileTool));
    tool_executor.register(Arc::new(ListDirectoryTool));
    tool_executor.register(Arc::new(SearchFilesTool));
    tool_executor.register(Arc::new(WebSearchTool::new(web_search_config)));

    // Get tool definitions
    let tool_defs = tool_executor.definitions();

    // Create the final state with tool definitions
    let state = Arc::new(AppState {
        tool_definitions: tool_defs,
        ..temp_state
    });

    // Load platform configs into state for API access
    state.load_platforms_config().await;

    // Load debug session configuration (WebUI chat independent settings)
    {
        let debug_config = state.load_debug_session().await;
        *state.debug_session.write().await = debug_config;
        tracing::info!("Debug session configuration loaded");
    }

    // ── Initialize unified database (ruri.db) ────────────────────────
    let db_path = db::database_path();

    match db::init(db_path.clone()).await {
        Ok(pool) => {
            tracing::info!("Unified database initialized at: {:?}", db_path);

            // Store the shared pool in AppState
            *state.db_pool.write().await = Some(pool.clone());

            // ── Conversation sub-module ─────────────────────────────
            match conversation::ConversationDatabase::new(pool.clone()).await {
                Ok(conv_db) => {
                    *state.conversation_db.write().await = Some(std::sync::Arc::new(conv_db));
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize conversation database: {}", e);
                    tracing::warn!("Conversation history features will be unavailable");
                }
            }

            // ── MCP configuration sub-module ───────────────────────
            let mcp_config_manager = mcp::config::McpConfigManager::new(pool.clone());
            if let Err(e) = mcp_config_manager.init().await {
                tracing::warn!("Failed to verify MCP database schema: {}", e);
            }
            *state.mcp_config.write().await = Some(mcp_config_manager);

            // ── Knowledge base sub-module ─────────────────────────────
            match crate::knowledge::KnowledgeBaseStore::new(pool.clone()).await {
                Ok(kb_store) => {
                    let kb_service =
                        crate::knowledge::KnowledgeBaseService::new(std::sync::Arc::new(kb_store));
                    *state.knowledge_base_service.write().await = Some(kb_service);
                    tracing::info!("Knowledge base service initialized");
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize knowledge base store: {}", e);
                    tracing::warn!("Knowledge base features will be unavailable");
                }
            }

            // ── Shell Command Blacklist — sync from DB ────────────
            match db::seed_shell_blacklist(&pool).await {
                Ok(patterns) => {
                    tracing::info!(
                        "Shell command blacklist loaded from DB: {} patterns",
                        patterns.len()
                    );
                    // Sync to global in-memory blacklist
                    *state.shell_command_blacklist.write().await = patterns.clone();
                    // Sync to ComputerUseConfig
                    {
                        let mut cu_config = state.computer_use_config.write().await;
                        cu_config.shell_command_blacklist = patterns;
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to sync shell command blacklist from DB: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to initialize unified database: {}", e);
            tracing::warn!("All database features will be unavailable");
        }
    }

    // ── Initialize chat platform adapters ────────────────────────
    // Only start adapters that are enabled in the platform config.
    // Platform configs are already loaded into
    // `state.platform_configs` by `load_platforms_config()` above.
    {
        state.sync_platforms().await;

        let pm = state.platform_manager.read().await;
        if !pm.is_empty() {
            tracing::info!("Enabled platform adapters: {}", pm.len());
        } else {
            tracing::info!("No platform adapters enabled");
        }
    }

    // ── Global shutdown token ──────────────────────────────────
    // All background tasks listen to this token so they can exit
    // cleanly when Ctrl+C / SIGTERM / API restart is received.
    let global_shutdown_token = tokio_util::sync::CancellationToken::new();

    // Spawn a task to process incoming platform messages and route them to the agent.
    let state_for_platform = state.clone();
    let platform_manager_ref = state.platform_manager.clone();
    let platform_loop_token = global_shutdown_token.clone();
    tokio::spawn(async move {
        // Get the initial event receiver
        let mut event_receiver = {
            let mut pm = platform_manager_ref.write().await;
            pm.take_event_receiver()
                .expect("event receiver should be available at startup")
        };

        loop {
            let event = tokio::select! {
                _ = platform_loop_token.cancelled() => {
                    tracing::info!("Platform event loop received shutdown signal");
                    break;
                }
                evt = event_receiver.recv() => evt,
            };
            let Some(event) = event else {
                break;
            };
            match event {
                platform::PlatformEvent::Message(msg) => {
                    tracing::info!(
                        message_type = %msg.message_type,
                        sender = %msg.sender.user_id,
                        text = %msg.message_str.chars().take(80).collect::<String>(),
                        "Received platform message"
                    );

                    // ── Command dispatch ─────────────────────────────
                    // Try to dispatch as a built-in command. If a known command
                    // matched, `dispatch` returns `Some(result)` and we skip
                    // the LLM. Otherwise (no prefix, prefix-only, or unrecognized
                    // command) we fall through to the agent / LLM.
                    {
                        // Resolve per-context prefix and enabled_commands from the
                        // config profile that owns this platform instance.
                        let (ctx_prefix, ctx_enabled_commands) = {
                            let profiles = state_for_platform.config_profiles.read().await;
                            profiles
                                .values()
                                .filter(|p| {
                                    p.is_active
                                        && p.enable
                                        && p.platform_ids.contains(&msg.platform_id)
                                })
                                .next()
                                .map(|p| (p.command_prefix.clone(), p.enabled_commands.clone()))
                                .unwrap_or_else(|| ("/".to_string(), Vec::new()))
                        };
                        let dispatcher = state_for_platform.command_dispatcher.read().await;
                        let cmd_ctx = command::CommandContext {
                            raw_message: msg.message_str.clone(),
                            command_name: String::new(), // filled by dispatch()
                            args: String::new(),
                            prefix: ctx_prefix,
                            enabled_commands: ctx_enabled_commands,
                            session_id: msg.session_id.clone(),
                            user_id: msg.sender.user_id.clone(),
                            platform_id: msg.platform_id.clone(),
                            self_id: msg.self_id.clone(),
                            message_type: msg.message_type,
                            group_id: msg.group_id.clone(),
                            state: state_for_platform.clone(),
                        };

                        if let Some(result) = dispatcher.dispatch(cmd_ctx).await {
                            // A known command was matched — send result back
                            let pm = platform_manager_ref.read().await;
                            if let Err(e) = pm
                                .send_text_to_platform(
                                    &msg.platform_id,
                                    msg.message_type,
                                    &msg.session_id,
                                    &result.reply,
                                )
                                .await
                            {
                                tracing::error!(
                                    error = %e,
                                    "Failed to send command reply to platform"
                                );
                            }
                            // Command handled, skip agent processing
                            continue;
                        }
                    }

                    // ── Agent processing (spawned) ──────────────────
                    // Spawn agent processing in a separate task so the event
                    // loop can immediately receive the next message (e.g. /stop
                    // to cancel a running shell command). Command dispatch above
                    // has already confirmed this is not a built-in command.
                    let state_clone = state_for_platform.clone();
                    let pm_clone = platform_manager_ref.clone();
                    let msg_clone = msg.clone();
                    tokio::spawn(async move {
                        // ── Conversation DB: ensure conversation exists ────────
                        let chat_type = match msg_clone.message_type {
                            platform::types::MessageType::GroupMessage => {
                                conversation::models::ChatType::Group
                            }
                            platform::types::MessageType::FriendMessage => {
                                conversation::models::ChatType::Private
                            }
                        };
                        let conversation_id = {
                            let conv_db = state_clone.conversation_db.read().await;
                            if let Some(db) = conv_db.as_ref() {
                                match db
                                    .get_or_create_conversation(
                                        msg_clone.platform_id.clone(),
                                        chat_type,
                                        msg_clone.session_id.clone(),
                                    )
                                    .await
                                {
                                    Ok(conv) => Some(conv.id),
                                    Err(e) => {
                                        tracing::warn!(
                                            error = %e,
                                            "Failed to get/create conversation for platform message"
                                        );
                                        None
                                    }
                                }
                            } else {
                                None
                            }
                        };

                        // Save user message to conversation database
                        if let Some(ref conv_id) = conversation_id {
                            let conv_db = state_clone.conversation_db.read().await;
                            if let Some(db) = conv_db.as_ref() {
                                if let Err(e) = db
                                    .add_message(conversation::models::AddMessageRequest {
                                        conversation_id: conv_id.clone(),
                                        role: "user".to_string(),
                                        content: msg_clone.message_str.clone(),
                                    })
                                    .await
                                {
                                    tracing::error!(
                                        error = %e,
                                        "Failed to add user message to conversation database"
                                    );
                                }
                            }
                        }

                        // ── Agent processing ─────────────────────────────
                        let profile_id = state_clone
                            .find_profile_by_platform_id(&msg_clone.platform_id)
                            .await;

                        let agent_result = state_clone
                            .build_agent_with_context_extended(
                                Some(&msg_clone.sender.user_id),
                                Some(&msg_clone.session_id),
                                None,
                                false,
                                profile_id.as_deref(),
                                conversation_id.as_deref(),
                            )
                            .await;

                        match agent_result {
                            Ok(mut agent) => {
                                let cancel_token = tokio_util::sync::CancellationToken::new();
                                let cancel_clone = cancel_token.clone();
                                {
                                    let mut tasks = state_clone.running_agent_tasks.write().await;
                                    tasks
                                        .insert(msg_clone.session_id.clone(), cancel_token.clone());
                                }

                                agent.set_cancel_token(cancel_token);

                                // ── Tool execution notification channel ──
                                // Set up a channel so the agent can notify us when tools
                                // are executing. We forward these as user-visible messages
                                // so the user knows work is in progress.
                                let (tool_notify_tx, mut tool_notify_rx) =
                                    tokio::sync::mpsc::unbounded_channel::<(String, String)>();
                                agent.set_tool_notify_tx(tool_notify_tx);
                                let pm_notify = pm_clone.clone();
                                let msg_notify = msg_clone.clone();
                                let cancel_notify = cancel_clone.clone();
                                let _notify_handle = tokio::spawn(async move {
                                    loop {
                                        tokio::select! {
                                            Some((tool_name, _args_preview)) = tool_notify_rx.recv() => {
                                                let status_msg = friendly_tool_message(&tool_name);
                                                let pm = pm_notify.read().await;
                                                let _ = pm
                                                    .send_text_to_platform(
                                                        &msg_notify.platform_id,
                                                        msg_notify.message_type,
                                                        &msg_notify.session_id,
                                                        &status_msg,
                                                    )
                                                    .await;
                                            }
                                            _ = cancel_notify.cancelled() => break,
                                            else => break,
                                        }
                                    }
                                });

                                // Build user message from platform message
                                let user_msg = {
                                    let mut parts: Vec<types::ContentPart> = Vec::new();

                                    for comp in &msg_clone.components {
                                        if let platform::types::MessageComponent::Image { url } =
                                            comp
                                        {
                                            parts.push(types::ContentPart {
                                                part_type: types::ContentPartType::ImageUrl,
                                                text: None,
                                                image_url: Some(types::ImageUrl {
                                                    url: url.clone(),
                                                    detail: None,
                                                }),
                                                image_data: None,
                                            });
                                        }
                                    }

                                    for comp in &msg_clone.components {
                                        if let platform::types::MessageComponent::File {
                                            name,
                                            url,
                                        } = comp
                                        {
                                            parts.push(types::ContentPart {
                                                part_type: types::ContentPartType::Text,
                                                text: Some(format!(
                                                    "[File attached: {}]({})",
                                                    name, url
                                                )),
                                                image_url: None,
                                                image_data: None,
                                            });
                                        }
                                    }

                                    if parts.is_empty() {
                                        types::ChatMessage::user(&msg_clone.message_str)
                                    } else {
                                        parts.push(types::ContentPart {
                                            part_type: types::ContentPartType::Text,
                                            text: Some(msg_clone.message_str.clone()),
                                            image_url: None,
                                            image_data: None,
                                        });
                                        types::ChatMessage {
                                            role: types::MessageRole::User,
                                            content: Some(types::MessageContent::Parts(parts)),
                                            name: None,
                                            tool_calls: None,
                                            tool_call_id: None,
                                        }
                                    }
                                };

                                // Run the agent with cancellation support.
                                let result = tokio::select! {
                                    _ = cancel_clone.cancelled() => {
                                        tracing::info!(
                                            session_id = %msg_clone.session_id,
                                            "Agent task was cancelled via /stop"
                                        );
                                        let pm = pm_clone.read().await;
                                        let _ = pm
                                            .send_text_to_platform(
                                                &msg_clone.platform_id,
                                                msg_clone.message_type,
                                                &msg_clone.session_id,
                                                "⏹ 任务已停止。",
                                            )
                                            .await;
                                        None
                                    }
                                    response = agent.chat_with_message(user_msg) => Some(response),
                                };

                                // Remove the cancellation token when done
                                {
                                    let mut tasks = state_clone.running_agent_tasks.write().await;
                                    tasks.remove(&msg_clone.session_id);
                                }

                                if let Some(response_result) = result {
                                    match response_result {
                                        Ok(response) => {
                                            let text = response
                                                .choices
                                                .first()
                                                .and_then(|c| c.message.content.as_ref())
                                                .and_then(|c| c.as_text_full())
                                                .unwrap_or_default();

                                            if !text.is_empty() {
                                                if let Some(ref conv_id) = conversation_id {
                                                    let conv_db =
                                                        state_clone.conversation_db.read().await;
                                                    if let Some(db) = conv_db.as_ref() {
                                                        if let Err(e) = db
                                                            .add_message(conversation::models::AddMessageRequest {
                                                                conversation_id: conv_id.clone(),
                                                                role: "assistant".to_string(),
                                                                content: text.clone(),
                                                            })
                                                            .await
                                                        {
                                                            tracing::error!(
                                                                error = %e,
                                                                "Failed to add assistant message to conversation database"
                                                            );
                                                        }
                                                    }
                                                }

                                                // ── Segmented Reply ────────────────────────
                                                // Check the config profile for this platform
                                                let (seg_enabled, seg_interval) = {
                                                    let profiles =
                                                        state_clone.config_profiles.read().await;
                                                    profiles
                                                        .values()
                                                        .filter(|p| {
                                                            p.is_active
                                                                && p.enable
                                                                && p.platform_ids.contains(
                                                                    &msg_clone.platform_id,
                                                                )
                                                        })
                                                        .next()
                                                        .map(|p| {
                                                            (
                                                                p.segmented_reply_enabled,
                                                                p.segmented_reply_interval_ms,
                                                            )
                                                        })
                                                        .unwrap_or((false, 500))
                                                };

                                                if seg_enabled {
                                                    let segments =
                                                        crate::types::split_text_into_segments(
                                                            &text,
                                                        );
                                                    let interval = std::time::Duration::from_millis(
                                                        seg_interval,
                                                    );
                                                    for segment in segments {
                                                        let pm = pm_clone.read().await;
                                                        if let Err(e) = pm
                                                            .send_text_to_platform(
                                                                &msg_clone.platform_id,
                                                                msg_clone.message_type,
                                                                &msg_clone.session_id,
                                                                &segment,
                                                            )
                                                            .await
                                                        {
                                                            tracing::error!(
                                                                error = %e,
                                                                "Failed to send segmented reply to platform"
                                                            );
                                                        }
                                                        drop(pm);
                                                        tokio::time::sleep(interval).await;
                                                    }
                                                } else {
                                                    let pm = pm_clone.read().await;
                                                    if let Err(e) = pm
                                                        .send_text_to_platform(
                                                            &msg_clone.platform_id,
                                                            msg_clone.message_type,
                                                            &msg_clone.session_id,
                                                            &text,
                                                        )
                                                        .await
                                                    {
                                                        tracing::error!(
                                                            error = %e,
                                                            "Failed to send reply to platform"
                                                        );
                                                    }
                                                }
                                            } else {
                                                tracing::warn!(
                                                    session_id = %msg_clone.session_id,
                                                    "Agent returned empty response for platform message"
                                                );
                                                let pm = pm_clone.read().await;
                                                let _ = pm
                                                    .send_text_to_platform(
                                                        &msg_clone.platform_id,
                                                        msg_clone.message_type,
                                                        &msg_clone.session_id,
                                                        "（AI 未返回有效回复，请重试）",
                                                    )
                                                    .await;
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                error = %e,
                                                "Agent failed to process platform message"
                                            );
                                            let error_reply = {
                                                let profiles =
                                                    state_clone.config_profiles.read().await;
                                                profiles
                                                    .values()
                                                    .filter(|p| p.is_active && p.enable)
                                                    .find_map(|p| p.custom_error_message.clone())
                                                    .unwrap_or_else(|| e.to_string())
                                            };
                                            let pm = pm_clone.read().await;
                                            let _ = pm
                                                .send_text_to_platform(
                                                    &msg_clone.platform_id,
                                                    msg_clone.message_type,
                                                    &msg_clone.session_id,
                                                    &error_reply,
                                                )
                                                .await;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "Failed to build agent for platform message");
                                let error_reply = {
                                    let profiles = state_clone.config_profiles.read().await;
                                    profiles
                                        .values()
                                        .filter(|p| p.is_active && p.enable)
                                        .find_map(|p| p.custom_error_message.clone())
                                        .unwrap_or_else(|| e.to_string())
                                };
                                let pm = pm_clone.read().await;
                                let _ = pm
                                    .send_text_to_platform(
                                        &msg_clone.platform_id,
                                        msg_clone.message_type,
                                        &msg_clone.session_id,
                                        &error_reply,
                                    )
                                    .await;
                            }
                        }
                    });
                }
                platform::PlatformEvent::StatusChanged {
                    platform_id,
                    status,
                } => {
                    tracing::info!(
                        platform_id = %platform_id,
                        status = %status,
                        "Platform adapter status changed"
                    );
                    // After a status change (e.g. re-login after session timeout),
                    // check if any adapter has updated credentials to persist.
                    state_for_platform.persist_adapter_credentials().await;
                }
                platform::PlatformEvent::Error {
                    platform_id,
                    message,
                } => {
                    tracing::error!(
                        platform_id = %platform_id,
                        error = %message,
                        "Platform adapter error"
                    );
                    // On error events (e.g. session timeout, re-login failure),
                    // also try to persist any credentials that might have been updated.
                    state_for_platform.persist_adapter_credentials().await;
                }
            }
        }
    });

    // ── Periodic credential persistence for platform adapters ────
    // Some adapters (e.g. WeChat after re-login) update credentials
    // inside spawned tasks. The event-based persistence above covers
    // most cases, but we also persist periodically as a safety net
    // to ensure no credentials are lost.
    {
        let state_for_persist = state.clone();
        let token = global_shutdown_token.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                        state_for_persist.persist_adapter_credentials().await;
                    }
                }
            }
        });
    }

    // ── Watch platforms.yaml for changes (hot-reload) ────────────
    {
        let state_for_watcher = state.clone();
        let platforms_path = api::state::ruri_config_dir().join("platforms.yaml");
        let token = global_shutdown_token.clone();

        tokio::spawn(async move {
            // Use a simple polling approach for file watching
            // Check every 5 seconds for file modification time changes
            let mut last_modified: Option<std::time::SystemTime> = None;

            // Initialize with the current modification time
            if let Ok(metadata) = std::fs::metadata(&platforms_path) {
                last_modified = metadata.modified().ok();
            }

            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
                }

                let current_modified = std::fs::metadata(&platforms_path)
                    .ok()
                    .and_then(|m| m.modified().ok());

                match (last_modified, current_modified) {
                    (Some(last), Some(current)) if current > last => {
                        tracing::info!("Detected change in platforms.yaml, reloading...");

                        // Reload platform configs into memory
                        state_for_watcher.load_platforms_config().await;

                        // Sync adapters with their enable state
                        state_for_watcher.sync_platforms().await;

                        tracing::info!("Platforms hot-reloaded successfully");

                        last_modified = Some(current);
                    }
                    (_, Some(current)) => {
                        // Update last_modified even if no change detected
                        last_modified = Some(current);
                    }
                    _ => {}
                }
            }
        });
    }

    // ── Watch config.json for changes (hot-reload) ─────────────
    {
        let state_for_watcher = state.clone();
        let config_path = state.config_path.clone();
        let token = global_shutdown_token.clone();

        tokio::spawn(async move {
            // Use a simple polling approach for file watching
            // Check every 5 seconds for file modification time changes
            let mut last_modified: Option<std::time::SystemTime> = None;

            // Initialize with the current modification time
            if let Ok(metadata) = std::fs::metadata(&config_path) {
                last_modified = metadata.modified().ok();
            }

            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
                }

                let current_modified = std::fs::metadata(&config_path)
                    .ok()
                    .and_then(|m| m.modified().ok());

                match (last_modified, current_modified) {
                    (Some(last), Some(current)) if current > last => {
                        tracing::info!("Detected change in config.json, reloading...");

                        match state_for_watcher.reload_config_from_file().await {
                            Ok(()) => {
                                // Sync platforms with their enable state
                                state_for_watcher.sync_platforms().await;

                                // Update command dispatcher state - merge from all active profiles
                                {
                                    let profiles = state_for_watcher.config_profiles.read().await;
                                    let active_profiles: Vec<_> = profiles
                                        .values()
                                        .filter(|p| p.is_active && p.enable)
                                        .collect();
                                    if !active_profiles.is_empty() {
                                        let mut merged_enabled_commands: Vec<String> = Vec::new();
                                        let mut merged_command_admin_required:
                                            std::collections::HashMap<String, bool> =
                                            std::collections::HashMap::new();
                                        let effective_prefix =
                                            active_profiles[0].command_prefix.clone();

                                        for profile in &active_profiles {
                                            for cmd in &profile.enabled_commands {
                                                if !merged_enabled_commands.contains(cmd) {
                                                    merged_enabled_commands.push(cmd.clone());
                                                }
                                            }
                                            for (cmd, admin_req) in &profile.command_admin_required
                                            {
                                                merged_command_admin_required
                                                    .insert(cmd.clone(), *admin_req);
                                            }
                                        }

                                        let mut dispatcher =
                                            state_for_watcher.command_dispatcher.write().await;
                                        dispatcher.set_prefix(effective_prefix);
                                        dispatcher.set_enabled_commands(merged_enabled_commands);
                                        drop(dispatcher);
                                        let mut computer_use_config =
                                            state_for_watcher.computer_use_config.write().await;
                                        computer_use_config.command_admin_required =
                                            merged_command_admin_required;
                                    }
                                }

                                tracing::info!("Config hot-reloaded successfully");
                            }
                            Err(e) => {
                                tracing::warn!("Failed to reload config: {}", e);
                            }
                        }

                        last_modified = Some(current);
                    }
                    (_, Some(current)) => {
                        // Update last_modified even if no change detected
                        last_modified = Some(current);
                    }
                    _ => {}
                }
            }
        });
    }

    // ── Create the API router ────────────────────────────────────
    let api_router = api::create_router(state.clone());

    // ── Create the full app with API routes and static file serving ─
    // CORS middleware for cross-origin cookie support (dev: frontend 8080 -> backend 3000)
    use tower_http::cors::{AllowOrigin, CorsLayer};
    let cors_middleware = CorsLayer::new()
        .allow_origin(AllowOrigin::mirror_request())
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::PATCH,
            axum::http::Method::OPTIONS,
            axum::http::Method::HEAD,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::COOKIE,
            header::ACCEPT,
            header::ORIGIN,
            header::REFERER,
        ])
        .allow_credentials(true);

    let app = Router::new()
        .merge(api_router)
        .fallback(static_handler)
        .layer(cors_middleware);

    // ── Start the server ─────────────────────────────────────────
    let bind_addr = if args.remote { "0.0.0.0" } else { "127.0.0.1" };
    let addr = format!("{}:{}", bind_addr, args.port)
        .parse::<std::net::SocketAddr>()
        .expect("failed to parse bind address");
    if args.remote {
        tracing::info!(
            "🌐 WebUI:  http://0.0.0.0:{} (accessible from network)",
            args.port
        );
        tracing::info!(
            "📡 API:    http://0.0.0.0:{}/api (accessible from network)",
            args.port
        );
    } else {
        tracing::info!("🌐 WebUI:  http://localhost:{}", args.port);
        tracing::info!("📡 API:    http://localhost:{}/api", args.port);
    }
    tracing::info!("");
    tracing::info!("Available API endpoints:");
    tracing::info!("  POST   /api/chat              Send a chat message");
    tracing::info!("  GET    /api/chat/history       Get chat history");
    tracing::info!("  DELETE /api/chat/history       Clear chat history");
    tracing::info!("  GET    /api/conversations      List conversations");
    tracing::info!("  POST   /api/conversations      Create conversation");
    tracing::info!("  GET    /api/conversations/:id  Get conversation");
    tracing::info!("  DELETE /api/conversations/:id  Delete conversation");
    tracing::info!("  POST   /api/conversations/:id/messages  Add message");
    tracing::info!("  GET    /api/conversations/:id/messages  Get messages");
    tracing::info!("  GET    /api/providers          List providers");
    tracing::info!("  POST   /api/providers          Create provider");
    tracing::info!("  GET    /api/providers/:id      Get provider");
    tracing::info!("  PUT    /api/providers/:id      Update provider");
    tracing::info!("  DELETE /api/providers/:id      Delete provider");
    tracing::info!("  POST   /api/providers/:id/activate  Set active provider");
    tracing::info!("  GET    /api/skills             List skills");
    tracing::info!("  POST   /api/skills             Add skill");
    tracing::info!("  POST   /api/skills/upload     Upload skill package (ZIP)");
    tracing::info!("  DELETE /api/skills/:name       Remove skill");
    tracing::info!("  PATCH  /api/skills/:name       Toggle skill");
    tracing::info!("  GET    /api/tools              List tools");
    tracing::info!("  GET    /api/agent/status       Get agent status");
    tracing::info!("  GET    /api/acp/config         Get ACP mode config");
    tracing::info!("  PUT    /api/acp/config         Update ACP mode config");
    tracing::info!("  GET    /api/platforms           List platforms");
    tracing::info!("  POST   /api/platforms           Create platform");
    tracing::info!("  GET    /api/platforms/:id       Get platform");
    tracing::info!("  PUT    /api/platforms/:id       Update platform");
    tracing::info!("  DELETE /api/platforms/:id       Delete platform");
    tracing::info!("  POST   /api/platforms/:id/restart Restart platform adapter");
    tracing::info!("  POST   /api/system/restart     Restart server");
    tracing::info!("");
    tracing::info!("ACP (Agent Client Protocol) mode:");
    tracing::info!("  Run with --acp to start in ACP mode (stdio transport)");
    tracing::info!("  Compatible with Zed, JetBrains, and other ACP clients");

    // Use SO_REUSEADDR to avoid "address already in use" on Windows when
    // restarting the server (port may still be in TIME_WAIT from old process).
    let socket = match addr {
        std::net::SocketAddr::V4(_) => TcpSocket::new_v4()?,
        std::net::SocketAddr::V6(_) => TcpSocket::new_v6()?,
    };
    socket.set_reuseaddr(true)?;
    socket.bind(addr)?;
    let listener = socket.listen(1024)?;

    // Use graceful shutdown so that `restart_system` can trigger a clean
    // server teardown before re-executing the binary. Also listen for
    // SIGTERM/SIGINT (Ctrl+C) to ensure the TCP listener is released,
    // child processes are cleaned up, and platform adapters are stopped.
    let shutdown_rx = state.server_shutdown_rx.clone();
    let platform_manager_for_shutdown = state.platform_manager.clone();
    let shutdown_token = global_shutdown_token.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let mut rx = shutdown_rx;

            // Build the shutdown signal future
            let api_shutdown = async {
                let _ = rx.changed().await;
                tracing::info!("Server shutdown triggered via API");
            };
            let ctrl_c_signal = async {
                let _ = tokio::signal::ctrl_c().await;
                tracing::info!("Server shutdown triggered via Ctrl+C / SIGINT");
            };

            #[cfg(unix)]
            let sigterm_signal = async {
                use tokio::signal::unix::{SignalKind, signal};
                let mut sigterm =
                    signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
                sigterm.recv().await;
                tracing::info!("Server shutdown triggered via SIGTERM");
            };

            #[cfg(unix)]
            {
                tokio::select! {
                    _ = api_shutdown => {},
                    _ = ctrl_c_signal => {},
                    _ = sigterm_signal => {},
                }
            }

            #[cfg(not(unix))]
            {
                tokio::select! {
                    _ = api_shutdown => {},
                    _ = ctrl_c_signal => {},
                }
            }

            // ── Signal all background tasks to stop ──────────────
            shutdown_token.cancel();

            // ── Shut down all platform adapters ──────────────────
            // This releases their connections (TCP, WebSocket, etc.)
            // and ensures clean termination.
            platform_manager_for_shutdown
                .write()
                .await
                .shutdown_all()
                .await;

            tracing::info!("Main HTTP server shutting down gracefully");
        })
        .await?;

    // After graceful shutdown, check if a restart was requested.
    // We do this here in `main()` rather than in a spawned task so that
    // the restart logic runs independently of the tokio runtime lifecycle.
    if state
        .restart_requested
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        tracing::info!("Restarting Ruri server...");

        let current_exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(e) => {
                tracing::error!("Failed to get current executable path: {}", e);
                return Ok(());
            }
        };

        let args: Vec<String> = std::env::args().skip(1).collect();

        // On Unix, use exec() to replace the current process with the new one.
        // This keeps the same terminal session so logs continue in the same
        // terminal window instead of running as a detached background process.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let error = std::process::Command::new(&current_exe).args(&args).exec();
            // exec() only returns on error
            tracing::error!("Failed to restart server (exec): {}", error);
        }

        #[cfg(not(unix))]
        {
            // On Windows, spawn the new process and then let the current
            // process terminate naturally (falling through to Ok(())).
            // The child inherits the parent's console by default, so it
            // continues running in the same terminal window.
            // We avoid std::process::exit(0) because it skips destructors
            // and may prevent proper console cleanup (e.g. Ctrl+C handler
            // restoration).
            match std::process::Command::new(&current_exe).args(&args).spawn() {
                Ok(child) => {
                    let pid = child.id();
                    tracing::info!(
                        "New server instance started (PID: {pid}), shutting down current instance"
                    );
                    // Detach the child handle so it continues running independently.
                    // On Windows, detaching allows the child to outlive the parent.
                    drop(child);
                }
                Err(e) => {
                    tracing::error!("Failed to restart server: {}", e);
                }
            }
            // Fall through to Ok(()) — let the process terminate naturally.
            // This ensures destructors run, console handlers are cleaned up,
            // and the TCP socket is properly released.
        }
    }

    Ok(())
}

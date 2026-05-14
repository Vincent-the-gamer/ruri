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

    // Spawn a task to process incoming platform messages and route them to the agent.
    let state_for_platform = state.clone();
    let platform_manager_ref = state.platform_manager.clone();
    tokio::spawn(async move {
        // Get the initial event receiver
        let mut event_receiver = {
            let mut pm = platform_manager_ref.write().await;
            pm.take_event_receiver()
                .expect("event receiver should be available at startup")
        };

        while let Some(event) = event_receiver.recv().await {
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
                        let dispatcher = state_for_platform.command_dispatcher.read().await;
                        let cmd_ctx = command::CommandContext {
                            raw_message: msg.message_str.clone(),
                            command_name: String::new(), // filled by dispatch()
                            args: String::new(),
                            prefix: dispatcher.prefix().to_string(),
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

                    // ── Conversation DB: ensure conversation exists ────────
                    let chat_type = match msg.message_type {
                        platform::types::MessageType::GroupMessage => {
                            conversation::models::ChatType::Group
                        }
                        platform::types::MessageType::FriendMessage => {
                            conversation::models::ChatType::Private
                        }
                    };
                    let conversation_id = {
                        let conv_db = state_for_platform.conversation_db.read().await;
                        if let Some(db) = conv_db.as_ref() {
                            match db
                                .get_or_create_conversation(
                                    msg.platform_id.clone(),
                                    chat_type,
                                    msg.session_id.clone(),
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
                        let conv_db = state_for_platform.conversation_db.read().await;
                        if let Some(db) = conv_db.as_ref() {
                            if let Err(e) = db
                                .add_message(conversation::models::AddMessageRequest {
                                    conversation_id: conv_id.clone(),
                                    role: "user".to_string(),
                                    content: msg.message_str.clone(),
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
                    // Build an agent using the config profile that owns this platform.
                    // Each platform instance is bound to exactly one config profile,
                    // so we resolve the profile by platform_id to ensure full context
                    // isolation (provider, persona, skills, knowledge bases, proxy, etc.).
                    let profile_id = state_for_platform
                        .find_profile_by_platform_id(&msg.platform_id)
                        .await;

                    let agent_result = state_for_platform
                        .build_agent_with_context_extended(
                            Some(&msg.sender.user_id),
                            Some(&msg.session_id),
                            None,
                            None,
                            false, // use_debug_session: false — platform messages use profile config
                            profile_id.as_deref(),
                        )
                        .await;

                    match agent_result {
                        Ok(mut agent) => {
                            // Register a cancellation token for this session so /stop can cancel it
                            let cancel_token = tokio_util::sync::CancellationToken::new();
                            let cancel_clone = cancel_token.clone();
                            {
                                let mut tasks =
                                    state_for_platform.running_agent_tasks.write().await;
                                tasks.insert(msg.session_id.clone(), cancel_token);
                            }

                            // Build user message from platform message (may include images/files)
                            let user_msg = {
                                let mut parts: Vec<types::ContentPart> = Vec::new();

                                // Add images as image_url parts
                                for comp in &msg.components {
                                    if let platform::types::MessageComponent::Image { url } = comp {
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

                                // Add file descriptions as text parts
                                for comp in &msg.components {
                                    if let platform::types::MessageComponent::File { name, url } =
                                        comp
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
                                    types::ChatMessage::user(&msg.message_str)
                                } else {
                                    // Add the user's text message as the last part
                                    parts.push(types::ContentPart {
                                        part_type: types::ContentPartType::Text,
                                        text: Some(msg.message_str.clone()),
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

                            // Run the agent chat non-streaming with cancellation support.
                            let result = tokio::select! {
                                _ = cancel_clone.cancelled() => {
                                    // Task was cancelled via /stop
                                    tracing::info!(
                                        session_id = %msg.session_id,
                                        "Agent task was cancelled via /stop"
                                    );
                                    let pm = platform_manager_ref.read().await;
                                    let _ = pm
                                        .send_text_to_platform(
                                            &msg.platform_id,
                                            msg.message_type,
                                            &msg.session_id,
                                            "⏹ 任务已停止。",
                                        )
                                        .await;
                                    None
                                }
                                response = agent.chat_with_message(user_msg) => Some(response),
                            };

                            // Remove the cancellation token when done
                            {
                                let mut tasks =
                                    state_for_platform.running_agent_tasks.write().await;
                                tasks.remove(&msg.session_id);
                            }

                            // Process the response (None means cancelled)
                            if let Some(response_result) = result {
                                match response_result {
                                    Ok(response) => {
                                        // Extract text from response
                                        let text = response
                                            .choices
                                            .first()
                                            .and_then(|c| c.message.content.as_ref())
                                            .and_then(|c| c.as_text_full())
                                            .unwrap_or_default();

                                        if !text.is_empty() {
                                            // Save assistant message to conversation database
                                            if let Some(ref conv_id) = conversation_id {
                                                let conv_db =
                                                    state_for_platform.conversation_db.read().await;
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

                                            // Send to platform
                                            let pm = platform_manager_ref.read().await;
                                            if let Err(e) = pm
                                                .send_text_to_platform(
                                                    &msg.platform_id,
                                                    msg.message_type,
                                                    &msg.session_id,
                                                    &text,
                                                )
                                                .await
                                            {
                                                tracing::error!(
                                                    error = %e,
                                                    "Failed to send reply to platform"
                                                );
                                            }
                                        } else {
                                            // Empty response
                                            tracing::warn!(
                                                session_id = %msg.session_id,
                                                "Agent returned empty response for platform message"
                                            );
                                            let pm = platform_manager_ref.read().await;
                                            let _ = pm
                                                .send_text_to_platform(
                                                    &msg.platform_id,
                                                    msg.message_type,
                                                    &msg.session_id,
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
                                                state_for_platform.config_profiles.read().await;
                                            profiles
                                                .values()
                                                .filter(|p| p.is_active && p.enable)
                                                .find_map(|p| p.custom_error_message.clone())
                                                .unwrap_or_else(|| e.to_string())
                                        };
                                        let pm = platform_manager_ref.read().await;
                                        let _ = pm
                                            .send_text_to_platform(
                                                &msg.platform_id,
                                                msg.message_type,
                                                &msg.session_id,
                                                &error_reply,
                                            )
                                            .await;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to build agent for platform message");
                            // Send error reply to user
                            let error_reply = {
                                let profiles = state_for_platform.config_profiles.read().await;
                                profiles
                                    .values()
                                    .filter(|p| p.is_active && p.enable)
                                    .find_map(|p| p.custom_error_message.clone())
                                    .unwrap_or_else(|| e.to_string())
                            };
                            let pm = platform_manager_ref.read().await;
                            let _ = pm
                                .send_text_to_platform(
                                    &msg.platform_id,
                                    msg.message_type,
                                    &msg.session_id,
                                    &error_reply,
                                )
                                .await;
                        }
                    }
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
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                state_for_persist.persist_adapter_credentials().await;
            }
        });
    }

    // ── Watch platforms.yaml for changes (hot-reload) ────────────
    {
        let state_for_watcher = state.clone();
        let platforms_path = api::state::ruri_config_dir().join("platforms.yaml");

        tokio::spawn(async move {
            // Use a simple polling approach for file watching
            // Check every 5 seconds for file modification time changes
            let mut last_modified: Option<std::time::SystemTime> = None;

            // Initialize with the current modification time
            if let Ok(metadata) = std::fs::metadata(&platforms_path) {
                last_modified = metadata.modified().ok();
            }

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

        tokio::spawn(async move {
            // Use a simple polling approach for file watching
            // Check every 5 seconds for file modification time changes
            let mut last_modified: Option<std::time::SystemTime> = None;

            // Initialize with the current modification time
            if let Ok(metadata) = std::fs::metadata(&config_path) {
                last_modified = metadata.modified().ok();
            }

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

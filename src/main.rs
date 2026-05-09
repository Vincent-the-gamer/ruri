//! Ruri - AI Agent application
mod acp;
mod agent;
mod api;
mod computer_use;
mod conversation;
mod db;
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
use rust_embed::RustEmbed;
use std::sync::Arc;

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
    // ── Check for ACP mode ──────────────────────────────────────
    let args: Vec<String> = std::env::args().collect();
    let acp_mode = args.iter().any(|arg| arg == "--acp");

    // Check for --acp-config <path> to override the config file location
    let acp_config_path = {
        let mut path = None;
        let mut iter = args.iter().peekable();
        while let Some(arg) = iter.next() {
            if arg == "--acp-config" {
                path = iter.next().map(std::path::PathBuf::from);
                break;
            }
        }
        path
    };

    // Initialize logging
    if acp_mode {
        // ACP mode: logging goes to stderr so it doesn't interfere with JSON-RPC on stdout
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .with_writer(std::io::stderr)
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
    let log_manager = logging::init_logging(1000);

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
        }
        Err(e) => {
            tracing::warn!("Failed to initialize unified database: {}", e);
            tracing::warn!("All database features will be unavailable");
        }
    }

    // ── Initialize chat platform adapters ────────────────────────
    // Only start adapters that are listed in the active config profile's
    // `active_platform_ids`. Platform configs are already loaded into
    // `state.platform_configs` by `load_platforms_config()` above.
    {
        state.sync_platforms_with_active_profile().await;

        let pm = state.platform_manager.read().await;
        if !pm.is_empty() {
            tracing::info!("Active platform adapters: {}", pm.len());
        } else {
            tracing::info!("No platform adapters configured in active profile");
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

                    // Build an agent and process the incoming message
                    match state_for_platform
                        .build_agent_with_context(
                            Some(&msg.sender.user_id),
                            Some(&msg.session_id),
                            None,
                        )
                        .await
                    {
                        Ok(mut agent) => {
                            match agent.chat(&msg.message_str).await {
                                Ok(response) => {
                                    if let Some(content) = response
                                        .choices
                                        .first()
                                        .and_then(|c| c.message.content.as_ref())
                                        .and_then(|c| c.as_text())
                                    {
                                        tracing::info!(
                                            response_len = content.len(),
                                            "Agent replied to platform message"
                                        );
                                        // Send the reply back to the originating platform
                                        let pm = platform_manager_ref.read().await;
                                        if let Err(e) = pm
                                            .send_text_to_platform(
                                                &msg.platform_id,
                                                msg.message_type,
                                                &msg.session_id,
                                                content,
                                            )
                                            .await
                                        {
                                            tracing::error!(
                                                error = %e,
                                                "Failed to send reply to platform"
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "Agent failed to process platform message");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to build agent for platform message");
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
                }
            }
        }
    });

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

                        // Sync adapters with the active profile (only
                        // start/stop those that differ)
                        state_for_watcher.sync_platforms_with_active_profile().await;

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

    // ── Create the API router ────────────────────────────────────
    let api_router = api::create_router(state.clone());

    // ── Create the full app with API routes and static file serving ─
    let app = Router::new().merge(api_router).fallback(static_handler);

    // ── Start the server ─────────────────────────────────────────
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("🌐 WebUI:  http://localhost:3000");
    tracing::info!("📡 API:    http://localhost:3000/api");
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

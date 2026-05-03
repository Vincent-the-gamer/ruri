//! Ruri - AI Agent application
#![allow(dead_code)] // Allow unused code for future features

mod acp;
mod agent;
mod api;
mod computer_use;
mod logging;
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
    tracing::info!("");
    tracing::info!("ACP (Agent Client Protocol) mode:");
    tracing::info!("  Run with --acp to start in ACP mode (stdio transport)");
    tracing::info!("  Compatible with Zed, JetBrains, and other ACP clients");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

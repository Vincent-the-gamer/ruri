//! OneBot v12 Connect server implementation.
//!
//! Implements the OneBot Connect communication methods:
//! - HTTP server for action calls (with optional event polling via `get_latest_events`)
//! - Forward WebSocket server for action + event
//! - HTTP Webhook event pushing

use crate::platform::onebot12::config::{HttpConfig, HttpWebhookConfig, WsConfig};
use crate::platform::onebot12::types::{
    ActionRequest, ActionResponse, Ob12Event, Ob12Self, retcode,
};
use axum::Json;
use axum::extract::ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::watch;
use tokio::sync::{Mutex, broadcast};

// ─── Callback type ──────────────────────────────────────────

/// Async callback for handling platform-specific actions (e.g. `send_message`).
pub type ActionCallback = Arc<
    dyn Fn(
            ActionRequest,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ActionResponse> + Send>>
        + Send
        + Sync,
>;

// ─── Server state ───────────────────────────────────────────

/// Shared state for the OneBot v12 server.
pub struct Ob12ServerState {
    /// Bot identity for this instance.
    pub bot_self: Ob12Self,
    /// Access token for authentication.
    pub access_token: Option<String>,
    /// Broadcast sender for events. Receivers subscribe to this.
    pub event_sender: broadcast::Sender<String>,
    /// Event buffer for HTTP polling (`get_latest_events`).
    pub event_buffer: Mutex<EventBuffer>,
    /// Callback for platform-specific actions.
    pub action_callback: ActionCallback,
    /// HTTP client for webhook pushing.
    pub http_client: Client,
    /// Webhook URL, if configured.
    pub webhook_url: Option<String>,
    /// Whether event polling is enabled (HTTP config).
    pub event_polling_enabled: bool,
}

// ─── Event buffer ───────────────────────────────────────────

/// Circular event buffer for HTTP polling.
pub struct EventBuffer {
    buffer: VecDeque<String>,
    max_size: usize, // 0 = unlimited
}

impl EventBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: VecDeque::new(),
            max_size,
        }
    }

    pub fn push(&mut self, event_json: String) {
        if self.max_size > 0 && self.buffer.len() >= self.max_size {
            self.buffer.pop_front();
        }
        self.buffer.push_back(event_json);
    }

    /// Drain up to `limit` events (0 = all).
    pub fn drain(&mut self, limit: usize) -> Vec<String> {
        let count = if limit == 0 {
            self.buffer.len()
        } else {
            limit.min(self.buffer.len())
        };
        self.buffer.drain(..count).collect()
    }
}

// ─── Push event ─────────────────────────────────────────────

/// Push an event to all consumers (buffer, broadcast, webhook).
pub async fn push_event(state: &Arc<Ob12ServerState>, event: &Ob12Event) {
    let json = match serde_json::to_string(event) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!(error = %e, "Failed to serialize OneBot v12 event");
            return;
        }
    };

    // 1. Event buffer (for HTTP polling)
    {
        let mut buf = state.event_buffer.lock().await;
        buf.push(json.clone());
    }

    // 2. Broadcast (for WebSocket clients)
    let _ = state.event_sender.send(json.clone());

    // 3. Webhook push
    if let Some(ref url) = state.webhook_url {
        let client = state.http_client.clone();
        let url = url.clone();
        let json_clone = json.clone();
        tokio::spawn(async move {
            if let Err(e) = client
                .post(&url)
                .header("Content-Type", "application/json")
                .body(json_clone)
                .send()
                .await
            {
                tracing::warn!(url = %url, error = %e, "Webhook push failed");
            }
        });
    }
}

// ─── Auth helper ────────────────────────────────────────────

#[derive(Deserialize)]
struct AccessTokenQuery {
    access_token: Option<String>,
}

fn check_auth(headers: &HeaderMap, query: &AccessTokenQuery, expected: Option<&str>) -> bool {
    let Some(token) = expected else {
        return true;
    };
    if token.is_empty() {
        return true;
    }

    // Authorization header
    if let Some(auth) = headers.get("Authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if auth_str == format!("Bearer {}", token) {
                return true;
            }
        }
    }

    // Query param
    if let Some(ref t) = query.access_token {
        if t == token {
            return true;
        }
    }

    false
}

// ─── HTTP action endpoint ───────────────────────────────────

async fn http_action(
    State(state): State<Arc<Ob12ServerState>>,
    headers: HeaderMap,
    Query(query): Query<AccessTokenQuery>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // Auth
    if !check_auth(&headers, &query, state.access_token.as_deref()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ActionResponse::failed(
                retcode::BAD_REQUEST,
                "Unauthorized".into(),
            )),
        );
    }

    // Parse action request
    let request: ActionRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::OK,
                Json(ActionResponse::failed(
                    retcode::BAD_REQUEST,
                    format!("Invalid JSON: {}", e),
                )),
            );
        }
    };

    let response = handle_action_internal(&state, request).await;
    (StatusCode::OK, Json(response))
}

// ─── WebSocket endpoint ─────────────────────────────────────

async fn ws_upgrade(
    State(state): State<Arc<Ob12ServerState>>,
    headers: HeaderMap,
    Query(query): Query<AccessTokenQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if !check_auth(&headers, &query, state.access_token.as_deref()) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(socket: WebSocket, state: Arc<Ob12ServerState>) {
    let (ws_sender, mut ws_receiver) = socket.split();
    let ws_sender = Arc::new(Mutex::new(ws_sender));

    // Send connect event on establishment
    let connect_event = make_connect_event();
    if let Ok(json) = serde_json::to_string(&connect_event) {
        let mut s = ws_sender.lock().await;
        let _ = s.send(Message::Text(Utf8Bytes::from(json))).await;
    }

    // Subscribe to event broadcast
    let mut event_rx = state.event_sender.subscribe();

    // Spawn write task: forward broadcast events → WS client
    let write_sender = ws_sender.clone();
    let write_task = tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(json) => {
                    let mut s = write_sender.lock().await;
                    if s.send(Message::Text(Utf8Bytes::from(json))).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::debug!("WS event receiver lagged, skipped {} messages", skipped);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    // Read loop: receive action requests → handle → send response
    let read_state = state.clone();
    let read_sender = ws_sender.clone();
    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                let request: ActionRequest = match serde_json::from_str(&text) {
                    Ok(r) => r,
                    Err(e) => {
                        let resp = ActionResponse::failed(
                            retcode::BAD_REQUEST,
                            format!("Invalid JSON: {}", e),
                        );
                        if let Ok(json) = serde_json::to_string(&resp) {
                            let mut s = read_sender.lock().await;
                            let _ = s.send(Message::Text(Utf8Bytes::from(json))).await;
                        }
                        continue;
                    }
                };

                let echo = request.echo.clone();
                let mut response = handle_action_internal(&read_state, request).await;
                if let Some(e) = echo {
                    response.echo = Some(e);
                }

                if let Ok(json) = serde_json::to_string(&response) {
                    let mut s = read_sender.lock().await;
                    if s.send(Message::Text(Utf8Bytes::from(json))).await.is_err() {
                        break;
                    }
                }
            }
            Message::Close(_) => break,
            _ => continue,
        }
    }

    write_task.abort();
}

// ─── Action handler ─────────────────────────────────────────

/// Public entry point for action handling (used by reverse WS client).
pub async fn handle_action_public(
    state: &Arc<Ob12ServerState>,
    request: ActionRequest,
) -> ActionResponse {
    handle_action_internal(state, request).await
}

async fn handle_action_internal(
    state: &Arc<Ob12ServerState>,
    request: ActionRequest,
) -> ActionResponse {
    match request.action.as_str() {
        "get_supported_actions" => {
            let actions = vec![
                "get_supported_actions",
                "get_status",
                "get_version",
                "get_latest_events",
                "send_message",
                "delete_message",
                "get_self_info",
            ];
            ActionResponse::ok(serde_json::json!(actions))
        }

        "get_status" => ActionResponse::ok(serde_json::json!({
            "good": true,
            "bots": [{
                "self": state.bot_self,
                "online": true,
            }]
        })),

        "get_version" => ActionResponse::ok(serde_json::json!({
            "impl": "ruri",
            "version": env!("CARGO_PKG_VERSION"),
            "onebot_version": "12",
        })),

        "get_latest_events" => {
            if !state.event_polling_enabled {
                return ActionResponse::failed(
                    retcode::UNSUPPORTED_ACTION,
                    "Event polling not enabled".into(),
                );
            }
            let limit = request
                .params
                .get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as usize;
            let mut buf = state.event_buffer.lock().await;
            let events: Vec<serde_json::Value> = buf
                .drain(limit)
                .iter()
                .filter_map(|s| serde_json::from_str(&s).ok())
                .collect();
            ActionResponse::ok(serde_json::json!(events))
        }

        // Platform-specific actions delegate to the callback
        "send_message"
        | "delete_message"
        | "get_self_info"
        | "get_user_info"
        | "get_friend_list"
        | "get_group_info"
        | "get_group_list"
        | "get_group_member_info"
        | "get_group_member_list"
        | "set_group_name"
        | "leave_group" => (state.action_callback)(request).await,

        _ => ActionResponse::failed(
            retcode::UNSUPPORTED_ACTION,
            format!("Unknown action: {}", request.action),
        ),
    }
}

// ─── Connect event helper ───────────────────────────────────

fn make_connect_event() -> Ob12Event {
    Ob12Event {
        id: uuid::Uuid::new_v4().to_string(),
        time: chrono::Utc::now().timestamp_millis() as f64 / 1000.0,
        event_type: "meta".to_owned(),
        detail_type: "connect".to_owned(),
        sub_type: String::new(),
        self_: None,
        extra: {
            let mut m = serde_json::Map::new();
            m.insert(
                "version".to_owned(),
                serde_json::json!({
                    "impl": "ruri",
                    "version": env!("CARGO_PKG_VERSION"),
                    "onebot_version": "12",
                }),
            );
            m
        },
    }
}

// ─── Server starters ────────────────────────────────────────

/// Start an HTTP server for action calls and (optionally) event polling.
///
/// `shutdown_rx` is a watch receiver that triggers graceful shutdown when it
/// becomes `true`, allowing the TCP listener to be released cleanly.
pub async fn start_http_server(
    state: Arc<Ob12ServerState>,
    config: &HttpConfig,
    mut shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let app = axum::Router::new()
        .route("/", post(http_action))
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "OneBot v12 HTTP server listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.changed().await;
            tracing::info!(%addr, "OneBot v12 HTTP server shutting down");
        })
        .await?;
    Ok(())
}

/// Start a forward WebSocket server for action calls and event pushing.
///
/// `shutdown_rx` is a watch receiver that triggers graceful shutdown when it
/// becomes `true`, allowing the TCP listener to be released cleanly.
pub async fn start_ws_server(
    state: Arc<Ob12ServerState>,
    config: &WsConfig,
    mut shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let app = axum::Router::new()
        .route("/", axum::routing::get(ws_upgrade))
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "OneBot v12 WebSocket server listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.changed().await;
            tracing::info!(%addr, "OneBot v12 WebSocket server shutting down");
        })
        .await?;
    Ok(())
}

/// Start a background task that subscribes to events and pushes them
/// to the configured Webhook URL.
pub fn start_webhook_pusher(state: Arc<Ob12ServerState>, config: &HttpWebhookConfig) {
    let mut rx = state.event_sender.subscribe();
    let client = state.http_client.clone();
    let url = config.url.clone();

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(json) => {
                    if let Err(e) = client
                        .post(&url)
                        .header("Content-Type", "application/json")
                        .body(json)
                        .send()
                        .await
                    {
                        tracing::warn!(url = %url, error = %e, "Webhook push failed");
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::debug!("Webhook receiver lagged, skipped {} messages", skipped);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });
}

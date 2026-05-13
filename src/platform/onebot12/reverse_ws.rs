//! OneBot v12 reverse WebSocket client implementation.
//!
//! Connects to a configured URL as a WebSocket client, providing
//! both action and event service over the connection.

use crate::platform::onebot12::config::WsReverseConfig;
use crate::platform::onebot12::server::Ob12ServerState;
use crate::platform::onebot12::types::{ActionRequest, ActionResponse, Ob12Event, retcode};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::{Utf8Bytes, client::IntoClientRequest};

/// Start a reverse WebSocket client that connects to the configured URL.
///
/// This function runs in a loop, reconnecting with the configured interval
/// when the connection drops. Uses exponential backoff on consecutive failures
/// (capped at 60 s) and resets the backoff on a successful connection.
pub async fn run_reverse_ws(
    state: Arc<Ob12ServerState>,
    config: &WsReverseConfig,
) -> anyhow::Result<()> {
    let base_delay_ms = config.reconnect_interval;
    let max_delay_ms = 60_000u64; // cap at 60 s
    let mut consecutive_failures: u32 = 0;

    loop {
        match connect_once(&state, config).await {
            Ok(()) => {
                consecutive_failures = 0;
                tracing::info!(url = %config.url, "Reverse WebSocket disconnected, reconnecting");
            }
            Err(e) => {
                consecutive_failures += 1;
                tracing::warn!(
                    url = %config.url,
                    error = %e,
                    failures = consecutive_failures,
                    "Reverse WebSocket connection failed, reconnecting"
                );
            }
        }

        // Exponential backoff: base * 2^(failures-1), capped at max_delay
        let delay_ms = if consecutive_failures == 0 {
            base_delay_ms
        } else {
            let exp = 2u64.saturating_pow(consecutive_failures.saturating_sub(1));
            (base_delay_ms * exp).min(max_delay_ms)
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }
}

async fn connect_once(
    state: &Arc<Ob12ServerState>,
    config: &WsReverseConfig,
) -> anyhow::Result<()> {
    let mut request = config.url.clone().into_client_request()?;

    // Set required headers per OneBot v12 spec
    let headers = request.headers_mut();
    headers.insert(
        "User-Agent",
        format!("OneBot/12 ruri/{}", env!("CARGO_PKG_VERSION"))
            .parse()
            .unwrap(),
    );
    headers.insert("Sec-WebSocket-Protocol", "12.ruri".parse().unwrap());
    if let Some(ref token) = state.access_token {
        if !token.is_empty() {
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
    }

    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| anyhow::anyhow!("WebSocket connect failed: {}", e))?;

    tracing::info!(url = %config.url, "Reverse WebSocket connected");

    let (ws_sender, mut ws_receiver) = ws_stream.split();
    let ws_sender = Arc::new(Mutex::new(ws_sender));

    // Send connect event
    let connect_event = make_connect_event();
    if let Ok(json) = serde_json::to_string(&connect_event) {
        let mut s = ws_sender.lock().await;
        let _ = s
            .send(tokio_tungstenite::tungstenite::Message::Text(
                Utf8Bytes::from(json),
            ))
            .await;
    }

    // Subscribe to event broadcast
    let mut event_rx = state.event_sender.subscribe();

    // Spawn write task: forward broadcast events → WS server
    let write_sender = ws_sender.clone();
    let write_task = tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(json) => {
                    let mut s = write_sender.lock().await;
                    if s.send(tokio_tungstenite::tungstenite::Message::Text(
                        Utf8Bytes::from(json),
                    ))
                    .await
                    .is_err()
                    {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::debug!(
                        "Reverse WS event receiver lagged, skipped {} messages",
                        skipped
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    // Read loop: receive action requests → handle → send response
    let read_state = state.clone();
    let read_sender = ws_sender.clone();
    while let Some(Ok(msg)) = ws_receiver.next().await {
        let text = match msg {
            tokio_tungstenite::tungstenite::Message::Text(t) => t,
            tokio_tungstenite::tungstenite::Message::Close(_) => break,
            _ => continue,
        };

        let request: ActionRequest = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(e) => {
                let resp =
                    ActionResponse::failed(retcode::BAD_REQUEST, format!("Invalid JSON: {}", e));
                if let Ok(json) = serde_json::to_string(&resp) {
                    let mut s = read_sender.lock().await;
                    let _ = s
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            Utf8Bytes::from(json),
                        ))
                        .await;
                }
                continue;
            }
        };

        let echo = request.echo.clone();
        let mut response =
            crate::platform::onebot12::server::handle_action_public(&read_state, request).await;
        if let Some(e) = echo {
            response.echo = Some(e);
        }

        if let Ok(json) = serde_json::to_string(&response) {
            let mut s = read_sender.lock().await;
            if s.send(tokio_tungstenite::tungstenite::Message::Text(
                Utf8Bytes::from(json),
            ))
            .await
            .is_err()
            {
                break;
            }
        }
    }

    write_task.abort();
    Ok(())
}

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

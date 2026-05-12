//! OneBot v12 platform adapter.
//!
//! This adapter acts as a OneBot v12 **implementation** — it starts
//! HTTP/WebSocket servers that OneBot applications can connect to.
//!
//! - Inbound platform messages are converted to OneBot v12 events
//!   and pushed to connected applications.
//! - OneBot v12 action requests (like `send_message`) are converted
//!   to Ruri's `OutboundMessage` and delivered through the Platform trait.

use crate::platform::onebot12::config::OneBot12Config;
use crate::platform::onebot12::server::{self, ActionCallback, Ob12ServerState};
use crate::platform::onebot12::types::{
    ActionRequest, ActionResponse, Ob12Event, Ob12Message, Ob12Self, Segment, retcode,
};
use crate::platform::trait_def::{Platform, PlatformEvent};
use crate::platform::types::{
    MessageComponent, MessageType, OutboundContent, OutboundMessage, PlatformMessage,
    PlatformStatus,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, watch};

/// OneBot v12 platform adapter.
///
/// Bridges between Ruri's internal message types and the OneBot v12 protocol,
/// allowing OneBot applications (NoneBot2, Koishi, etc.) to connect to Ruri.
pub struct OneBot12Adapter {
    config: OneBot12Config,
    instance_id: String,
    status: PlatformStatus,
    /// Shared server state.
    server_state: Arc<Ob12ServerState>,
    /// Channel to signal shutdown.
    shutdown_tx: Option<watch::Sender<bool>>,
    /// Pending outbound messages from action callbacks.
    /// When a OneBot app sends `send_message`, the result goes here
    /// and the adapter's `run()` method picks it up to forward as a PlatformEvent.
    /// Kept alive for the action callback's cloned sender.
    _outbound_tx: mpsc::Sender<OutboundMessage>,
    outbound_rx: Arc<Mutex<mpsc::Receiver<OutboundMessage>>>,
}

impl OneBot12Adapter {
    /// Create a new adapter from config.
    pub fn from_config(instance_id: String, extra: &serde_json::Value) -> Result<Self, String> {
        let config: OneBot12Config = serde_json::from_value(extra.clone())
            .map_err(|e| format!("Invalid OneBot12 config: {}", e))?;

        config.validate()?;

        let bot_self = Ob12Self {
            platform: config.platform.clone(),
            user_id: config.self_user_id.clone(),
        };

        let (event_sender, _) = tokio::sync::broadcast::channel(256);

        let event_buffer_size = config
            .http
            .as_ref()
            .map(|h| h.event_buffer_size)
            .unwrap_or(0);

        let event_polling_enabled = config
            .http
            .as_ref()
            .map(|h| h.event_enabled)
            .unwrap_or(false);

        let (outbound_tx, outbound_rx) = mpsc::channel::<OutboundMessage>(256);
        let outbound_rx = Arc::new(Mutex::new(outbound_rx));

        let webhook_url = config.http_webhook.as_ref().map(|w| w.url.clone());

        let http_client = reqwest::Client::new();

        // Action callback: handles `send_message` and other platform-specific actions
        let outbound_tx_cb = outbound_tx.clone();
        let bot_self_cb = bot_self.clone();
        let action_callback: ActionCallback = Arc::new(move |request: ActionRequest| {
            let outbound_tx = outbound_tx_cb.clone();
            let bot_self = bot_self_cb.clone();
            Box::pin(async move {
                match request.action.as_str() {
                    "send_message" => handle_send_message(&request, &outbound_tx, &bot_self).await,
                    "delete_message" => {
                        // Not supported — only forward, cannot recall
                        ActionResponse::failed(
                            retcode::UNSUPPORTED_ACTION,
                            "delete_message not supported".into(),
                        )
                    }
                    "get_self_info" => ActionResponse::ok(serde_json::json!({
                        "user_id": bot_self.user_id,
                        "user_name": "ruri",
                        "user_displayname": "",
                    })),
                    _ => ActionResponse::failed(
                        retcode::UNSUPPORTED_ACTION,
                        format!("Unknown action: {}", request.action),
                    ),
                }
            })
        });

        let server_state = Arc::new(Ob12ServerState {
            bot_self,
            access_token: config.access_token.clone(),
            event_sender,
            event_buffer: tokio::sync::Mutex::new(server::EventBuffer::new(event_buffer_size)),
            action_callback,
            http_client,
            webhook_url,
            event_polling_enabled,
        });

        Ok(Self {
            config,
            instance_id,
            status: PlatformStatus::Pending,
            server_state,
            shutdown_tx: None,
            _outbound_tx: outbound_tx,
            outbound_rx,
        })
    }
}

/// Handle a `send_message` action request.
async fn handle_send_message(
    request: &ActionRequest,
    outbound_tx: &mpsc::Sender<OutboundMessage>,
    _bot_self: &Ob12Self,
) -> ActionResponse {
    let detail_type = request
        .params
        .get("detail_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let (target_type, target_id) = match detail_type {
        "private" => {
            let user_id = match request.params.get("user_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => {
                    return ActionResponse::failed(retcode::BAD_PARAM, "Missing user_id".into());
                }
            };
            (MessageType::FriendMessage, user_id)
        }
        "group" => {
            let group_id = match request.params.get("group_id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => {
                    return ActionResponse::failed(retcode::BAD_PARAM, "Missing group_id".into());
                }
            };
            (MessageType::GroupMessage, group_id)
        }
        _ => {
            return ActionResponse::failed(
                retcode::UNSUPPORTED_PARAM,
                format!("Unsupported detail_type: {}", detail_type),
            );
        }
    };

    // Parse message — can be string, segment, or segment array
    let message_value = match request.params.get("message") {
        Some(v) => v.clone(),
        None => return ActionResponse::failed(retcode::BAD_PARAM, "Missing message".into()),
    };

    let ob12_msg: Ob12Message = match serde_json::from_value(message_value) {
        Ok(m) => m,
        Err(e) => {
            return ActionResponse::failed(retcode::BAD_PARAM, format!("Invalid message: {}", e));
        }
    };

    let segments = ob12_msg.to_segments();

    // Convert to OutboundContent:
    // For simplicity, if the message is all text, use Text content;
    // otherwise extract text and ignore non-text segments.
    let text_content: String = segments
        .iter()
        .filter_map(|seg| {
            if seg.segment_type == "text" {
                seg.data
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("");

    let content = OutboundContent::Text {
        content: text_content,
    };

    let msg = OutboundMessage {
        target_type,
        target_id,
        content,
    };

    // Send the outbound message through the channel
    if let Err(e) = outbound_tx.send(msg).await {
        return ActionResponse::failed(
            retcode::INTERNAL_HANDLER_ERROR,
            format!("Failed to queue message: {}", e),
        );
    }

    // Return success with a pseudo message ID
    let message_id = uuid::Uuid::new_v4().to_string();
    let time = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
    ActionResponse::ok(serde_json::json!({
        "message_id": message_id,
        "time": time,
    }))
}

/// Convert an `OutboundMessage` (from OneBot action callback) back to a
/// `PlatformMessage` so it can be injected into Ruri's event pipeline.
fn outbound_to_platform_message(
    outbound: &OutboundMessage,
    bot_self: &Ob12Self,
    platform_id: &str,
) -> PlatformMessage {
    let (message_type, group_id, session_id) = match outbound.target_type {
        MessageType::GroupMessage => (
            MessageType::GroupMessage,
            outbound.target_id.clone(),
            outbound.target_id.clone(),
        ),
        MessageType::FriendMessage => (
            MessageType::FriendMessage,
            String::new(),
            outbound.target_id.clone(),
        ),
    };

    let message_str = match &outbound.content {
        OutboundContent::Text { content } => content.clone(),
        OutboundContent::Markdown { text, .. } => text.clone(),
        OutboundContent::Image { photo_url } => format!("[image: {}]", photo_url),
        OutboundContent::File { file_name, .. } => format!("[file: {}]", file_name),
    };

    PlatformMessage {
        platform_id: platform_id.to_string(),
        message_id: uuid::Uuid::new_v4().to_string(),
        message_type,
        message_str: message_str.clone(),
        components: vec![MessageComponent::Plain { text: message_str }],
        sender: crate::platform::types::MessageSender {
            user_id: bot_self.user_id.clone(),
            nickname: "ruri".to_string(),
        },
        self_id: bot_self.user_id.clone(),
        group_id,
        session_id,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        raw: None,
    }
}

#[async_trait]
impl Platform for OneBot12Adapter {
    async fn run(&mut self, event_sender: mpsc::Sender<PlatformEvent>) -> anyhow::Result<()> {
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);
        self.status = PlatformStatus::Running;

        let state = self.server_state.clone();
        let config = self.config.clone();
        let instance_id = self.instance_id.clone();

        // Start servers based on config
        if let Some(ref http_config) = config.http {
            let state = state.clone();
            let http_config = http_config.clone();
            tokio::spawn(async move {
                if let Err(e) = server::start_http_server(state, &http_config).await {
                    tracing::error!(error = %e, "OneBot v12 HTTP server error");
                }
            });
        }

        if let Some(ref ws_config) = config.ws {
            let state = state.clone();
            let ws_config = ws_config.clone();
            tokio::spawn(async move {
                if let Err(e) = server::start_ws_server(state, &ws_config).await {
                    tracing::error!(error = %e, "OneBot v12 WebSocket server error");
                }
            });
        }

        if let Some(ref webhook_config) = config.http_webhook {
            let state = state.clone();
            server::start_webhook_pusher(state, webhook_config);
        }

        if let Some(ref ws_reverse_config) = config.ws_reverse {
            let state = state.clone();
            let ws_reverse_config = ws_reverse_config.clone();
            tokio::spawn(async move {
                crate::platform::onebot12::reverse_ws::run_reverse_ws(state, &ws_reverse_config)
                    .await
            });
        }

        // Start heartbeat task (every 5 seconds)
        {
            let state = self.server_state.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    let heartbeat = Ob12Event {
                        id: uuid::Uuid::new_v4().to_string(),
                        time: chrono::Utc::now().timestamp_millis() as f64 / 1000.0,
                        event_type: "meta".to_owned(),
                        detail_type: "heartbeat".to_owned(),
                        sub_type: String::new(),
                        self_: None,
                        extra: {
                            let mut m = serde_json::Map::new();
                            m.insert("interval".to_owned(), serde_json::json!(5000));
                            m
                        },
                    };
                    server::push_event(&state, &heartbeat).await;
                }
            });
        }

        // Forward outbound messages from OneBot action callbacks as PlatformEvents.
        // When a OneBot app sends a `send_message` action, the action callback
        // puts the OutboundMessage into outbound_tx. We poll outbound_rx here
        // and convert each message into a PlatformEvent that enters Ruri's pipeline.
        let outbound_rx = self.outbound_rx.clone();
        let bot_self = self.server_state.bot_self.clone();
        let instance_id_for_event = self.instance_id.clone();

        tokio::spawn(async move {
            loop {
                let msg = {
                    let mut rx = outbound_rx.lock().await;
                    rx.recv().await
                };
                match msg {
                    Some(outbound) => {
                        let platform_msg = outbound_to_platform_message(
                            &outbound,
                            &bot_self,
                            &instance_id_for_event,
                        );
                        let event = PlatformEvent::Message(platform_msg);
                        if event_sender.send(event).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        });

        // Wait for shutdown signal
        let _ = shutdown_rx.changed().await;

        tracing::info!(platform_id = %instance_id, "OneBot v12 adapter stopped");
        Ok(())
    }

    async fn terminate(&mut self) -> anyhow::Result<()> {
        self.status = PlatformStatus::Stopped;
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        Ok(())
    }

    async fn send_message(&self, message: OutboundMessage) -> anyhow::Result<()> {
        // When Ruri wants to send a message through this adapter,
        // we push it as a OneBot v12 event so that connected OneBot
        // applications can process it.

        let content_text = match &message.content {
            OutboundContent::Text { content } => content.clone(),
            OutboundContent::Markdown { text, .. } => text.clone(),
            OutboundContent::Image { photo_url } => format!("[image: {}]", photo_url),
            OutboundContent::File { file_name, .. } => format!("[file: {}]", file_name),
        };

        let segments = match &message.content {
            OutboundContent::Text { content } => vec![Segment::text(content)],
            OutboundContent::Markdown { text, .. } => vec![Segment::text(text)],
            OutboundContent::Image { photo_url } => vec![Segment::image(photo_url)],
            OutboundContent::File {
                media_id,
                file_name,
                ..
            } => {
                let mut data = serde_json::Map::new();
                data.insert(
                    "file_id".to_owned(),
                    serde_json::Value::String(media_id.clone()),
                );
                data.insert(
                    "name".to_owned(),
                    serde_json::Value::String(file_name.clone()),
                );
                vec![Segment::new("file", data)]
            }
        };

        let (detail_type, target_key, target_value) = match message.target_type {
            MessageType::GroupMessage => ("group", "group_id", message.target_id.clone()),
            MessageType::FriendMessage => ("private", "user_id", message.target_id.clone()),
        };

        let event = Ob12Event {
            id: uuid::Uuid::new_v4().to_string(),
            time: chrono::Utc::now().timestamp_millis() as f64 / 1000.0,
            event_type: "message".to_owned(),
            detail_type: detail_type.to_owned(),
            sub_type: String::new(),
            self_: Some(self.server_state.bot_self.clone()),
            extra: {
                let mut m = serde_json::Map::new();
                m.insert(
                    "message_id".to_owned(),
                    serde_json::json!(uuid::Uuid::new_v4().to_string()),
                );
                m.insert("message".to_owned(), serde_json::json!(segments));
                m.insert(
                    "alt_message".to_owned(),
                    serde_json::Value::String(content_text),
                );
                m.insert(
                    target_key.to_owned(),
                    serde_json::Value::String(target_value),
                );
                m
            },
        };

        server::push_event(&self.server_state, &event).await;
        Ok(())
    }

    fn status(&self) -> PlatformStatus {
        self.status
    }

    fn platform_type(&self) -> &str {
        "onebot12"
    }
}

//! DingTalk Stream client — WebSocket-based connection for receiving bot messages.
//!
//! The Stream mode lets the robot receive messages through a persistent
//! WebSocket connection instead of requiring a public HTTPS callback URL.
//! This is ideal for bots running behind NAT/firewalls.
//!
//! Connection flow:
//! 1. Obtain an access token via OAuth2
//! 2. Open a Stream connection (get WSS endpoint + ticket)
//! 3. Connect to the WebSocket endpoint
//! 4. Handle ping/pong keepalive and incoming messages
//! 5. ACK every message on the same WebSocket

use crate::platform::dingtalk::config::*;
use crate::platform::types::{MessageComponent, MessageSender, MessageType, PlatformMessage};
use crate::transport::proxy_ws::connect_ws_with_proxy;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::protocol::Message};

// ─── DingTalk Stream protocol structures ─────────────────────────

/// Frame header in the DingTalk Stream protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamHeaders {
    #[serde(rename = "contentType", default)]
    pub content_type: String,
    #[serde(default)]
    pub topic: String,
    #[serde(rename = "messageId", default)]
    pub message_id: String,
    #[serde(rename = "eventId", default)]
    pub event_id: String,
    #[serde(rename = "eventBornTime", default)]
    pub event_born_time: i64,
    #[serde(rename = "eventType", default)]
    pub event_type: String,
}

/// A frame in the DingTalk Stream protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamFrame {
    pub headers: StreamHeaders,
    pub data: serde_json::Value,
}

/// Response from the access token endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessTokenResponse {
    #[serde(rename = "accessToken", default)]
    access_token: String,
    #[serde(rename = "expireIn", default)]
    expire_in: i64,
}

/// Response from opening a stream connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StreamOpenResponse {
    endpoint: String,
    ticket: String,
}

/// Parsed DingTalk bot message (from the callback data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingtalkBotMessage {
    #[serde(rename = "msgId", default)]
    pub msg_id: String,
    #[serde(rename = "conversationId", default)]
    pub conversation_id: String,
    #[serde(rename = "conversationType", default)]
    pub conversation_type: String,
    #[serde(rename = "senderId", default)]
    pub sender_id: String,
    #[serde(rename = "senderNick", default)]
    pub sender_nick: String,
    #[serde(rename = "senderStaffId", default)]
    pub sender_staff_id: String,
    #[serde(rename = "chatbotUserId", default)]
    pub chatbot_user_id: String,
    #[serde(rename = "chatbotCorpId", default)]
    pub chatbot_corp_id: String,
    #[serde(rename = "messageType", default)]
    pub message_type: String,
    #[serde(rename = "robotCode", default)]
    pub robot_code: String,
    #[serde(rename = "createAt", default)]
    pub create_at: i64,
    /// The text content (for text messages).
    #[serde(default)]
    pub text: Option<DingtalkTextContent>,
    /// The rich text content.
    #[serde(rename = "richText", default)]
    pub rich_text: Option<DingtalkRichTextContent>,
    /// The image content.
    #[serde(rename = "imageContent", default)]
    pub image_content: Option<DingtalkImageContent>,
    /// Extensions / extra fields.
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingtalkTextContent {
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingtalkRichTextContent {
    #[serde(rename = "richTextList", default)]
    pub rich_text_list: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingtalkImageContent {
    #[serde(rename = "downloadCode", default)]
    pub download_code: String,
}

// ─── Stream runner ───────────────────────────────────────────────

/// Run the DingTalk Stream connection loop.
///
/// This spawns a background task that:
/// 1. Connects to DingTalk's WebSocket
/// 2. Handles ping/pong + ACK
/// 3. Converts incoming messages to [`PlatformMessage`]
/// 4. Sends them through `event_sender`
pub async fn run_dingtalk_stream(
    platform_id: String,
    config: DingtalkConfig,
    event_sender: tokio::sync::mpsc::Sender<PlatformMessage>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let http = Client::new();
    let mut access_token = String::new();

    let max_retries = 5u32;
    let mut retry_count = 0u32;
    let base_delay_secs = 5u64;

    loop {
        if *shutdown_rx.borrow() {
            tracing::info!("DingTalk stream client shutting down");
            return Ok(());
        }

        match run_once(
            &platform_id,
            &config,
            &http,
            &mut access_token,
            &event_sender,
            &mut shutdown_rx,
        )
        .await
        {
            Ok(()) => {
                tracing::info!("DingTalk stream connection closed normally");
                retry_count = 0; // reset on clean close, allow reconnecting
                // If shutdown hasn't been requested, try reconnecting after a short delay
                if *shutdown_rx.borrow() {
                    return Ok(());
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    tracing::error!(error = %e, "DingTalk stream max retries reached");
                    return Err(e);
                }
                let delay = base_delay_secs * 2u64.pow(retry_count - 1);
                tracing::warn!(error = %e, retry = retry_count, delay_secs = delay, "Retrying");
                tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
            }
        }
    }
}

/// Single connection attempt with split WebSocket.
async fn run_once(
    platform_id: &str,
    config: &DingtalkConfig,
    http: &Client,
    access_token: &mut String,
    event_sender: &tokio::sync::mpsc::Sender<PlatformMessage>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> anyhow::Result<()> {
    // 1. Get access token
    *access_token = fetch_access_token(http, &config.client_id, &config.client_secret).await?;

    // 2. Open stream connection
    let (endpoint, ticket) = open_stream(http, access_token, config).await?;

    // 3. Connect WebSocket (with optional proxy)
    let ws_url = format!("{}?ticket={}", endpoint, ticket);
    let ws = connect_ws_with_proxy(&ws_url, config.proxy_url.as_deref()).await?;

    tracing::info!("DingTalk stream WebSocket connected");

    let (mut ws_sink, mut ws_stream) = ws.split();

    // 4. Message loop
    loop {
        tokio::select! {
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        match msg {
                            Message::Text(text) => {
                                handle_text_frame(
                                    &text,
                                    &mut ws_sink,
                                    event_sender,
                                    platform_id,
                                    &config.client_id,
                                )
                                .await?;
                            }
                            Message::Ping(data) => {
                                tracing::debug!("Received ping, sending pong");
                                let _ = ws_sink.send(Message::Pong(data)).await;
                            }
                            Message::Close(_) => {
                                tracing::info!("WebSocket close frame received");
                                let _ = ws_sink.close().await;
                                return Ok(());
                            }
                            other => {
                                tracing::debug!(?other, "Ignoring WS message type");
                            }
                        }
                    }
                    Some(Err(e)) => {
                        return Err(anyhow::anyhow!("WebSocket read error: {}", e));
                    }
                    None => {
                        return Ok(());
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::info!("Shutdown signal, closing WebSocket");
                    let _ = ws_sink.close().await;
                    return Ok(());
                }
            }
        }
    }
}

/// Fetch a new access token from DingTalk.
async fn fetch_access_token(
    http: &Client,
    client_id: &str,
    client_secret: &str,
) -> anyhow::Result<String> {
    let payload = serde_json::json!({
        "appKey": client_id,
        "appSecret": client_secret,
    });

    let resp = http
        .post(ENDPOINT_ACCESS_TOKEN)
        .json(&payload)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "Access token request failed: status={}, body={}",
            status,
            body
        );
    }

    let data: AccessTokenResponse = resp.json().await?;
    tracing::debug!(expire_in = data.expire_in, "Obtained access token");
    Ok(data.access_token)
}

/// Open a stream connection via the REST API.
async fn open_stream(
    http: &Client,
    access_token: &str,
    config: &DingtalkConfig,
) -> anyhow::Result<(String, String)> {
    let payload = serde_json::json!({
        "clientId": config.client_id,
        "clientSecret": config.client_secret,
        "subscriptions": [
            {
                "type": "EVENT",
                "topic": SUBSCRIPTION_BOT_MESSAGE,
            }
        ],
        "ua": "ruri-dingtalk-adapter",
    });

    let resp = http
        .post(ENDPOINT_STREAM_OPEN)
        .header("x-acs-dingtalk-access-token", access_token)
        .json(&payload)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Stream open failed: status={}, body={}", status, body);
    }

    let data: StreamOpenResponse = resp.json().await?;
    Ok((data.endpoint, data.ticket))
}

/// Handle a text frame from the WebSocket.
async fn handle_text_frame(
    text: &str,
    ws_sink: &mut futures_util::stream::SplitSink<
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        Message,
    >,
    event_sender: &tokio::sync::mpsc::Sender<PlatformMessage>,
    platform_id: &str,
    _client_id: &str,
) -> anyhow::Result<()> {
    let frame: StreamFrame = match serde_json::from_str(text) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(error = %e, text = %text, "Failed to parse WS frame");
            return Ok(());
        }
    };

    let topic = &frame.headers.topic;

    // 1. Handle ping (keepalive)
    if topic == "ping" {
        tracing::debug!("Received ping, sending pong");
        let pong = serde_json::json!({
            "headers": {
                "contentType": "application/json",
                "topic": "pong",
            },
            "data": frame.data,
        });
        ws_sink
            .send(Message::Text(pong.to_string().into()))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send pong: {}", e))?;
        return Ok(());
    }

    // 2. ACK the message
    if !frame.headers.message_id.is_empty() {
        let ack = serde_json::json!({
            "headers": {
                "contentType": "application/json",
                "topic": "ack",
                "messageId": frame.headers.message_id,
            },
            "data": {},
        });
        ws_sink
            .send(Message::Text(ack.to_string().into()))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send ACK: {}", e))?;
    }

    // 3. Handle bot messages
    if topic == TOPIC_BOT_MESSAGE
        || topic == SUBSCRIPTION_BOT_MESSAGE
        || topic.contains("bot/messages")
    {
        let ding_msg: DingtalkBotMessage = match serde_json::from_value(frame.data.clone()) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse DingTalk bot message");
                return Ok(());
            }
        };

        let platform_msg = convert_dingtalk_message(ding_msg, platform_id);

        if let Err(e) = event_sender.send(platform_msg).await {
            tracing::warn!(error = %e, "Failed to send platform message to event channel");
        }
    } else {
        tracing::debug!(topic = %topic, "Ignoring unhandled WS topic");
    }

    Ok(())
}

/// Convert a DingTalk bot message into a [`PlatformMessage`].
fn convert_dingtalk_message(msg: DingtalkBotMessage, platform_id: &str) -> PlatformMessage {
    let message_type = if msg.conversation_type == "2" {
        MessageType::GroupMessage
    } else {
        MessageType::FriendMessage
    };

    let mut components: Vec<MessageComponent> = Vec::new();
    let mut message_str = String::new();

    match msg.message_type.as_str() {
        "text" => {
            if let Some(text) = &msg.text {
                message_str = text.content.clone();
                components.push(MessageComponent::Plain {
                    text: text.content.clone(),
                });
            }
        }
        "picture" => {
            if let Some(img) = &msg.image_content {
                if !img.download_code.is_empty() {
                    components.push(MessageComponent::Image {
                        url: format!("dingtalk://download/{}", img.download_code),
                    });
                }
            }
        }
        "richText" => {
            if let Some(rich) = &msg.rich_text {
                let mut plain_parts = Vec::new();
                for item in &rich.rich_text_list {
                    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                        if !text.is_empty() {
                            plain_parts.push(text.to_string());
                            components.push(MessageComponent::Plain {
                                text: text.to_string(),
                            });
                        }
                    }
                    // Rich text can also contain inline pictures
                    if item
                        .get("type")
                        .and_then(|v| v.as_str())
                        .is_some_and(|t| t == "picture")
                    {
                        if let Some(code) = item.get("downloadCode").and_then(|v| v.as_str()) {
                            components.push(MessageComponent::Image {
                                url: format!("dingtalk://download/{}", code),
                            });
                        }
                    }
                }
                message_str = plain_parts.join("");
            }
        }
        "audio" | "voice" => {
            // Extract download code from extensions
            if let Some(ext) = &msg.extensions {
                if let Some(code) = ext
                    .get("content")
                    .and_then(|c| c.get("downloadCode"))
                    .and_then(|v| v.as_str())
                {
                    components.push(MessageComponent::Voice {
                        url: format!("dingtalk://download/{}", code),
                    });
                }
            }
        }
        "file" => {
            if let Some(ext) = &msg.extensions {
                let name = ext
                    .get("content")
                    .and_then(|c| c.get("fileName"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("file")
                    .to_string();
                if let Some(code) = ext
                    .get("content")
                    .and_then(|c| c.get("downloadCode"))
                    .and_then(|v| v.as_str())
                {
                    components.push(MessageComponent::File {
                        name,
                        url: format!("dingtalk://download/{}", code),
                    });
                }
            }
        }
        other => {
            tracing::debug!(message_type = other, "Unhandled DingTalk message type");
        }
    }

    let group_id = if message_type == MessageType::GroupMessage {
        msg.conversation_id.clone()
    } else {
        String::new()
    };

    let session_id = if message_type == MessageType::GroupMessage {
        group_id.clone()
    } else {
        normalize_dingtalk_id(&msg.sender_id)
    };

    let timestamp = if msg.create_at > 0 {
        (msg.create_at / 1000) as u64
    } else {
        0
    };

    PlatformMessage {
        platform_id: platform_id.to_string(),
        message_id: msg.msg_id.clone(),
        message_type,
        message_str,
        components,
        sender: MessageSender {
            user_id: normalize_dingtalk_id(&msg.sender_id),
            nickname: msg.sender_nick.clone(),
        },
        self_id: normalize_dingtalk_id(&msg.chatbot_user_id),
        group_id,
        session_id,
        timestamp,
        raw: Some(serde_json::to_value(&msg).unwrap_or(serde_json::Value::Null)),
    }
}

/// Normalize a DingTalk user ID by stripping the LWCP prefix.
fn normalize_dingtalk_id(id: &str) -> String {
    let prefix = "$:LWCP_v1:$";
    if let Some(stripped) = id.strip_prefix(prefix) {
        stripped.to_string()
    } else {
        id.to_string()
    }
}

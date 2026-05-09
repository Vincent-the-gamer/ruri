//! WeChat ClawBot (openclaw-weixin) platform adapter for Ruri.
//!
//! This adapter uses the WeChat iLink HTTP API with:
//! - QR code scanning for login
//! - Long-polling (getUpdates) for receiving messages
//! - REST API for sending messages
//!
//! # Configuration
//!
//! ```yaml
//! platforms:
//!   - type: weixin_oc
//!     id: my-wechat-bot
//!     enable: true
//!     token: ""           # auto-filled after QR login
//!     account_id: ""      # auto-filled after QR login
//! ```
//!
//! # Login Flow
//!
//! If `token` is empty, the adapter will:
//! 1. Request a QR code from `ilink/bot/get_bot_qrcode`
//! 2. Print the QR code URL to the console
//! 3. Poll `ilink/bot/get_qrcode_status` until scanned and confirmed
//! 4. Save the token and account_id for subsequent restarts
//!
//! # Message Flow
//!
//! Once logged in, the adapter runs a long-poll loop calling
//! `ilink/bot/getupdates` to receive incoming messages and converts
//! them into Ruri's unified [`PlatformMessage`] format.

use crate::platform::trait_def::{Platform, PlatformEvent};
use crate::platform::types::{
    MessageComponent, MessageType, OutboundContent, OutboundMessage, PlatformMessage,
    PlatformStatus,
};
use crate::platform::weixin_oc::api::WeixinApi;
use crate::platform::weixin_oc::config::WeixinOcConfig;
use crate::platform::weixin_oc::types::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, watch};

/// Context token cache: maps (account_id, user_id) → context_token.
type ContextTokenMap = HashMap<String, String>;

/// WeChat ClawBot platform adapter.
pub struct WeixinOcAdapter {
    config: WeixinOcConfig,
    instance_id: String,
    status: PlatformStatus,
    api: Arc<WeixinApi>,
    /// Channel to signal the poll task to shut down.
    shutdown_tx: Option<watch::Sender<bool>>,
    /// Context token cache for replying in conversations.
    context_tokens: Arc<Mutex<ContextTokenMap>>,
}

impl WeixinOcAdapter {
    /// Create a new adapter from an instance ID and JSON config.
    pub fn from_config(instance_id: String, extra: &serde_json::Value) -> Result<Self, String> {
        let config: WeixinOcConfig = serde_json::from_value(extra.clone())
            .map_err(|e| format!("Invalid weixin_oc config: {}", e))?;

        let api = WeixinApi::new(config.clone())?;

        Ok(Self {
            config,
            instance_id,
            status: PlatformStatus::Pending,
            api: Arc::new(api),
            shutdown_tx: None,
            context_tokens: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Run the QR code login flow.
    async fn do_qr_login(&self) -> anyhow::Result<()> {
        let api = &self.api;
        let poll_interval = self.config.qr_poll_interval_ms;

        // Step 1: Get QR code
        tracing::info!(
            platform_id = %self.instance_id,
            "Starting WeChat QR login..."
        );

        let qr_resp = api.qr_login_start().await?;

        tracing::info!("请使用手机微信扫码登录，二维码有效期 5 分钟，过期后会自动刷新。");

        // Print QR code URL
        tracing::info!("QR code URL: {}", qr_resp.qrcode_img_content);

        // Try to render a terminal QR code
        print_qr_code(&qr_resp.qrcode_img_content);

        // Step 2: Poll until confirmed
        let login_timeout_ms: u64 = 480_000; // 8 minutes
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_millis(login_timeout_ms);
        let mut max_qr_refresh = 3;
        let mut current_qrcode = qr_resp.qrcode.clone();
        let mut checked_redirect = false;

        while std::time::Instant::now() < deadline {
            let poll_timeout = std::cmp::min(
                35_000,
                (deadline - std::time::Instant::now()).as_millis() as u64,
            );
            if poll_timeout == 0 {
                break;
            }

            let result = api.qr_login_wait(&current_qrcode, poll_timeout).await?;

            match result.status.as_str() {
                "wait" => {
                    // Still waiting, continue polling
                }
                "scaned" => {
                    tracing::info!("👀 已扫码，请在微信中确认登录...");
                }
                "confirmed" => {
                    if let (Some(token), Some(account_id)) =
                        (&result.bot_token, &result.ilink_bot_id)
                    {
                        tracing::info!("✅ 微信登录成功！account_id={}", account_id);
                        api.save_login(token.clone(), account_id.clone(), result.baseurl.clone())
                            .await;
                        return Ok(());
                    } else {
                        anyhow::bail!("Login confirmed but missing bot_token or ilink_bot_id");
                    }
                }
                "expired" => {
                    max_qr_refresh -= 1;
                    if max_qr_refresh <= 0 {
                        anyhow::bail!("二维码多次过期，请重新启动适配器");
                    }
                    tracing::warn!("二维码已过期，正在刷新...({}次剩余)", max_qr_refresh);
                    // Get a new QR code
                    let new_qr = api.qr_login_start().await?;
                    current_qrcode = new_qr.qrcode.clone();
                    tracing::info!("新二维码已生成，请重新扫描");
                    print_qr_code(&new_qr.qrcode_img_content);
                }
                "scaned_but_redirect" => {
                    if !checked_redirect {
                        if let Some(ref host) = result.redirect_host {
                            tracing::info!("IDC redirect, switching polling host to: {}", host);
                        }
                        checked_redirect = true;
                    }
                }
                other => {
                    tracing::warn!("Unknown QR status: {}", other);
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(poll_interval)).await;
        }

        anyhow::bail!("微信登录超时，请重试")
    }

    /// Run the long-poll loop for receiving messages.
    async fn run_poll_loop(
        api: Arc<WeixinApi>,
        instance_id: String,
        cdn_base_url: String,
        context_tokens: Arc<Mutex<ContextTokenMap>>,
        event_sender: mpsc::Sender<PlatformEvent>,
        shutdown_rx: watch::Receiver<bool>,
        self_id: String,
    ) {
        loop {
            // Check shutdown
            if *shutdown_rx.borrow() {
                tracing::info!(platform_id = %instance_id, "Poll loop shutting down");
                break;
            }

            match api.get_updates().await {
                Ok(updates) => {
                    if let Some(errcode) = updates.errcode {
                        if errcode == -14 {
                            tracing::warn!(
                                platform_id = %instance_id,
                                "Session timeout (errcode=-14), need to re-login"
                            );
                            let _ = event_sender
                                .send(PlatformEvent::Error {
                                    platform_id: instance_id.clone(),
                                    message: "WeChat session timeout, need to re-login".to_string(),
                                })
                                .await;
                            break;
                        }
                    }

                    if let Some(msgs) = updates.msgs {
                        for msg in msgs {
                            // Only process USER messages (type=1)
                            if msg.message_type != Some(1) {
                                continue;
                            }

                            if let Some(platform_msg) =
                                convert_weixin_message(&msg, &instance_id, &self_id, &cdn_base_url)
                            {
                                // Cache context_token
                                if let Some(ref token) = msg.context_token {
                                    if let Some(ref from_id) = msg.from_user_id {
                                        let mut ctx = context_tokens.lock().await;
                                        ctx.insert(from_id.clone(), token.clone());
                                    }
                                }

                                let event = PlatformEvent::Message(platform_msg);
                                if event_sender.send(event).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        platform_id = %instance_id,
                        error = %e,
                        "getUpdates error"
                    );
                    // Brief sleep before retry
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }

        tracing::info!(platform_id = %instance_id, "Poll loop ended");
    }
}

/// Print a QR code to the terminal using the `qrcode` crate.
fn print_qr_code(url: &str) {
    use qrcode::QrCode;

    match QrCode::new(url) {
        Ok(code) => {
            let string = code
                .render::<char>()
                .quiet_zone(false)
                .module_dimensions(2, 1)
                .build();
            println!("\n{}\n", string);
        }
        Err(e) => {
            tracing::warn!("Failed to render QR code: {}", e);
        }
    }

    println!("如果二维码未能成功展示，请用浏览器打开以下链接扫码：");
    println!("{}", url);
}

/// Convert a WeixinMessage to a Ruri PlatformMessage.
fn convert_weixin_message(
    msg: &WeixinMessage,
    platform_id: &str,
    self_id: &str,
    _cdn_base_url: &str,
) -> Option<PlatformMessage> {
    let from_user_id = msg.from_user_id.as_deref().unwrap_or("unknown");
    let _to_user_id = msg.to_user_id.as_deref().unwrap_or(self_id);

    let message_id = msg
        .message_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let timestamp = msg.create_time_ms.map(|ms| ms / 1000).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    });

    // Determine message type: WeChat personal doesn't have native groups from this API,
    // so all messages are treated as friend messages unless a group_id is present.
    let group_id = msg.group_id.clone().unwrap_or_default();
    let (message_type, session_id) = if group_id.is_empty() {
        (MessageType::FriendMessage, from_user_id.to_string())
    } else {
        (
            MessageType::GroupMessage,
            format!("{}_{}", group_id, from_user_id),
        )
    };

    // Parse message components
    let mut components = Vec::new();
    let mut message_str = String::new();

    if let Some(ref items) = msg.item_list {
        for item in items {
            match item.item_type {
                Some(1) => {
                    // Text
                    if let Some(ref text_item) = item.text_item {
                        if let Some(ref text) = text_item.text {
                            components.push(MessageComponent::Plain { text: text.clone() });
                            message_str.push_str(text);
                        }
                    }
                }
                Some(2) => {
                    // Image
                    if let Some(ref image_item) = item.image_item {
                        let url = image_item
                            .url
                            .clone()
                            .or_else(|| image_item.media.as_ref().and_then(|m| m.full_url.clone()))
                            .unwrap_or_else(|| "(image)".to_string());
                        components.push(MessageComponent::Image { url });
                        message_str.push_str("[图片]");
                    }
                }
                Some(3) => {
                    // Voice — WeChat auto-transcribes to text
                    if let Some(ref voice_item) = item.voice_item {
                        if let Some(ref text) = voice_item.text {
                            components.push(MessageComponent::Plain { text: text.clone() });
                            message_str.push_str(text);
                        } else {
                            components.push(MessageComponent::Voice {
                                url: voice_item
                                    .media
                                    .as_ref()
                                    .and_then(|m| m.full_url.clone())
                                    .unwrap_or_default(),
                            });
                            message_str.push_str("[语音]");
                        }
                    }
                }
                Some(4) => {
                    // File
                    if let Some(ref file_item) = item.file_item {
                        let name = file_item
                            .file_name
                            .clone()
                            .unwrap_or_else(|| "file".to_string());
                        let url = file_item
                            .media
                            .as_ref()
                            .and_then(|m| m.full_url.clone())
                            .unwrap_or_default();
                        components.push(MessageComponent::File { name, url });
                        message_str.push_str("[文件]");
                    }
                }
                Some(5) => {
                    // Video
                    components.push(MessageComponent::Image {
                        url: "(video)".to_string(),
                    });
                    message_str.push_str("[视频]");
                }
                _ => {}
            }
        }
    }

    // If no components were parsed, add a fallback text
    if components.is_empty() {
        components.push(MessageComponent::Plain {
            text: "(empty message)".to_string(),
        });
    }

    Some(PlatformMessage {
        platform_id: platform_id.to_string(),
        message_id,
        message_type,
        message_str,
        components,
        sender: crate::platform::types::MessageSender {
            user_id: from_user_id.to_string(),
            nickname: String::new(),
        },
        self_id: self_id.to_string(),
        group_id,
        session_id,
        timestamp,
        raw: Some(serde_json::to_value(msg).unwrap_or(serde_json::Value::Null)),
    })
}

#[async_trait]
impl Platform for WeixinOcAdapter {
    async fn run(&mut self, event_sender: mpsc::Sender<PlatformEvent>) -> anyhow::Result<()> {
        // Step 1: If no token, perform QR login
        let needs_login = {
            let state = self.api.state();
            let guard = state.read().await;
            guard.token.is_none() || guard.token.as_deref() == Some("")
        };
        if needs_login {
            tracing::info!(
                platform_id = %self.instance_id,
                "No WeChat token found, starting QR login..."
            );
            self.do_qr_login().await?;
        }

        // Reset the sync cursor before starting polling
        {
            let state = self.api.state();
            let mut guard = state.write().await;
            guard.get_updates_buf = String::new();
        }

        // Step 2: Get self_id
        let self_id = {
            let state = self.api.state();
            let guard = state.read().await;
            guard
                .account_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        };

        self.status = PlatformStatus::Running;

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        let api = self.api.clone();
        let instance_id = self.instance_id.clone();
        let cdn_base_url = self.config.cdn_base_url.clone();
        let context_tokens = self.context_tokens.clone();

        // Spawn the poll loop
        tokio::spawn(async move {
            Self::run_poll_loop(
                api,
                instance_id,
                cdn_base_url,
                context_tokens,
                event_sender,
                shutdown_rx,
                self_id,
            )
            .await;
        });

        tracing::info!(
            platform_id = %self.instance_id,
            "WeChat ClawBot adapter started"
        );

        Ok(())
    }

    async fn terminate(&mut self) -> anyhow::Result<()> {
        self.status = PlatformStatus::Stopped;
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        tracing::info!(platform_id = %self.instance_id, "WeChat ClawBot adapter terminated");
        Ok(())
    }

    async fn send_message(&self, message: OutboundMessage) -> anyhow::Result<()> {
        match &message.content {
            OutboundContent::Text { content } => {
                // Look up cached context_token
                let context_token = {
                    let ctx = self.context_tokens.lock().await;
                    ctx.get(&message.target_id).cloned()
                };

                self.api
                    .send_text_message(&message.target_id, content, context_token.as_deref())
                    .await
            }
            OutboundContent::Image { photo_url } => {
                // For image sending, we need to:
                // 1. Download the image (if it's a URL)
                // 2. Encrypt it with AES-128-ECB
                // 3. Get upload URL via getUploadUrl
                // 4. Upload to CDN
                // 5. Send a message with the image CDNMedia reference
                //
                // For now, send as text with the URL since full image upload
                // requires more complex CDN flow.
                tracing::warn!(
                    "Image sending via CDN upload is not yet fully implemented, sending URL as text"
                );
                self.api
                    .send_text_message(&message.target_id, photo_url, None)
                    .await
            }
            OutboundContent::Markdown { title: _, text } => {
                // WeChat does not natively support markdown, send as plain text
                let context_token = {
                    let ctx = self.context_tokens.lock().await;
                    ctx.get(&message.target_id).cloned()
                };
                self.api
                    .send_text_message(&message.target_id, text, context_token.as_deref())
                    .await
            }
            OutboundContent::File {
                media_id: _,
                file_name: _,
                file_type: _,
            } => {
                // File sending via CDN upload requires the full upload flow
                tracing::warn!("File sending via CDN upload is not yet fully implemented");
                anyhow::bail!("File sending via CDN upload is not yet fully implemented");
            }
        }
    }

    fn status(&self) -> PlatformStatus {
        self.status
    }

    fn platform_type(&self) -> &str {
        "weixin_oc"
    }
}

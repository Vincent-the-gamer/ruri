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
    /// Typing ticket cache: maps user_id → typing_ticket.
    typing_tickets: Arc<Mutex<HashMap<String, String>>>,
    /// Whether the config has been updated (e.g. after QR login) and
    /// should be persisted via `persist_config_hint()`.
    config_dirty: bool,
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
            typing_tickets: Arc::new(Mutex::new(HashMap::new())),
            config_dirty: false,
        })
    }

    /// Copy the current token and account_id from the API state back into
    /// `self.config` and mark the config as dirty so that
    /// [`persist_config_hint()`] will return the updated value.
    async fn sync_config_from_api(&mut self) {
        let state = self.api.state();
        let guard = state.read().await;
        self.config.token = guard.token.clone();
        self.config.account_id = guard.account_id.clone();
        drop(guard);
        self.config_dirty = true;
    }

    /// Run the QR code login flow.
    async fn do_qr_login(&self) -> anyhow::Result<()> {
        let api = &self.api;
        let poll_interval = self.config.qr_poll_interval_ms;

        // Step 1: Get QR code
        tracing::info!(
            platform_id = %self.instance_id,
            "Starting Personal WeChat QR login..."
        );

        let qr_resp = api.qr_login_start().await?;

        tracing::info!("请使用手机微信扫码登录个人微信，二维码有效期 5 分钟，过期后会自动刷新。");

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
                    tracing::info!("👀 已扫码，请在个人微信中确认登录...");
                }
                "confirmed" => {
                    if let (Some(token), Some(account_id)) =
                        (&result.bot_token, &result.ilink_bot_id)
                    {
                        tracing::info!("✅ 个人微信登录成功！account_id={}", account_id);
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
                    tracing::info!("👀 已扫码，请在个人微信中确认登录...");
                    if let Some(ref host) = result.redirect_host {
                        api.set_qr_redirect_url(host).await;
                    } else {
                        tracing::warn!(
                            "Received scaned_but_redirect but redirect_host is missing, continuing with current host"
                        );
                    }
                }
                other => {
                    tracing::warn!("Unknown QR status: {}", other);
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(poll_interval)).await;
        }

        anyhow::bail!("个人微信登录超时，请重试")
    }

    /// Maximum consecutive failures before backing off.
    const MAX_CONSECUTIVE_FAILURES: u32 = 3;
    /// Backoff delay after too many consecutive failures (30 seconds).
    const BACKOFF_DELAY_MS: u64 = 30_000;
    /// Retry delay between individual failures (2 seconds).
    const RETRY_DELAY_MS: u64 = 2_000;
    /// Default long-poll timeout in milliseconds.
    const DEFAULT_LONG_POLL_TIMEOUT_MS: u64 = 35_000;

    /// Run the long-poll loop for receiving messages.
    ///
    /// Returns the reason the loop exited so the caller can decide
    /// whether to re-login or give up.
    async fn run_poll_loop(
        api: Arc<WeixinApi>,
        instance_id: String,
        cdn_base_url: String,
        context_tokens: Arc<Mutex<ContextTokenMap>>,
        typing_tickets: Arc<Mutex<HashMap<String, String>>>,
        event_sender: mpsc::Sender<PlatformEvent>,
        shutdown_rx: watch::Receiver<bool>,
        self_id: String,
    ) -> PollLoopExitReason {
        let mut consecutive_failures: u32 = 0;
        let mut next_poll_timeout_ms: u64 = Self::DEFAULT_LONG_POLL_TIMEOUT_MS;

        loop {
            // Check shutdown
            if *shutdown_rx.borrow() {
                tracing::info!(platform_id = %instance_id, "Poll loop shutting down");
                return PollLoopExitReason::Shutdown;
            }

            match api.get_updates_with_timeout(next_poll_timeout_ms).await {
                Ok(updates) => {
                    // Check for API-level errors in the response body
                    // (the server may return HTTP 200 but include an error code)
                    let is_api_error = (updates.ret.is_some() && updates.ret != Some(0))
                        || (updates.errcode.is_some() && updates.errcode != Some(0));

                    if is_api_error {
                        let errcode = updates.errcode.unwrap_or(0);
                        if errcode == -14 {
                            tracing::warn!(
                                platform_id = %instance_id,
                                "Session timeout (errcode=-14), need to re-login"
                            );
                            let _ = event_sender
                                .send(PlatformEvent::Error {
                                    platform_id: instance_id.clone(),
                                    message: "WeChat session timeout, attempting re-login..."
                                        .to_string(),
                                })
                                .await;
                            return PollLoopExitReason::SessionTimeout;
                        }

                        consecutive_failures += 1;
                        tracing::error!(
                            platform_id = %instance_id,
                            ret = ?updates.ret,
                            errcode = ?updates.errcode,
                            errmsg = ?updates.errmsg,
                            consecutive_failures,
                            "getUpdates API error in response body"
                        );

                        if consecutive_failures >= Self::MAX_CONSECUTIVE_FAILURES {
                            tracing::error!(
                                platform_id = %instance_id,
                                "{} consecutive failures, backing off for {}s",
                                Self::MAX_CONSECUTIVE_FAILURES,
                                Self::BACKOFF_DELAY_MS / 1000
                            );
                            consecutive_failures = 0;
                            tokio::time::sleep(std::time::Duration::from_millis(
                                Self::BACKOFF_DELAY_MS,
                            ))
                            .await;
                        } else {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                Self::RETRY_DELAY_MS,
                            ))
                            .await;
                        }
                        continue;
                    }

                    // Success — reset failure counter
                    consecutive_failures = 0;

                    // Use server-suggested long-poll timeout for next request
                    if let Some(timeout_ms) = updates.longpolling_timeout_ms {
                        if timeout_ms > 0 {
                            next_poll_timeout_ms = timeout_ms;
                            tracing::debug!(
                                platform_id = %instance_id,
                                "Updated next poll timeout to {}ms",
                                timeout_ms
                            );
                        }
                    }

                    if let Some(msgs) = updates.msgs {
                        for msg in msgs {
                            // Only process USER messages (type=1)
                            if msg.message_type != Some(1) {
                                continue;
                            }

                            let from_user_id = msg.from_user_id.as_deref().unwrap_or("unknown");
                            let item_types: Vec<String> = msg
                                .item_list
                                .as_ref()
                                .map(|items| {
                                    items
                                        .iter()
                                        .map(|i| format!("{}", i.item_type.unwrap_or(0)))
                                        .collect()
                                })
                                .unwrap_or_default();
                            tracing::info!(
                                platform_id = %instance_id,
                                from = %from_user_id,
                                item_types = ?item_types,
                                has_context_token = msg.context_token.is_some(),
                                "Inbound message"
                            );

                            if let Some(platform_msg) =
                                convert_weixin_message(&msg, &instance_id, &self_id, &cdn_base_url)
                            {
                                // Cache context_token keyed by session_id so that
                                // `send_message` can look it up via `target_id` (which
                                // equals session_id for both friend and group messages).
                                if let Some(ref token) = msg.context_token {
                                    let mut ctx = context_tokens.lock().await;
                                    ctx.insert(platform_msg.session_id.clone(), token.clone());
                                }

                                // Fetch typing ticket for this user (best-effort)
                                let user_id_for_typing = platform_msg.session_id.clone();
                                let ctx_token_for_config = msg.context_token.clone();
                                let api_clone = api.clone();
                                let typing_tickets_clone = typing_tickets.clone();
                                tokio::spawn(async move {
                                    if let Ok(config_resp) = api_clone
                                        .get_config(
                                            &user_id_for_typing,
                                            ctx_token_for_config.as_deref(),
                                        )
                                        .await
                                    {
                                        if let Some(ticket) = config_resp.typing_ticket {
                                            let mut tickets = typing_tickets_clone.lock().await;
                                            tickets.insert(user_id_for_typing, ticket);
                                        }
                                    }
                                });

                                // Send typing indicator (best-effort)
                                let typing_tickets_for_send = typing_tickets.clone();
                                let api_for_typing = api.clone();
                                let user_id_typing = platform_msg.session_id.clone();
                                tokio::spawn(async move {
                                    let tickets = typing_tickets_for_send.lock().await;
                                    if let Some(ticket) = tickets.get(&user_id_typing) {
                                        let _ = api_for_typing
                                            .send_typing(&user_id_typing, ticket, 1)
                                            .await;
                                    }
                                });

                                let event = PlatformEvent::Message(platform_msg);
                                if event_sender.send(event).await.is_err() {
                                    return PollLoopExitReason::SendError;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    consecutive_failures += 1;
                    tracing::error!(
                        platform_id = %instance_id,
                        error = %e,
                        consecutive_failures,
                        "getUpdates error"
                    );

                    if consecutive_failures >= Self::MAX_CONSECUTIVE_FAILURES {
                        // Check if the error looks like an authentication failure
                        // (e.g. 401/403 when the token has expired). In that case,
                        // trigger a re-login instead of just backing off.
                        let error_msg = e.to_string().to_lowercase();
                        let is_auth_error = error_msg.contains("401")
                            || error_msg.contains("403")
                            || error_msg.contains("unauthorized")
                            || error_msg.contains("forbidden");
                        if is_auth_error {
                            tracing::warn!(
                                platform_id = %instance_id,
                                "Persistent auth errors detected, triggering session timeout for re-login"
                            );
                            return PollLoopExitReason::SessionTimeout;
                        }

                        tracing::error!(
                            platform_id = %instance_id,
                            "{} consecutive failures, backing off for {}s",
                            Self::MAX_CONSECUTIVE_FAILURES,
                            Self::BACKOFF_DELAY_MS / 1000
                        );
                        consecutive_failures = 0;
                        tokio::time::sleep(std::time::Duration::from_millis(
                            Self::BACKOFF_DELAY_MS,
                        ))
                        .await;
                    } else {
                        tokio::time::sleep(std::time::Duration::from_millis(Self::RETRY_DELAY_MS))
                            .await;
                    }
                }
            }
        }
    }
}

/// Reason the poll loop exited.
#[derive(Debug)]
enum PollLoopExitReason {
    /// Graceful shutdown requested.
    Shutdown,
    /// Session timed out (errcode=-14); the caller should re-login.
    SessionTimeout,
    /// The event sender channel was closed (receiver dropped).
    SendError,
}

/// Standalone QR login function that can be called from the spawned task.
///
/// This is the same logic as `WeixinOcAdapter::do_qr_login` but doesn't
/// require `&self`, so it works inside a `tokio::spawn` closure.
async fn do_qr_login_standalone(
    api: &WeixinApi,
    instance_id: &str,
    qr_poll_interval_ms: u64,
) -> anyhow::Result<()> {
    // Step 1: Get QR code
    tracing::info!(
        platform_id = %instance_id,
        "Starting Personal WeChat QR login..."
    );

    let qr_resp = api.qr_login_start().await?;

    tracing::info!("请使用手机微信扫码登录个人微信，二维码有效期 5 分钟，过期后会自动刷新。");

    // Print QR code URL
    tracing::info!("QR code URL: {}", qr_resp.qrcode_img_content);

    // Try to render a terminal QR code
    print_qr_code(&qr_resp.qrcode_img_content);

    // Step 2: Poll until confirmed
    let login_timeout_ms: u64 = 480_000; // 8 minutes
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(login_timeout_ms);
    let mut max_qr_refresh = 3;
    let mut current_qrcode = qr_resp.qrcode.clone();

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
                tracing::info!("👀 已扫码，请在个人微信中确认登录...");
            }
            "confirmed" => {
                if let (Some(token), Some(account_id)) = (&result.bot_token, &result.ilink_bot_id) {
                    tracing::info!("✅ 个人微信登录成功！account_id={}", account_id);
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
                tracing::info!("👀 已扫码，请在个人微信中确认登录...");
                if let Some(ref host) = result.redirect_host {
                    api.set_qr_redirect_url(host).await;
                } else {
                    tracing::warn!(
                        "Received scaned_but_redirect but redirect_host is missing, continuing with current host"
                    );
                }
            }
            other => {
                tracing::warn!("Unknown QR status: {}", other);
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(qr_poll_interval_ms)).await;
    }

    anyhow::bail!("个人微信登录超时，请重试")
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

    // WeChat ClawBot only supports private (1-on-1) messages;
    // all messages are treated as friend messages.
    let _group_id = msg.group_id.clone().unwrap_or_default();
    let session_id = from_user_id.to_string();

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
        message_type: MessageType::FriendMessage,
        message_str,
        components,
        sender: crate::platform::types::MessageSender {
            user_id: from_user_id.to_string(),
            nickname: String::new(),
        },
        self_id: self_id.to_string(),
        group_id: String::new(),
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
            // Sync the updated token/account_id back into self.config
            // so persist_config_hint() can return them.
            self.sync_config_from_api().await;
        }

        // Reset the sync cursor before starting polling
        {
            let state = self.api.state();
            let mut guard = state.write().await;
            guard.get_updates_buf = String::new();
        }

        // Get self_id
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
        let typing_tickets = self.typing_tickets.clone();
        let qr_poll_interval_ms = self.config.qr_poll_interval_ms;

        // Spawn the main loop that handles both polling and re-login.
        // On session timeout (errcode=-14) the poll loop returns
        // `PollLoopExitReason::SessionTimeout`, and the outer loop
        // re-does QR login before resuming polling.
        tokio::spawn(async move {
            let poll_shutdown_rx = shutdown_rx;
            loop {
                let exit_reason = WeixinOcAdapter::run_poll_loop(
                    api.clone(),
                    instance_id.clone(),
                    cdn_base_url.clone(),
                    context_tokens.clone(),
                    typing_tickets.clone(),
                    event_sender.clone(),
                    poll_shutdown_rx.clone(),
                    self_id.clone(),
                )
                .await;

                match exit_reason {
                    PollLoopExitReason::Shutdown | PollLoopExitReason::SendError => {
                        tracing::info!(
                            platform_id = %instance_id,
                            "Poll loop exited ({:?}), not re-connecting",
                            exit_reason
                        );
                        break;
                    }
                    PollLoopExitReason::SessionTimeout => {
                        tracing::info!(
                            platform_id = %instance_id,
                            "Session expired, starting QR re-login..."
                        );

                        // Clear stale credentials in the API state
                        {
                            let state = api.state();
                            let mut guard = state.write().await;
                            guard.token = None;
                            guard.account_id = None;
                            guard.get_updates_buf = String::new();
                            guard.qr_redirect_base_url = None;
                        }

                        // Re-login via QR
                        match do_qr_login_standalone(&api, &instance_id, qr_poll_interval_ms).await
                        {
                            Ok(()) => {
                                tracing::info!(
                                    platform_id = %instance_id,
                                    "QR re-login succeeded, resuming poll loop"
                                );
                                // Notify that config should be persisted
                                let _ = event_sender
                                    .send(PlatformEvent::StatusChanged {
                                        platform_id: instance_id.clone(),
                                        status: PlatformStatus::Running,
                                    })
                                    .await;
                                // Continue the outer loop → poll again
                            }
                            Err(e) => {
                                tracing::error!(
                                    platform_id = %instance_id,
                                    error = %e,
                                    "QR re-login failed, giving up"
                                );
                                let _ = event_sender
                                    .send(PlatformEvent::Error {
                                        platform_id: instance_id.clone(),
                                        message: format!("QR re-login failed: {}", e),
                                    })
                                    .await;
                                break;
                            }
                        }
                    }
                }
            }
        });

        tracing::info!(
            platform_id = %self.instance_id,
            "WeChat ClawBot adapter started"
        );

        Ok(())
    }

    async fn terminate(&mut self) -> anyhow::Result<()> {
        // Sync latest credentials from API state before shutting down,
        // so persist_config_hint() returns the most up-to-date values
        // when the platform manager drains config updates on shutdown.
        //
        // IMPORTANT: We only sync NON-EMPTY credentials. If the API state
        // has token=None (e.g. during a session timeout while awaiting
        // QR re-login), we must NOT overwrite the existing valid token
        // in self.config. Otherwise the next persist_config_hint() call
        // would return a config with an empty token, which would then
        // be written to platforms.yaml — permanently losing the login
        // credentials.
        {
            let state = self.api.state();
            if let Ok(guard) = state.try_read() {
                let mut changed = false;
                // Only replace credentials when the API state has NEW non-empty values
                if guard.token.is_some() && guard.token != self.config.token {
                    self.config.token = guard.token.clone();
                    changed = true;
                }
                if guard.account_id.is_some() && guard.account_id != self.config.account_id {
                    self.config.account_id = guard.account_id.clone();
                    changed = true;
                }
                if guard.base_url != self.config.base_url {
                    self.config.base_url = guard.base_url.clone();
                    changed = true;
                }
                if changed {
                    self.config_dirty = true;
                    tracing::info!(
                        platform_id = %self.instance_id,
                        "Synced latest credentials before shutdown (dirty={})",
                        self.config_dirty
                    );
                }
            }
        }
        self.status = PlatformStatus::Stopped;
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        tracing::info!(platform_id = %self.instance_id, "WeChat ClawBot adapter terminated");
        Ok(())
    }

    async fn send_message(&self, message: OutboundMessage) -> anyhow::Result<()> {
        // WeChat ClawBot only supports private (1-on-1) messages.
        // target_id is always the user's ID (same as session_id for friend messages).
        let to_user_id = &message.target_id;

        // Look up cached context_token by session_id (= target_id)
        let context_token = {
            let ctx = self.context_tokens.lock().await;
            ctx.get(&message.target_id).cloned()
        };

        // Cancel typing indicator before sending reply (best-effort)
        let typing_ticket = {
            let tickets = self.typing_tickets.lock().await;
            tickets.get(&message.target_id).cloned()
        };
        if let Some(ref ticket) = typing_ticket {
            let _ = self
                .api
                .send_typing(to_user_id, ticket, 2) // 2 = cancel
                .await;
        }

        match &message.content {
            OutboundContent::Text { content } => {
                self.api
                    .send_text_message(to_user_id, content, context_token.as_deref())
                    .await
            }
            OutboundContent::Image { photo_url } => {
                // Full CDN upload not yet implemented, send URL as text.
                tracing::warn!(
                    "Image sending via CDN upload is not yet fully implemented, sending URL as text"
                );
                self.api
                    .send_text_message(to_user_id, photo_url, context_token.as_deref())
                    .await
            }
            OutboundContent::Markdown { title: _, text } => {
                // WeChat does not natively support markdown, send as plain text
                self.api
                    .send_text_message(to_user_id, text, context_token.as_deref())
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

    fn persist_config_hint(&self) -> Option<serde_json::Value> {
        // Check if credentials have been updated in the API state since
        // the config was last persisted.  This covers both the initial
        // QR login case (`config_dirty` is set by `sync_config_from_api`)
        // AND the re-login case (where the spawned task updates the API
        // state but can't reach `self.config`).
        //
        // We use `try_read` because this is a sync method and the lock
        // is an async `tokio::sync::RwLock`.  If we can't acquire the
        // lock immediately we return None – we'll retry on the next call.
        //
        // IMPORTANT: We only consider the config dirty if the API state
        // has NEW non-empty credentials to persist. If the API state has
        // token=None (e.g. during a session timeout while awaiting QR
        // re-login), we must NOT flag as dirty, because that would cause
        // us to return a config that overwrites the existing valid token
        // in platforms.yaml with an empty one — permanently losing the
        // login credentials.
        let binding = self.api.state();
        let api_state = binding.try_read().ok();
        let dirty = if self.config_dirty {
            // config_dirty can be set by sync_config_from_api() after QR login,
            // which only sets it when guard.token.is_some(). Safe to persist.
            true
        } else if let Some(guard) = &api_state {
            // Only treat as dirty if the API state has NEW credentials
            // (non-empty) that differ from what's stored in self.config.
            // If the API state has cleared credentials (session timeout),
            // we do NOT treat this as dirty — we want to preserve the
            // existing token in the config file.
            let has_new_token = guard.token.is_some() && guard.token != self.config.token;
            let has_new_account =
                guard.account_id.is_some() && guard.account_id != self.config.account_id;
            has_new_token || has_new_account
        } else {
            false
        };

        if !dirty {
            return None;
        }

        // Build an updated config value.  Try to read the current API
        // state so we always return the latest credentials.
        // Only update credentials from API state when they are non-empty,
        // to avoid clearing valid tokens during session timeout.
        let mut config = self.config.clone();
        if let Some(guard) = &api_state {
            if guard.token.is_some() {
                config.token = guard.token.clone();
            }
            if guard.account_id.is_some() {
                config.account_id = guard.account_id.clone();
            }
        }
        serde_json::to_value(&config).ok()
    }

    fn mark_config_persisted(&mut self) {
        self.config_dirty = false;
        // Sync self.config from the API state so persist_config_hint
        // won't keep returning Some on future calls.
        if let Ok(guard) = self.api.state().try_read() {
            if guard.token.is_some() {
                self.config.token = guard.token.clone();
            }
            if guard.account_id.is_some() {
                self.config.account_id = guard.account_id.clone();
            }
        }
    }
}

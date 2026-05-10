//! Discord platform adapter for Ruri.
//!
//! This adapter supports two modes of connecting to the Discord Gateway:
//!
//! 1. **Serenity mode** (default, no proxy): Uses the `serenity` crate's built-in
//!    Gateway client. This is the recommended mode when no proxy is needed.
//!
//! 2. **Custom Gateway mode** (with proxy): When `proxy_url` is configured,
//!    serenity's built-in client cannot be used (it doesn't support WebSocket
//!    proxies). Instead, we implement a minimal Discord Gateway v10 client
//!    using `reqwest` for REST API calls (with proxy) and `tokio-tungstenite`
//!    for the Gateway WebSocket (connected through the proxy).
//!
//! # Configuration
//!
//! Each Discord bot instance needs a YAML config block like:
//!
//! ```yaml
//! platforms:
//!   - type: discord
//!     id: my-discord-bot
//!     enable: true
//!     token: "BOT_TOKEN_HERE"
//!     pre_response_reactions: true   # optional
//!     reaction_emojis: ["👍", "🤔", "⏳"]  # optional
//!     proxy_url: "http://127.0.0.1:7890"  # optional: HTTP/SOCKS5 proxy
//! ```

use crate::platform::discord::config::DiscordConfig;
use crate::platform::trait_def::{Platform, PlatformEvent};
use crate::platform::types::{
    MessageComponent, MessageType, OutboundContent, OutboundMessage, PlatformMessage,
    PlatformStatus,
};
use crate::transport::proxy_ws::connect_ws_with_proxy;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serenity::async_trait as serenity_async_trait;
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::client::ClientBuilder;
use serenity::model::channel::Message as DiscordMessage;
use serenity::model::channel::ReactionType;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, watch};
use tracing as log;

// ─── TypeMap keys for sharing data with the event handler ────────────────────

/// Key for storing the event sender in the client's TypeMap.
struct EventSenderKey;

impl TypeMapKey for EventSenderKey {
    type Value = mpsc::Sender<PlatformEvent>;
}

/// Key for storing the platform ID in the client's TypeMap.
struct PlatformIdKey;

impl TypeMapKey for PlatformIdKey {
    type Value = String;
}

/// Key for storing pre-response reaction config.
struct PreResponseConfigKey;

impl TypeMapKey for PreResponseConfigKey {
    type Value = (bool, Vec<String>); // (enabled, emojis)
}

/// Key for storing the bot's own user ID.
struct BotUserIdKey;

impl TypeMapKey for BotUserIdKey {
    type Value = Arc<Mutex<String>>;
}

// ─── Discord adapter ─────────────────────────────────────────────────────────

/// HTTP client type used by the adapter.
/// In serenity mode this wraps serenity's `Http`; in proxy mode this wraps
/// a plain `reqwest::Client`.
enum HttpClient {
    Serenity(Arc<serenity::http::Http>),
    Reqwest {
        client: reqwest::Client,
        token: String,
    },
}

/// Discord platform adapter.
pub struct DiscordAdapter {
    config: DiscordConfig,
    instance_id: String,
    status: PlatformStatus,
    /// Channel to signal the Discord client to shut down.
    shutdown_tx: Option<watch::Sender<bool>>,
    /// HTTP client for sending messages.
    /// In serenity mode: wraps the serenity Http handle.
    /// In proxy mode: wraps a reqwest Client with proxy configured.
    http: Arc<Mutex<Option<HttpClient>>>,
}

impl DiscordAdapter {
    /// Create a new adapter from an instance ID and JSON config.
    pub fn from_config(instance_id: String, extra: &serde_json::Value) -> Result<Self, String> {
        let config: DiscordConfig = serde_json::from_value(extra.clone())
            .map_err(|e| format!("Invalid Discord config: {}", e))?;

        if config.token.is_empty() {
            return Err("Discord config missing `token`".into());
        }

        Ok(Self {
            config,
            instance_id,
            status: PlatformStatus::Pending,
            shutdown_tx: None,
            http: Arc::new(Mutex::new(None)),
        })
    }
}

/// Convert a Discord message into our unified [`PlatformMessage`].
fn convert_discord_message(
    msg: &DiscordMessage,
    platform_id: &str,
    self_id: &str,
) -> PlatformMessage {
    let message_type = if msg.guild_id.is_some() {
        MessageType::GroupMessage
    } else {
        MessageType::FriendMessage
    };

    let mut components: Vec<MessageComponent> = Vec::new();

    // Text content
    if !msg.content.is_empty() {
        components.push(MessageComponent::Plain {
            text: msg.content.clone(),
        });
    }

    // Attachments (images, files, etc.)
    for attachment in &msg.attachments {
        if attachment
            .content_type
            .as_ref()
            .is_some_and(|ct| ct.starts_with("image/"))
        {
            components.push(MessageComponent::Image {
                url: attachment.url.clone(),
            });
        } else if attachment
            .content_type
            .as_ref()
            .is_some_and(|ct| ct.starts_with("audio/"))
        {
            components.push(MessageComponent::Voice {
                url: attachment.url.clone(),
            });
        } else {
            components.push(MessageComponent::File {
                name: attachment.filename.clone(),
                url: attachment.url.clone(),
            });
        }
    }

    // Mentions → At components
    for mention in &msg.mentions {
        components.push(MessageComponent::At {
            user_id: mention.id.to_string(),
        });
    }

    let message_str = msg.content.clone();

    let group_id = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();

    // Session ID: always use channel_id so replies go to the right channel
    let session_id = msg.channel_id.to_string();

    let sender_user_id = msg.author.id.to_string();

    PlatformMessage {
        platform_id: platform_id.to_string(),
        message_id: msg.id.to_string(),
        message_type,
        message_str,
        components,
        sender: crate::platform::types::MessageSender {
            user_id: sender_user_id,
            nickname: msg.author.name.clone(),
        },
        self_id: self_id.to_string(),
        group_id,
        session_id,
        timestamp: msg.timestamp.unix_timestamp() as u64,
        raw: Some(serde_json::to_value(msg).unwrap_or(serde_json::Value::Null)),
    }
}

// ─── Serenity EventHandler ───────────────────────────────────────────────────

/// Custom event handler that forwards incoming Discord messages to the
/// platform event channel and adds pre-response reactions.
struct Handler;

#[serenity_async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        let bot_name = ready.user.name.clone();
        let bot_id = ready.user.id.to_string();

        // Store the bot's own user ID in TypeMap
        {
            let data = ctx.data.read().await;
            if let Some(self_id) = data.get::<BotUserIdKey>() {
                let mut id = self_id.lock().await;
                *id = bot_id;
            }
        }

        log::info!(bot_name = %bot_name, "Discord bot is ready");

        // Send a status-changed event
        {
            let data = ctx.data.read().await;
            if let Some(sender) = data.get::<EventSenderKey>() {
                if let Some(platform_id) = data.get::<PlatformIdKey>() {
                    let event = PlatformEvent::StatusChanged {
                        platform_id: platform_id.clone(),
                        status: PlatformStatus::Running,
                    };
                    if sender.send(event).await.is_err() {
                        log::warn!("Event channel closed, cannot send ready status");
                    }
                }
            }
        }
    }

    async fn message(&self, ctx: Context, msg: DiscordMessage) {
        // Ignore messages from the bot itself
        let self_id = {
            let data = ctx.data.read().await;
            let id_lock = data.get::<BotUserIdKey>().cloned();
            match id_lock {
                Some(lock) => {
                    let id = lock.lock().await;
                    id.clone()
                }
                None => String::new(),
            }
        };

        if self_id == msg.author.id.to_string() {
            return;
        }

        log::debug!(
            author = %msg.author.name,
            content = %msg.content.chars().take(80).collect::<String>(),
            "Received Discord message"
        );

        // Add a pre-response reaction if configured
        {
            let data = ctx.data.read().await;
            if let Some((enabled, emojis)) = data.get::<PreResponseConfigKey>() {
                if *enabled && !emojis.is_empty() {
                    let emoji_index = (msg.id.get() as usize) % emojis.len();
                    let emoji = &emojis[emoji_index];
                    if let Ok(reaction) = emoji.parse::<ReactionType>() {
                        if let Err(e) = msg.react(&ctx.http, reaction).await {
                            log::warn!(
                                error = %e,
                                "Failed to add pre-response reaction on Discord message"
                            );
                        }
                    }
                }
            }
        }

        let platform_id = {
            let data = ctx.data.read().await;
            data.get::<PlatformIdKey>().cloned().unwrap_or_default()
        };

        let platform_msg = convert_discord_message(&msg, &platform_id, &self_id);

        // Forward the message through the event channel
        {
            let data = ctx.data.read().await;
            if let Some(sender) = data.get::<EventSenderKey>() {
                let event = PlatformEvent::Message(platform_msg);
                if sender.send(event).await.is_err() {
                    log::warn!("Event channel closed, cannot forward Discord message");
                }
            }
        }
    }
}

// ─── Platform trait implementation ───────────────────────────────────────────

#[async_trait]
impl Platform for DiscordAdapter {
    async fn run(&mut self, event_sender: mpsc::Sender<PlatformEvent>) -> anyhow::Result<()> {
        let config = self.config.clone();
        let instance_id = self.instance_id.clone();

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        self.status = PlatformStatus::Running;

        if config.proxy_url.is_some() {
            // Custom gateway mode with proxy support
            self.run_custom_gateway(config, instance_id, event_sender, shutdown_rx)
                .await?;
        } else {
            // Standard serenity mode (no proxy)
            self.run_serenity_gateway(config, instance_id, event_sender, shutdown_rx)
                .await?;
        }

        Ok(())
    }

    async fn terminate(&mut self) -> anyhow::Result<()> {
        self.status = PlatformStatus::Stopped;
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        log::info!(platform_id = %self.instance_id, "Discord adapter terminated");
        Ok(())
    }

    async fn send_message(&self, message: OutboundMessage) -> anyhow::Result<()> {
        let http_guard = self.http.lock().await;
        let http = http_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Discord HTTP client not initialized"))?;

        match http {
            HttpClient::Serenity(serenity_http) => {
                let serenity_http = serenity_http.clone();
                drop(http_guard);
                self.send_message_serenity(&serenity_http, message).await
            }
            HttpClient::Reqwest { client, token } => {
                let client = client.clone();
                let token = token.clone();
                drop(http_guard);
                self.send_message_reqwest(&client, &token, message).await
            }
        }
    }

    fn status(&self) -> PlatformStatus {
        self.status
    }

    fn platform_type(&self) -> &str {
        "discord"
    }
}

// ─── Serenity gateway mode ───────────────────────────────────────────────────

impl DiscordAdapter {
    /// Standard serenity-based gateway connection (no proxy).
    async fn run_serenity_gateway(
        &mut self,
        config: DiscordConfig,
        instance_id: String,
        event_sender: mpsc::Sender<PlatformEvent>,
        shutdown_rx: watch::Receiver<bool>,
    ) -> anyhow::Result<()> {
        // Intents: we need message content, guild messages, and DMs
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILDS;

        // Build the serenity client
        let mut client = ClientBuilder::new(&config.token, intents)
            .event_handler(Handler)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Discord client: {}", e))?;

        // Insert shared data into the client's TypeMap
        {
            let mut data = client.data.write().await;
            data.insert::<EventSenderKey>(event_sender);
            data.insert::<PlatformIdKey>(instance_id.clone());
            data.insert::<PreResponseConfigKey>((
                config.pre_response_reactions,
                config.reaction_emojis.clone(),
            ));
            data.insert::<BotUserIdKey>(Arc::new(Mutex::new(String::new())));
        }

        // Clone the Http handle before moving the client into the gateway task.
        let http = client.http.clone();
        {
            let mut http_guard = self.http.lock().await;
            *http_guard = Some(HttpClient::Serenity(http));
        }

        // Spawn the gateway connection as a background task.
        tokio::spawn(async move {
            loop {
                let result = client.start().await;

                if let Err(e) = &result {
                    log::error!(
                        error = %e,
                        platform_id = %instance_id,
                        "Discord gateway error"
                    );
                }

                // Check if we should shut down
                if shutdown_rx.has_changed().unwrap_or(false) {
                    log::info!(
                        platform_id = %instance_id,
                        "Discord adapter shutting down"
                    );
                    return;
                }

                // Connection was lost, wait before reconnecting
                log::info!(
                    platform_id = %instance_id,
                    "Discord connection lost, reconnecting in 5 seconds..."
                );

                for _ in 0..50 {
                    if shutdown_rx.has_changed().unwrap_or(false) {
                        return;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        });

        log::info!(
            platform_id = %self.instance_id,
            "Discord adapter started (serenity mode)"
        );

        Ok(())
    }

    /// Send a message using serenity's HTTP client (non-proxy mode).
    async fn send_message_serenity(
        &self,
        http: &Arc<serenity::http::Http>,
        message: OutboundMessage,
    ) -> anyhow::Result<()> {
        let channel_id: ChannelId = message
            .target_id
            .parse::<u64>()
            .map_err(|e| {
                anyhow::anyhow!("Invalid Discord channel ID '{}': {}", message.target_id, e)
            })?
            .into();

        match &message.content {
            OutboundContent::Text { content } => {
                let chunks = split_message(content, 2000);
                for chunk in chunks {
                    channel_id
                        .say(http, &chunk)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to send Discord message: {}", e))?;
                }
            }
            OutboundContent::Markdown { title, text } => {
                let embed = CreateEmbed::new().title(title).description(text);
                let builder = CreateMessage::new().embed(embed);
                channel_id
                    .send_message(http, builder)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to send Discord embed: {}", e))?;
            }
            OutboundContent::Image { photo_url } => {
                let embed = CreateEmbed::new().image(photo_url);
                let builder = CreateMessage::new().embed(embed);
                channel_id
                    .send_message(http, builder)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to send Discord image: {}", e))?;
            }
            OutboundContent::File {
                file_name,
                media_id,
                ..
            } => {
                let msg_text = format!("📄 **{}**: {}", file_name, media_id);
                channel_id
                    .say(http, &msg_text)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to send Discord file message: {}", e))?;
            }
        }

        Ok(())
    }

    /// Send a message using reqwest HTTP client (proxy mode).
    async fn send_message_reqwest(
        &self,
        client: &reqwest::Client,
        token: &str,
        message: OutboundMessage,
    ) -> anyhow::Result<()> {
        let channel_id = message.target_id;
        let base_url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            channel_id
        );

        match &message.content {
            OutboundContent::Text { content } => {
                let chunks = split_message(content, 2000);
                for chunk in chunks {
                    let body = serde_json::json!({"content": chunk});
                    let resp = client
                        .post(&base_url)
                        .header("Authorization", format!("Bot {}", token))
                        .json(&body)
                        .send()
                        .await?;

                    if !resp.status().is_success() {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        anyhow::bail!(
                            "Failed to send Discord message: status={}, body={}",
                            status,
                            body
                        );
                    }
                }
            }
            OutboundContent::Markdown { title, text } => {
                let body = serde_json::json!({
                    "embeds": [{"title": title, "description": text}]
                });
                let resp = client
                    .post(&base_url)
                    .header("Authorization", format!("Bot {}", token))
                    .json(&body)
                    .send()
                    .await?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let body_text = resp.text().await.unwrap_or_default();
                    anyhow::bail!(
                        "Failed to send Discord embed: status={}, body={}",
                        status,
                        body_text
                    );
                }
            }
            OutboundContent::Image { photo_url } => {
                let body = serde_json::json!({
                    "embeds": [{"image": {"url": photo_url}}]
                });
                let resp = client
                    .post(&base_url)
                    .header("Authorization", format!("Bot {}", token))
                    .json(&body)
                    .send()
                    .await?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let body_text = resp.text().await.unwrap_or_default();
                    anyhow::bail!(
                        "Failed to send Discord image: status={}, body={}",
                        status,
                        body_text
                    );
                }
            }
            OutboundContent::File {
                file_name,
                media_id,
                ..
            } => {
                let msg_text = format!("📄 **{}**: {}", file_name, media_id);
                let body = serde_json::json!({"content": msg_text});
                let resp = client
                    .post(&base_url)
                    .header("Authorization", format!("Bot {}", token))
                    .json(&body)
                    .send()
                    .await?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let body_text = resp.text().await.unwrap_or_default();
                    anyhow::bail!(
                        "Failed to send Discord file message: status={}, body={}",
                        status,
                        body_text
                    );
                }
            }
        }

        Ok(())
    }
}

// ─── Custom Gateway mode (proxy support) ─────────────────────────────────────

impl DiscordAdapter {
    /// Run the custom Discord Gateway client through a proxy.
    async fn run_custom_gateway(
        &mut self,
        config: DiscordConfig,
        instance_id: String,
        event_sender: mpsc::Sender<PlatformEvent>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> anyhow::Result<()> {
        // Build reqwest HTTP client with proxy for REST API calls
        let http_client = if let Some(ref proxy_url) = config.proxy_url {
            let proxy = reqwest::Proxy::all(proxy_url)
                .map_err(|e| anyhow::anyhow!("Invalid proxy URL '{}': {}", proxy_url, e))?;
            reqwest::Client::builder()
                .proxy(proxy)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build HTTP client with proxy: {}", e))?
        } else {
            reqwest::Client::new()
        };

        // Store the HTTP client
        {
            let mut http_guard = self.http.lock().await;
            *http_guard = Some(HttpClient::Reqwest {
                client: http_client.clone(),
                token: config.token.clone(),
            });
        }

        let token = config.token.clone();
        let platform_id = instance_id.clone();
        let proxy_url = config.proxy_url.clone();
        let pre_response_reactions = config.pre_response_reactions;
        let reaction_emojis = config.reaction_emojis.clone();

        // Spawn the gateway loop as a background task
        tokio::spawn(async move {
            let max_retries = 10u32;
            let mut retry_count = 0u32;
            let base_delay_secs = 5u64;

            loop {
                if *shutdown_rx.borrow() {
                    log::info!(platform_id = %platform_id, "Discord custom gateway shutting down");
                    return;
                }

                match run_custom_gateway_once(
                    &http_client,
                    &token,
                    &platform_id,
                    proxy_url.as_deref(),
                    &event_sender,
                    pre_response_reactions,
                    &reaction_emojis,
                    &mut shutdown_rx,
                )
                .await
                {
                    Ok(()) => {
                        log::info!(platform_id = %platform_id, "Discord custom gateway closed normally");
                        retry_count = 0;
                        if *shutdown_rx.borrow() {
                            return;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    }
                    Err(e) => {
                        retry_count += 1;
                        if retry_count >= max_retries {
                            log::error!(
                                error = %e,
                                platform_id = %platform_id,
                                "Discord custom gateway max retries reached"
                            );

                            // Send error event
                            let event = PlatformEvent::Error {
                                platform_id: platform_id.clone(),
                                message: format!(
                                    "Gateway connection failed after {} retries: {}",
                                    max_retries, e
                                ),
                            };
                            if event_sender.send(event).await.is_err() {
                                log::warn!("Event channel closed, cannot send error event");
                            }
                            return;
                        }
                        let delay = base_delay_secs * 2u64.pow(retry_count.min(5) - 1);
                        log::warn!(
                            error = %e,
                            retry = retry_count,
                            delay_secs = delay,
                            platform_id = %platform_id,
                            "Discord custom gateway error, retrying"
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                    }
                }
            }
        });

        log::info!(
            platform_id = %self.instance_id,
            proxy_url = ?config.proxy_url,
            "Discord adapter started (custom gateway mode with proxy)"
        );

        Ok(())
    }
}

// ─── Discord Gateway protocol types ──────────────────────────────────────────

/// Gateway opcodes.
mod op {
    pub const DISPATCH: u64 = 0;
    pub const HEARTBEAT: u64 = 1;
    pub const IDENTIFY: u64 = 2;
    pub const RESUME: u64 = 6;
    pub const RECONNECT: u64 = 7;
    pub const INVALID_SESSION: u64 = 9;
    pub const HELLO: u64 = 10;
    pub const HEARTBEAT_ACK: u64 = 11;
}

/// Discord Gateway intents bitmask.
const INTENTS: u64 = 1 |       // GUILDS
    512 |                    // GUILD_MESSAGES
    4096 |                   // DIRECT_MESSAGES
    32768; // MESSAGE_CONTENT (1 << 15)

/// A single gateway payload.
#[derive(Debug)]
struct GatewayPayload {
    op: u64,
    t: Option<String>,
    s: Option<u64>,
    d: serde_json::Value,
}

impl GatewayPayload {
    fn from_value(value: serde_json::Value) -> anyhow::Result<Self> {
        let obj = value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Gateway payload is not an object"))?;
        Ok(Self {
            op: obj
                .get("op")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("Missing 'op' in gateway payload"))?,
            t: obj.get("t").and_then(|v| v.as_str()).map(|s| s.to_string()),
            s: obj.get("s").and_then(|v| v.as_u64()),
            d: obj.get("d").cloned().unwrap_or(serde_json::Value::Null),
        })
    }
}

/// Fetch the Gateway bot URL from Discord REST API.
async fn fetch_gateway_url(http: &reqwest::Client, token: &str) -> anyhow::Result<String> {
    let resp = http
        .get("https://discord.com/api/v10/gateway/bot")
        .header("Authorization", format!("Bot {}", token))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "Failed to get gateway URL: status={}, body={}",
            status,
            body
        );
    }

    let data: serde_json::Value = resp.json().await?;
    let url = data["url"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No 'url' in gateway/bot response"))?;

    Ok(format!("{}/?v=10&encoding=json", url))
}

/// Add a pre-response reaction to a message via REST API.
async fn add_pre_response_reaction(
    http: &reqwest::Client,
    token: &str,
    channel_id: &str,
    message_id: &str,
    emojis: &[String],
    message_id_u64: u64,
) {
    if emojis.is_empty() {
        return;
    }
    let emoji_index = (message_id_u64 as usize) % emojis.len();
    let emoji = &emojis[emoji_index];

    // URL-encode the emoji for the reaction endpoint
    let encoded_emoji = urlencoding::encode(emoji);
    let url = format!(
        "https://discord.com/api/v10/channels/{}/messages/{}/reactions/{}/@me",
        channel_id, message_id, encoded_emoji
    );

    if let Err(e) = http
        .put(&url)
        .header("Authorization", format!("Bot {}", token))
        .send()
        .await
    {
        log::warn!(error = %e, "Failed to add pre-response reaction on Discord message");
    }
}

/// Convert a MESSAGE_CREATE JSON payload into a PlatformMessage.
fn convert_gateway_message(
    data: &serde_json::Value,
    platform_id: &str,
    self_id: &str,
) -> Option<PlatformMessage> {
    let author = data.get("author")?;
    let author_id = author.get("id")?.as_str()?;

    // Skip messages from the bot itself
    if author_id == self_id {
        return None;
    }

    let content = data
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let channel_id = data
        .get("channel_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let message_id = data
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let guild_id = data
        .get("guild_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let message_type = if guild_id.is_empty() {
        MessageType::FriendMessage
    } else {
        MessageType::GroupMessage
    };

    let author_name = author
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let mut components: Vec<MessageComponent> = Vec::new();

    // Text content
    if !content.is_empty() {
        components.push(MessageComponent::Plain {
            text: content.clone(),
        });
    }

    // Attachments
    if let Some(attachments) = data.get("attachments").and_then(|v| v.as_array()) {
        for attachment in attachments {
            let url = attachment
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let content_type = attachment
                .get("content_type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let filename = attachment
                .get("filename")
                .and_then(|v| v.as_str())
                .unwrap_or("file")
                .to_string();

            if content_type.starts_with("image/") {
                components.push(MessageComponent::Image { url });
            } else if content_type.starts_with("audio/") {
                components.push(MessageComponent::Voice { url });
            } else if !url.is_empty() {
                components.push(MessageComponent::File {
                    name: filename,
                    url,
                });
            }
        }
    }

    // Mentions
    if let Some(mentions) = data.get("mentions").and_then(|v| v.as_array()) {
        for mention in mentions {
            if let Some(user_id) = mention.get("id").and_then(|v| v.as_str()) {
                components.push(MessageComponent::At {
                    user_id: user_id.to_string(),
                });
            }
        }
    }

    let timestamp = data
        .get("timestamp")
        .and_then(|v| v.as_str())
        .and_then(|ts| {
            // Parse ISO 8601 timestamp to unix seconds
            chrono::DateTime::parse_from_rfc3339(ts).ok()
        })
        .map(|dt| dt.timestamp() as u64)
        .unwrap_or(0);

    Some(PlatformMessage {
        platform_id: platform_id.to_string(),
        message_id,
        message_type,
        message_str: content,
        components,
        sender: crate::platform::types::MessageSender {
            user_id: author_id.to_string(),
            nickname: author_name,
        },
        self_id: self_id.to_string(),
        group_id: guild_id,
        session_id: channel_id,
        timestamp,
        raw: Some(data.clone()),
    })
}

/// Run a single connection attempt to the Discord Gateway via custom implementation.
async fn run_custom_gateway_once(
    http: &reqwest::Client,
    token: &str,
    platform_id: &str,
    proxy_url: Option<&str>,
    event_sender: &mpsc::Sender<PlatformEvent>,
    pre_response_reactions: bool,
    reaction_emojis: &[String],
    shutdown_rx: &mut watch::Receiver<bool>,
) -> anyhow::Result<()> {
    // 1. Get Gateway URL
    let ws_url = fetch_gateway_url(http, token).await?;
    log::info!(url = %ws_url, "Fetched Discord gateway URL");

    // 2. Connect WebSocket through proxy
    let ws = connect_ws_with_proxy(&ws_url, proxy_url).await?;
    let (mut ws_sink, mut ws_stream) = ws.split();

    log::info!("Discord custom gateway WebSocket connected");

    // 3. Receive Hello (op=10) → extract heartbeat_interval
    let heartbeat_interval = {
        let msg = ws_stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("No Hello received from gateway"))?
            .map_err(|e| anyhow::anyhow!("WebSocket read error waiting for Hello: {}", e))?;

        let text = match msg {
            tokio_tungstenite::tungstenite::Message::Text(t) => t,
            other => {
                anyhow::bail!("Expected text frame for Hello, got: {:?}", other);
            }
        };

        let payload: serde_json::Value = serde_json::from_str(&text)?;
        let gw = GatewayPayload::from_value(payload)?;

        if gw.op != op::HELLO {
            anyhow::bail!("Expected Hello (op=10), got op={}", gw.op);
        }

        gw.d["heartbeat_interval"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("No heartbeat_interval in Hello"))?
    };

    log::info!(
        heartbeat_interval_ms = heartbeat_interval,
        "Received Hello from Discord gateway"
    );

    // 4. Send Identify
    let identify = serde_json::json!({
        "op": op::IDENTIFY,
        "d": {
            "token": token,
            "intents": INTENTS,
            "properties": {
                "os": std::env::consts::OS,
                "browser": "ruri",
                "device": "ruri"
            }
        }
    });

    ws_sink
        .send(tokio_tungstenite::tungstenite::Message::Text(
            identify.to_string().into(),
        ))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send Identify: {}", e))?;

    log::info!("Sent Identify to Discord gateway");

    // 5. Wait for Ready (op=0, t=READY) and dispatch events
    let mut self_id = String::new();
    let mut session_id: Option<String> = None;
    let mut last_seq: Option<u64> = None;
    let mut heartbeat_ack_received = true; // Assume true initially

    // Spawn heartbeat task
    let (hb_tx, mut hb_rx) = mpsc::channel::<()>(1);
    let heartbeat_handle = {
        let hb_interval = heartbeat_interval;
        tokio::spawn(async move {
            // Initial heartbeat: wait heartbeat_interval * 0.8
            let jitter = (hb_interval as f64 * 0.8) as u64;
            tokio::time::sleep(tokio::time::Duration::from_millis(jitter)).await;

            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_millis(hb_interval));
            interval.tick().await; // First tick completes immediately

            loop {
                interval.tick().await;
                if hb_tx.send(()).await.is_err() {
                    break;
                }
            }
        })
    };

    // Main event loop
    let result = loop {
        tokio::select! {
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(ws_msg)) => {
                        let text = match ws_msg {
                            tokio_tungstenite::tungstenite::Message::Text(t) => t,
                            tokio_tungstenite::tungstenite::Message::Close(_) => {
                                log::info!("Discord gateway WebSocket close frame received");
                                break Ok(());
                            }
                            tokio_tungstenite::tungstenite::Message::Ping(data) => {
                                let _ = ws_sink.send(tokio_tungstenite::tungstenite::Message::Pong(data)).await;
                                continue;
                            }
                            _ => continue,
                        };

                        let payload: serde_json::Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(e) => {
                                log::warn!(error = %e, "Failed to parse gateway payload");
                                continue;
                            }
                        };

                        let gw = match GatewayPayload::from_value(payload) {
                            Ok(gw) => gw,
                            Err(e) => {
                                log::warn!(error = %e, "Failed to extract gateway payload fields");
                                continue;
                            }
                        };

                        // Update sequence number
                        if gw.s.is_some() {
                            last_seq = gw.s;
                        }

                        match gw.op {
                            op::DISPATCH => {
                                // Handle Ready event
                                if gw.t.as_deref() == Some("READY") {
                                    if let Some(user) = gw.d.get("user") {
                                        self_id = user
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let bot_name = user
                                            .get("username")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        log::info!(
                                            bot_name = %bot_name,
                                            bot_id = %self_id,
                                            "Discord bot is ready (custom gateway)"
                                        );
                                    }
                                    session_id = gw
                                        .d
                                        .get("session_id")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());

                                    // Send status event
                                    let event = PlatformEvent::StatusChanged {
                                        platform_id: platform_id.to_string(),
                                        status: PlatformStatus::Running,
                                    };
                                    if event_sender.send(event).await.is_err() {
                                        log::warn!("Event channel closed");
                                        break Ok(());
                                    }
                                }

                                // Handle MESSAGE_CREATE
                                if gw.t.as_deref() == Some("MESSAGE_CREATE") {
                                    if let Some(platform_msg) =
                                        convert_gateway_message(&gw.d, platform_id, &self_id)
                                    {
                                        // Add pre-response reaction if configured
                                        if pre_response_reactions && !reaction_emojis.is_empty() {
                                            if let Ok(msg_id_u64) = platform_msg.message_id.parse::<u64>() {
                                                add_pre_response_reaction(
                                                    http,
                                                    token,
                                                    &platform_msg.session_id,
                                                    &platform_msg.message_id,
                                                    reaction_emojis,
                                                    msg_id_u64,
                                                )
                                                .await;
                                            }
                                        }

                                        log::debug!(
                                            author = %platform_msg.sender.nickname,
                                            content = %platform_msg.message_str.chars().take(80).collect::<String>(),
                                            "Received Discord message (custom gateway)"
                                        );

                                        let event = PlatformEvent::Message(platform_msg);
                                        if event_sender.send(event).await.is_err() {
                                            log::warn!("Event channel closed");
                                            break Ok(());
                                        }
                                    }
                                }
                            }
                            op::HEARTBEAT => {
                                // Server requests an immediate heartbeat
                                let hb = serde_json::json!({
                                    "op": op::HEARTBEAT,
                                    "d": last_seq
                                });
                                if let Err(e) = ws_sink
                                    .send(tokio_tungstenite::tungstenite::Message::Text(
                                        hb.to_string().into(),
                                    ))
                                    .await
                                {
                                    log::warn!(error = %e, "Failed to send heartbeat");
                                    break Err(anyhow::anyhow!("Failed to send heartbeat: {}", e));
                                }
                                log::debug!("Sent heartbeat (requested by server)");
                            }
                            op::RECONNECT => {
                                log::info!("Discord gateway requested reconnect (op=7)");
                                break Err(anyhow::anyhow!("Gateway requested reconnect"));
                            }
                            op::INVALID_SESSION => {
                                let can_resume = gw.d.as_bool().unwrap_or(false);
                                if can_resume && session_id.is_some() && last_seq.is_some() {
                                    log::info!("Invalid session, attempting resume");
                                    let resume = serde_json::json!({
                                        "op": op::RESUME,
                                        "d": {
                                            "token": token,
                                            "session_id": session_id,
                                            "seq": last_seq
                                        }
                                    });
                                    if let Err(e) = ws_sink
                                        .send(tokio_tungstenite::tungstenite::Message::Text(
                                            resume.to_string().into(),
                                        ))
                                        .await
                                    {
                                        log::warn!(error = %e, "Failed to send Resume");
                                        break Err(anyhow::anyhow!("Failed to send Resume: {}", e));
                                    }
                                } else {
                                    log::info!("Invalid session, will re-identify on next connection");
                                    let _ = session_id.take();
                                    break Err(anyhow::anyhow!("Invalid session, cannot resume"));
                                }
                            }
                            op::HEARTBEAT_ACK => {
                                heartbeat_ack_received = true;
                                log::debug!("Heartbeat ACK received");
                            }
                            _ => {
                                log::debug!(op = gw.op, "Ignoring unknown gateway opcode");
                            }
                        }
                    }
                    Some(Err(e)) => {
                        break Err(anyhow::anyhow!("WebSocket read error: {}", e));
                    }
                    None => {
                        break Ok(());
                    }
                }
            }
            _ = hb_rx.recv() => {
                // Send heartbeat
                if !heartbeat_ack_received {
                    log::warn!("No heartbeat ACK received since last heartbeat, connection may be dead");
                    break Err(anyhow::anyhow!("Heartbeat timeout"));
                }
                heartbeat_ack_received = false;

                let hb = serde_json::json!({
                    "op": op::HEARTBEAT,
                    "d": last_seq
                });
                if let Err(e) = ws_sink
                    .send(tokio_tungstenite::tungstenite::Message::Text(hb.to_string().into()))
                    .await
                {
                    break Err(anyhow::anyhow!("Failed to send heartbeat: {}", e));
                }
                log::debug!("Sent heartbeat");
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    log::info!(platform_id = %platform_id, "Shutdown signal, closing Discord gateway WebSocket");
                    let _ = ws_sink.close().await;
                    break Ok(());
                }
            }
        }
    };

    // Clean up heartbeat task
    heartbeat_handle.abort();

    // Close WebSocket
    let _ = ws_sink.close().await;

    result
}

// ─── Utility ─────────────────────────────────────────────────────────────────

/// Split a message into chunks of at most `max_len` characters,
/// trying to break on newline boundaries when possible.
fn split_message(content: &str, max_len: usize) -> Vec<String> {
    if content.len() <= max_len {
        return vec![content.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = content;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        // Try to find a newline to break at, searching backwards from max_len
        let cut_point = if let Some(pos) = remaining[..max_len].rfind('\n') {
            pos + 1 // include the newline in the first chunk
        } else {
            max_len
        };

        chunks.push(remaining[..cut_point].to_string());
        remaining = &remaining[cut_point..];
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_message_short() {
        let result = split_message("hello", 10);
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn test_split_message_exact() {
        let result = split_message("hello", 5);
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn test_split_message_long() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let result = split_message(content, 12);
        assert!(result.len() > 1);
        assert_eq!(result.join(""), content);
    }
}

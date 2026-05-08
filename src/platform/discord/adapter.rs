//! Discord platform adapter for Ruri.
//!
//! This adapter uses the [`serenity`] crate to connect to Discord via the
//! Gateway (WebSocket) and interact with the REST API. It supports:
//! - Guild (server) messages and DM (direct message) messages
//! - Text, markdown-as-embed, and image replies
//! - Pre-response reactions (emoji added while the bot is processing a message)
//! - Optional HTTP proxy for connecting to Discord
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
//!     proxy: ""                     # optional
//!     pre_response_reactions: true   # optional
//!     reaction_emojis: ["👍", "🤔", "⏳"]  # optional
//! ```

use crate::platform::discord::config::DiscordConfig;
use crate::platform::trait_def::{Platform, PlatformEvent};
use crate::platform::types::{
    MessageComponent, MessageType, OutboundContent, OutboundMessage, PlatformMessage,
    PlatformMetadata, PlatformStatus,
};
use async_trait::async_trait;
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

/// Discord platform adapter.
pub struct DiscordAdapter {
    config: DiscordConfig,
    instance_id: String,
    status: PlatformStatus,
    /// Channel to signal the Discord client to shut down.
    shutdown_tx: Option<watch::Sender<bool>>,
    /// The serenity HTTP client for sending messages.
    /// This is cloned from the Client before the gateway task takes ownership,
    /// and is safe to use from other tasks since it's Arc-based.
    http: Arc<Mutex<Option<Arc<serenity::http::Http>>>>,
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
    fn meta(&self) -> PlatformMetadata {
        PlatformMetadata {
            name: "discord".to_string(),
            description: "Discord 机器人适配器 (基于 serenity)".to_string(),
            id: self.instance_id.clone(),
            support_streaming_message: false,
        }
    }

    async fn run(&mut self, event_sender: mpsc::Sender<PlatformEvent>) -> anyhow::Result<()> {
        let config = self.config.clone();
        let instance_id = self.instance_id.clone();

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        self.status = PlatformStatus::Running;

        // Intents: we need message content, guild messages, and DMs
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILDS;

        // Build the serenity client, with optional proxy support
        let mut client = if config.proxy.is_empty() {
            Client::builder(&config.token, intents)
                .event_handler(Handler)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create Discord client: {}", e))?
        } else {
            // Build a custom Http client with proxy, then pass it to the ClientBuilder
            let http = serenity::http::HttpBuilder::new(&config.token)
                .proxy(&config.proxy)
                .ratelimiter_disabled(true) // proxy handles rate limiting
                .build();
            ClientBuilder::new_with_http(http, intents)
                .event_handler(Handler)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create Discord client with proxy: {}", e))?
        };

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
        // serenity's Http is Arc-based internally, so it's safe to share.
        let http = client.http.clone();
        {
            let mut http_guard = self.http.lock().await;
            *http_guard = Some(http);
        }

        // Spawn the gateway connection as a background task.
        // The client takes ownership and runs until shutdown is signaled.
        tokio::spawn(async move {
            // serenity automatically reconnects with exponential backoff,
            // so client.start() will keep running until a fatal error or shutdown.
            // We use a tokio::select to also watch for the shutdown signal.
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
            "Discord adapter started"
        );

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
            .ok_or_else(|| anyhow::anyhow!("Discord HTTP client not initialized"))?
            .clone();
        drop(http_guard);

        let channel_id: ChannelId = message
            .target_id
            .parse::<u64>()
            .map_err(|e| {
                anyhow::anyhow!("Invalid Discord channel ID '{}': {}", message.target_id, e)
            })?
            .into();

        match &message.content {
            OutboundContent::Text { content } => {
                // Discord has a 2000-character limit per message.
                // If the content exceeds this, we split it into multiple messages.
                let chunks = split_message(content, 2000);
                for chunk in chunks {
                    channel_id
                        .say(&http, &chunk)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to send Discord message: {}", e))?;
                }
            }
            OutboundContent::Markdown { title, text } => {
                // Use a Discord embed for markdown content
                let embed = CreateEmbed::new().title(title).description(text);
                let builder = CreateMessage::new().embed(embed);
                channel_id
                    .send_message(&http, builder)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to send Discord embed: {}", e))?;
            }
            OutboundContent::Image { photo_url } => {
                let embed = CreateEmbed::new().image(photo_url);
                let builder = CreateMessage::new().embed(embed);
                channel_id
                    .send_message(&http, builder)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to send Discord image: {}", e))?;
            }
            OutboundContent::File {
                file_name,
                media_id,
                ..
            } => {
                // For file URLs, we send a text message with the link
                // since we can't easily upload files from URLs via serenity
                // without downloading them first.
                let msg_text = format!("📄 **{}**: {}", file_name, media_id);
                channel_id
                    .say(&http, &msg_text)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to send Discord file message: {}", e))?;
            }
        }

        Ok(())
    }

    fn status(&self) -> PlatformStatus {
        self.status
    }

    fn platform_type(&self) -> &str {
        "discord"
    }
}

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

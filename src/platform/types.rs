use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// The type of chat message (group or private/direct).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    GroupMessage,
    FriendMessage,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::GroupMessage => write!(f, "group_message"),
            MessageType::FriendMessage => write!(f, "friend_message"),
        }
    }
}

/// Information about the sender of a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSender {
    /// Platform-specific user ID.
    pub user_id: String,
    /// Display name / nickname.
    #[serde(default)]
    pub nickname: String,
}

/// A single component within a platform message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageComponent {
    /// Plain text.
    Plain { text: String },
    /// An @-mention.
    At { user_id: String },
    /// An image (URL or local path).
    Image { url: String },
    /// A voice / audio message.
    Voice { url: String },
    /// A file attachment.
    File { name: String, url: String },
}

impl MessageComponent {
    /// Extract plain text from this component, if any.
    pub fn as_plain(&self) -> Option<&str> {
        match self {
            MessageComponent::Plain { text } => Some(text),
            _ => None,
        }
    }
}

/// A unified inbound message from any chat platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMessage {
    /// Platform adapter instance ID that received this message.
    #[serde(default)]
    pub platform_id: String,
    /// Unique message ID within the platform.
    pub message_id: String,
    /// Type of message (group / friend).
    pub message_type: MessageType,
    /// The concatenated plain-text content of the message.
    pub message_str: String,
    /// Individual message components (text, images, at, etc.).
    pub components: Vec<MessageComponent>,
    /// Information about the sender.
    pub sender: MessageSender,
    /// The bot's own ID on this platform.
    pub self_id: String,
    /// Group / conversation ID (empty for private messages).
    #[serde(default)]
    pub group_id: String,
    /// Session ID — either the group_id (for group messages) or the sender's user_id (for private).
    pub session_id: String,
    /// Unix timestamp (seconds) of when the message was created.
    pub timestamp: u64,
    /// Raw JSON payload from the platform (useful for platform-specific fields).
    #[serde(default)]
    pub raw: Option<serde_json::Value>,
}

impl PlatformMessage {
    /// Get the epoch timestamp. Falls back to current time if `timestamp` is 0.
    pub fn timestamp_or_now(&self) -> u64 {
        if self.timestamp > 0 {
            return self.timestamp;
        }
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// A message to be sent to a platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    /// Target type.
    pub target_type: MessageType,
    /// Target ID (group conversation ID or staff/user ID).
    pub target_id: String,
    /// The content to send.
    pub content: OutboundContent,
}

/// Content of an outbound message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboundContent {
    /// Plain text message.
    Text { content: String },
    /// Markdown message.
    Markdown { title: String, text: String },
    /// Image message (URL).
    Image { photo_url: String },
    /// File message.
    File {
        media_id: String,
        file_name: String,
        file_type: String,
    },
}

/// Metadata about a registered platform adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMetadata {
    /// Unique platform adapter name (e.g. "dingtalk").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Instance ID from config.
    pub id: String,
    /// Whether the adapter supports streaming (progressive) messages.
    pub support_streaming_message: bool,
}

/// Running status of a platform adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformStatus {
    Pending,
    Running,
    Error,
    Stopped,
}

impl std::fmt::Display for PlatformStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformStatus::Pending => write!(f, "pending"),
            PlatformStatus::Running => write!(f, "running"),
            PlatformStatus::Error => write!(f, "error"),
            PlatformStatus::Stopped => write!(f, "stopped"),
        }
    }
}

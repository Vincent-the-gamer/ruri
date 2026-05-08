use crate::platform::types::{
    MessageType, OutboundContent, OutboundMessage, PlatformMessage, PlatformMetadata,
    PlatformStatus,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// A chat platform adapter that can receive and send messages.
///
/// Each configuration file maps to one `Platform` instance. The adapter:
/// - Connects to the chat platform (via WebSocket, HTTP callback, etc.)
/// - Converts platform-specific messages into [`PlatformMessage`]
/// - Sends them through the `event_sender` channel
/// - Receives outbound messages and delivers them to the platform
#[async_trait]
pub trait Platform: Send + Sync {
    /// Return metadata about this platform adapter.
    fn meta(&self) -> PlatformMetadata;

    /// Start the platform adapter (connect, listen, etc.).
    ///
    /// The adapter should:
    /// 1. Connect to the chat platform
    /// 2. For each inbound message, convert it to [`PlatformMessage`] and
    ///    send it through `event_sender`
    /// 3. Run until [`terminate()`] is called or an unrecoverable error occurs
    async fn run(&mut self, event_sender: mpsc::Sender<PlatformEvent>) -> anyhow::Result<()>;

    /// Gracefully stop the platform adapter.
    async fn terminate(&mut self) -> anyhow::Result<()>;

    /// Send a message to the platform.
    async fn send_message(&self, message: OutboundMessage) -> anyhow::Result<()>;

    /// Send a text reply to a session (convenience method).
    async fn send_text(
        &self,
        target_type: MessageType,
        target_id: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        self.send_message(OutboundMessage {
            target_type,
            target_id: target_id.to_string(),
            content: OutboundContent::Text {
                content: text.to_string(),
            },
        })
        .await
    }

    /// Get the current status of this adapter.
    fn status(&self) -> PlatformStatus;

    /// Return the platform type name (e.g. "dingtalk").
    fn platform_type(&self) -> &str;
}

/// An event emitted by a platform adapter.
#[derive(Debug, Clone)]
pub enum PlatformEvent {
    /// A new message was received from the platform.
    Message(PlatformMessage),

    /// The platform adapter status changed.
    StatusChanged {
        platform_id: String,
        status: PlatformStatus,
    },

    /// The platform adapter encountered an error.
    Error {
        platform_id: String,
        message: String,
    },
}

//! ACP protocol-compliant file system tools for interacting with IDE file system.
//!
//! These tools use the ACP protocol to request file operations from the client
//! (e.g., Zed editor), enabling proper permission handling and IDE integration.
//!
//! NOTE: These tools are prepared for future ACP protocol integration.

use std::collections::HashMap;
use std::sync::Arc;

use agent_client_protocol::schema::{ReadTextFileResponse, WriteTextFileResponse};
use tokio::sync::{RwLock, oneshot};
use tracing::{debug, warn};

/// Manages pending ACP requests and their responses.
#[derive(Clone)]
pub struct RequestManager {
    /// Maps request IDs to channels for sending responses.
    pending_reads: Arc<RwLock<HashMap<String, oneshot::Sender<Result<String, String>>>>>,
    pending_writes: Arc<RwLock<HashMap<String, oneshot::Sender<Result<(), String>>>>>,
}

impl RequestManager {
    pub fn new() -> Self {
        Self {
            pending_reads: Arc::new(RwLock::new(HashMap::new())),
            pending_writes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a pending read request and return a channel to wait for the response.
    pub async fn register_read_request(
        &self,
        request_id: String,
    ) -> oneshot::Receiver<Result<String, String>> {
        let (sender, receiver) = oneshot::channel();
        self.pending_reads.write().await.insert(request_id, sender);
        receiver
    }

    /// Register a pending write request and return a channel to wait for the response.
    pub async fn register_write_request(
        &self,
        request_id: String,
    ) -> oneshot::Receiver<Result<(), String>> {
        let (sender, receiver) = oneshot::channel();
        self.pending_writes.write().await.insert(request_id, sender);
        receiver
    }

    /// Handle a read file response.
    pub async fn handle_read_response(&self, request_id: String, response: ReadTextFileResponse) {
        let mut pending = self.pending_reads.write().await;
        if let Some(sender) = pending.remove(&request_id) {
            let result = Ok(response.content);
            if sender.send(result).is_err() {
                debug!(request_id, "Receiver dropped before response was sent");
            }
        } else {
            warn!(request_id, "No pending read request found for response");
        }
    }

    /// Handle a write file response.
    pub async fn handle_write_response(
        &self,
        request_id: String,
        _response: WriteTextFileResponse,
    ) {
        let mut pending = self.pending_writes.write().await;
        if let Some(sender) = pending.remove(&request_id) {
            let result = Ok(());
            if sender.send(result).is_err() {
                debug!(request_id, "Receiver dropped before response was sent");
            }
        } else {
            warn!(request_id, "No pending write request found for response");
        }
    }

    /// Clean up old pending requests (should be called periodically).
    pub async fn cleanup(&self) {
        // Note: We can't easily clean up one-shot senders without tracking their creation time
        // For now, we'll just log warnings about old requests during response handling
        debug!(
            "RequestManager cleanup: {} pending reads, {} pending writes",
            self.pending_reads.read().await.len(),
            self.pending_writes.read().await.len()
        );
    }
}

impl Default for RequestManager {
    fn default() -> Self {
        Self::new()
    }
}

mod read_file;
mod write_file;

// Re-export for potential use in the future (ACP tools)

// TODO: Add more ACP tools as needed:
// - AcpListDirectoryTool
// - AcpSearchFilesTool
// - AcpCreateFileTool
// - AcpEditFileTool

/// Helper function to parse JSON arguments for ACP tools.
pub(super) fn parse_args(args: &str) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(args)
}

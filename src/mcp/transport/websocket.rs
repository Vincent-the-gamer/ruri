//! WebSocket transport for MCP
//!
//! This module provides WebSocket based transport for communicating with remote MCP servers.

use super::McpTransport;
use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse, TransportConfig};
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};
use tracing::debug;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSplitWrite = futures_util::stream::SplitSink<WsStream, Message>;
type WsSplitRead = futures_util::stream::SplitStream<WsStream>;

/// WebSocket transport for MCP
///
/// Supports lazy connection - the WebSocket connection is established
/// on the first request if not already connected.
pub struct WebSocketMcpTransport {
    url: String,
    headers: Vec<(String, String)>,
    write: Mutex<Option<WsSplitWrite>>,
    read: Mutex<Option<WsSplitRead>>,
    connected: Mutex<bool>,
}

impl WebSocketMcpTransport {
    /// Create a new WebSocket MCP transport without connecting yet.
    ///
    /// The actual connection will be established lazily on the first request.
    /// This is useful because `McpClient::new()` is not async.
    pub fn new_lazy(config: &TransportConfig) -> Result<Self> {
        match config {
            TransportConfig::WebSocket { url, headers } => {
                let mut http_headers = Vec::new();
                if let Some(headers_map) = headers {
                    for (key, value) in headers_map {
                        http_headers.push((key.clone(), value.clone()));
                    }
                }

                debug!(
                    "Created lazy WebSocket transport (will connect on first request): {}",
                    url
                );

                Ok(Self {
                    url: url.clone(),
                    headers: http_headers,
                    write: Mutex::new(None),
                    read: Mutex::new(None),
                    connected: Mutex::new(false),
                })
            }
            _ => Err(anyhow::anyhow!(
                "Invalid transport configuration for WebSocket transport"
            )),
        }
    }

    /// Ensure the WebSocket connection is established.
    ///
    /// If already connected, this is a no-op.
    /// If not connected, this will establish the connection.
    async fn ensure_connected(&self) -> Result<()> {
        let mut connected = self.connected.lock().await;
        if *connected {
            return Ok(());
        }

        debug!("Establishing WebSocket connection to: {}", self.url);

        // Build request with optional headers
        let mut request = self.url.as_str().into_client_request()?;

        if !self.headers.is_empty() {
            let headers = request.headers_mut();
            for (key, value) in &self.headers {
                let header_name =
                    tokio_tungstenite::tungstenite::http::HeaderName::from_bytes(key.as_bytes())
                        .context(format!("Invalid header name: {}", key))?;
                let header_value =
                    tokio_tungstenite::tungstenite::http::HeaderValue::from_str(value)
                        .context(format!("Invalid header value for {}: {}", key, value))?;
                headers.insert(header_name, header_value);
            }
        }

        let (ws_stream, _) = connect_async(request)
            .await
            .context("Failed to connect to WebSocket server")?;

        debug!("WebSocket connection established");

        let (write, read) = ws_stream.split();

        *self.write.lock().await = Some(write);
        *self.read.lock().await = Some(read);
        *connected = true;

        Ok(())
    }

    /// Mark connection as disconnected
    async fn mark_disconnected(&self) {
        *self.connected.lock().await = false;
    }
}

#[async_trait::async_trait]
impl McpTransport for WebSocketMcpTransport {
    async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Ensure we are connected
        self.ensure_connected().await?;

        let request_json =
            serde_json::to_string(&request).context("Failed to serialize JSON-RPC request")?;

        debug!("Sending WebSocket request: {}", request_json);

        // Send request
        {
            let mut write_guard = self.write.lock().await;
            if write_guard.is_none() {
                return Err(anyhow::anyhow!("WebSocket connection is closed"));
            }

            let write_stream = write_guard.as_mut().unwrap();
            write_stream
                .send(Message::Text(request_json))
                .await
                .context("Failed to send WebSocket message")?;
        }

        // Receive response
        let mut read_guard = self.read.lock().await;
        if read_guard.is_none() {
            return Err(anyhow::anyhow!("WebSocket connection is closed"));
        }

        let read_stream = read_guard.as_mut().unwrap();

        let message = read_stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("No response from WebSocket server"))?
            .context("Failed to receive WebSocket message")?;

        let response_text = match message {
            Message::Text(text) => text,
            Message::Close(_) => {
                self.mark_disconnected().await;
                return Err(anyhow::anyhow!("WebSocket connection closed by server"));
            }
            Message::Ping(data) => {
                // Respond to ping with pong
                let mut write_guard = self.write.lock().await;
                if let Some(write_stream) = write_guard.as_mut() {
                    let _ = write_stream.send(Message::Pong(data)).await;
                }
                drop(write_guard);
                drop(read_guard);
                // Try to read the next message by recursively calling
                // Note: we need to rebuild the request since we consumed it
                return Err(anyhow::anyhow!(
                    "Received ping during request, please retry"
                ));
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unexpected WebSocket message type: {:?}",
                    message
                ));
            }
        };

        debug!("Received WebSocket response: {}", response_text);

        let json_response: JsonRpcResponse =
            serde_json::from_str(&response_text).context("Failed to parse JSON-RPC response")?;

        Ok(json_response)
    }

    async fn send_notification(&mut self, request: JsonRpcRequest) -> Result<()> {
        // Ensure we are connected
        self.ensure_connected().await?;

        let request_json =
            serde_json::to_string(&request).context("Failed to serialize JSON-RPC notification")?;

        debug!("Sending WebSocket notification: {}", request_json);

        let mut write_guard = self.write.lock().await;
        if write_guard.is_none() {
            return Err(anyhow::anyhow!("WebSocket connection is closed"));
        }

        let write_stream = write_guard.as_mut().unwrap();

        write_stream
            .send(Message::Text(request_json))
            .await
            .context("Failed to send WebSocket notification")?;

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        debug!("Shutting down WebSocket transport");

        // Close write stream
        let mut write_guard = self.write.lock().await;
        if let Some(mut write_stream) = write_guard.take() {
            let _ = write_stream.close().await;
        }
        drop(write_guard);

        // Clear read stream
        let mut read_guard = self.read.lock().await;
        *read_guard = None;
        drop(read_guard);

        *self.connected.lock().await = false;

        debug!("WebSocket transport shut down");

        Ok(())
    }
}

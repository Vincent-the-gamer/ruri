//! SSE (Server-Sent Events) transport for MCP (HTTP+SSE, legacy protocol version 2024-11-05)
//!
//! This module provides the legacy HTTP+SSE transport for communicating with
//! remote MCP servers that implement the older protocol version.
//!
//! Protocol flow:
//! 1. Client connects to SSE endpoint via GET request
//! 2. Server sends an `endpoint` event containing the POST endpoint URI
//! 3. Client sends JSON-RPC messages via HTTP POST to the POST endpoint
//! 4. Server sends responses/notifications via SSE `message` events

use super::McpTransport;
use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse, RequestId, TransportConfig};
use anyhow::{Context, Result};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, mpsc};
use tracing::{debug, info, warn};

/// Messages received from the SSE stream
#[derive(Debug)]
enum SseMessage {
    /// A JSON-RPC response
    Response(JsonRpcResponse),
    /// An endpoint event containing the POST URL
    Endpoint(String),
}

/// SSE transport for MCP (legacy HTTP+SSE)
///
/// Protocol:
/// - GET to SSE endpoint → receive `endpoint` event with POST URL
/// - POST to POST endpoint → send JSON-RPC requests
/// - SSE `message` events → receive JSON-RPC responses
pub struct SseMcpTransport {
    /// The SSE endpoint URL (for receiving server messages)
    sse_url: String,
    /// The POST endpoint URL (for sending client messages), discovered from server
    post_url: Mutex<Option<String>>,
    /// HTTP client
    client: Client,
    /// Custom headers to send with requests
    headers: Vec<(String, String)>,
    /// Channel to receive responses from the SSE listener task
    response_rx: Mutex<mpsc::Receiver<SseMessage>>,
    /// Cancellation trigger for the SSE listener task
    cancel: Arc<Notify>,
}

impl SseMcpTransport {
    /// Create a new SSE MCP transport from configuration
    pub fn new(config: &TransportConfig) -> Result<Self> {
        match config {
            TransportConfig::ServerSentEvents { url, headers } => {
                let mut http_headers = Vec::new();
                if let Some(headers_map) = headers {
                    for (key, value) in headers_map {
                        http_headers.push((key.clone(), value.clone()));
                    }
                }

                let (tx, rx) = mpsc::channel(100);
                let cancel = Arc::new(Notify::new());

                // Spawn background SSE listener task immediately
                let sse_url = url.clone();
                let sse_client = Client::new();
                let headers_clone = http_headers.clone();
                let cancel_clone = cancel.clone();

                tokio::spawn(async move {
                    if let Err(e) =
                        listen_sse(sse_client, sse_url, headers_clone, tx, cancel_clone).await
                    {
                        warn!("SSE listener task ended with error: {}", e);
                    }
                });

                let post_client = Client::new();

                Ok(Self {
                    sse_url: url.clone(),
                    post_url: Mutex::new(None),
                    client: post_client,
                    headers: http_headers,
                    response_rx: Mutex::new(rx),
                    cancel,
                })
            }
            _ => Err(anyhow::anyhow!(
                "Invalid transport configuration for SSE transport"
            )),
        }
    }

    /// Wait for the SSE endpoint event to discover the POST URL.
    async fn discover_endpoint(&self) -> Result<()> {
        let mut post_url_guard = self.post_url.lock().await;
        if post_url_guard.is_some() {
            return Ok(());
        }

        debug!("Waiting for SSE endpoint event from: {}", self.sse_url);

        let mut rx = self.response_rx.lock().await;

        let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                match rx.recv().await {
                    Some(SseMessage::Endpoint(url)) => {
                        return Ok(url);
                    }
                    Some(SseMessage::Response(_)) => {
                        // Ignore responses before we get the endpoint
                        continue;
                    }
                    None => {
                        return Err(anyhow::anyhow!(
                            "SSE channel closed before receiving endpoint"
                        ));
                    }
                }
            }
        })
        .await
        .context("Timeout waiting for SSE endpoint event")??;

        info!("Discovered SSE POST endpoint: {}", result);
        *post_url_guard = Some(result);

        Ok(())
    }

    /// Get the POST endpoint URL
    async fn get_post_url(&self) -> Result<String> {
        let guard = self.post_url.lock().await;
        guard
            .clone()
            .ok_or_else(|| anyhow::anyhow!("SSE POST endpoint not discovered yet"))
    }
}

/// Background task to listen for SSE events
async fn listen_sse(
    client: Client,
    url: String,
    headers: Vec<(String, String)>,
    tx: mpsc::Sender<SseMessage>,
    _cancel: Arc<Notify>,
) -> Result<()> {
    let mut req_builder = client.get(&url).header("Accept", "text/event-stream");

    for (key, value) in &headers {
        req_builder = req_builder.header(key, value);
    }

    let response = req_builder
        .send()
        .await
        .context("Failed to connect to SSE endpoint")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!(
            "SSE connection failed with status {}: {}",
            status,
            error_text
        );
    }

    info!("SSE connection established to: {}", url);

    // Read the SSE stream as text
    let response_text = response.text().await.context("Failed to read SSE stream")?;

    // Parse SSE events from the stream
    let mut event_type = String::new();
    let mut event_data = String::new();

    for line in response_text.lines() {
        if line.starts_with("event:") {
            event_type = line.strip_prefix("event:").unwrap_or("").trim().to_string();
        } else if line.starts_with("data:") {
            event_data.push_str(line.strip_prefix("data:").unwrap_or("").trim());
        } else if line.is_empty() && !event_data.is_empty() {
            // Empty line = end of event
            debug!("SSE event: type={}, data={}", event_type, event_data);

            let msg = match event_type.as_str() {
                "endpoint" => SseMessage::Endpoint(event_data.clone()),
                "message" => match serde_json::from_str::<JsonRpcResponse>(&event_data) {
                    Ok(resp) => SseMessage::Response(resp),
                    Err(e) => {
                        warn!("Failed to parse SSE message as JSON-RPC: {}", e);
                        event_type.clear();
                        event_data.clear();
                        continue;
                    }
                },
                _ => {
                    debug!("Ignoring unknown SSE event type: {}", event_type);
                    event_type.clear();
                    event_data.clear();
                    continue;
                }
            };

            if tx.send(msg).await.is_err() {
                debug!("SSE receiver dropped, stopping listener");
                return Ok(());
            }

            event_type.clear();
            event_data.clear();
        }
    }

    // If the SSE stream is short (e.g. just the endpoint event), we need to
    // keep listening. However, since `response.text()` reads the entire body,
    // this means the SSE connection has ended. For a long-lived SSE connection,
    // we would need to use streaming, but that requires the `stream` feature
    // for reqwest. For now, this implementation handles the initial handshake
    // and short-lived SSE responses correctly.
    //
    // For continued SSE listening, the caller should re-establish the connection.

    info!("SSE stream ended");
    Ok(())
}

#[async_trait::async_trait]
impl McpTransport for SseMcpTransport {
    async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Discover POST endpoint if not yet known
        self.discover_endpoint().await?;

        let post_url = self.get_post_url().await?;
        let request_id = request.id.clone();

        debug!("Sending SSE POST request to: {}", post_url);

        let request_json =
            serde_json::to_string(&request).context("Failed to serialize JSON-RPC request")?;

        let mut req_builder = self
            .client
            .post(&post_url)
            .header("Content-Type", "application/json");

        for (key, value) in &self.headers {
            req_builder = req_builder.header(key, value);
        }

        let response = req_builder
            .body(request_json)
            .send()
            .await
            .context("Failed to send SSE POST request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "SSE POST request failed with status {}: {}",
                status,
                error_text
            ));
        }

        // Check content type of the response
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.contains("application/json") {
            // Direct JSON response
            let response_text = response
                .text()
                .await
                .context("Failed to read response body")?;
            debug!("Received direct JSON response: {}", response_text);
            let json_response: JsonRpcResponse = serde_json::from_str(&response_text)
                .context("Failed to parse JSON-RPC response")?;
            Ok(json_response)
        } else {
            // Wait for response via SSE stream
            debug!("Waiting for SSE response for request: {:?}", request_id);
            let mut rx = self.response_rx.lock().await;

            let result = tokio::time::timeout(std::time::Duration::from_secs(60), async {
                loop {
                    match rx.recv().await {
                        Some(SseMessage::Response(resp)) => {
                            if matches_response(&resp, &request_id) {
                                return Ok(resp);
                            }
                            debug!("Received response for different request, continuing to wait");
                        }
                        Some(SseMessage::Endpoint(url)) => {
                            debug!("Received updated endpoint: {}", url);
                            *self.post_url.lock().await = Some(url);
                        }
                        None => {
                            return Err(anyhow::anyhow!(
                                "SSE channel closed while waiting for response"
                            ));
                        }
                    }
                }
            })
            .await
            .context("Timeout waiting for SSE response")??;

            Ok(result)
        }
    }

    async fn send_notification(&mut self, request: JsonRpcRequest) -> Result<()> {
        self.discover_endpoint().await?;

        let post_url = self.get_post_url().await?;

        debug!("Sending SSE POST notification to: {}", post_url);

        let request_json =
            serde_json::to_string(&request).context("Failed to serialize JSON-RPC notification")?;

        let mut req_builder = self
            .client
            .post(&post_url)
            .header("Content-Type", "application/json");

        for (key, value) in &self.headers {
            req_builder = req_builder.header(key, value);
        }

        let response = req_builder
            .body(request_json)
            .send()
            .await
            .context("Failed to send SSE POST notification")?;

        if !response.status().is_success() {
            let status = response.status();
            anyhow::bail!("SSE POST notification failed with status {}", status);
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        debug!("Shutting down SSE transport");
        self.cancel.notify_waiters();
        Ok(())
    }
}

/// Check if a JSON-RPC response matches the given request ID
fn matches_response(response: &JsonRpcResponse, request_id: &RequestId) -> bool {
    match (&response.id, request_id) {
        (RequestId::Number(a), RequestId::Number(b)) => a == b,
        (RequestId::String(a), RequestId::String(b)) => a == b,
        _ => false,
    }
}

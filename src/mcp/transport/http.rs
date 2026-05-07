//! Streamable HTTP transport for MCP (protocol version 2025-03-26)
//!
//! This module provides the Streamable HTTP transport for communicating with
//! remote MCP servers that implement the newer protocol version.
//!
//! Protocol flow:
//! 1. Client sends JSON-RPC requests via HTTP POST to the MCP endpoint
//! 2. Server responds with either:
//!    - `application/json` - a direct JSON-RPC response
//!    - `text/event-stream` - an SSE stream containing responses
//! 3. Server may include `Mcp-Session-Id` header for session management
//! 4. Client can optionally open a GET SSE stream for server-initiated messages
//!
//! For notifications (no response expected), server returns 202 Accepted.

use super::McpTransport;
use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse, RequestId, TransportConfig};
use anyhow::{Context, Result};
use reqwest::Client;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Streamable HTTP transport for MCP
pub struct HttpMcpTransport {
    /// The MCP endpoint URL
    url: String,
    /// HTTP client
    client: Client,
    /// Custom headers to send with requests
    headers: Vec<(String, String)>,
    /// Session ID for the MCP session (received from server during initialization)
    session_id: Mutex<Option<String>>,
}

impl HttpMcpTransport {
    /// Create a new Streamable HTTP MCP transport from configuration
    pub fn new(config: &TransportConfig) -> Result<Self> {
        match config {
            TransportConfig::Http { url, headers } => {
                let mut http_headers = Vec::new();
                if let Some(headers_map) = headers {
                    for (key, value) in headers_map {
                        http_headers.push((key.clone(), value.clone()));
                    }
                }

                Ok(Self {
                    url: url.clone(),
                    client: Client::new(),
                    headers: http_headers,
                    session_id: Mutex::new(None),
                })
            }
            _ => Err(anyhow::anyhow!(
                "Invalid transport configuration for HTTP transport"
            )),
        }
    }
}

#[async_trait::async_trait]
impl McpTransport for HttpMcpTransport {
    async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let request_id = request.id.clone();

        debug!("Sending Streamable HTTP POST request to: {}", self.url);

        let request_json =
            serde_json::to_string(&request).context("Failed to serialize JSON-RPC request")?;

        debug!("Request payload: {}", request_json);

        let mut req_builder = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            // Streamable HTTP requires Accept header listing both types
            .header("Accept", "application/json, text/event-stream");

        // Add custom headers
        for (key, value) in &self.headers {
            req_builder = req_builder.header(key, value);
        }

        // Add session ID if available
        let session_id = self.session_id.lock().await;
        if let Some(sid) = session_id.as_ref() {
            req_builder = req_builder.header("Mcp-Session-Id", sid.as_str());
            debug!("Including session ID: {}", sid);
        }
        drop(session_id);

        let response = req_builder
            .body(request_json)
            .send()
            .await
            .context("Failed to send HTTP request")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "HTTP request failed with status {}: {}",
                status,
                error_text
            ));
        }

        // Store session ID if present
        if let Some(sid) = response.headers().get("Mcp-Session-Id") {
            if let Ok(sid_str) = sid.to_str() {
                debug!("Received session ID: {}", sid_str);
                *self.session_id.lock().await = Some(sid_str.to_string());
            }
        }

        // Determine response type
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.contains("text/event-stream") {
            // SSE stream response - parse events to find our response
            debug!("Received SSE stream response");
            self.parse_sse_response(response, request_id).await
        } else {
            // Direct JSON response
            let response_text = response
                .text()
                .await
                .context("Failed to read response body")?;

            debug!("Received JSON response: {}", response_text);

            let json_response: JsonRpcResponse = serde_json::from_str(&response_text)
                .context("Failed to parse JSON-RPC response")?;

            Ok(json_response)
        }
    }

    async fn send_notification(&mut self, request: JsonRpcRequest) -> Result<()> {
        debug!("Sending Streamable HTTP POST notification to: {}", self.url);

        let request_json =
            serde_json::to_string(&request).context("Failed to serialize JSON-RPC notification")?;

        let mut req_builder = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json");

        // Add custom headers
        for (key, value) in &self.headers {
            req_builder = req_builder.header(key, value);
        }

        // Add session ID if available
        let session_id = self.session_id.lock().await;
        if let Some(sid) = session_id.as_ref() {
            req_builder = req_builder.header("Mcp-Session-Id", sid.as_str());
        }
        drop(session_id);

        let response = req_builder
            .body(request_json)
            .send()
            .await
            .context("Failed to send HTTP notification")?;

        let status = response.status();
        // For notifications, server typically returns 202 Accepted
        if !status.is_success() && status.as_u16() != 202 {
            anyhow::bail!("HTTP notification failed with status {}", status);
        }

        // Store session ID if present
        if let Some(sid) = response.headers().get("Mcp-Session-Id") {
            if let Ok(sid_str) = sid.to_str() {
                *self.session_id.lock().await = Some(sid_str.to_string());
            }
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        debug!("Shutting down Streamable HTTP transport");

        // Optionally send DELETE to terminate the session
        let session_id = self.session_id.lock().await;
        if let Some(sid) = session_id.as_ref() {
            debug!("Terminating MCP session: {}", sid);

            let mut req_builder = self.client.delete(&self.url);
            req_builder = req_builder.header("Mcp-Session-Id", sid.as_str());

            for (key, value) in &self.headers {
                req_builder = req_builder.header(key, value);
            }

            // Best-effort DELETE, ignore errors
            match req_builder.send().await {
                Ok(resp) => {
                    debug!("Session termination response: {}", resp.status());
                }
                Err(e) => {
                    debug!("Failed to send session termination: {}", e);
                }
            }
        }

        info!("Streamable HTTP transport shut down");
        Ok(())
    }
}

impl HttpMcpTransport {
    /// Parse an SSE stream response to find the JSON-RPC response matching the request ID.
    async fn parse_sse_response(
        &self,
        response: reqwest::Response,
        request_id: RequestId,
    ) -> Result<JsonRpcResponse> {
        let response_text = response
            .text()
            .await
            .context("Failed to read SSE stream response")?;

        let mut event_type = String::new();
        let mut event_data = String::new();

        for line in response_text.lines() {
            if line.starts_with("event:") {
                event_type = line.strip_prefix("event:").unwrap_or("").trim().to_string();
            } else if line.starts_with("data:") {
                event_data.push_str(line.strip_prefix("data:").unwrap_or("").trim());
            } else if line.is_empty() && !event_data.is_empty() {
                // End of event
                debug!(
                    "SSE event in HTTP response: type={}, data={}",
                    event_type, event_data
                );

                // Try to parse as JSON-RPC response
                if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(&event_data) {
                    if matches_response(&resp, &request_id) {
                        return Ok(resp);
                    }
                }

                event_type.clear();
                event_data.clear();
            }
        }

        Err(anyhow::anyhow!(
            "No matching JSON-RPC response found in SSE stream for request {:?}",
            request_id
        ))
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

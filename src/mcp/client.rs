//! MCP Client implementation
//!
//! This module provides a client for connecting to and communicating with MCP servers.

use super::transport::McpTransport;
use super::transport::http::HttpMcpTransport;
use super::transport::sse::SseMcpTransport;
use super::transport::stdio::StdioMcpTransport;
use super::transport::websocket::WebSocketMcpTransport;
use super::types::*;
use anyhow::{Context, Result};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// MCP Client for communicating with an MCP server
pub struct McpClient {
    server_id: String,
    config: McpServerConfig,
    transport: Arc<Mutex<dyn McpTransport>>,
    request_count: RequestCounter,
}

/// Simple request counter for generating unique request IDs
struct RequestCounter {
    counter: Arc<std::sync::Mutex<i64>>,
}

impl RequestCounter {
    fn new() -> Self {
        Self {
            counter: Arc::new(std::sync::Mutex::new(0)),
        }
    }

    fn next(&self) -> i64 {
        let mut counter = self.counter.lock().unwrap();
        *counter += 1;
        *counter
    }
}

impl McpClient {
    /// Create a new MCP client from configuration
    pub fn new(config: McpServerConfig) -> Result<Self> {
        let server_id = config.id.clone();

        // Create appropriate transport based on config
        let transport: Arc<Mutex<dyn McpTransport>> = match &config.transport_config {
            TransportConfig::Stdio { .. } => {
                let trans = StdioMcpTransport::new(&config.transport_config)?;
                Arc::new(Mutex::new(trans))
            }
            TransportConfig::Http { .. } => {
                let trans = HttpMcpTransport::new(&config.transport_config)?;
                Arc::new(Mutex::new(trans))
            }
            TransportConfig::ServerSentEvents { .. } => {
                let trans = SseMcpTransport::new(&config.transport_config)?;
                Arc::new(Mutex::new(trans))
            }
            TransportConfig::WebSocket { .. } => {
                // WebSocket transport needs async connect, so we use a lazy approach
                // The actual connection will be established on first request
                let trans = WebSocketMcpTransport::new_lazy(&config.transport_config)?;
                Arc::new(Mutex::new(trans))
            }
        };

        info!(
            "Created MCP client for server: {} (transport: {:?})",
            config.name, config.transport_type
        );

        Ok(Self {
            server_id,
            config,
            transport,
            request_count: RequestCounter::new(),
        })
    }

    /// Initialize the connection with the MCP server
    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        debug!("Initializing MCP connection for server: {}", self.server_id);

        let request_id = self.request_count.next();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(request_id),
            method: "initialize".to_string(),
            params: Some(json!(InitializeParams {
                protocol_version: "2024-11-05".to_string(),
                capabilities: ClientCapabilities {},
                client_info: Some(Implementation {
                    name: "Ruri".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                }),
            })),
        };

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("MCP initialize error: {}", error.message));
        }

        let result = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in initialize response"))?;
        let init_result: InitializeResult =
            serde_json::from_value(result).context("Failed to parse initialize result")?;

        // Send initialized notification
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Null,
            method: "notifications/initialized".to_string(),
            params: None,
        };
        self.send_notification(request).await?;

        debug!(
            "MCP connection initialized successfully for server: {}",
            self.server_id
        );

        Ok(init_result)
    }

    /// List available tools from the MCP server
    pub async fn list_tools(&mut self) -> Result<ListToolsResult> {
        debug!("Listing tools from MCP server: {}", self.server_id);

        let request_id = self.request_count.next();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(request_id),
            method: "tools/list".to_string(),
            params: Some(json!(ListToolsParams { cursor: None })),
        };

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("MCP list_tools error: {}", error.message));
        }

        let result = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in list_tools response"))?;
        let tools_result: ListToolsResult =
            serde_json::from_value(result).context("Failed to parse list_tools result")?;

        debug!(
            "Found {} tools from server: {}",
            tools_result.tools.len(),
            self.server_id
        );

        Ok(tools_result)
    }

    /// Call a tool on the MCP server
    pub async fn call_tool(
        &mut self,
        name: String,
        arguments: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<CallToolResult> {
        debug!("Calling tool '{}' on server: {}", name, self.server_id);

        let request_id = self.request_count.next();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(request_id),
            method: "tools/call".to_string(),
            params: Some(json!(CallToolParams { name, arguments })),
        };

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("MCP call_tool error: {}", error.message));
        }

        let result = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in call_tool response"))?;
        let tool_result: CallToolResult =
            serde_json::from_value(result).context("Failed to parse call_tool result")?;

        Ok(tool_result)
    }

    /// Get the server configuration
    pub fn config(&self) -> &McpServerConfig {
        &self.config
    }

    /// Get the server ID
    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    /// Send a JSON-RPC request and wait for response
    async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let mut transport = self.transport.lock().await;
        transport.send_request(request).await
    }

    /// Send a JSON-RPC notification (no response expected)
    async fn send_notification(&mut self, request: JsonRpcRequest) -> Result<()> {
        let mut transport = self.transport.lock().await;
        transport.send_notification(request).await
    }

    /// Shutdown the connection
    pub async fn shutdown(&mut self) -> Result<()> {
        debug!("Shutting down MCP client for server: {}", self.server_id);

        // Send shutdown request
        let request_id = self.request_count.next();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(request_id),
            method: "shutdown".to_string(),
            params: None,
        };

        let _ = self.send_notification(request).await;

        // Shutdown transport
        let mut transport = self.transport.lock().await;
        transport.shutdown().await?;

        info!("MCP client shut down for server: {}", self.server_id);

        Ok(())
    }
}

/// Create a new MCP client and initialize it
pub async fn create_and_connect(config: McpServerConfig) -> Result<McpClient> {
    let mut client = McpClient::new(config)?;
    client.initialize().await?;
    Ok(client)
}

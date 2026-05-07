//! MCP transport layer
//!
//! This module provides different transport implementations for MCP client communication.

pub mod http;
pub mod sse;
pub mod stdio;
pub mod websocket;

use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse};
use async_trait::async_trait;

/// MCP transport trait
///
/// All transport implementations must implement this trait to provide
/// a unified interface for MCP communication.
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Send a JSON-RPC request and receive a response
    async fn send_request(&mut self, request: JsonRpcRequest) -> anyhow::Result<JsonRpcResponse>;

    /// Send a JSON-RPC notification (no response expected)
    async fn send_notification(&mut self, request: JsonRpcRequest) -> anyhow::Result<()>;

    /// Shutdown the transport connection
    async fn shutdown(&mut self) -> anyhow::Result<()>;
}

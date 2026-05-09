//! MCP (Model Context Protocol) type definitions
//!
//! This module defines the core data structures for MCP server configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP Server configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub transport_type: TransportType,
    pub transport_config: TransportConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Transport type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    Stdio,
    #[serde(rename = "sse")]
    ServerSentEvents,
    #[serde(rename = "websocket")]
    WebSocket,
    #[serde(rename = "http")]
    Http,
}

/// Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransportConfig {
    #[serde(rename = "stdio")]
    Stdio {
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        env: Option<HashMap<String, String>>,
    },
    #[serde(rename = "sse")]
    ServerSentEvents {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
    #[serde(rename = "websocket")]
    WebSocket {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
    #[serde(rename = "http")]
    Http {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
}

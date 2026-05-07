//! MCP Server manager
//!
//! This module manages connections to MCP servers and their tools.

use super::client::create_and_connect;
use super::config::McpConfigManager;
use super::tool_adapter::McpToolManager;
use super::types::{McpServerConfig, McpServerStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

/// MCP Server manager
pub struct McpManager {
    config_manager: McpConfigManager,
    clients: HashMap<String, Arc<Mutex<super::client::McpClient>>>,
    tool_manager: McpToolManager,
}

impl McpManager {
    pub fn new(config_manager: McpConfigManager) -> Self {
        Self {
            config_manager,
            clients: HashMap::new(),
            tool_manager: McpToolManager::new(),
        }
    }

    /// Initialize the MCP manager and connect to enabled servers
    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        info!("Initializing MCP manager");

        // Initialize database schema
        self.config_manager.init().await?;

        // Get all enabled servers
        let servers = self.config_manager.list_servers().await?;
        let enabled_servers: Vec<_> = servers
            .into_iter()
            .filter(|s| s.enabled.unwrap_or(true))
            .collect();

        info!("Found {} enabled MCP servers", enabled_servers.len());

        // Connect to each enabled server
        for server_config in enabled_servers {
            self.connect_server(server_config).await?;
        }

        info!(
            "MCP manager initialized with {} active servers",
            self.clients.len()
        );

        Ok(())
    }

    /// Connect to an MCP server
    pub async fn connect_server(&mut self, config: McpServerConfig) -> anyhow::Result<()> {
        if self.clients.contains_key(&config.id) {
            warn!("MCP server {} is already connected, skipping", config.id);
            return Ok(());
        }

        info!("Connecting to MCP server: {} ({})", config.name, config.id);

        // Create and connect client
        let client = match create_and_connect(config.clone()).await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to connect to MCP server {}: {}", config.name, e);
                return Err(e);
            }
        };

        let client_arc = Arc::new(Mutex::new(client));

        // Register tools from this server
        self.tool_manager
            .register_tools_from_client(config.id.clone(), client_arc.clone())
            .await?;

        // Store the client
        self.clients.insert(config.id.clone(), client_arc);

        info!(
            "Connected to MCP server: {} ({} tools)",
            config.name,
            self.tool_manager.count()
        );

        Ok(())
    }

    /// Disconnect from an MCP server
    pub async fn disconnect_server(&mut self, server_id: &str) -> anyhow::Result<()> {
        if let Some(client) = self.clients.remove(server_id) {
            info!("Disconnecting from MCP server: {}", server_id);

            let mut mc = client.lock().await;
            mc.shutdown().await?;

            info!("Disconnected from MCP server: {}", server_id);
        }

        Ok(())
    }

    /// Reconnect to an MCP server
    pub async fn reconnect_server(&mut self, server_id: &str) -> anyhow::Result<()> {
        info!("Reconnecting to MCP server: {}", server_id);

        // Disconnect if already connected
        if self.clients.contains_key(server_id) {
            self.disconnect_server(server_id).await?;
        }

        // Get server config
        let config = self
            .config_manager
            .get_server(server_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Server config not found: {}", server_id))?;

        // Connect again
        self.connect_server(config).await?;

        Ok(())
    }

    /// Reconnect all enabled MCP servers
    pub async fn reconnect_all(&mut self) -> anyhow::Result<()> {
        info!("Reconnecting all MCP servers");

        // Clear current connections
        let server_ids: Vec<String> = self.clients.keys().cloned().collect();
        for server_id in server_ids {
            self.disconnect_server(&server_id).await?;
        }

        // Clear tool manager
        self.tool_manager.clear();

        // Re-initialize
        self.initialize().await?;

        Ok(())
    }

    /// Get all MCP server status
    pub async fn get_all_status(&self) -> Vec<McpServerStatus> {
        let servers = match self.config_manager.list_servers().await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to list MCP servers: {}", e);
                return Vec::new();
            }
        };

        servers
            .into_iter()
            .map(|server| {
                let connected = self.clients.contains_key(&server.id);
                McpServerStatus {
                    id: server.id.clone(),
                    name: server.name,
                    connected,
                    tools_count: if connected {
                        Some(1) // TODO: Get actual tool count
                    } else {
                        None
                    },
                    error: None,
                }
            })
            .collect()
    }

    /// Get tool manager reference
    pub fn tool_manager(&self) -> &McpToolManager {
        &self.tool_manager
    }

    /// Get config manager reference
    pub fn config_manager(&self) -> &McpConfigManager {
        &self.config_manager
    }
}

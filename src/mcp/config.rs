//! MCP Server configuration management
//!
//! This module handles storage and retrieval of MCP server configurations.

use super::types::McpServerConfig;
use sqlx::{Row, sqlite::SqlitePool};
use tracing::info;

/// MCP server configuration manager
pub struct McpConfigManager {
    pool: SqlitePool,
}

impl McpConfigManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Verify that the MCP servers table is available.
    ///
    /// Schema creation is handled centrally by `crate::db::init_schema()`.
    /// This method only performs a lightweight existence check.
    pub async fn init(&self) -> anyhow::Result<()> {
        sqlx::query("SELECT 1 FROM mcp_servers LIMIT 1")
            .execute(&self.pool)
            .await?;

        info!("MCP servers table verified");

        Ok(())
    }

    /// List all MCP server configurations
    pub async fn list_servers(&self) -> anyhow::Result<Vec<McpServerConfig>> {
        let rows = sqlx::query("SELECT * FROM mcp_servers ORDER BY name")
            .fetch_all(&self.pool)
            .await?;

        let mut servers = Vec::new();
        for row in rows {
            let transport_config_json: String = row.get("transport_config");
            let transport_config: super::types::TransportConfig =
                serde_json::from_str(&transport_config_json)
                    .map_err(|e| anyhow::anyhow!("Failed to parse transport config: {}", e))?;

            let transport_type_str: String = row.get("transport_type");
            let transport_type = match transport_type_str.as_str() {
                "stdio" => super::types::TransportType::Stdio,
                "sse" => super::types::TransportType::ServerSentEvents,
                "websocket" => super::types::TransportType::WebSocket,
                "http" => super::types::TransportType::Http,
                _ => super::types::TransportType::Stdio,
            };

            let server = McpServerConfig {
                id: row.get("id"),
                name: row.get("name"),
                transport_type,
                transport_config,
                enabled: Some(row.get::<i32, _>("enabled") == 1),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };
            servers.push(server);
        }

        Ok(servers)
    }

    /// Get a specific MCP server configuration
    pub async fn get_server(&self, id: &str) -> anyhow::Result<Option<McpServerConfig>> {
        let row = sqlx::query("SELECT * FROM mcp_servers WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let transport_config_json: String = row.get("transport_config");
            let transport_config: super::types::TransportConfig =
                serde_json::from_str(&transport_config_json)?;

            let transport_type_str: String = row.get("transport_type");
            let transport_type = match transport_type_str.as_str() {
                "stdio" => super::types::TransportType::Stdio,
                "sse" => super::types::TransportType::ServerSentEvents,
                "websocket" => super::types::TransportType::WebSocket,
                "http" => super::types::TransportType::Http,
                _ => super::types::TransportType::Stdio,
            };

            let server = McpServerConfig {
                id: row.get("id"),
                name: row.get("name"),
                transport_type,
                transport_config,
                enabled: Some(row.get::<i32, _>("enabled") == 1),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };

            Ok(Some(server))
        } else {
            Ok(None)
        }
    }

    /// Create a new MCP server configuration
    pub async fn create_server(&self, server: &McpServerConfig) -> anyhow::Result<()> {
        let transport_config_json = serde_json::to_string(&server.transport_config)?;

        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO mcp_servers (id, name, transport_type, transport_config, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&server.id)
        .bind(&server.name)
        .bind(match server.transport_type {
            super::types::TransportType::Stdio => "stdio",
            super::types::TransportType::ServerSentEvents => "sse",
            super::types::TransportType::WebSocket => "websocket",
            super::types::TransportType::Http => "http",
        })
        .bind(&transport_config_json)
        .bind(server.enabled.unwrap_or(true) as i32)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        info!("Created MCP server: {}", server.name);

        Ok(())
    }

    /// Update an existing MCP server configuration
    pub async fn update_server(&self, server: &McpServerConfig) -> anyhow::Result<()> {
        let transport_config_json = serde_json::to_string(&server.transport_config)?;

        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            UPDATE mcp_servers
            SET name = ?, transport_type = ?, transport_config = ?, enabled = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&server.name)
        .bind(match server.transport_type {
            super::types::TransportType::Stdio => "stdio",
            super::types::TransportType::ServerSentEvents => "sse",
            super::types::TransportType::WebSocket => "websocket",
            super::types::TransportType::Http => "http",
        })
        .bind(&transport_config_json)
        .bind(server.enabled.unwrap_or(true) as i32)
        .bind(now)
        .bind(&server.id)
        .execute(&self.pool)
        .await?;

        info!("Updated MCP server: {}", server.name);

        Ok(())
    }

    /// Delete an MCP server configuration
    pub async fn delete_server(&self, id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM mcp_servers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        info!("Deleted MCP server: {}", id);

        Ok(())
    }

    /// Enable or disable an MCP server
    pub async fn set_enabled(&self, id: &str, enabled: bool) -> anyhow::Result<()> {
        let now = chrono::Utc::now();

        sqlx::query("UPDATE mcp_servers SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(enabled as i32)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        info!(
            "{} MCP server: {}",
            if enabled { "Enabled" } else { "Disabled" },
            id
        );

        Ok(())
    }
}

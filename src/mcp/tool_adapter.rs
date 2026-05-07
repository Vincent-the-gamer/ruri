//! MCP Tool adapter
//!
//! This module provides an adapter to use MCP tools as Ruri tools.

use super::client::McpClient;
use super::types::McpTool;
use crate::agent::tool_executor::{Tool, ToolError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A Ruri Tool that wraps an MCP tool
pub struct McpToolAdapter {
    server_id: String,
    tool: McpTool,
    client: Arc<Mutex<McpClient>>,
}

impl McpToolAdapter {
    /// Create a new MCP tool adapter
    pub fn new(server_id: String, tool: McpTool, client: Arc<Mutex<McpClient>>) -> Self {
        Self {
            server_id,
            tool,
            client,
        }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    /// Return the tool definition for the model
    fn definition(&self) -> crate::types::ToolDefinition {
        // Convert MCP tool schema to Ruri tool definition
        let schema = &self.tool.input_schema;

        // Extract properties from JSON Schema
        let mut properties = HashMap::new();
        let mut required = Vec::new();

        if let Some(obj) = schema.as_object() {
            if let Some(props) = obj.get("properties").and_then(|p| p.as_object()) {
                for (name, prop) in props {
                    if let Some(prop_obj) = prop.as_object() {
                        let type_str = prop_obj
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("string");

                        let param_type = match type_str {
                            "string" => crate::types::ParameterType::String,
                            "number" => crate::types::ParameterType::Number,
                            "integer" => crate::types::ParameterType::Integer,
                            "boolean" => crate::types::ParameterType::Boolean,
                            "array" => crate::types::ParameterType::Array,
                            "object" => crate::types::ParameterType::Object,
                            _ => crate::types::ParameterType::String,
                        };

                        let description = prop_obj
                            .get("description")
                            .and_then(|d| d.as_str())
                            .map(|s| s.to_string());

                        properties.insert(
                            name.clone(),
                            crate::types::ToolParameter {
                                param_type,
                                description,
                                enum_values: None,
                            },
                        );
                    }
                }
            }

            if let Some(req) = obj.get("required").and_then(|r| r.as_array()) {
                for item in req {
                    if let Some(s) = item.as_str() {
                        required.push(s.to_string());
                    }
                }
            }
        }

        crate::types::ToolDefinition {
            tool_type: crate::types::ToolType::Function,
            function: crate::types::ToolFunction {
                name: self.tool.name.clone(),
                description: Some(format!(
                    "[MCP:{}] {}",
                    self.server_id,
                    self.tool.description.clone()
                )),
                parameters: Some(crate::types::ToolParameters {
                    schema_type: crate::types::SchemaType::Object,
                    properties: Some(properties),
                    required: if required.is_empty() {
                        None
                    } else {
                        Some(required)
                    },
                }),
            },
        }
    }

    /// Execute the tool by calling the MCP server
    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        // Parse arguments
        let arguments: Option<HashMap<String, serde_json::Value>> =
            if args.trim().is_empty() || args == "{}" {
                None
            } else {
                match serde_json::from_str(args) {
                    Ok(map) => Some(map),
                    Err(e) => {
                        return Err(ToolError::InvalidArguments(format!(
                            "Failed to parse tool arguments: {}",
                            e
                        )));
                    }
                }
            };

        // Call the MCP tool
        let result = {
            // Acquire lock and call the MCP tool
            let mut client = self.client.lock().await;
            client
                .call_tool(self.tool.name.clone(), arguments.clone())
                .await
        };

        match result {
            Ok(result) => {
                // Format the result as string
                let content: Vec<String> = result
                    .content
                    .into_iter()
                    .map(|item| match item {
                        super::types::ContentItem::Text { text } => text,
                        super::types::ContentItem::Image { data, .. } => {
                            format!("[Image data: {} bytes]", data.len())
                        }
                        super::types::ContentItem::Resource { uri, text, blob } => {
                            let mut parts = vec![format!("Resource: {}", uri)];
                            if let Some(t) = text {
                                parts.push(t);
                            }
                            if let Some(b) = blob {
                                parts.push(format!("[Blob: {} bytes]", b.len()));
                            }
                            parts.join("\n")
                        }
                    })
                    .collect();

                Ok(content.join("\n"))
            }
            Err(e) => Err(ToolError::ExecutionError(format!(
                "MCP tool execution failed: {}",
                e
            ))),
        }
    }
}

/// Manages MCP tool adapters
pub struct McpToolManager {
    adapters: Vec<Arc<McpToolAdapter>>,
}

impl McpToolManager {
    pub fn new() -> Self {
        Self {
            adapters: Vec::new(),
        }
    }

    /// Register tools from an MCP client
    pub async fn register_tools_from_client(
        &mut self,
        server_id: String,
        client: Arc<Mutex<McpClient>>,
    ) -> Result<(), ToolError> {
        // List tools from the MCP server
        let mut mc = client.lock().await;

        let tools_result = mc.list_tools().await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to list tools from MCP server: {}", e))
        })?;

        drop(mc);

        // Create adapters for each tool
        for tool in tools_result.tools {
            let adapter = Arc::new(McpToolAdapter::new(
                server_id.clone(),
                tool.clone(),
                client.clone(),
            ));
            self.adapters.push(adapter);
        }

        Ok(())
    }

    /// Get all registered tool adapters
    pub fn adapters(&self) -> &[Arc<McpToolAdapter>] {
        &self.adapters
    }

    /// Clear all adapters
    pub fn clear(&mut self) {
        self.adapters.clear();
    }

    /// Get count of registered tools
    pub fn count(&self) -> usize {
        self.adapters.len()
    }
}

impl Default for McpToolManager {
    fn default() -> Self {
        Self::new()
    }
}

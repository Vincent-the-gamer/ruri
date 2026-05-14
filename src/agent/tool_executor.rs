use crate::types::{FunctionCall, ToolDefinition, ToolResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// A tool that can be executed by the Agent.
#[async_trait]
pub trait Tool: Send + Sync {
    /// The tool definition (schema) to send to the model.
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with the given arguments.
    async fn execute(&self, args: &str) -> Result<String, ToolError>;
}

/// Error type for tool execution.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool execution failed: {0}")]
    ExecutionError(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Tool not found: {0}")]
    NotFound(String),
}

/// Registry that manages available tools and dispatches calls.
pub struct ToolExecutor {
    tools: HashMap<String, Arc<dyn Tool>>,
    /// Custom error message to show users when a tool call fails.
    /// If not set, the raw error message is returned.
    custom_error_message: Option<String>,
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            custom_error_message: None,
        }
    }

    /// Set a custom error message for tool execution failures.
    pub fn set_custom_error_message(&mut self, message: Option<String>) {
        self.custom_error_message = message;
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let def = tool.definition();
        let name = def.function.name.clone();
        self.tools.insert(name, tool);
    }

    /// Get all tool definitions for sending to the model.
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Execute a tool call by name.
    pub async fn execute(&self, function_call: &FunctionCall) -> Result<ToolResult, ToolError> {
        let tool = self
            .tools
            .get(&function_call.name)
            .ok_or_else(|| ToolError::NotFound(function_call.name.clone()))?;

        // Log tool call with arguments
        tracing::info!(
            tool = %function_call.name,
            arguments = %function_call.arguments,
            "Executing tool"
        );

        let start = std::time::Instant::now();
        let result = tool.execute(&function_call.arguments).await;
        let duration = start.elapsed();

        match result {
            Ok(content) => {
                // Log success with result preview (truncate if too long)
                let preview = if content.len() > 500 {
                    let end = content.floor_char_boundary(500);
                    format!("{}... ({} chars total)", &content[..end], content.len())
                } else {
                    content.clone()
                };
                tracing::info!(
                    tool = %function_call.name,
                    duration_ms = duration.as_millis(),
                    result_preview = %preview,
                    "Tool execution completed"
                );
                Ok(ToolResult {
                    tool_call_id: String::new(), // Will be filled by the caller
                    content,
                })
            }
            Err(e) => {
                // Log error
                tracing::error!(
                    tool = %function_call.name,
                    duration_ms = duration.as_millis(),
                    error = %e,
                    "Tool execution failed"
                );
                // If a custom error message is set, use it; otherwise use the raw error
                let content = if let Some(ref msg) = self.custom_error_message {
                    msg.clone()
                } else {
                    format!("Error: {}", e)
                };
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    content,
                })
            }
        }
    }

    /// Execute a function call and set the tool_call_id.
    pub async fn execute_with_id(
        &self,
        tool_call_id: impl Into<String>,
        function_call: &FunctionCall,
    ) -> ToolResult {
        match self.execute(function_call).await {
            Ok(mut result) => {
                result.tool_call_id = tool_call_id.into();
                result
            }
            Err(e) => ToolResult {
                tool_call_id: tool_call_id.into(),
                content: format!("Error: {}", e),
            },
        }
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

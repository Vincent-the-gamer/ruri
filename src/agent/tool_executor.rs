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

    #[error("Timeout executing tool: {0}")]
    Timeout(String),
}

/// Registry that manages available tools and dispatches calls.
pub struct ToolExecutor {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let def = tool.definition();
        let name = def.function.name.clone();
        self.tools.insert(name, tool);
    }

    /// Register multiple tools.
    pub fn register_all(&mut self, tools: Vec<Arc<dyn Tool>>) {
        for tool in tools {
            self.register(tool);
        }
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

        tracing::info!(tool = %function_call.name, "Executing tool");

        match tool.execute(&function_call.arguments).await {
            Ok(result) => Ok(ToolResult {
                tool_call_id: String::new(), // Will be filled by the caller
                content: result,
            }),
            Err(e) => Ok(ToolResult {
                tool_call_id: String::new(),
                content: format!("Error: {}", e),
            }),
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

    /// Check if a tool is registered.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// List registered tool names.
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

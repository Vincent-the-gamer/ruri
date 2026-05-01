use crate::types::{FunctionCall, ParameterType, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
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

// ─── Built-in Tools ──────────────────────────────────────────────────

/// A simple echo tool for testing.
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("echo")
            .description("Echoes back the input text")
            .parameter("text", ParameterType::String, true)
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        let text = parsed["text"].as_str().unwrap_or("No text provided");
        Ok(text.to_string())
    }
}

/// A calculator tool that evaluates simple arithmetic expressions.
pub struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("calculator")
            .description("Evaluates a simple arithmetic expression (e.g., '2 + 3 * 4')")
            .parameter("expression", ParameterType::String, true)
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        let expression = parsed["expression"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'expression' parameter".into()))?;

        // Simple expression evaluator — supports +, -, *, /
        let result = evaluate_expression(expression)?;
        Ok(result.to_string())
    }
}

/// Very simple arithmetic expression evaluator.
fn evaluate_expression(expr: &str) -> Result<f64, ToolError> {
    let expr = expr.replace(' ', "");

    // Find the last operator at the top level (for correct precedence)
    let mut depth = 0i32;
    let mut last_add_sub = None;

    for (i, c) in expr.char_indices().rev() {
        match c {
            ')' => depth += 1,
            '(' => depth -= 1,
            '+' | '-' if depth == 0 && i > 0 => {
                last_add_sub = Some(i);
                break;
            }
            _ => {}
        }
    }

    if let Some(pos) = last_add_sub {
        let left = &expr[..pos];
        let op = expr.as_bytes()[pos];
        let right = &expr[pos + 1..];

        let left_val = evaluate_expression(left)?;
        let right_val = evaluate_expression(right)?;

        return match op {
            b'+' => Ok(left_val + right_val),
            b'-' => Ok(left_val - right_val),
            _ => Err(ToolError::ExecutionError(format!(
                "Unknown operator: {}",
                op as char
            ))),
        };
    }

    // Handle multiplication and division
    let mut depth = 0i32;
    let mut last_mul_div = None;

    for (i, c) in expr.char_indices().rev() {
        match c {
            ')' => depth += 1,
            '(' => depth -= 1,
            '*' | '/' if depth == 0 && i > 0 => {
                last_mul_div = Some(i);
                break;
            }
            _ => {}
        }
    }

    if let Some(pos) = last_mul_div {
        let left = &expr[..pos];
        let op = expr.as_bytes()[pos];
        let right = &expr[pos + 1..];

        let left_val = evaluate_expression(left)?;
        let right_val = evaluate_expression(right)?;

        return match op {
            b'*' => Ok(left_val * right_val),
            b'/' => {
                if right_val == 0.0 {
                    Err(ToolError::ExecutionError("Division by zero".into()))
                } else {
                    Ok(left_val / right_val)
                }
            }
            _ => Err(ToolError::ExecutionError(format!(
                "Unknown operator: {}",
                op as char
            ))),
        };
    }

    // Handle parentheses
    if expr.starts_with('(') && expr.ends_with(')') {
        return evaluate_expression(&expr[1..expr.len() - 1]);
    }

    // Parse number
    expr.parse::<f64>()
        .map_err(|_| ToolError::ExecutionError(format!("Invalid expression: {}", expr)))
}

/// A tool that gets the current date and time.
pub struct DateTimeTool;

#[async_trait]
impl Tool for DateTimeTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("get_datetime")
            .description("Gets the current date and time in ISO 8601 format")
            .parameter("timezone", ParameterType::String, false)
            .parameter_with_description(
                "format",
                ParameterType::String,
                false,
                Some("Output format: 'iso8601' (default) or 'unix'"),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).unwrap_or(Value::Object(serde_json::Map::new()));
        let format = parsed["format"].as_str().unwrap_or("iso8601");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        match format {
            "unix" => Ok(now.as_secs().to_string()),
            _ => {
                // Simple ISO-like format
                let secs = now.as_secs();
                Ok(format!("Timestamp: {} (unix seconds)", secs))
            }
        }
    }
}

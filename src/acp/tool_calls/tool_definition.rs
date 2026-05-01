use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Re-export ACP schema types for convenience
use agent_client_protocol::schema::{
    ContentBlock, Diff, Terminal, TextContent, ToolCall, ToolCallContent, ToolCallLocation,
    ToolCallUpdate, ToolKind,
};

/// Represents a tool call in the ACP protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpToolCall {
    /// Unique identifier for this tool call within the session.
    pub tool_call_id: String,
    /// Human-readable title describing what the tool is doing.
    pub title: String,
    /// The category of tool being invoked.
    pub kind: ToolKind,
    /// File locations affected by this tool call.
    pub locations: Vec<ToolCallLocation>,
    /// Content produced by the tool call.
    pub content: Vec<ToolCallContent>,
    /// Raw input parameters sent to the tool.
    pub raw_input: Option<String>,
    /// Raw output returned by the tool.
    pub raw_output: Option<String>,
    /// Current execution status of the tool call.
    pub status: ToolCallStatus,
}

/// Represents an update to an existing tool call in the ACP protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpToolCallUpdate {
    /// The ID of the tool call being updated.
    pub tool_call_id: String,
    /// Update the human-readable title.
    pub title: Option<String>,
    /// Update the execution status.
    pub status: Option<ToolCallStatus>,
    /// Replace the content collection.
    pub content: Option<Vec<ToolCallContent>>,
    /// Update the raw input.
    pub raw_input: Option<String>,
    /// Update the raw output.
    pub raw_output: Option<String>,
    /// Replace the locations collection.
    pub locations: Option<Vec<ToolCallLocation>>,
}

/// Execution status of a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCallStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Convert `ruri` types to ACP types.
impl From<ToolResult> for Vec<ToolCallContent> {
    fn from(result: ToolResult) -> Self {
        // Convert the content string to a TextContent block
        let text_content = TextContent::new(result.content);
        vec![ContentBlock::Text(text_content).into()]
    }
}

impl From<ToolResult> for AcpToolCall {
    fn from(result: ToolResult) -> Self {
        Self {
            tool_call_id: result.tool_call_id,
            title: "Tool Execution".to_string(),
            kind: ToolKind::Other,
            locations: Vec::new(),
            content: result.into(),
            raw_input: None,
            raw_output: None,
            status: ToolCallStatus::Completed,
        }
    }
}

impl From<ToolResult> for AcpToolCallUpdate {
    fn from(result: ToolResult) -> Self {
        Self {
            tool_call_id: result.tool_call_id,
            title: None,
            status: Some(ToolCallStatus::Completed),
            content: Some(result.into()),
            raw_input: None,
            raw_output: None,
            locations: None,
        }
    }
}

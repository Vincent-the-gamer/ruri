use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::schema::WriteTextFileRequest;
use async_trait::async_trait;

use crate::acp::session::SessionManager;
use crate::agent::tool_executor::{Tool, ToolError};
use crate::types::ParameterType;
use crate::types::ToolDefinition;

use super::RequestManager;

/// ACP-based file writing tool that requests to write file content via the client.
pub struct AcpWriteFileTool {
    /// The ID of the ACP session this tool belongs to.
    session_id: String,
    /// Session manager for accessing client connections.
    session_manager: Arc<SessionManager>,
    /// Request manager for handling async responses.
    request_manager: Arc<RequestManager>,
}

impl AcpWriteFileTool {
    pub fn new(
        session_id: String,
        session_manager: Arc<SessionManager>,
        request_manager: Arc<RequestManager>,
    ) -> Self {
        Self {
            session_id,
            session_manager,
            request_manager,
        }
    }
}

#[async_trait]
impl Tool for AcpWriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("acp_write_file")
            .description(
                "Write content to a text file via ACP protocol. \
                 This requests the client (IDE) to perform the write operation, \
                 enabling proper permission handling.",
            )
            .parameter_with_description(
                "path",
                ParameterType::String,
                true,
                Some("The path to the file to write (relative or absolute)."),
            )
            .parameter_with_description(
                "contents",
                ParameterType::String,
                true,
                Some("The content to write to the file."),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        // Parse arguments
        let parsed = super::parse_args(args)
            .map_err(|e| ToolError::InvalidArguments(format!("Failed to parse args: {}", e)))?;

        let path_str = parsed
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;

        let contents = parsed
            .get("contents")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'contents' parameter".into()))?;

        let path = PathBuf::from(path_str);

        // Get client connection
        let connection = self
            .session_manager
            .get_connection(&self.session_id)
            .await
            .ok_or_else(|| {
                ToolError::ExecutionError(format!(
                    "No connection available for session {}",
                    self.session_id
                ))
            })?;

        // For now, return placeholder error until ConnectionTo API is implemented
        Err(ToolError::ExecutionError(
            "ACP write file not yet fully implemented - need to implement ConnectionTo API usage"
                .into(),
        ))

        // TODO: Implement actual file writing via ACP protocol
        // Steps:
        // 1. Generate request ID
        // 2. Register pending request
        // 3. Send WriteTextFileRequest to client
        // 4. Wait for response
        // 5. Return success
    }
}

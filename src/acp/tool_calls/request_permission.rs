use crate::agent::tool_executor::ToolExecutor;
use agent_client_protocol::schema::{
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, ToolCall,
};
use std::sync::Arc;
use tracing::{info, warn};

/// Handles ACP tool call permission requests.
pub struct PermissionHandler {
    tool_executor: Arc<ToolExecutor>,
}

impl PermissionHandler {
    pub fn new(tool_executor: Arc<ToolExecutor>) -> Self {
        Self { tool_executor }
    }

    /// Handle a permission request from the client.
    pub fn handle_request(
        &self,
        request: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse, String> {
        let tool_name = request.tool_call.title.clone();

        // Check if the tool is registered in our executor
        if !self.tool_executor.has_tool(&tool_name) {
            warn!(
                tool = %tool_name,
                "Tool not found in executor"
            );
            // If the tool doesn't exist, we reject the request
            return Ok(RequestPermissionResponse::new(
                RequestPermissionOutcome::Selected(SelectedPermissionOutcome {
                    option_id: "reject_always".to_string(),
                }),
            ));
        }

        // Define standard ACP permission options
        let options = vec![
            agent_client_protocol::schema::PermissionOption {
                option_id: "allow_once".to_string(),
                name: "Allow once".to_string(),
                kind: agent_client_protocol::schema::PermissionOptionKind::AllowOnce,
            },
            agent_client_protocol::schema::PermissionOption {
                option_id: "allow_always".to_string(),
                name: "Allow always".to_string(),
                kind: agent_client_protocol::schema::PermissionOptionKind::AllowAlways,
            },
            agent_client_protocol::schema::PermissionOption {
                option_id: "reject_once".to_string(),
                name: "Reject once".to_string(),
                kind: agent_client_protocol::schema::PermissionOptionKind::RejectOnce,
            },
            agent_client_protocol::schema::PermissionOption {
                option_id: "reject_always".to_string(),
                name: "Reject always".to_string(),
                kind: agent_client_protocol::schema::PermissionOptionKind::RejectAlways,
            },
        ];

        info!(
            tool = %tool_name,
            "Handling permission request"
        );

        // Return response with options
        // In a real implementation, this would prompt the user
        Ok(RequestPermissionResponse::new(
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome {
                option_id: "allow_always".to_string(),
            }),
        ))
    }
}

impl Default for PermissionHandler {
    fn default() -> Self {
        Self::new(Arc::new(crate::agent::tool_executor::ToolExecutor::new()))
    }
}

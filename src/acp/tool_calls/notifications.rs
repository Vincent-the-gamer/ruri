use agent_client_protocol::schema::{
    AgentNotification, SessionNotification, SessionUpdate, ToolCall, ToolCallUpdate,
};
use crate::agent::tool_executor::ToolExecutor;
use crate::types::ToolResult;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Handles ACP tool call notifications.
pub struct ToolCallHandler {
    tool_executor: Arc<ToolExecutor>,
}

impl ToolCallHandler {
    pub fn new(tool_executor: Arc<ToolExecutor>) -> Self {
        Self { tool_executor }
    }

    /// Send a new tool call notification to the client.
    pub fn send_tool_call_notification(
        &self,
        session_id: String,
        tool_call: ToolCall,
    ) -> Result<AgentNotification, String> {
        let update = SessionUpdate::ToolCall(tool_call);
        let notification = SessionNotification::new(session_id, update);

        info!(
            tool_call_id = %tool_call.tool_call_id,
            tool_name = %tool_call.title,
            "Sending tool call notification"
        );

        Ok(AgentNotification::SessionNotification(notification))
    }

    /// Send an update to an existing tool call.
    pub fn send_tool_call_update(
        &self,
        session_id: String,
        tool_call_update: ToolCallUpdate,
    ) -> Result<AgentNotification, String> {
        let update = SessionUpdate::ToolCallUpdate(tool_call_update);
        let notification = SessionNotification::new(session_id, update);

        debug!(
            tool_call_id = %tool_call_update.tool_call_id,
            status = ?tool_call_update.status,
            "Sending tool call update notification"
        );

        Ok(AgentNotification::SessionNotification(notification))
    }

    /// Send a tool call result as content.
    pub fn send_tool_result(
        &self,
        session_id: String,
        tool_call_id: String,
        content: String,
    ) -> Result<AgentNotification, String> {
        let update = SessionUpdate::ContentChunk {
            content: crate::acp::tool_calls::tool_definition::TextContent::new(content),
        };
        let notification = SessionNotification::new(session_id, update);

        debug!(
            tool_call_id = %tool_call_id,
            content_len = content.len(),
            "Sending tool result notification"
        );

        Ok(AgentNotification::SessionNotification(notification))
    }

    /// Execute a tool call and send notifications.
    pub async fn execute_and_notify(
        &self,
        session_id: String,
        tool_call_id: String,
        function_call: &crate::types::FunctionCall,
    ) -> Result<ToolResult, String> {
        info!(
            tool_call_id = %tool_call_id,
            tool_name = %function_call.name,
            "Executing tool call"
        );

        // Send pending notification
        let tool_call = ToolCall {
            tool_call_id: tool_call_id.clone(),
            title: function_call.name.clone(),
            kind: crate::acp::tool_calls::tool_definition::ToolKind::Other,
            locations: Vec::new(),
            content: Vec::new(),
            raw_input: Some(function_call.arguments.clone()),
            raw_output: None,
            status: crate::acp::tool_calls::tool_definition::ToolCallStatus::InProgress,
        };

        self.send_tool_call_notification(session_id.clone(), tool_call)?;

        // Execute the tool
        let result = self.tool_executor.execute_with_id(&tool_call_id, function_call).await;

        // Send update notification
        let tool_call_update = ToolCallUpdate {
            tool_call_id: tool_call_id.clone(),
            title: Some(result.content.clone()),
            status: Some(match &result {
                Ok(_) => crate::acp::tool_calls::tool_definition::ToolCallStatus::Completed,
                Err(_) => crate::acp::tool_calls::tool_definition::ToolCallStatus::Failed,
            }),
            content: Some(result.clone().into()),
            raw_input: None,
            raw_output: Some(result.content.clone()),
            locations: None,
        };

        self.send_tool_call_update(session_id, tool_call_update)?;

        // Send result as content
        if let Ok(ref result) = result {
            self.send_tool_result(session_id, tool_call_id, result.content.clone())?;
        }

        result
    }
}

impl Default for ToolCallHandler {
    fn default() -> Self {
        Self::new(Arc::new(crate::agent::tool_executor::ToolExecutor::new()))
    }
}
```

```ruri\src\acp\tool_calls\mod.rs
pub mod tool_definition;
pub mod notifications;
pub mod request_permission;

pub use tool_definition::*;
pub use notifications::*;
pub use request_permission::*;
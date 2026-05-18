use crate::agent::tool_executor::{Tool, ToolError};
use crate::computer_use::permissions::PermissionChecker;
use crate::computer_use::workspace::WorkspaceManager;
use crate::types::{ParameterType, ToolDefinition};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::fs;
use tracing::info;

/// Context for tool execution with permission and workspace information
#[derive(Clone)]
pub struct ComputerUseContext {
    pub user_id: String,
    pub session_id: String,
    pub permission_checker: Arc<PermissionChecker>,
    pub workspace_manager: Arc<WorkspaceManager>,
    /// When true, shell commands require explicit user confirmation via the
    /// `confirmed` parameter. When false, shell commands execute directly
    /// (only blocked by the global blacklist).
    /// In chat mode, this is false — dangerous commands are handled by the
    /// blacklist instead of requiring per-command confirmation.
    /// In ACP mode, this is also false — ACP has its own permission system
    /// where the user confirms shell execution via button clicks.
    pub require_shell_confirmation: bool,
}

/// Shell tool with permission checking
pub struct ShellTool {
    context: Arc<ComputerUseContext>,
}

impl ShellTool {
    pub fn new(context: Arc<ComputerUseContext>) -> Self {
        Self { context }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("shell")
            .description(
                "Execute a shell command in the workspace directory. \
                 This tool is only available to admin users by default. \
                 On Linux/macOS, uses bash. On Windows, uses PowerShell. \
                 The command will run with the workspace as the working directory.",
            )
            .parameter_with_description(
                "command",
                ParameterType::String,
                true,
                Some("The shell command to execute."),
            )
            .parameter_with_description(
                "timeout",
                ParameterType::Integer,
                false,
                Some("Optional timeout in seconds (default: 30)."),
            )
            .parameter_with_description(
                "confirmed",
                ParameterType::Boolean,
                false,
                Some("Set to true to confirm execution of the command. Required in chat mode when the command is potentially dangerous; set to true after the user has approved."),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        // Check permissions
        self.context
            .permission_checker
            .can_execute_shell(&self.context.user_id)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let command = parsed["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".into()))?;

        // Always block blacklisted commands (configured via global shell command blacklist)
        if self
            .context
            .permission_checker
            .is_command_blacklisted(command)
        {
            return Err(ToolError::ExecutionError(
                "⚠️ This command has been blocked by the shell command blacklist. \
                 It matches a dangerous command pattern configured by the administrator. \
                 Please use a different command or contact your administrator to adjust the blacklist settings."
                    .to_string(),
            ));
        }

        // In chat mode (require_shell_confirmation=true), ask for user confirmation
        // before executing any shell command. The model will present the confirmation
        // request to the user and re-call this tool with confirmed=true.
        if self.context.require_shell_confirmation {
            let confirmed = parsed["confirmed"].as_bool().unwrap_or(false);
            if !confirmed {
                return Ok(format!(
                    "⚠️ About to execute shell command:\n```\n{}\n```\n\nPlease confirm this operation. Reply with 'yes' or 'proceed' to execute.",
                    command
                ));
            }
        }

        let timeout_secs = parsed["timeout"].as_u64().unwrap_or(30);

        // Get working directory (workspace for this session)
        let working_dir = self
            .context
            .workspace_manager
            .get_workspace_path(&self.context.session_id);

        // Ensure workspace exists
        self.context
            .workspace_manager
            .create_workspace(&self.context.session_id)
            .await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        info!(
            "Executing shell command for user '{}' in session '{}': {}",
            self.context.user_id, self.context.session_id, command
        );

        // Execute the command
        #[cfg(target_os = "windows")]
        let result = {
            tokio::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", command])
                .current_dir(&working_dir)
                .output()
                .await
        };

        #[cfg(not(target_os = "windows"))]
        let result = {
            tokio::process::Command::new("bash")
                .arg("-c")
                .arg(command)
                .current_dir(&working_dir)
                .output()
                .await
        };

        let output = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), async {
            result
                .map_err(|e| ToolError::ExecutionError(format!("Failed to execute command: {}", e)))
        })
        .await
        .map_err(|_| {
            ToolError::ExecutionError(format!("Command timed out after {} seconds", timeout_secs))
        })??;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            if stderr.is_empty() {
                Ok(stdout)
            } else {
                Ok(format!("{}\n[stderr]\n{}", stdout, stderr))
            }
        } else {
            Err(ToolError::ExecutionError(format!(
                "Command failed with exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr
            )))
        }
    }
}

/// Python tool with permission checking (optional feature)
pub struct PythonTool {
    context: Arc<ComputerUseContext>,
}

impl PythonTool {
    pub fn new(context: Arc<ComputerUseContext>) -> Self {
        Self { context }
    }
}

#[async_trait]
impl Tool for PythonTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("python")
            .description(
                "Execute Python code in the workspace directory. \
                 This tool is only available to admin users by default. \
                 The code will run with the workspace as the working directory. \
                 Make sure to use absolute paths when reading/writing files, \
                 or use the file tools to prepare files first.",
            )
            .parameter_with_description(
                "code",
                ParameterType::String,
                true,
                Some("The Python code to execute."),
            )
            .parameter_with_description(
                "timeout",
                ParameterType::Integer,
                false,
                Some("Optional timeout in seconds (default: 60)."),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        // Check permissions
        self.context
            .permission_checker
            .can_execute_python(&self.context.user_id)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let code = parsed["code"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'code' parameter".into()))?;

        let timeout_secs = parsed["timeout"].as_u64().unwrap_or(60);

        // Get working directory
        let working_dir = self
            .context
            .workspace_manager
            .get_workspace_path(&self.context.session_id);

        // Ensure workspace exists
        self.context
            .workspace_manager
            .create_workspace(&self.context.session_id)
            .await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        info!(
            "Executing Python code for user '{}' in session '{}'",
            self.context.user_id, self.context.session_id
        );

        // Create a temporary file for the code
        let temp_file = working_dir.join(".ruri_temp_script.py");
        fs::write(&temp_file, code)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to write temp file: {}", e)))?;

        // Execute Python
        let result = tokio::process::Command::new("python3")
            .arg(&temp_file)
            .current_dir(&working_dir)
            .output()
            .await;

        // Clean up temp file
        let _ = fs::remove_file(&temp_file).await;

        let output = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), async {
            result
                .map_err(|e| ToolError::ExecutionError(format!("Failed to execute Python: {}", e)))
        })
        .await
        .map_err(|_| {
            ToolError::ExecutionError(format!(
                "Python execution timed out after {} seconds",
                timeout_secs
            ))
        })??;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            if stderr.is_empty() {
                Ok(stdout)
            } else {
                Ok(format!("{}\n[stderr]\n{}", stdout, stderr))
            }
        } else {
            Err(ToolError::ExecutionError(format!(
                "Python execution failed with exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr
            )))
        }
    }
}

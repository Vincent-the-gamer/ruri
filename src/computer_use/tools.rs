use crate::agent::tool_executor::{Tool, ToolError};
use crate::computer_use::permissions::PermissionChecker;
use crate::computer_use::workspace::WorkspaceManager;
use crate::types::{ParameterType, ToolDefinition};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::fs;
use tracing::info;

#[cfg(target_os = "windows")]
use base64;

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

        // Execute the shell command in a blocking thread with timeout.
        // Uses std::process::Command to avoid the Windows overlapped I/O hang
        // where tokio::process::Command::wait_with_output() waits for all
        // pipe handles to close even after the process has exited.
        let output = execute_shell_sync_with_timeout(command, &working_dir, timeout_secs).await?;

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

        // Execute Python with timeout in a blocking thread.
        // Uses std::process::Command to avoid the Windows overlapped I/O hang.
        #[cfg(target_os = "windows")]
        let python_exe = "python";
        #[cfg(not(target_os = "windows"))]
        let python_exe = "python3";

        let output =
            execute_python_sync_with_timeout(python_exe, &temp_file, &working_dir, timeout_secs)
                .await?;

        // Clean up temp file after execution
        let _ = fs::remove_file(&temp_file).await;

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

// ─── Synchronous execution helpers (run inside spawn_blocking) ───

/// Execute a shell command synchronously with `std::process::Command` and
/// return the output.
///
/// On Windows, we create stdout/stderr pipes with non-inheritable handles to
/// prevent child processes from holding the pipe write ends open after
/// PowerShell exits. See `builtin_tools::create_noninheritable_pipe`.
#[cfg(target_os = "windows")]
fn run_shell_sync(
    command: &str,
    working_dir: &std::path::Path,
) -> Result<std::process::Output, String> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // Prepend UTF-8 output encoding fix so PowerShell outputs valid UTF-8.
    let command_with_encoding = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $OutputEncoding = [System.Text.Encoding]::UTF8; {command}"
    );

    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        command_with_encoding
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect::<Vec<u8>>(),
    );

    let (mut stdout_reader, stdout_writer) =
        crate::agent::builtin_tools::create_noninheritable_pipe()
            .map_err(|e| format!("Failed to create stdout pipe: {}", e))?;
    let (mut stderr_reader, stderr_writer) =
        crate::agent::builtin_tools::create_noninheritable_pipe()
            .map_err(|e| format!("Failed to create stderr pipe: {}", e))?;

    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-EncodedCommand", &encoded])
        .current_dir(working_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(stdout_writer))
        .stderr(std::process::Stdio::from(stderr_writer))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for command: {}", e))?;

    let mut stdout = Vec::new();
    stdout_reader
        .read_to_end(&mut stdout)
        .map_err(|e| format!("Failed to read stdout: {}", e))?;
    let mut stderr = Vec::new();
    stderr_reader
        .read_to_end(&mut stderr)
        .map_err(|e| format!("Failed to read stderr: {}", e))?;

    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

#[cfg(not(target_os = "windows"))]
fn run_shell_sync(
    command: &str,
    working_dir: &std::path::Path,
) -> Result<std::process::Output, String> {
    use std::process::Command;

    Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(working_dir)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))
}

/// Async wrapper that runs `run_shell_sync` in a blocking thread with timeout.
async fn execute_shell_sync_with_timeout(
    command: &str,
    working_dir: &std::path::Path,
    timeout_secs: u64,
) -> Result<std::process::Output, ToolError> {
    let command = command.to_string();
    let working_dir = working_dir.to_path_buf();

    let blocking_task = tokio::task::spawn_blocking(move || run_shell_sync(&command, &working_dir));

    tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), blocking_task)
        .await
        .map_err(|_| {
            ToolError::ExecutionError(format!("Command timed out after {} seconds", timeout_secs))
        })?
        .map_err(|e| ToolError::ExecutionError(format!("Blocking task panicked: {}", e)))?
        .map_err(|e| ToolError::ExecutionError(e))
}

/// Execute a Python script synchronously with `std::process::Command`.
#[cfg(target_os = "windows")]
fn run_python_sync(
    python_exe: &str,
    script_path: &std::path::Path,
    working_dir: &std::path::Path,
) -> Result<std::process::Output, String> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let (mut stdout_reader, stdout_writer) =
        crate::agent::builtin_tools::create_noninheritable_pipe()
            .map_err(|e| format!("Failed to create stdout pipe: {}", e))?;
    let (mut stderr_reader, stderr_writer) =
        crate::agent::builtin_tools::create_noninheritable_pipe()
            .map_err(|e| format!("Failed to create stderr pipe: {}", e))?;

    let mut child = Command::new(python_exe)
        .arg(script_path)
        .current_dir(working_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(stdout_writer))
        .stderr(std::process::Stdio::from(stderr_writer))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to spawn python: {}", e))?;

    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for python: {}", e))?;

    let mut stdout = Vec::new();
    stdout_reader
        .read_to_end(&mut stdout)
        .map_err(|e| format!("Failed to read stdout: {}", e))?;
    let mut stderr = Vec::new();
    stderr_reader
        .read_to_end(&mut stderr)
        .map_err(|e| format!("Failed to read stderr: {}", e))?;

    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

#[cfg(not(target_os = "windows"))]
fn run_python_sync(
    python_exe: &str,
    script_path: &std::path::Path,
    working_dir: &std::path::Path,
) -> Result<std::process::Output, String> {
    use std::process::Command;

    Command::new(python_exe)
        .arg(script_path)
        .current_dir(working_dir)
        .output()
        .map_err(|e| format!("Failed to execute python: {}", e))
}

/// Async wrapper that runs `run_python_sync` in a blocking thread with timeout.
async fn execute_python_sync_with_timeout(
    python_exe: &str,
    script_path: &std::path::Path,
    working_dir: &std::path::Path,
    timeout_secs: u64,
) -> Result<std::process::Output, ToolError> {
    let python_exe = python_exe.to_string();
    let script_path = script_path.to_path_buf();
    let working_dir = working_dir.to_path_buf();

    let blocking_task = tokio::task::spawn_blocking(move || {
        run_python_sync(&python_exe, &script_path, &working_dir)
    });

    tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), blocking_task)
        .await
        .map_err(|_| {
            ToolError::ExecutionError(format!(
                "Python execution timed out after {} seconds",
                timeout_secs
            ))
        })?
        .map_err(|e| ToolError::ExecutionError(format!("Blocking task panicked: {}", e)))?
        .map_err(|e| ToolError::ExecutionError(e))
}

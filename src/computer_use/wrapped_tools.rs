use crate::agent::builtin_tools::validate_file_path;
use crate::agent::tool_executor::{Tool, ToolError};
use crate::computer_use::tools::ComputerUseContext;
use crate::types::{ParameterType, ToolDefinition};
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

/// Wrapped file read tool with permission checking
pub struct WrappedReadFileTool {
    context: Arc<ComputerUseContext>,
}

impl WrappedReadFileTool {
    pub fn new(context: Arc<ComputerUseContext>) -> Self {
        Self { context }
    }
}

#[async_trait]
impl Tool for WrappedReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("read_file")
            .description("Read a file from the workspace or allowed paths. Relative paths are resolved relative to the session workspace.")
            .parameter_with_description(
                "path",
                ParameterType::String,
                true,
                Some("The path to the file to read (relative or absolute based on permissions)"),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let path_str = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;

        validate_file_path(path_str)?;

        let path = PathBuf::from(path_str);

        // Check permissions
        self.context
            .permission_checker
            .can_read_path(&self.context.user_id, &self.context.session_id, &path)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        // Resolve path
        let resolved_path = if path.is_absolute() {
            path
        } else {
            self.context
                .workspace_manager
                .resolve_path(&self.context.session_id, &path)
        };

        // Ensure workspace exists
        self.context
            .workspace_manager
            .create_workspace(&self.context.session_id)
            .await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        // Read file
        let content = fs::read_to_string(&resolved_path)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?;

        Ok(content)
    }
}

/// Wrapped file write tool with permission checking
pub struct WrappedWriteFileTool {
    context: Arc<ComputerUseContext>,
}

impl WrappedWriteFileTool {
    pub fn new(context: Arc<ComputerUseContext>) -> Self {
        Self { context }
    }
}

#[async_trait]
impl Tool for WrappedWriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("write_file")
            .description("Write content to a file in the workspace or allowed paths. Relative paths are resolved relative to the session workspace.")
            .parameter_with_description(
                "path",
                ParameterType::String,
                true,
                Some("The path to write the file to"),
            )
            .parameter_with_description(
                "content",
                ParameterType::String,
                true,
                Some("The content to write to the file"),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let path_str = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;

        validate_file_path(path_str)?;

        let content = parsed["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' parameter".into()))?;

        let path = PathBuf::from(path_str);

        // Check permissions
        self.context
            .permission_checker
            .can_write_path(&self.context.user_id, &self.context.session_id, &path)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        // Resolve path
        let resolved_path = if path.is_absolute() {
            path
        } else {
            self.context
                .workspace_manager
                .resolve_path(&self.context.session_id, &path)
        };

        // Ensure workspace and parent directories exist
        self.context
            .workspace_manager
            .create_workspace(&self.context.session_id)
            .await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        if let Some(parent) = resolved_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                ToolError::ExecutionError(format!("Failed to create directories: {}", e))
            })?;
        }

        // Write file
        fs::write(&resolved_path, content)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

        Ok(format!("Successfully wrote to {}", resolved_path.display()))
    }
}

/// Wrapped list directory tool with permission checking
pub struct WrappedListDirectoryTool {
    context: Arc<ComputerUseContext>,
}

impl WrappedListDirectoryTool {
    pub fn new(context: Arc<ComputerUseContext>) -> Self {
        Self { context }
    }
}

#[async_trait]
impl Tool for WrappedListDirectoryTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("list_directory")
            .description("List contents of a directory in the workspace or allowed paths. Relative paths are resolved relative to the session workspace.")
            .parameter_with_description(
                "path",
                ParameterType::String,
                false,
                Some("The directory path to list (defaults to workspace root if not specified)"),
            )
            .parameter_with_description(
                "recursive",
                ParameterType::Boolean,
                false,
                Some("Whether to list recursively"),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let path_str = parsed["path"].as_str().unwrap_or(".");
        let recursive = parsed["recursive"].as_bool().unwrap_or(false);

        validate_file_path(path_str)?;

        let path = PathBuf::from(path_str);

        // Check permissions
        self.context
            .permission_checker
            .can_read_path(&self.context.user_id, &self.context.session_id, &path)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        // Resolve path
        let resolved_path = if path.is_absolute() {
            path
        } else {
            self.context
                .workspace_manager
                .resolve_path(&self.context.session_id, &path)
        };

        // Ensure workspace exists
        self.context
            .workspace_manager
            .create_workspace(&self.context.session_id)
            .await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        // List directory
        let mut result = String::new();
        list_directory_recursive(&resolved_path, &mut result, recursive, 0)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(result)
    }
}

fn list_directory_recursive(
    path: &Path,
    result: &mut String,
    recursive: bool,
    depth: usize,
) -> std::io::Result<()> {
    let entries = std::fs::read_dir(path)?;
    let indent = "  ".repeat(depth);

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            result.push_str(&format!("{}📁 {}\n", indent, name));
            if recursive {
                list_directory_recursive(&entry.path(), result, recursive, depth + 1)?;
            }
        } else {
            result.push_str(&format!("{}📄 {}\n", indent, name));
        }
    }

    Ok(())
}

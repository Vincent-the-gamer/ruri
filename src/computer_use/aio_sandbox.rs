use crate::agent::tool_executor::{Tool, ToolError};
use crate::types::{ParameterType, ToolDefinition};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

/// AIO Sandbox HTTP client
#[derive(Clone)]
pub struct AioSandboxClient {
    endpoint: String,
    http_client: Client,
}

impl AioSandboxClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Execute a shell command in the sandbox
    pub async fn exec_command(&self, command: &str) -> Result<AioShellResult, AioSandboxError> {
        let url = format!("{}/v1/shell/exec", self.endpoint);
        let body = serde_json::json!({"command": command});

        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AioSandboxError::ConnectionError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AioSandboxError::ApiError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let result: AioApiResponse<AioShellData> = resp
            .json()
            .await
            .map_err(|e| AioSandboxError::ParseError(e.to_string()))?;

        Ok(AioShellResult {
            output: result.data.output,
            exit_code: result.data.exit_code,
        })
    }

    /// Read a file from the sandbox
    pub async fn read_file(&self, path: &str) -> Result<String, AioSandboxError> {
        let url = format!("{}/v1/file/read", self.endpoint);

        let resp = self
            .http_client
            .get(&url)
            .query(&[("file", path)])
            .send()
            .await
            .map_err(|e| AioSandboxError::ConnectionError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AioSandboxError::ApiError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let result: AioApiResponse<AioFileReadData> = resp
            .json()
            .await
            .map_err(|e| AioSandboxError::ParseError(e.to_string()))?;

        Ok(result.data.content)
    }

    /// Write a file to the sandbox
    pub async fn write_file(&self, path: &str, content: &str) -> Result<(), AioSandboxError> {
        let url = format!("{}/v1/file/write", self.endpoint);
        let body = serde_json::json!({"file": path, "content": content});

        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AioSandboxError::ConnectionError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AioSandboxError::ApiError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        Ok(())
    }

    /// List directory contents in the sandbox
    pub async fn list_directory(
        &self,
        path: &str,
    ) -> Result<Vec<AioFileEntry>, AioSandboxError> {
        let url = format!("{}/v1/file/list", self.endpoint);

        let resp = self
            .http_client
            .get(&url)
            .query(&[("path", path)])
            .send()
            .await
            .map_err(|e| AioSandboxError::ConnectionError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AioSandboxError::ApiError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let result: AioApiResponse<AioFileListData> = resp
            .json()
            .await
            .map_err(|e| AioSandboxError::ParseError(e.to_string()))?;

        Ok(result.data.entries)
    }
}

// API response types
#[derive(Debug, Deserialize)]
struct AioApiResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct AioShellData {
    output: String,
    exit_code: i32,
}

#[derive(Debug, Deserialize)]
struct AioFileReadData {
    content: String,
}

#[derive(Debug, Deserialize)]
struct AioFileListData {
    entries: Vec<AioFileEntry>,
}

#[derive(Debug, Deserialize)]
pub struct AioFileEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String,
}

pub struct AioShellResult {
    pub output: String,
    pub exit_code: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum AioSandboxError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

// ─── Sandbox Tools ────────────────────────────────────────────────

/// Shell tool that executes commands in AIO Sandbox
pub struct AioSandboxShellTool {
    client: Arc<AioSandboxClient>,
}

impl AioSandboxShellTool {
    pub fn new(client: Arc<AioSandboxClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for AioSandboxShellTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("shell")
            .description(
                "Execute a shell command in the AIO Sandbox environment. \
                 The command runs inside an isolated Docker container.",
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
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let command = parsed["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".into()))?;

        let _timeout_secs = parsed["timeout"].as_u64().unwrap_or(30);

        info!("Executing shell command in AIO Sandbox: {}", command);

        let result = self.client.exec_command(command).await.map_err(|e| {
            ToolError::ExecutionError(format!("AIO Sandbox shell error: {}", e))
        })?;

        if result.exit_code == 0 {
            Ok(result.output)
        } else {
            Err(ToolError::ExecutionError(format!(
                "Command failed with exit code {}: {}",
                result.exit_code, result.output
            )))
        }
    }
}

/// Read file tool that reads from AIO Sandbox
pub struct AioSandboxReadFileTool {
    client: Arc<AioSandboxClient>,
}

impl AioSandboxReadFileTool {
    pub fn new(client: Arc<AioSandboxClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for AioSandboxReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("read_file")
            .description("Read a file from the AIO Sandbox environment.")
            .parameter_with_description(
                "path",
                ParameterType::String,
                true,
                Some("The path to the file to read in the sandbox"),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let path = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;

        info!("Reading file from AIO Sandbox: {}", path);

        self.client.read_file(path).await.map_err(|e| {
            ToolError::ExecutionError(format!("AIO Sandbox read file error: {}", e))
        })
    }
}

/// Write file tool that writes to AIO Sandbox
pub struct AioSandboxWriteFileTool {
    client: Arc<AioSandboxClient>,
}

impl AioSandboxWriteFileTool {
    pub fn new(client: Arc<AioSandboxClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for AioSandboxWriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("write_file")
            .description("Write content to a file in the AIO Sandbox environment.")
            .parameter_with_description(
                "path",
                ParameterType::String,
                true,
                Some("The path to write the file to in the sandbox"),
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

        let path = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;

        let content = parsed["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' parameter".into()))?;

        info!("Writing file to AIO Sandbox: {}", path);

        self.client.write_file(path, content).await.map_err(|e| {
            ToolError::ExecutionError(format!("AIO Sandbox write file error: {}", e))
        })?;

        Ok(format!("Successfully wrote to {}", path))
    }
}

/// List directory tool that lists from AIO Sandbox
pub struct AioSandboxListDirectoryTool {
    client: Arc<AioSandboxClient>,
}

impl AioSandboxListDirectoryTool {
    pub fn new(client: Arc<AioSandboxClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for AioSandboxListDirectoryTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("list_directory")
            .description("List contents of a directory in the AIO Sandbox environment.")
            .parameter_with_description(
                "path",
                ParameterType::String,
                false,
                Some("The directory path to list (defaults to /home/gem if not specified)"),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let path = parsed["path"].as_str().unwrap_or("/home/gem");

        info!("Listing directory in AIO Sandbox: {}", path);

        let entries = self.client.list_directory(path).await.map_err(|e| {
            ToolError::ExecutionError(format!("AIO Sandbox list directory error: {}", e))
        })?;

        let mut result = String::new();
        for entry in entries {
            let icon = if entry.entry_type == "dir" {
                "📁"
            } else {
                "📄"
            };
            result.push_str(&format!("{} {}\n", icon, entry.name));
        }

        if result.is_empty() {
            result = "(empty directory)\n".to_string();
        }

        Ok(result)
    }
}

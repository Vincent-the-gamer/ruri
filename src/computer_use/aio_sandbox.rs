use crate::agent::tool_executor::{Tool, ToolError};
use crate::types::{ParameterType, ToolDefinition};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn};

const MAX_RETRIES: u32 = 3;

/// Check if an HTTP status code represents a transient server error that may resolve on retry.
fn is_transient_error(status: StatusCode) -> bool {
    matches!(status.as_u16(), 502 | 503 | 504)
}

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

    /// Build a descriptive API error message with endpoint context for transient errors.
    fn make_api_error(status: StatusCode, text: &str, endpoint: &str) -> AioSandboxError {
        let message = if is_transient_error(status) {
            format!(
                "The sandbox server is temporarily unavailable (HTTP {}). Please check if the sandbox container is running and the endpoint '{}' is reachable. Response: {}",
                status, endpoint, text
            )
        } else {
            format!(
                "API request to endpoint '{}' failed with HTTP {}: {}",
                endpoint, status, text
            )
        };
        AioSandboxError::ApiError(message)
    }

    /// Execute an async request with exponential backoff retry on transient errors.
    ///
    /// Only retries on 5xx transient errors (502, 503, 504). Client errors (4xx)
    /// and parse errors are returned immediately without retry.
    async fn retry_request<F, Fut, T>(&self, f: F) -> Result<T, AioSandboxError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, AioSandboxError>>,
    {
        let mut last_err = None;
        for attempt in 0..=MAX_RETRIES {
            match f().await {
                Ok(val) => {
                    if attempt > 0 {
                        info!(
                            "AIO Sandbox request succeeded on attempt {}/{}",
                            attempt + 1,
                            MAX_RETRIES + 1
                        );
                    }
                    return Ok(val);
                }
                Err(ref e) => {
                    let should_retry =
                        attempt < MAX_RETRIES && matches!(e, AioSandboxError::ApiError(_));

                    if !should_retry {
                        return Err(e.clone());
                    }

                    // Only retry if it's a transient error; non-transient ApiErrors (4xx) are not retried
                    let is_transient = match e {
                        AioSandboxError::ApiError(msg) => {
                            // Check for 502/503/504 in the message as a heuristic
                            msg.contains("502")
                                || msg.contains("503")
                                || msg.contains("504")
                                || msg.contains("temporarily unavailable")
                        }
                        _ => false,
                    };

                    if !is_transient {
                        return Err(e.clone());
                    }

                    let delay_secs = 1u64 << attempt; // 1s, 2s, 4s
                    warn!(
                        "AIO Sandbox request failed (attempt {}/{}): {}. Retrying in {}s...",
                        attempt + 1,
                        MAX_RETRIES + 1,
                        e,
                        delay_secs
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                    last_err = Some(e.clone());
                }
            }
        }
        Err(last_err.unwrap())
    }

    /// Execute a shell command in the sandbox
    pub async fn exec_command(&self, command: &str) -> Result<AioShellResult, AioSandboxError> {
        self.retry_request(|| async {
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
                return Err(Self::make_api_error(status, &text, &self.endpoint));
            }

            let result: AioApiResponse<AioShellData> = resp
                .json()
                .await
                .map_err(|e| AioSandboxError::ParseError(e.to_string()))?;

            Ok(AioShellResult {
                output: result.data.output,
                exit_code: result.data.exit_code,
                status: result.data.status,
            })
        })
        .await
    }

    /// Read a file from the sandbox
    pub async fn read_file(&self, path: &str) -> Result<String, AioSandboxError> {
        self.retry_request(|| async {
            let url = format!("{}/v1/file/read", self.endpoint);
            let body = serde_json::json!({"file": path});

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
                return Err(Self::make_api_error(status, &text, &self.endpoint));
            }

            let result: AioApiResponse<AioFileReadData> = resp
                .json()
                .await
                .map_err(|e| AioSandboxError::ParseError(e.to_string()))?;

            Ok(result.data.content)
        })
        .await
    }

    /// Write a file to the sandbox
    pub async fn write_file(&self, path: &str, content: &str) -> Result<(), AioSandboxError> {
        self.retry_request(|| {
            let url = format!("{}/v1/file/write", self.endpoint);
            let body = serde_json::json!({"file": path, "content": content});
            async move {
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
                    return Err(Self::make_api_error(status, &text, &self.endpoint));
                }

                Ok(())
            }
        })
        .await
    }

    /// List directory contents in the sandbox
    pub async fn list_directory(&self, path: &str) -> Result<AioFileListResult, AioSandboxError> {
        self.retry_request(|| async {
            let url = format!("{}/v1/file/list", self.endpoint);
            let body = serde_json::json!({"path": path});

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
                return Err(Self::make_api_error(status, &text, &self.endpoint));
            }

            let result: AioApiResponse<AioFileListResult> = resp
                .json()
                .await
                .map_err(|e| AioSandboxError::ParseError(e.to_string()))?;

            Ok(result.data)
        })
        .await
    }

    /// Create a directory (and all parent directories) in the sandbox
    pub async fn create_directory(&self, path: &str) -> Result<(), AioSandboxError> {
        let cmd = format!("mkdir -p {}", path);
        let result = self.exec_command(&cmd).await?;
        let exit_code = result.exit_code.unwrap_or(-1);
        let output = result.output.unwrap_or_default();
        if exit_code != 0 {
            return Err(AioSandboxError::ApiError(format!(
                "Failed to create directory '{}': {}",
                path, output
            )));
        }
        Ok(())
    }

    /// Replace text in a file using the native /v1/file/replace API
    pub async fn replace_in_file(
        &self,
        file: &str,
        old_str: &str,
        new_str: &str,
    ) -> Result<String, AioSandboxError> {
        self.retry_request(|| {
            let url = format!("{}/v1/file/replace", self.endpoint);
            let body = serde_json::json!({"file": file, "old_str": old_str, "new_str": new_str});
            async move {
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
                    return Err(Self::make_api_error(status, &text, &self.endpoint));
                }

                let result: AioApiResponse<AioFileReplaceData> = resp
                    .json()
                    .await
                    .map_err(|e| AioSandboxError::ParseError(e.to_string()))?;

                Ok(format!(
                    "Successfully edited {}: replaced {} occurrence(s)",
                    result.data.file, result.data.replaced_count
                ))
            }
        })
        .await
    }

    /// Find files by name pattern
    pub async fn find_files(&self, path: &str, glob: &str) -> Result<Vec<String>, AioSandboxError> {
        self.retry_request(|| {
            let url = format!("{}/v1/file/find", self.endpoint);
            let body = serde_json::json!({"path": path, "glob": glob});
            async move {
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
                    return Err(Self::make_api_error(status, &text, &self.endpoint));
                }

                let result: AioApiResponse<AioFileFindData> = resp
                    .json()
                    .await
                    .map_err(|e| AioSandboxError::ParseError(e.to_string()))?;

                Ok(result.data.files)
            }
        })
        .await
    }

    /// Search in file content using regex
    pub async fn search_in_file(
        &self,
        file: &str,
        regex: &str,
    ) -> Result<AioFileSearchResult, AioSandboxError> {
        self.retry_request(|| {
            let url = format!("{}/v1/file/search", self.endpoint);
            let body = serde_json::json!({"file": file, "regex": regex});
            async move {
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
                    return Err(Self::make_api_error(status, &text, &self.endpoint));
                }

                let result: AioApiResponse<AioFileSearchData> = resp
                    .json()
                    .await
                    .map_err(|e| AioSandboxError::ParseError(e.to_string()))?;

                Ok(AioFileSearchResult {
                    file: result.data.file,
                    matches: result.data.matches,
                    line_numbers: result.data.line_numbers,
                })
            }
        })
        .await
    }
}

// API response types
#[derive(Debug, Deserialize)]
struct AioApiResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AioShellData {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    status: Option<String>,
    output: Option<String>,
    exit_code: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct AioFileReadData {
    content: String,
}

#[derive(Debug, Deserialize)]
struct AioFileReplaceData {
    file: String,
    #[serde(default)]
    replaced_count: i64,
}

#[derive(Debug, Deserialize)]
struct AioFileFindData {
    #[serde(default)]
    files: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AioFileSearchData {
    file: String,
    #[serde(default)]
    matches: Vec<String>,
    #[serde(default)]
    line_numbers: Vec<i64>,
}

pub struct AioFileSearchResult {
    #[allow(dead_code)]
    pub file: String,
    pub matches: Vec<String>,
    pub line_numbers: Vec<i64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AioFileListResult {
    pub path: String,
    #[serde(default)]
    pub files: Vec<AioFileInfo>,
    #[serde(default)]
    pub total_count: i64,
    #[serde(default)]
    pub directory_count: i64,
    #[serde(default)]
    pub file_count: i64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AioFileInfo {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    #[serde(default)]
    pub size: Option<i64>,
    #[serde(default)]
    pub modified_time: Option<String>,
    #[serde(default)]
    pub permissions: Option<String>,
    #[serde(default)]
    pub extension: Option<String>,
}

pub struct AioShellResult {
    pub output: Option<String>,
    pub exit_code: Option<i32>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error)]
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

        let timeout_secs = parsed["timeout"].as_u64();

        info!("Executing shell command in AIO Sandbox: {}", command);

        // Build request body with optional timeout
        let mut body = serde_json::json!({"command": command});
        if let Some(timeout) = timeout_secs {
            body["timeout"] = serde_json::json!(timeout);
        }

        let result =
            self.client.exec_command(command).await.map_err(|e| {
                ToolError::ExecutionError(format!("AIO Sandbox shell error: {}", e))
            })?;

        // Check status first – if not completed, report accordingly
        let status = result.status.as_deref().unwrap_or("unknown");
        let exit_code = result.exit_code.unwrap_or(-1);
        let output = result.output.unwrap_or_default();

        match status {
            "completed" => {
                if exit_code == 0 {
                    Ok(output)
                } else {
                    Err(ToolError::ExecutionError(format!(
                        "Command failed with exit code {}: {}",
                        exit_code, output
                    )))
                }
            }
            "running" => Err(ToolError::ExecutionError(format!(
                "Command is still running (session may still be active). Partial output: {}",
                output
            ))),
            other => Err(ToolError::ExecutionError(format!(
                "Command ended with status '{}': {}",
                other, output
            ))),
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

        self.client
            .read_file(path)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("AIO Sandbox read file error: {}", e)))
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

        let list_result = self.client.list_directory(path).await.map_err(|e| {
            ToolError::ExecutionError(format!("AIO Sandbox list directory error: {}", e))
        })?;

        let mut result = String::new();
        for entry in &list_result.files {
            let icon = if entry.is_directory { "📁" } else { "📄" };
            let size_info = match entry.size {
                Some(s) => format!(" ({} bytes)", s),
                None => String::new(),
            };
            result.push_str(&format!("{} {}{}\n", icon, entry.name, size_info));
        }

        if result.is_empty() {
            result = "(empty directory)\n".to_string();
        } else {
            result.push_str(&format!(
                "\nTotal: {} items ({} directories, {} files)",
                list_result.total_count, list_result.directory_count, list_result.file_count
            ));
        }

        Ok(result)
    }
}

/// Create file tool that creates a new file in AIO Sandbox
pub struct AioSandboxCreateFileTool {
    client: Arc<AioSandboxClient>,
}

impl AioSandboxCreateFileTool {
    pub fn new(client: Arc<AioSandboxClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for AioSandboxCreateFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("create_file")
            .description(
                "Create a new file with the given content in the AIO Sandbox environment. \
                 This will fail if the file already exists — use write_file if you want to \
                 overwrite an existing file. Parent directories will be created automatically.",
            )
            .parameter_with_description(
                "path",
                ParameterType::String,
                true,
                Some("The path to create the file at in the sandbox"),
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

        info!("Creating file in AIO Sandbox: {}", path);

        // First check if file already exists by trying to read it
        match self.client.read_file(path).await {
            Ok(_) => {
                return Err(ToolError::ExecutionError(format!(
                    "File '{}' already exists. Use write_file to overwrite.",
                    path
                )));
            }
            Err(_) => {
                // File doesn't exist, proceed to create it
            }
        }

        // Create parent directories if needed, then write the file
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                self.client
                    .create_directory(&parent.display().to_string())
                    .await
                    .map_err(|e| {
                        ToolError::ExecutionError(format!(
                            "Failed to create parent directory for file '{}': {}",
                            path, e
                        ))
                    })?;
            }
        }

        self.client.write_file(path, content).await.map_err(|e| {
            ToolError::ExecutionError(format!("AIO Sandbox create file error: {}", e))
        })?;

        Ok(format!("Successfully created file {}", path))
    }
}

/// Edit file tool that edits a file in AIO Sandbox
pub struct AioSandboxEditFileTool {
    client: Arc<AioSandboxClient>,
}

impl AioSandboxEditFileTool {
    pub fn new(client: Arc<AioSandboxClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for AioSandboxEditFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("edit_file")
            .description(
                "Make a targeted replacement in a file in the AIO Sandbox environment. \
                 Finds the exact `old_text` in the file and replaces it with `new_text`. \
                 The `old_text` must match exactly (including whitespace and indentation). \
                 Fails if `old_text` is not found or found multiple times. \
                 Use this for precise surgical edits instead of rewriting entire files.",
            )
            .parameter_with_description(
                "path",
                ParameterType::String,
                true,
                Some("The path to the file to edit in the sandbox"),
            )
            .parameter_with_description(
                "old_text",
                ParameterType::String,
                true,
                Some("The exact text to find and replace"),
            )
            .parameter_with_description(
                "new_text",
                ParameterType::String,
                true,
                Some("The text to replace `old_text` with"),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let path = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;

        let old_text = parsed["old_text"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'old_text' parameter".into()))?;

        let new_text = parsed["new_text"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'new_text' parameter".into()))?;

        info!("Editing file in AIO Sandbox: {}", path);

        // Use the native /v1/file/replace API endpoint
        self.client
            .replace_in_file(path, old_text, new_text)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("AIO Sandbox edit file error: {}", e)))
    }
}

/// Find files tool that searches by name pattern in AIO Sandbox
pub struct AioSandboxFindFilesTool {
    client: Arc<AioSandboxClient>,
}

impl AioSandboxFindFilesTool {
    pub fn new(client: Arc<AioSandboxClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for AioSandboxFindFilesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("find_files")
            .description(
                "Find files by name pattern in the AIO Sandbox environment. \
                 Supports glob syntax like '**/*.py' or 'src/**/*.rs'.",
            )
            .parameter_with_description(
                "path",
                ParameterType::String,
                true,
                Some("The directory path to search in"),
            )
            .parameter_with_description(
                "glob",
                ParameterType::String,
                true,
                Some("The filename pattern to match (glob syntax, e.g. '**/*.py')"),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let path = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;

        let glob = parsed["glob"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'glob' parameter".into()))?;

        info!("Finding files in AIO Sandbox: {} pattern: {}", path, glob);

        let files = self.client.find_files(path, glob).await.map_err(|e| {
            ToolError::ExecutionError(format!("AIO Sandbox find files error: {}", e))
        })?;

        if files.is_empty() {
            Ok(format!("No files matching '{}' found in {}", glob, path))
        } else {
            let mut result = format!(
                "Found {} file(s) matching '{}' in {}:\n",
                files.len(),
                glob,
                path
            );
            for file in &files {
                result.push_str(&format!("  {}\n", file));
            }
            Ok(result)
        }
    }
}

/// Search in file tool that searches file content using regex in AIO Sandbox
pub struct AioSandboxSearchInFileTool {
    client: Arc<AioSandboxClient>,
}

impl AioSandboxSearchInFileTool {
    pub fn new(client: Arc<AioSandboxClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for AioSandboxSearchInFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("search_in_file")
            .description(
                "Search for a regex pattern in a file in the AIO Sandbox environment. \
                 Returns matched content and line numbers.",
            )
            .parameter_with_description(
                "path",
                ParameterType::String,
                true,
                Some("The path to the file to search in the sandbox"),
            )
            .parameter_with_description(
                "regex",
                ParameterType::String,
                true,
                Some("The regular expression pattern to search for"),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed: Value =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let path = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;

        let regex = parsed["regex"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'regex' parameter".into()))?;

        info!(
            "Searching in file in AIO Sandbox: {} pattern: {}",
            path, regex
        );

        let search_result = self.client.search_in_file(path, regex).await.map_err(|e| {
            ToolError::ExecutionError(format!("AIO Sandbox search in file error: {}", e))
        })?;

        if search_result.matches.is_empty() {
            Ok(format!("No matches found for '{}' in {}", regex, path))
        } else {
            let mut result = format!(
                "Found {} match(es) for '{}' in {}:\n",
                search_result.matches.len(),
                regex,
                path
            );
            for (_i, (line_num, match_text)) in search_result
                .line_numbers
                .iter()
                .zip(search_result.matches.iter())
                .enumerate()
            {
                result.push_str(&format!("  L{}: {}\n", line_num, match_text));
            }
            Ok(result)
        }
    }
}

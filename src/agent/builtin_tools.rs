//! Built-in tools for file system operations and code editing.

use crate::agent::tool_executor::{Tool, ToolError};
use crate::types::{ParameterType, ToolDefinition};
use async_trait::async_trait;
use serde_json::Value;
use std::ffi::OsStr;
use std::path::Path;

#[cfg(target_os = "windows")]
use base64;

// ─── Process Group Guard ────────────────────────────────────────────

/// A wrapper that ensures process trees are killed on drop.
///
/// On Unix: uses process groups (SIGTERM/SIGKILL to the PGID).
/// On Windows: uses Job Objects to track and terminate the entire process tree.
///
/// Used by shell tools to prevent orphan processes from holding ports
/// when the server is stopped or the tool is cancelled.
pub struct ProcessGroupGuard {
    #[cfg(unix)]
    pgid: i32,
    /// Job object handle on Windows. null means no job was assigned
    /// (guard is a no-op).
    #[cfg(windows)]
    job_handle: *mut std::ffi::c_void,
}

// SAFETY: The `job_handle` field on Windows is a kernel HANDLE (job object),
// which is safe to send and share across threads. On Unix, `pgid` is `i32`
// which already implements Send + Sync.
unsafe impl Send for ProcessGroupGuard {}
unsafe impl Sync for ProcessGroupGuard {}

impl ProcessGroupGuard {
    #[cfg(unix)]
    pub fn new(pid: u32) -> Self {
        Self { pgid: pid as i32 }
    }

    #[cfg(windows)]
    pub fn new(pid: u32) -> Self {
        // On Windows, create a Job Object that kills all member processes
        // when the handle is closed (JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE).
        // All children spawned by the assigned process automatically join
        // the job, giving us full process-tree cleanup.
        unsafe {
            use windows_sys::Win32::Foundation::CloseHandle;
            use windows_sys::Win32::System::JobObjects::{
                AssignProcessToJobObject, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            };
            use windows_sys::Win32::System::Threading::{
                OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
            };

            // Use raw FFI for CreateJobObjectW because windows-sys may not
            // expose it consistently across all Windows target triples.
            unsafe extern "system" {
                fn CreateJobObjectW(
                    lpjobattributes: *const std::ffi::c_void,
                    lpname: *const std::ffi::c_void,
                ) -> *mut std::ffi::c_void;
            }

            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Self {
                    job_handle: std::ptr::null_mut(),
                };
            }

            // Set KILL_ON_JOB_CLOSE: all processes in this job are
            // terminated when the last job handle is closed.
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let size = std::mem::size_of_val(&info);
            let ret = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const std::ffi::c_void,
                size as u32,
            );
            if ret == 0 {
                CloseHandle(job);
                return Self {
                    job_handle: std::ptr::null_mut(),
                };
            }

            // Open the child process to assign it to the job.
            let handle = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
            if handle.is_null() {
                CloseHandle(job);
                return Self {
                    job_handle: std::ptr::null_mut(),
                };
            }

            let ret = AssignProcessToJobObject(job, handle);
            CloseHandle(handle); // no longer needed
            if ret == 0 {
                CloseHandle(job);
                return Self {
                    job_handle: std::ptr::null_mut(),
                };
            }

            Self { job_handle: job }
        }
    }

    #[cfg(not(any(unix, windows)))]
    pub fn new(_pid: u32) -> Self {
        Self {}
    }

    #[cfg(unix)]
    pub fn kill(&self) {
        let pgid = -self.pgid;
        unsafe {
            libc::kill(pgid, libc::SIGTERM);
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        unsafe {
            libc::kill(pgid, libc::SIGKILL);
        }
    }

    #[cfg(windows)]
    pub fn kill(&self) {
        if !self.job_handle.is_null() {
            unsafe {
                use windows_sys::Win32::System::JobObjects::TerminateJobObject;
                TerminateJobObject(self.job_handle, 1);
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    pub fn kill(&self) {}

    /// Disarm the guard so that `Drop` does not kill the process tree.
    ///
    /// Call this when the process has exited normally to avoid a
    /// redundant kill attempt during cleanup.
    pub fn disarm(&mut self) {
        #[cfg(unix)]
        {
            self.pgid = 0;
        }
        #[cfg(windows)]
        {
            self.job_handle = std::ptr::null_mut();
        }
    }
}

impl Drop for ProcessGroupGuard {
    #[cfg(unix)]
    fn drop(&mut self) {
        if self.pgid > 0 {
            self.kill();
        }
    }

    #[cfg(windows)]
    fn drop(&mut self) {
        if !self.job_handle.is_null() {
            // TerminateJobObject kills all processes in the job.
            // Closing the handle (with KILL_ON_JOB_CLOSE set) also
            // terminates them, but we call TerminateJobObject explicitly
            // for immediate effect before the handle is closed.
            unsafe {
                use windows_sys::Win32::Foundation::CloseHandle;
                use windows_sys::Win32::System::JobObjects::TerminateJobObject;
                TerminateJobObject(self.job_handle, 1);
                CloseHandle(self.job_handle);
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    fn drop(&mut self) {}
}

// ─── Helper ────────────────────────────────────────────────────────

/// Parse a JSON args string into a serde_json::Value.
fn parse_args(args: &str) -> Result<Value, ToolError> {
    serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))
}

/// Validate that the given path is not a Unix device path (e.g. `/dev/stdin`,
/// `/dev/null`). On Windows these get resolved to `C:\dev\stdin` etc., which
/// is almost certainly not what the caller intended.
pub fn validate_file_path(path: &str) -> Result<(), ToolError> {
    // Normalise separators so the check also covers `\dev\stdin` on Windows.
    let normalized = path.replace('\\', "/");
    if normalized.starts_with("/dev/") {
        return Err(ToolError::InvalidArguments(format!(
            "Path '{}' is a Unix device path which does not exist on Windows. \
             Please provide a valid file path instead.",
            path
        )));
    }
    Ok(())
}

// ─── ReadFileTool ──────────────────────────────────────────────────

/// Read the contents of a file at the given path.
pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("read_file")
            .description(
                "Read the contents of a file at the given path. \
                 Returns the file content as a string. \
                 Use start_line and end_line to read a specific range (1-based, inclusive).",
            )
            .parameter("path", ParameterType::String, true)
            .parameter_with_description(
                "start_line",
                ParameterType::Integer,
                false,
                Some("The line number to start reading from (1-based index, inclusive)."),
            )
            .parameter_with_description(
                "end_line",
                ParameterType::Integer,
                false,
                Some("The line number to end reading at (1-based index, inclusive)."),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed = parse_args(args)?;
        let path = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;

        validate_file_path(path)?;

        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to read file '{}': {}", path, e))
        })?;

        let start_line = parsed["start_line"].as_u64();
        let end_line = parsed["end_line"].as_u64();

        let (selected_content, header) = match (start_line, end_line) {
            (Some(start), Some(end)) => {
                let lines: Vec<&str> = content.lines().collect();
                let total = lines.len();
                let start = (start as usize).saturating_sub(1).min(total);
                let end = (end as usize).min(total);
                if start >= end {
                    return Ok(String::new());
                }
                let selected: Vec<&str> = lines[start..end].to_vec();
                let header = format!("[read_file: {path} (lines {}-{})]", start + 1, end);
                (selected.join("\n"), header)
            }
            (Some(start), None) => {
                let lines: Vec<&str> = content.lines().collect();
                let total = lines.len();
                let start = (start as usize).saturating_sub(1).min(total);
                let selected: Vec<&str> = lines[start..].to_vec();
                let header = format!("[read_file: {path} (lines {}-{})]", start + 1, total);
                (selected.join("\n"), header)
            }
            (None, Some(end)) => {
                let lines: Vec<&str> = content.lines().collect();
                let end = (end as usize).min(lines.len());
                let selected: Vec<&str> = lines[..end].to_vec();
                let header = format!("[read_file: {path} (lines 1-{end})]");
                (selected.join("\n"), header)
            }
            (None, None) => {
                let header = format!("[read_file: {path}]");
                (content, header)
            }
        };

        Ok(format!("{header}\n\n{selected_content}"))
    }
}

// ─── WriteFileTool ─────────────────────────────────────────────────

/// Write content to a file, creating it if it doesn't exist and overwriting if it does.
pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("write_file")
            .description(
                "Write content to a file. Creates the file if it does not exist, \
                 and overwrites the entire file content if it does. \
                 Parent directories will be created automatically if they don't exist.",
            )
            .parameter("path", ParameterType::String, true)
            .parameter("content", ParameterType::String, true)
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed = parse_args(args)?;
        let path = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;
        let content = parsed["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' parameter".into()))?;

        validate_file_path(path)?;

        // Create parent directories if needed
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    ToolError::ExecutionError(format!(
                        "Failed to create parent directories for '{}': {}",
                        path, e
                    ))
                })?;
            }
        }

        tokio::fs::write(path, content).await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to write file '{}': {}", path, e))
        })?;

        Ok(format!(
            "Successfully wrote {} bytes to {}",
            content.len(),
            path
        ))
    }
}

// ─── CreateFileTool ────────────────────────────────────────────────

/// Create a new file with the given content. Fails if the file already exists.
pub struct CreateFileTool;

#[async_trait]
impl Tool for CreateFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("create_file")
            .description(
                "Create a new file with the given content. \
                 This will fail if the file already exists — use write_file if you want to \
                 overwrite an existing file. Parent directories will be created automatically.",
            )
            .parameter("path", ParameterType::String, true)
            .parameter("content", ParameterType::String, true)
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed = parse_args(args)?;
        let path = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;
        let content = parsed["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' parameter".into()))?;

        validate_file_path(path)?;

        // Check if file already exists
        if Path::new(path).exists() {
            return Err(ToolError::ExecutionError(format!(
                "File '{}' already exists. Use write_file to overwrite.",
                path
            )));
        }

        // Create parent directories if needed
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    ToolError::ExecutionError(format!(
                        "Failed to create parent directories for '{}': {}",
                        path, e
                    ))
                })?;
            }
        }

        tokio::fs::write(path, content).await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to create file '{}': {}", path, e))
        })?;

        Ok(format!("Successfully created file {}", path))
    }
}

// ─── EditFileTool ──────────────────────────────────────────────────

/// Make a targeted replacement in a file.
pub struct EditFileTool;

#[async_trait]
impl Tool for EditFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("edit_file")
            .description(
                "Make a targeted replacement in a file. \
                 Finds the exact `old_text` in the file and replaces it with `new_text`. \
                 The `old_text` must match exactly (including whitespace and indentation). \
                 Fails if `old_text` is not found or found multiple times. \
                 Use this for precise surgical edits instead of rewriting entire files.",
            )
            .parameter("path", ParameterType::String, true)
            .parameter("old_text", ParameterType::String, true)
            .parameter("new_text", ParameterType::String, true)
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed = parse_args(args)?;
        let path = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;
        let old_text = parsed["old_text"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'old_text' parameter".into()))?;
        let new_text = parsed["new_text"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'new_text' parameter".into()))?;

        validate_file_path(path)?;

        // Read the file
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to read file '{}': {}", path, e))
        })?;

        // Count occurrences
        let count = content.matches(old_text).count();
        if count == 0 {
            return Err(ToolError::ExecutionError(format!(
                "old_text not found in file '{}'",
                path
            )));
        }
        if count > 1 {
            return Err(ToolError::ExecutionError(format!(
                "old_text found {} times in file '{}'. Please provide more context to make the match unique.",
                count, path
            )));
        }

        // Replace
        let new_content = content.replacen(old_text, new_text, 1);

        // Write back
        tokio::fs::write(path, &new_content).await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to write file '{}': {}", path, e))
        })?;

        Ok(format!(
            "Successfully edited {}: replaced 1 occurrence ({} chars -> {} chars)",
            path,
            old_text.len(),
            new_text.len()
        ))
    }
}

// ─── DeleteFileTool ─────────────────────────────────────────────────

/// Delete a file or directory at the given path.
pub struct DeleteFileTool;

#[async_trait]
impl Tool for DeleteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("delete_file")
            .description(
                "Delete a file or directory at the given path. \
                 If the path is a directory, it will be deleted recursively \
                 along with all its contents. This action cannot be undone.",
            )
            .parameter("path", ParameterType::String, true)
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed = parse_args(args)?;
        let path = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;

        validate_file_path(path)?;

        let file_path = Path::new(path);
        if !file_path.exists() {
            return Err(ToolError::ExecutionError(format!(
                "Path '{}' does not exist",
                path
            )));
        }

        if file_path.is_dir() {
            tokio::fs::remove_dir_all(file_path).await.map_err(|e| {
                ToolError::ExecutionError(format!("Failed to delete directory '{}': {}", path, e))
            })?;
            Ok(format!("Successfully deleted directory {}", path))
        } else {
            tokio::fs::remove_file(file_path).await.map_err(|e| {
                ToolError::ExecutionError(format!("Failed to delete file '{}': {}", path, e))
            })?;
            Ok(format!("Successfully deleted file {}", path))
        }
    }
}

// ─── ListDirectoryTool ─────────────────────────────────────────────

/// List files and directories at a given path.
pub struct ListDirectoryTool;

#[async_trait]
impl Tool for ListDirectoryTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("list_directory")
            .description(
                "List files and directories at the given path. \
                 Returns the names of entries, with '/' appended to directory names. \
                 If the path does not exist, returns an error.",
            )
            .parameter("path", ParameterType::String, true)
            .parameter_with_description(
                "recursive",
                ParameterType::Boolean,
                false,
                Some("Whether to list recursively (default: false)."),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed = parse_args(args)?;
        let path = parsed["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".into()))?;
        let recursive = parsed["recursive"].as_bool().unwrap_or(false);

        validate_file_path(path)?;

        let dir_path = Path::new(path);
        if !dir_path.exists() {
            return Err(ToolError::ExecutionError(format!(
                "Path '{}' does not exist",
                path
            )));
        }
        if !dir_path.is_dir() {
            return Err(ToolError::ExecutionError(format!(
                "Path '{}' is not a directory",
                path
            )));
        }

        let entries = list_dir_recursive(dir_path, recursive)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        if entries.is_empty() {
            Ok("(empty directory)".to_string())
        } else {
            Ok(entries.join("\n"))
        }
    }
}

/// Recursively list directory entries relative to base_dir.
fn list_dir_recursive(base_dir: &Path, recursive: bool) -> std::io::Result<Vec<String>> {
    let mut result = Vec::new();
    list_dir_recursive_inner(base_dir, base_dir, recursive, &mut result)?;
    Ok(result)
}

fn list_dir_recursive_inner(
    base_dir: &Path,
    current_dir: &Path,
    recursive: bool,
    result: &mut Vec<String>,
) -> std::io::Result<()> {
    let mut entries: Vec<std::fs::DirEntry> =
        std::fs::read_dir(current_dir)?.collect::<Result<_, _>>()?;

    // Sort: directories first, then files; alphabetically within each group
    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in entries {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy().to_string();

        // Skip hidden files/dirs (starting with '.')
        if name.starts_with('.') {
            continue;
        }

        let is_dir = entry.file_type()?.is_dir();
        let relative = current_dir
            .strip_prefix(base_dir)
            .unwrap_or(current_dir)
            .to_path_buf();

        let display_path = if relative.as_os_str().is_empty() {
            if is_dir { format!("{}/", name) } else { name }
        } else {
            let rel_str = relative.to_string_lossy();
            if is_dir {
                format!("{}/{}/", rel_str, name)
            } else {
                format!("{}/{}", rel_str, name)
            }
        };

        result.push(display_path);

        if recursive && is_dir {
            list_dir_recursive_inner(base_dir, &entry.path(), recursive, result)?;
        }
    }

    Ok(())
}

// ─── SearchFilesTool ───────────────────────────────────────────────

/// Search for files by name pattern (glob) or content (regex).
pub struct SearchFilesTool;

#[async_trait]
impl Tool for SearchFilesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("search_files")
            .description(
                "Search for files in a directory tree. Two modes:\n\
                 1. By filename pattern (glob) — set 'pattern' to a glob like '**/*.rs' or '*config*'.\n\
                 2. By file content (regex) — set 'query' to a regex pattern to search within file contents.\n\
                 You must provide either 'pattern' or 'query' (or both). \
                 'path' defaults to '.' (current directory).",
            )
            .parameter_with_description(
                "path",
                ParameterType::String,
                false,
                Some("The root directory to search in (default: '.')."),
            )
            .parameter_with_description(
                "pattern",
                ParameterType::String,
                false,
                Some("A glob pattern to match file names, e.g. '**/*.rs', '*config*', 'src/**/*.ts'."),
            )
            .parameter_with_description(
                "query",
                ParameterType::String,
                false,
                Some("A regex pattern to search within file contents."),
            )
            .parameter_with_description(
                "max_results",
                ParameterType::Integer,
                false,
                Some("Maximum number of results to return (default: 50)."),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed = parse_args(args)?;
        let root = parsed["path"].as_str().unwrap_or(".");
        let pattern = parsed["pattern"].as_str();
        let query = parsed["query"].as_str();
        let max_results = parsed["max_results"].as_u64().unwrap_or(50) as usize;

        if pattern.is_none() && query.is_none() {
            return Err(ToolError::InvalidArguments(
                "Must provide at least one of 'pattern' or 'query'".into(),
            ));
        }

        let glob_pattern = pattern.unwrap_or("**/*");
        let root_path = Path::new(root);

        if !root_path.exists() {
            return Err(ToolError::ExecutionError(format!(
                "Path '{}' does not exist",
                root
            )));
        }

        // Build a regex for content search if query is provided
        let content_regex = if let Some(q) = query {
            Some(regex::Regex::new(q).map_err(|e| {
                ToolError::InvalidArguments(format!("Invalid regex '{}': {}", q, e))
            })?)
        } else {
            None
        };

        let mut results = Vec::new();
        search_files_inner(
            root_path,
            glob_pattern,
            content_regex.as_ref(),
            max_results,
            &mut results,
        )?;

        if results.is_empty() {
            Ok("No results found.".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }
}

fn search_files_inner(
    root: &Path,
    glob_pattern: &str,
    content_regex: Option<&regex::Regex>,
    max_results: usize,
    results: &mut Vec<String>,
) -> Result<(), ToolError> {
    // Walk the directory tree and collect matching files
    let walker = walkdir::WalkDir::new(root).into_iter().filter_entry(|e| {
        // Skip hidden directories
        e.file_name()
            .to_str()
            .map(|s| !s.starts_with('.'))
            .unwrap_or(true)
    });

    let entries: Vec<_> = walker
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .take(max_results * 2) // overshoot in case of content filter
        .collect();

    for entry in entries {
        if results.len() >= max_results {
            break;
        }

        let path = entry.path();

        // Check glob pattern — simple suffix/pattern matching
        if !matches_glob(path, glob_pattern) {
            continue;
        }

        // If no content regex, just record the path
        if content_regex.is_none() {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            results.push(rel);
            continue;
        }

        // Search file content
        let regex = content_regex.unwrap();
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let mut match_lines = Vec::new();
                for (i, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        match_lines.push((i + 1, line.trim().to_string()));
                        if match_lines.len() >= 3 {
                            break; // limit matches per file
                        }
                    }
                }
                if !match_lines.is_empty() {
                    let rel = path
                        .strip_prefix(root)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();
                    for (line_no, line_content) in match_lines {
                        results.push(format!("{}:{}: {}", rel, line_no, line_content));
                    }
                }
            }
            Err(_) => continue, // Skip files we can't read (binary, etc.)
        }
    }

    Ok(())
}

/// Simple glob matching. Supports:
/// - `**` — match any number of directories
/// - `*` — match any sequence of characters (except '/')
/// - `?` — match a single character
/// - literal characters
fn matches_glob(path: &Path, pattern: &str) -> bool {
    let path_str = path.to_string_lossy();

    // Use a simple approach: convert glob to a regex-ish check
    // Handle common patterns:
    //   **/*.rs  -> ends with .rs, any directory depth
    //   *foo*    -> contains 'foo' in filename
    //   src/**/*.ts -> under src/, ends with .ts

    let path_str = path_str.replace('\\', "/");

    // Split pattern into segments
    let pattern = pattern.replace('\\', "/");

    if pattern.contains("**") {
        // **/ means any directory depth
        // **/foo*.rs -> find files matching foo*.rs at any depth
        let parts: Vec<&str> = pattern.split("**/").collect();
        if parts.len() == 2 {
            let prefix = parts[0]; // e.g. "src/" or ""
            let suffix = parts[1]; // e.g. "*.rs"

            if !prefix.is_empty() && !path_str.starts_with(prefix) {
                return false;
            }

            // Check suffix against the filename (or the tail of the path)
            return matches_simple_glob(
                path.file_name().and_then(OsStr::to_str).unwrap_or(""),
                suffix,
            ) || matches_simple_glob(&path_str, suffix);
        }
        // More complex ** pattern — just check the last segment
        let last_segment = pattern.split("**/").last().unwrap_or(&pattern);
        return matches_simple_glob(
            path.file_name().and_then(OsStr::to_str).unwrap_or(""),
            last_segment,
        );
    }

    // No ** — simple pattern matching
    if pattern.contains('/') {
        // Path-relative pattern
        matches_simple_glob(&path_str, &pattern)
    } else {
        // File-name-only pattern
        matches_simple_glob(
            path.file_name().and_then(OsStr::to_str).unwrap_or(""),
            &pattern,
        )
    }
}

/// Simple glob matching for a single path segment (no **).
fn matches_simple_glob(text: &str, pattern: &str) -> bool {
    let text: Vec<char> = text.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();

    matches_glob_inner(&text, &pattern)
}

fn matches_glob_inner(text: &[char], pattern: &[char]) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }

    match pattern[0] {
        '*' => {
            // * matches zero or more characters
            for i in 0..=text.len() {
                if matches_glob_inner(&text[i..], &pattern[1..]) {
                    return true;
                }
            }
            false
        }
        '?' => {
            if text.is_empty() {
                return false;
            }
            matches_glob_inner(&text[1..], &pattern[1..])
        }
        c => {
            if text.is_empty() || text[0] != c {
                return false;
            }
            matches_glob_inner(&text[1..], &pattern[1..])
        }
    }
}

// ─── BashTool ───────────────────────────────────────────────────────

/// Execute a bash/shell command and return the output.
pub struct BashTool {
    shell_command_blacklist: Vec<String>,
}

impl BashTool {
    pub fn new(blacklist: Vec<String>) -> Self {
        Self {
            shell_command_blacklist: blacklist,
        }
    }

    /// Check if a shell command is blacklisted.
    fn is_blacklisted(&self, command: &str) -> bool {
        let lower = command.to_lowercase();
        self.shell_command_blacklist
            .iter()
            .any(|entry| lower.contains(&entry.to_lowercase()))
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self {
            shell_command_blacklist: Vec::new(),
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("bash")
            .description(
                "Execute a shell command and return the output (cross-platform support). \
                 On Linux/macOS, uses bash. On Windows, uses PowerShell. \
                 Use this tool to run shell commands like curl, git, npm, etc. \
                 Returns both stdout and stderr. \
                 Be careful with commands that may take a long time or require user input.",
            )
            .parameter_with_description(
                "command",
                ParameterType::String,
                true,
                Some("The bash command to execute (e.g., 'curl https://example.com', 'ls -la')."),
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
        let parsed = parse_args(args)?;
        let command = parsed["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".into()))?;

        // Check global shell command blacklist
        if self.is_blacklisted(command) {
            return Err(ToolError::ExecutionError(
                "⚠️ This command has been blocked by the shell command blacklist. \
                 It matches a dangerous command pattern configured by the administrator. \
                 Please use a different command or contact your administrator to adjust the blacklist settings."
                    .to_string(),
            ));
        }

        let timeout_secs = parsed["timeout"].as_u64().unwrap_or(30);

        // Execute the shell command in a blocking thread with timeout.
        // Uses std::process::Command to avoid the Windows overlapped I/O hang
        // where tokio::process::Command::wait_with_output() waits for all
        // pipe handles to close even after the process has exited.
        let output =
            execute_shell_with_timeout(command, &self.shell_command_blacklist, timeout_secs)
                .await?;
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

/// Synchronous shell command execution (runs inside spawn_blocking).
///
/// Uses `std::process::Command` instead of `tokio::process::Command` to avoid
/// the Windows overlapped I/O hang where `wait_with_output()` waits for all
/// pipe handles to close even after the process has exited.
///
/// On Windows, we create stdout/stderr pipes with non-inheritable handles.
/// This prevents child processes spawned by PowerShell from inheriting the
/// pipe write ends, which would cause `read_to_end()` to block indefinitely.
/// With non-inheritable pipes, when PowerShell exits, the write end is
/// closed and `read_to_end()` returns immediately.
#[cfg(target_os = "windows")]
fn run_shell_command_sync(
    command: &str,
    _blacklist: &[String],
    guard_holder: &std::sync::Arc<std::sync::Mutex<Option<ProcessGroupGuard>>>,
) -> Result<std::process::Output, String> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // Encode command as UTF-16LE and base64-encode it to avoid PowerShell
    // argument parsing quirks with special characters.
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        command
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect::<Vec<u8>>(),
    );

    // Create non-inheritable pipes for stdout and stderr.
    // This ensures that when PowerShell spawns child processes (curl, git,
    // npm, etc.), they do NOT inherit the pipe write handles, so
    // `read_to_end()` returns as soon as PowerShell itself exits.
    let (mut stdout_reader, stdout_writer) =
        create_noninheritable_pipe().map_err(|e| format!("Failed to create stdout pipe: {}", e))?;
    let (mut stderr_reader, stderr_writer) =
        create_noninheritable_pipe().map_err(|e| format!("Failed to create stderr pipe: {}", e))?;

    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-EncodedCommand", &encoded])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(stdout_writer))
        .stderr(std::process::Stdio::from(stderr_writer))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    // Create a guard that creates a Job Object and assigns this process to it.
    // When killed via `guard.kill()`, TerminateJobObject atomically terminates
    // the entire process tree — including any grandchildren spawned by PowerShell.
    let guard = ProcessGroupGuard::new(child.id());
    *guard_holder.lock().unwrap() = Some(guard);

    // Wait for the process to exit. If the timeout fires in the async layer,
    // `guard.kill()` will terminate the job, causing this `wait()` to return.
    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for command: {}", e))?;

    // Now read the pipes. Since the pipe handles are non-inheritable,
    // child processes of PowerShell cannot hold the write end open.
    // `read_to_end()` will return as soon as PowerShell exits.
    let mut stdout = Vec::new();
    stdout_reader
        .read_to_end(&mut stdout)
        .map_err(|e| format!("Failed to read stdout: {}", e))?;
    let mut stderr = Vec::new();
    stderr_reader
        .read_to_end(&mut stderr)
        .map_err(|e| format!("Failed to read stderr: {}", e))?;

    // Normal completion — disarm the guard so Drop doesn't double-kill.
    if let Some(ref mut guard) = *guard_holder.lock().unwrap() {
        guard.disarm();
    }

    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

/// Create a pipe (reader, writer) where both ends have the inheritable
/// flag cleared. This prevents grandchild processes from inheriting the
/// pipe handles, which would block `read_to_end()` on Windows.
///
/// The writer handle is still usable by the direct child process because
/// we pass it explicitly via `Stdio::from(writer)` — Windows' `CreateProcess`
/// duplicates the handle into `STARTUPINFO.hStdOutput`, and the duplicated
/// handle inside the child is also non-inheritable.
#[cfg(target_os = "windows")]
pub(crate) fn create_noninheritable_pipe() -> Result<(std::fs::File, std::fs::File), std::io::Error>
{
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::HANDLE_FLAG_INHERIT;

    unsafe extern "system" {
        fn SetHandleInformation(hObject: *mut std::ffi::c_void, dwMask: u32, dwFlags: u32) -> i32;
        fn CreatePipe(
            hReadPipe: *mut *mut std::ffi::c_void,
            hWritePipe: *mut *mut std::ffi::c_void,
            lpPipeAttributes: *const std::ffi::c_void,
            nSize: u32,
        ) -> i32;
    }

    unsafe {
        let mut read_handle: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut write_handle: *mut std::ffi::c_void = std::ptr::null_mut();

        if CreatePipe(&mut read_handle, &mut write_handle, std::ptr::null(), 0) == 0 {
            return Err(std::io::Error::last_os_error());
        }

        // Clear the inheritable flag on both ends. The write end is passed
        // to the child explicitly via STARTUPINFO, so it does not need to
        // be inheritable.
        SetHandleInformation(read_handle, HANDLE_FLAG_INHERIT, 0);
        SetHandleInformation(write_handle, HANDLE_FLAG_INHERIT, 0);

        let read_file = std::fs::File::from_raw_handle(read_handle as _);
        let write_file = std::fs::File::from_raw_handle(write_handle as _);

        Ok((read_file, write_file))
    }
}

#[cfg(not(target_os = "windows"))]
fn run_shell_command_sync(
    command: &str,
    _blacklist: &[String],
    guard_holder: &std::sync::Arc<std::sync::Mutex<Option<ProcessGroupGuard>>>,
) -> Result<std::process::Output, String> {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    let mut cmd = Command::new("bash");
    cmd.arg("-c").arg(command);

    // Create a new session / process group via setsid().
    // This ensures that when we kill the process group (kill -PGID),
    // the entire process tree — including grandchildren — is terminated.
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    // Create a guard that can kill the process group on timeout.
    let guard = ProcessGroupGuard::new(child.id());
    *guard_holder.lock().unwrap() = Some(guard);

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for command: {}", e))?;

    // Normal completion — disarm the guard so Drop doesn't double-kill.
    if let Some(ref mut guard) = *guard_holder.lock().unwrap() {
        guard.disarm();
    }

    Ok(output)
}

/// Timeout wrapper for shell command execution.
///
/// On timeout, the entire process tree is killed via `ProcessGroupGuard`:
/// - Unix: `kill(-pgid, SIGKILL)` kills the process group created by `setsid()`.
/// - Windows: `TerminateJobObject` atomically terminates the Job Object.
///
/// This is critical on Windows where `tokio::spawn_blocking` cannot be
/// preempted — merely cancelling the `JoinHandle` future does NOT stop
/// the blocking thread or the OS process. The guard ensures the OS process
/// tree is actually terminated, which causes `child.wait()` to return and
/// the blocking thread to be released back to the pool.
async fn execute_shell_with_timeout(
    command: &str,
    blacklist: &[String],
    timeout_secs: u64,
) -> Result<std::process::Output, ToolError> {
    let blacklist = blacklist.to_vec();
    let command = command.to_string();

    // Shared guard holder: the blocking thread writes the guard after
    // spawning the child, and the async timeout handler reads it to kill.
    let guard_holder = std::sync::Arc::new(std::sync::Mutex::new(None::<ProcessGroupGuard>));
    let gh = guard_holder.clone();

    let blocking_task =
        tokio::task::spawn_blocking(move || run_shell_command_sync(&command, &blacklist, &gh));

    match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), blocking_task).await {
        Ok(join_result) => join_result
            .map_err(|e| ToolError::ExecutionError(format!("Blocking task panicked: {}", e)))?
            .map_err(|e| ToolError::ExecutionError(e)),
        Err(_elapsed) => {
            // Timeout — kill the entire process tree.
            // This causes `child.wait()` in the blocking thread to return,
            // releasing the thread back to the spawn_blocking pool.
            let mut lock = guard_holder.lock().unwrap();
            if let Some(ref guard) = *lock {
                guard.kill();
            }
            // Disarm to prevent Drop from double-killing (avoid extra 200ms
            // sleep on Unix for an already-dead process group).
            if let Some(ref mut guard) = *lock {
                guard.disarm();
            }
            Err(ToolError::ExecutionError(format!(
                "Command timed out after {} seconds",
                timeout_secs
            )))
        }
    }
}

// ─── WebSearchTool ─────────────────────────────────

/// Search the web using DuckDuckGo.
/// Web search tool that supports multiple search engines.
///
/// Supported engines:
/// - DuckDuckGo (free, no API key required)
/// - Tavily (requires API key)
/// - BoCha (requires API key)
/// - Baidu AI Search (requires API key)
/// - Brave Search (requires API key)
pub struct WebSearchTool {
    config: std::sync::Arc<tokio::sync::RwLock<crate::types::WebSearchConfig>>,
}

impl WebSearchTool {
    /// Create a new WebSearchTool with the given configuration.
    pub fn new(config: std::sync::Arc<tokio::sync::RwLock<crate::types::WebSearchConfig>>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function("web_search")
            .description(
                "Search the web for current information. \n\
                 Returns search results with titles, URLs, snippets, and when available, \n\
                 full page content. \n\
                 Use this tool when you need information that may not be in your training data, \n\
                 such as current events, recent news, live data, or up-to-date documentation. \n\
                 Supports multiple search engines: DuckDuckGo (free), Tavily, BoCha, Baidu, Brave."
            )
            .parameter_with_description(
                "query",
                ParameterType::String,
                true,
                Some("The search query. Be specific and include relevant keywords."),
            )
            .parameter_with_description(
                "max_results",
                ParameterType::Integer,
                false,
                Some("Maximum number of results to return (default: 5, max: 20). Use fewer results for focused queries."),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        let parsed = parse_args(args)?;
        let query = parsed["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query' parameter".into()))?;

        // Read configuration
        let config = self.config.read().await;

        // Check if web search is enabled
        if !config.enabled {
            return Ok(
                "Web search is currently disabled. I will answer based on my training knowledge instead.".to_string(),
            );
        }

        // Use max_results from args or config, with a sensible default
        let max_results = parsed["max_results"]
            .as_u64()
            .map(|n| n.max(1).min(20) as usize)
            .unwrap_or(config.max_results.min(20).max(1));

        tracing::info!(
            query = %query,
            max_results = %max_results,
            engine = ?config.search_engine,
            "Executing web search"
        );

        // Get API key if needed
        let api_key = config.api_key.clone();

        // Perform the search using the configured engine
        let results = match config.search_engine {
            crate::types::SearchEngine::DuckDuckGo => {
                search_duckduckgo(query, max_results).await
            }
            crate::types::SearchEngine::Tavily => {
                let key = match api_key {
                    Some(k) => k,
                    None => {
                        return Ok(
                            "Web search (Tavily) is not configured. I will answer based on my training knowledge instead.".to_string(),
                        );
                    }
                };
                search_tavily(query, max_results, &key).await
            }
            crate::types::SearchEngine::BoCha => {
                let key = match api_key {
                    Some(k) => k,
                    None => {
                        return Ok(
                            "Web search (BoCha) is not configured. I will answer based on my training knowledge instead.".to_string(),
                        );
                    }
                };
                search_bocha(query, max_results, &key).await
            }
            crate::types::SearchEngine::Baidu => {
                let key = match api_key {
                    Some(k) => k,
                    None => {
                        return Ok(
                            "Web search (Baidu) is not configured. I will answer based on my training knowledge instead.".to_string(),
                        );
                    }
                };
                search_baidu(query, max_results, &key).await
            }
            crate::types::SearchEngine::Brave => {
                let key = match api_key {
                    Some(k) => k,
                    None => {
                        return Ok(
                            "Web search (Brave) is not configured. I will answer based on my training knowledge instead.".to_string(),
                        );
                    }
                };
                search_brave(query, max_results, &key).await
            }
        }
        .map_err(|e| ToolError::ExecutionError(format!("Search failed: {}", e)))?;

        if results.is_empty() {
            return Ok("No search results found.".to_string());
        }

        // Format results as markdown with proper structure for consumption
        let mut output = String::new();
        let engine_name = match config.search_engine {
            crate::types::SearchEngine::DuckDuckGo => "DuckDuckGo",
            crate::types::SearchEngine::Tavily => "Tavily",
            crate::types::SearchEngine::BoCha => "BoCha",
            crate::types::SearchEngine::Baidu => "Baidu AI Search",
            crate::types::SearchEngine::Brave => "Brave Search",
        };
        output.push_str(&format!(
            "## Web Search Results\n\n**Query:** {}\n**Engine:** {}\n**Results:** {}\n\n",
            query,
            engine_name,
            results.len()
        ));

        for (i, result) in results.iter().enumerate() {
            output.push_str(&format!("### {}. {}\n\n", i + 1, result.title));
            output.push_str(&format!("**URL:** {}\n\n", result.url));
            if !result.snippet.is_empty() {
                output.push_str(&format!("**Snippet:** {}\n\n", result.snippet));
            }
            if !result.content.is_empty() {
                output.push_str(&format!("**Content:**\n{}\n\n", result.content));
            }
            output.push_str("---\n\n");
        }

        Ok(output.trim_end().to_string())
    }
}

/// A single search result.
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
    /// Full page content when available (e.g., from Tavily advanced search)
    content: String,
}

/// Perform a web search using DuckDuckGo.
async fn search_duckduckgo(
    query: &str,
    max_results: usize,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    use scraper::{Html, Selector};

    // URL encode the query
    let encoded_query = urlencode(query);
    let search_url = format!("https://duckduckgo.com/html/?q={}", encoded_query);

    tracing::debug!(url = %search_url, "Fetching search results");

    // Make the HTTP request
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; RuriBot/1.0; +https://github.com/ruri)")
        .build()?;

    let response = client
        .get(&search_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    let html_content = response.text().await?;
    let document = Html::parse_document(&html_content);

    // Parse search results from DuckDuckGo HTML
    let result_selector = Selector::parse(".result").unwrap();
    let title_selector = Selector::parse(".result__title .result__a").unwrap();
    let snippet_selector = Selector::parse(".result__snippet").unwrap();

    let mut results = Vec::new();

    for result_elem in document.select(&result_selector).take(max_results) {
        let title = result_elem
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "No title".to_string());

        // Get URL from the link
        let url = result_elem
            .select(&title_selector)
            .next()
            .and_then(|el| el.value().attr("href"))
            .map(|h| h.to_string())
            .unwrap_or_else(|| "No URL".to_string());

        // Try to get snippet
        let snippet = result_elem
            .select(&snippet_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "No description available".to_string());

        results.push(SearchResult {
            title,
            url,
            snippet,
            content: String::new(),
        });
    }

    Ok(results)
}

fn urlencode(s: &str) -> String {
    let mut encoded = String::new();
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                encoded.push(c);
            }
            _ => {
                for byte in c.to_string().as_bytes() {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    encoded
}

/// Perform a web search using Tavily API.
/// Uses advanced search depth to fetch actual page content.
async fn search_tavily(
    query: &str,
    max_results: usize,
    api_key: &str,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    #[derive(serde::Serialize)]
    struct TavilyRequest {
        query: String,
        max_results: usize,
        /// Use advanced search to get full page content
        search_depth: &'static str,
        /// Include extracted content from pages
        include_answer: bool,
        /// Include raw content
        include_raw_content: bool,
    }

    #[derive(serde::Deserialize)]
    struct TavilyResponse {
        results: Vec<TavilyResult>,
        #[serde(default)]
        answer: Option<String>,
    }

    #[derive(serde::Deserialize)]
    struct TavilyResult {
        title: String,
        url: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        snippet: Option<String>,
        #[serde(default)]
        raw_content: Option<String>,
        #[serde(default)]
        #[serde(rename = "score")]
        _score: Option<f64>,
    }

    let client = reqwest::Client::builder()
        .user_agent("RuriBot/1.0")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let request = TavilyRequest {
        query: query.to_string(),
        max_results,
        search_depth: "advanced",
        include_answer: true,
        include_raw_content: true,
    };

    let request_body = serde_json::to_string(&request).unwrap_or_default();
    tracing::debug!(query = %query, max_results = max_results, request_body = %request_body, "Sending Tavily search request");

    let response = client
        .post("https://api.tavily.com/search")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await?;

    let tavily_response: TavilyResponse = response.json().await?;

    tracing::info!(
        requested_max_results = max_results,
        received_results_count = tavily_response.results.len(),
        has_answer = tavily_response.answer.is_some(),
        "Tavily search completed"
    );

    let mut results: Vec<SearchResult> = tavily_response
        .results
        .into_iter()
        .map(|r| SearchResult {
            title: r.title,
            url: r.url,
            snippet: r.snippet.unwrap_or_default(),
            content: r.raw_content.or(r.content).unwrap_or_default(),
        })
        .collect();

    // If Tavily provided a direct answer, prepend it as a special result
    if let Some(answer) = tavily_response.answer {
        results.insert(
            0,
            SearchResult {
                title: "Direct Answer".to_string(),
                url: "tavily://direct-answer".to_string(),
                snippet: answer.clone(),
                content: answer,
            },
        );
    }

    Ok(results)
}

/// Perform a web search using BoCha API.
async fn search_bocha(
    query: &str,
    max_results: usize,
    api_key: &str,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    // BoCha API implementation
    // Note: Adjust the endpoint and parameters based on actual BoCha API documentation
    let encoded_query = urlencode(query);
    let url = format!(
        "https://api.bocha.io/search?q={}&count={}",
        encoded_query, max_results
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    #[derive(serde::Deserialize)]
    struct BoChaResponse {
        data: Vec<BoChaResult>,
    }

    #[derive(serde::Deserialize)]
    struct BoChaResult {
        title: String,
        link: String,
        snippet: String,
    }

    let bocha_response: BoChaResponse = response.json().await?;

    Ok(bocha_response
        .data
        .into_iter()
        .map(|r| SearchResult {
            title: r.title,
            url: r.link,
            snippet: r.snippet,
            content: String::new(),
        })
        .collect())
}

/// Perform a web search using Baidu AI Search API.
async fn search_baidu(
    query: &str,
    max_results: usize,
    api_key: &str,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    // Baidu AI Search API implementation
    // Note: Adjust the endpoint and parameters based on actual Baidu API documentation
    let encoded_query = urlencode(query);
    let url = format!(
        "https://api.baidu.com/api/v1/search?query={}&num={}",
        encoded_query, max_results
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    #[derive(serde::Deserialize)]
    struct BaiduResponse {
        results: Vec<BaiduResult>,
    }

    #[derive(serde::Deserialize)]
    struct BaiduResult {
        title: String,
        url: String,
        desc: String,
    }

    let baidu_response: BaiduResponse = response.json().await?;

    Ok(baidu_response
        .results
        .into_iter()
        .map(|r| SearchResult {
            title: r.title,
            url: r.url,
            snippet: r.desc,
            content: String::new(),
        })
        .collect())
}

/// Perform a web search using Brave Search API.
async fn search_brave(
    query: &str,
    max_results: usize,
    api_key: &str,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    let encoded_query = urlencode(query);
    let url = format!(
        "https://api.search.brave.com/res/v1/web/search?q={}&count={}&summarize=true",
        encoded_query, max_results
    );

    let client = reqwest::Client::builder()
        .user_agent("RuriBot/1.0")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .header("X-Subscription-Token", api_key)
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?;

    #[derive(serde::Deserialize)]
    struct BraveResponse {
        web: BraveWebResults,
        #[serde(default)]
        summarize: Option<BraveSummary>,
    }

    #[derive(serde::Deserialize)]
    struct BraveWebResults {
        results: Vec<BraveResult>,
    }

    #[derive(serde::Deserialize)]
    struct BraveResult {
        title: String,
        url: String,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        #[serde(rename = "page_age")]
        _page_age: Option<String>,
    }

    #[derive(serde::Deserialize)]
    struct BraveSummary {
        #[serde(default)]
        summary: Vec<String>,
    }

    let brave_response: BraveResponse = response.json().await?;

    let mut results: Vec<SearchResult> = brave_response
        .web
        .results
        .into_iter()
        .map(|r| SearchResult {
            title: r.title,
            url: r.url,
            snippet: r.description.unwrap_or_default(),
            content: String::new(),
        })
        .collect();

    // If Brave provided a summary, prepend it
    if let Some(summary_data) = brave_response.summarize {
        let summary_text = summary_data.summary.join("\n");
        if !summary_text.is_empty() {
            results.insert(
                0,
                SearchResult {
                    title: "Brave Summary".to_string(),
                    url: "brave://summary".to_string(),
                    snippet: summary_text.clone(),
                    content: summary_text,
                },
            );
        }
    }

    Ok(results)
}

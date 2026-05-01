//! Built-in tools for file system operations and code editing.

use crate::agent::tool_executor::{Tool, ToolError};
use crate::types::{ParameterType, ToolDefinition};
use async_trait::async_trait;
use serde_json::Value;
use std::ffi::OsStr;
use std::path::Path;

// ─── Helper ────────────────────────────────────────────────────────

/// Parse a JSON args string into a serde_json::Value.
fn parse_args(args: &str) -> Result<Value, ToolError> {
    serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))
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

        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to read file '{}': {}", path, e))
        })?;

        let start_line = parsed["start_line"].as_u64();
        let end_line = parsed["end_line"].as_u64();

        match (start_line, end_line) {
            (Some(start), Some(end)) => {
                let lines: Vec<&str> = content.lines().collect();
                let total = lines.len();
                let start = (start as usize).saturating_sub(1).min(total);
                let end = (end as usize).min(total);
                if start >= end {
                    return Ok(String::new());
                }
                let selected: Vec<&str> = lines[start..end].to_vec();
                Ok(selected.join("\n"))
            }
            (Some(start), None) => {
                let lines: Vec<&str> = content.lines().collect();
                let total = lines.len();
                let start = (start as usize).saturating_sub(1).min(total);
                let selected: Vec<&str> = lines[start..].to_vec();
                Ok(selected.join("\n"))
            }
            (None, Some(end)) => {
                let lines: Vec<&str> = content.lines().collect();
                let end = (end as usize).min(lines.len());
                let selected: Vec<&str> = lines[..end].to_vec();
                Ok(selected.join("\n"))
            }
            (None, None) => Ok(content),
        }
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

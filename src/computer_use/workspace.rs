use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::info;

/// Workspace manager for handling session-specific workspaces
pub struct WorkspaceManager {
    /// Base directory for all workspaces
    base_dir: PathBuf,
}

impl WorkspaceManager {
    /// Create a new workspace manager with the given base directory
    pub fn new<P: Into<PathBuf>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Get the workspace directory for a given session ID
    pub fn get_workspace_path(&self, session_id: &str) -> PathBuf {
        // Normalize session ID by replacing characters that are not suitable for file names
        let normalized = normalize_session_id(session_id);
        self.base_dir.join("workspaces").join(normalized)
    }

    /// Create a workspace for a given session ID
    pub async fn create_workspace(&self, session_id: &str) -> Result<PathBuf, WorkspaceError> {
        let workspace_path = self.get_workspace_path(session_id);

        // Create the workspace directory and all parent directories
        fs::create_dir_all(&workspace_path).await.map_err(|e| {
            WorkspaceError::CreateFailed(workspace_path.display().to_string(), e.to_string())
        })?;

        info!(
            "Created workspace for session '{}': {}",
            session_id,
            workspace_path.display()
        );

        Ok(workspace_path)
    }

    /// Resolve a relative path to an absolute path within the workspace
    pub fn resolve_path(&self, session_id: &str, relative_path: &Path) -> PathBuf {
        let workspace_path = self.get_workspace_path(session_id);
        workspace_path.join(relative_path)
    }
}

/// Normalize a session ID to make it suitable for use as a directory name
pub fn normalize_session_id(session_id: &str) -> String {
    session_id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("Failed to create workspace '{0}': {1}")]
    CreateFailed(String, String),
}

/// Get the default data directory for ruri
pub fn default_data_dir() -> PathBuf {
    // Use the current working directory's 'data' folder
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("data")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_normalize_session_id() {
        assert_eq!(normalize_session_id("user123"), "user123");
        assert_eq!(normalize_session_id("user/session:123"), "user_session_123");
        assert_eq!(normalize_session_id("test-file.name"), "test-file.name");
    }

    #[test]
    fn test_workspace_path() {
        let temp_dir = tempdir().unwrap();
        let manager = WorkspaceManager::new(temp_dir.path());

        let path = manager.get_workspace_path("session123");
        // Use Path::ends_with to compare path components in a cross-platform way
        assert!(path.ends_with(Path::new("workspaces").join("session123")));

        let path = manager.get_workspace_path("user/session:123");
        assert!(path.ends_with(Path::new("workspaces").join("user_session_123")));
    }

    #[tokio::test]
    async fn test_create_workspace() {
        let temp_dir = tempdir().unwrap();
        let manager = WorkspaceManager::new(temp_dir.path());

        let path = manager.create_workspace("test_session").await.unwrap();
        assert!(path.exists());
        assert!(path.is_dir());
    }

    #[test]
    fn test_resolve_path() {
        let temp_dir = tempdir().unwrap();
        let manager = WorkspaceManager::new(temp_dir.path());

        let resolved = manager.resolve_path("test_session", Path::new("notes/todo.txt"));
        // Use Path::ends_with for cross-platform path comparison
        assert!(
            resolved.ends_with(
                Path::new("workspaces")
                    .join("test_session")
                    .join("notes")
                    .join("todo.txt")
            )
        );
    }
}

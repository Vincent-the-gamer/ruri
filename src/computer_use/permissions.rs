use crate::computer_use::config::{ComputerUseConfig, UserRole};
use std::path::{Path, PathBuf};

/// Permission checker for computer use operations
pub struct PermissionChecker {
    config: ComputerUseConfig,
    data_dir: PathBuf,
    temp_dir: PathBuf,
}

impl PermissionChecker {
    /// Create a new permission checker
    pub fn new(config: ComputerUseConfig, data_dir: PathBuf, temp_dir: PathBuf) -> Self {
        Self {
            config,
            data_dir,
            temp_dir,
        }
    }

    /// Check if a user can read a file at the given path
    pub fn can_read_path(
        &self,
        user_id: &str,
        session_id: &str,
        path: &Path,
    ) -> Result<(), PermissionError> {
        let resolved = self.resolve_path_for_user(user_id, session_id, path)?;

        // Check if the resolved path is within allowed directories
        if !self.is_path_allowed(user_id, session_id, &resolved) {
            return Err(PermissionError::PathNotAllowed {
                path: resolved.display().to_string(),
                user_id: user_id.to_string(),
            });
        }

        Ok(())
    }

    /// Check if a user can write to a file at the given path
    pub fn can_write_path(
        &self,
        user_id: &str,
        session_id: &str,
        path: &Path,
    ) -> Result<(), PermissionError> {
        let resolved = self.resolve_path_for_user(user_id, session_id, path)?;

        // Check if the resolved path is within allowed directories
        if !self.is_path_allowed(user_id, session_id, &resolved) {
            return Err(PermissionError::PathNotAllowed {
                path: resolved.display().to_string(),
                user_id: user_id.to_string(),
            });
        }

        Ok(())
    }

    /// Check if a user can execute shell commands
    pub fn can_execute_shell(&self, user_id: &str) -> Result<(), PermissionError> {
        if !self.config.can_use_power_tools(user_id) {
            return Err(PermissionError::ShellNotAllowed {
                user_id: user_id.to_string(),
            });
        }
        Ok(())
    }

    /// Check if a user can execute Python code
    pub fn can_execute_python(&self, user_id: &str) -> Result<(), PermissionError> {
        if !self.config.can_use_power_tools(user_id) {
            return Err(PermissionError::PythonNotAllowed {
                user_id: user_id.to_string(),
            });
        }
        Ok(())
    }

    /// Resolve a path for a user, handling relative paths and workspace
    fn resolve_path_for_user(
        &self,
        user_id: &str,
        session_id: &str,
        path: &Path,
    ) -> Result<PathBuf, PermissionError> {
        // Admin can access any path
        if self.config.is_admin(user_id) {
            return Ok(if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.get_workspace_path(session_id).join(path)
            });
        }

        // Non-admin: resolve relative paths to workspace
        if path.is_absolute() {
            // Non-admin can't use absolute paths
            return Err(PermissionError::AbsolutePathNotAllowed {
                user_id: user_id.to_string(),
            });
        }

        Ok(self.get_workspace_path(session_id).join(path))
    }

    /// Check if a path is within allowed directories for a user
    fn is_path_allowed(&self, user_id: &str, session_id: &str, path: &Path) -> bool {
        let role = self.config.get_user_role(user_id);

        match role {
            UserRole::Admin => {
                // Admin can access any path
                true
            }
            UserRole::Regular => {
                // Non-admin can only access specific directories
                let canonical_path = match path.canonicalize() {
                    Ok(p) => p,
                    Err(_) => return false, // Path doesn't exist yet
                };

                // Allowed directories for non-admin:
                // 1. Skills directory
                // 2. Current session workspace
                // 3. System temp directory's .astrbot folder (we'll use .ruri)
                // 4. Configured allowed_paths

                let workspace_path = self.get_workspace_path(session_id);
                let skills_path = self.data_dir.join("skills");
                let ruri_temp_path = self.temp_dir.join(".ruri");

                // Check if path starts with any allowed directory
                [workspace_path, skills_path, ruri_temp_path]
                    .iter()
                    .any(|allowed| canonical_path.starts_with(allowed))
                    || self.config.allowed_paths.iter().any(|allowed| {
                        let allowed_path = PathBuf::from(allowed);
                        canonical_path.starts_with(&allowed_path)
                    })
            }
        }
    }

    /// Get the workspace path for a session
    fn get_workspace_path(&self, session_id: &str) -> PathBuf {
        use super::workspace::normalize_session_id;
        let normalized = normalize_session_id(session_id);
        self.data_dir.join("workspaces").join(normalized)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("Path '{path}' is not allowed for user '{user_id}'")]
    PathNotAllowed { path: String, user_id: String },

    #[error("User '{user_id}' is not allowed to use absolute paths")]
    AbsolutePathNotAllowed { user_id: String },

    #[error("User '{user_id}' is not allowed to execute shell commands")]
    ShellNotAllowed { user_id: String },

    #[error("User '{user_id}' is not allowed to execute Python code")]
    PythonNotAllowed { user_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_checker(
        admin_ids: Vec<String>,
        data_dir: PathBuf,
        temp_dir: PathBuf,
    ) -> PermissionChecker {
        let mut config = ComputerUseConfig::default();
        config.admin_ids = admin_ids;
        config.require_admin = true;
        PermissionChecker::new(config, data_dir, temp_dir)
    }

    #[test]
    fn test_admin_can_access_any_path() {
        let temp_dir = tempdir().unwrap();
        let checker = create_test_checker(
            vec!["admin1".to_string()],
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf(),
        );

        assert!(
            checker
                .can_read_path("admin1", "session1", Path::new("/any/path"))
                .is_ok()
        );
        assert!(
            checker
                .can_write_path("admin1", "session1", Path::new("/any/path"))
                .is_ok()
        );
    }

    #[test]
    fn test_non_admin_cannot_use_absolute_paths() {
        let temp_dir = tempdir().unwrap();
        let checker = create_test_checker(
            vec!["admin1".to_string()],
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf(),
        );

        assert!(
            checker
                .can_read_path("user1", "session1", Path::new("/absolute/path"))
                .is_err()
        );
    }

    #[test]
    fn test_non_admin_can_use_relative_paths() {
        let temp_dir = tempdir().unwrap();
        let data_dir = temp_dir.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        let checker = create_test_checker(
            vec!["admin1".to_string()],
            data_dir,
            temp_dir.path().to_path_buf(),
        );

        // Create workspace
        let workspace = checker.get_workspace_path("session1");
        std::fs::create_dir_all(&workspace).unwrap();

        // Should be able to read/write relative paths within workspace
        assert!(
            checker
                .can_read_path("user1", "session1", Path::new("notes.txt"))
                .is_ok()
        );
        assert!(
            checker
                .can_write_path("user1", "session1", Path::new("notes.txt"))
                .is_ok()
        );
    }

    #[test]
    fn test_non_admin_cannot_execute_shell() {
        let temp_dir = tempdir().unwrap();
        let checker = create_test_checker(
            vec!["admin1".to_string()],
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf(),
        );

        assert!(checker.can_execute_shell("user1").is_err());
        assert!(checker.can_execute_shell("admin1").is_ok());
    }

    #[test]
    fn test_non_admin_cannot_execute_python() {
        let temp_dir = tempdir().unwrap();
        let checker = create_test_checker(
            vec!["admin1".to_string()],
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf(),
        );

        assert!(checker.can_execute_python("user1").is_err());
        assert!(checker.can_execute_python("admin1").is_ok());
    }
}

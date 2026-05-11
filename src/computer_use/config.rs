use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Computer Use runtime mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ComputerUseRuntime {
    /// Computer use is disabled
    None,
    /// Run in local environment (Ruri host machine)
    Local,
    /// Run in AIO Sandbox (isolated Docker container via HTTP API)
    #[serde(rename = "aio_sandbox")]
    AioSandbox,
}

impl Default for ComputerUseRuntime {
    fn default() -> Self {
        Self::None
    }
}

/// Computer Use configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerUseConfig {
    /// Runtime mode: none, local, or aio_sandbox
    #[serde(default)]
    pub runtime: ComputerUseRuntime,

    /// Whether admin privileges are required for powerful operations
    #[serde(default = "default_require_admin")]
    pub require_admin: bool,

    /// List of admin user IDs
    #[serde(default)]
    pub admin_ids: Vec<String>,

    /// Additional paths that non-admin users can access
    #[serde(default)]
    pub allowed_paths: Vec<String>,

    /// Per-command admin requirement overrides.
    /// Key: command name (e.g. "reset"), Value: true = admin required, false = open to all.
    /// Commands not present in this map use their default `require_admin()` value.
    #[serde(default)]
    pub command_admin_required: HashMap<String, bool>,

    /// AIO Sandbox configuration (used when runtime is AioSandbox)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aio_sandbox_config: Option<AioSandboxConfig>,
}

impl Default for ComputerUseConfig {
    fn default() -> Self {
        Self {
            runtime: ComputerUseRuntime::None,
            require_admin: true,
            admin_ids: Vec::new(),
            allowed_paths: Vec::new(),
            command_admin_required: HashMap::new(),
            aio_sandbox_config: None,
        }
    }
}

fn default_require_admin() -> bool {
    true
}

/// AIO Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AioSandboxConfig {
    /// AIO Sandbox endpoint URL (e.g., http://localhost:8080)
    #[serde(default = "default_aio_sandbox_endpoint")]
    pub endpoint: String,
}

fn default_aio_sandbox_endpoint() -> String {
    "http://localhost:8080".to_string()
}

impl Default for AioSandboxConfig {
    fn default() -> Self {
        Self {
            endpoint: default_aio_sandbox_endpoint(),
        }
    }
}

/// User role for permission checking
#[derive(Debug, Clone, PartialEq)]
pub enum UserRole {
    Admin,
    Regular,
}

impl ComputerUseConfig {
    /// Check if a user ID is an admin
    pub fn is_admin(&self, user_id: &str) -> bool {
        self.admin_ids.iter().any(|id| id == user_id)
    }

    /// Get user role based on user ID
    pub fn get_user_role(&self, user_id: &str) -> UserRole {
        if self.is_admin(user_id) {
            UserRole::Admin
        } else {
            UserRole::Regular
        }
    }

    /// Check if user can use shell/python tools
    pub fn can_use_power_tools(&self, user_id: &str) -> bool {
        if !self.require_admin {
            return true;
        }
        self.is_admin(user_id)
    }

    /// Check whether a specific command requires admin privileges.
    ///
    /// Priority:
    /// 1. If the command has an override in `command_admin_required`, use that.
    /// 2. Otherwise, fall back to the command's default `require_admin` value.
    pub fn is_command_admin_required(
        &self,
        command_name: &str,
        default_require_admin: bool,
    ) -> bool {
        self.command_admin_required
            .get(command_name)
            .copied()
            .unwrap_or(default_require_admin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ComputerUseConfig::default();
        assert_eq!(config.runtime, ComputerUseRuntime::None);
        assert!(config.require_admin);
        assert!(config.admin_ids.is_empty());
    }

    #[test]
    fn test_admin_check() {
        let mut config = ComputerUseConfig::default();
        config.admin_ids.push("admin1".to_string());

        assert!(config.is_admin("admin1"));
        assert!(!config.is_admin("user1"));
    }

    #[test]
    fn test_permission_check() {
        let mut config = ComputerUseConfig::default();
        config.admin_ids.push("admin1".to_string());

        assert!(config.can_use_power_tools("admin1"));
        assert!(!config.can_use_power_tools("user1"));

        // Test with require_admin = false
        config.require_admin = false;
        assert!(config.can_use_power_tools("user1"));
    }

    #[test]
    fn test_command_admin_required() {
        let mut config = ComputerUseConfig::default();

        // Default: no overrides, fall back to command default
        assert!(config.is_command_admin_required("reset", true));
        assert!(!config.is_command_admin_required("help", false));

        // Add overrides
        config
            .command_admin_required
            .insert("reset".to_string(), false);
        config
            .command_admin_required
            .insert("help".to_string(), true);

        // Override takes priority
        assert!(!config.is_command_admin_required("reset", true)); // default true, overridden to false
        assert!(config.is_command_admin_required("help", false)); // default false, overridden to true

        // Commands without override still use default
        assert!(config.is_command_admin_required("stop", true));
        assert!(!config.is_command_admin_required("whoami", false));
    }
}

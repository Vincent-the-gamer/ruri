use serde::{Deserialize, Serialize};

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
}

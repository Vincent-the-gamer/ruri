use serde::{Deserialize, Serialize};

/// Computer Use runtime mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ComputerUseRuntime {
    /// Computer use is disabled
    None,
    /// Run in local environment (Ruri host machine)
    Local,
    /// Run in isolated sandbox
    Sandbox,
}

impl Default for ComputerUseRuntime {
    fn default() -> Self {
        Self::None
    }
}

/// Computer Use configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerUseConfig {
    /// Runtime mode: none, local, or sandbox
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

    /// Sandbox configuration (used when runtime is Sandbox)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_config: Option<SandboxConfig>,
}

impl Default for ComputerUseConfig {
    fn default() -> Self {
        Self {
            runtime: ComputerUseRuntime::None,
            require_admin: true,
            admin_ids: Vec::new(),
            allowed_paths: Vec::new(),
            sandbox_config: None,
        }
    }
}

fn default_require_admin() -> bool {
    true
}

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Sandbox driver: "shipyard_neo", "cua", etc.
    pub driver: String,

    /// Sandbox endpoint URL (for remote sandboxes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// Sandbox profile (e.g., for CUA)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// Time-to-live for sandbox sessions (in seconds)
    #[serde(default = "default_ttl")]
    pub ttl_secs: u64,

    /// Enable browser automation capabilities
    #[serde(default)]
    pub enable_browser: bool,
}

fn default_ttl() -> u64 {
    3600 // 1 hour
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            driver: "shipyard_neo".to_string(),
            endpoint: None,
            profile: None,
            ttl_secs: default_ttl(),
            enable_browser: false,
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

    /// Check if computer use is enabled
    pub fn is_enabled(&self) -> bool {
        self.runtime != ComputerUseRuntime::None
    }

    /// Check if user can use shell/python tools
    pub fn can_use_power_tools(&self, user_id: &str) -> bool {
        if !self.require_admin {
            return true;
        }
        self.is_admin(user_id)
    }

    /// Check if user can use file tools
    pub fn can_use_file_tools(&self, _user_id: &str) -> bool {
        // File tools are available to all users, but with directory restrictions
        self.is_enabled()
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

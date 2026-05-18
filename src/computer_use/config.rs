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

    /// Shell command blacklist — any command containing one of these
    /// substrings (case-insensitive) will be blocked regardless of
    /// admin status.
    #[serde(default = "default_shell_blacklist")]
    pub shell_command_blacklist: Vec<String>,

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
            shell_command_blacklist: default_shell_blacklist(),
            aio_sandbox_config: None,
        }
    }
}

fn default_require_admin() -> bool {
    true
}

fn default_shell_blacklist() -> Vec<String> {
    vec![
        // ── Linux / macOS ──
        "sudo ".to_string(),
        "rm -rf".to_string(),
        "dd if=".to_string(),
        "mkfs.".to_string(),
        ":(){ :|:& };:".to_string(),
        "chmod 777".to_string(),
        "chown -R".to_string(),
        "> /dev/sda".to_string(),
        "mv /* ".to_string(),
        "| sh".to_string(),
        "| bash".to_string(),
        "fdisk".to_string(),
        "parted".to_string(),
        "shutdown".to_string(),
        "reboot".to_string(),
        "halt".to_string(),
        "poweroff".to_string(),
        "init 0".to_string(),
        "init 6".to_string(),
        "kill -9".to_string(),
        "pkill".to_string(),
        "killall".to_string(),
        "iptables -F".to_string(),
        "ufw disable".to_string(),
        "systemctl disable".to_string(),
        "modprobe -r".to_string(),
        "rmmod".to_string(),
        "diskutil eraseDisk".to_string(),
        "diskutil unmount".to_string(),
        "hdiutil".to_string(),
        "launchctl unload".to_string(),
        "csrutil disable".to_string(),
        "fdesetup".to_string(),
        "softwareupdate".to_string(),
        // ── Windows ──
        "format ".to_string(),
        "del /f /s".to_string(),
        "rmdir /s".to_string(),
        "diskpart".to_string(),
        "reg delete".to_string(),
        "reg add".to_string(),
        "bcdedit".to_string(),
        "icacls ".to_string(),
        "takeown".to_string(),
        "cipher /w".to_string(),
        "sc delete".to_string(),
        "sc stop".to_string(),
        "net stop".to_string(),
        "Remove-Item -Force -Recurse".to_string(),
        "Set-ExecutionPolicy".to_string(),
        "Stop-Process -Force".to_string(),
        "Clear-RecycleBin".to_string(),
        "Disable-WindowsOptionalFeature".to_string(),
        "Reset-ComputerMachinePassword".to_string(),
    ]
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
    /// Reserved user ID for debug sessions that always has admin privileges
    pub const DEBUG_ADMIN_ID: &str = "debug_admin";

    /// Check if a user ID is an admin
    /// Debug session users (identified by `DEBUG_ADMIN_ID`) are always treated as admin.
    pub fn is_admin(&self, user_id: &str) -> bool {
        if user_id == Self::DEBUG_ADMIN_ID {
            return true;
        }
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

    /// Check if a shell command is blacklisted.
    /// Returns `true` if the command (case-insensitive) contains any
    /// of the blacklisted substrings.
    pub fn is_shell_command_blacklisted(&self, command: &str) -> bool {
        let lower = command.to_lowercase();
        self.shell_command_blacklist
            .iter()
            .any(|entry| lower.contains(&entry.to_lowercase()))
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
    fn test_debug_admin_always_has_admin_privileges() {
        let mut config = ComputerUseConfig::default();
        config.require_admin = true;
        // Even with no admin_ids configured, debug_admin is always admin
        assert!(config.is_admin(ComputerUseConfig::DEBUG_ADMIN_ID));
        assert!(config.can_use_power_tools(ComputerUseConfig::DEBUG_ADMIN_ID));

        // Regular user without admin_ids should not be admin
        assert!(!config.is_admin("user1"));
        assert!(!config.can_use_power_tools("user1"));
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

use serde::{Deserialize, Serialize};

/// Global proxy configuration for the application.
///
/// This configuration is used for all outbound HTTP/HTTPS requests
/// that support proxy connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// The proxy URL (e.g., "http://127.0.0.1:7890" or "socks5://127.0.0.1:1080").
    /// Empty string means no proxy.
    #[serde(default)]
    pub url: String,

    /// Optional username for proxy authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Optional password for proxy authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// Whether to bypass proxy for localhost addresses.
    #[serde(default = "default_bypass_localhost")]
    pub bypass_localhost: bool,

    /// List of host patterns to bypass proxy (e.g., "*.local", "192.168.*").
    #[serde(default)]
    pub bypass_hosts: Vec<String>,
}

fn default_bypass_localhost() -> bool {
    true
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            username: None,
            password: None,
            bypass_localhost: default_bypass_localhost(),
            bypass_hosts: Vec::new(),
        }
    }
}

impl ProxyConfig {
    /// Create a new ProxyConfig with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a proxy is configured.
    pub fn is_configured(&self) -> bool {
        !self.url.is_empty()
    }

    /// Set the proxy URL.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Set the proxy username.
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the proxy password.
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set whether to bypass localhost.
    pub fn with_bypass_localhost(mut self, bypass: bool) -> Self {
        self.bypass_localhost = bypass;
        self
    }

    /// Add a host pattern to bypass.
    pub fn with_bypass_host(mut self, host: impl Into<String>) -> Self {
        self.bypass_hosts.push(host.into());
        self
    }

    /// Add multiple host patterns to bypass.
    pub fn with_bypass_hosts(mut self, hosts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.bypass_hosts = hosts.into_iter().map(Into::into).collect();
        self
    }

    /// Get the proxy URL as a string reference.
    pub fn url(&self) -> Option<&str> {
        if self.is_configured() {
            Some(&self.url)
        } else {
            None
        }
    }

    /// Get the proxy username as a string reference.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Get the proxy password as a string reference.
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
}

impl std::fmt::Display for ProxyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_configured() {
            write!(f, "Proxy: {}", self.url)
        } else {
            write!(f, "No proxy configured")
        }
    }
}

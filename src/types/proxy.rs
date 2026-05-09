use serde::{Deserialize, Serialize};

/// Proxy mode: all traffic or only matched domains.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// Route all traffic through the proxy (except bypass_domains).
    Global,
    /// Only route traffic to domains in `proxy_domains` through the proxy.
    Rules,
}

impl Default for ProxyMode {
    fn default() -> Self {
        Self::Global
    }
}

impl std::fmt::Display for ProxyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyMode::Global => write!(f, "global"),
            ProxyMode::Rules => write!(f, "rules"),
        }
    }
}

/// Global proxy configuration for the application.
///
/// Supports two modes:
/// - **Global**: All outbound connections go through the proxy (except bypassed domains).
/// - **Rules**: Only connections to domains listed in `proxy_domains` go through the proxy.
///
/// Example YAML:
/// ```yaml
/// proxy_config:
///   enabled: true
///   url: "socks5://127.0.0.1:1080"
///   mode: "rules"
///   proxy_domains:
///     - "discord.gg"
///     - "discord.com"
///     - "gateway.discord.gg"
///   bypass_domains:
///     - "oapi.dingtalk.com"
///     - "*.local"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Master switch: whether the proxy is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// The proxy URL (e.g., "http://127.0.0.1:7890" or "socks5://127.0.0.1:1080").
    /// Empty string means no proxy.
    #[serde(default)]
    pub url: String,

    /// Proxy mode: "global" or "rules".
    /// - Global: all traffic goes through the proxy (except bypass_domains).
    /// - Rules: only traffic to `proxy_domains` goes through the proxy.
    #[serde(default)]
    pub mode: ProxyMode,

    /// Domains that should be proxied (only used in "rules" mode).
    /// Supports exact matches and wildcard patterns (e.g., "*.discord.gg").
    #[serde(default)]
    pub proxy_domains: Vec<String>,

    /// Domains that should bypass the proxy (used in both modes).
    /// Supports exact matches and wildcard patterns (e.g., "*.local", "192.168.*").
    #[serde(default)]
    pub bypass_domains: Vec<String>,

    /// Optional username for proxy authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Optional password for proxy authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// Whether to bypass proxy for localhost addresses.
    #[serde(default = "default_bypass_localhost")]
    pub bypass_localhost: bool,
}

fn default_bypass_localhost() -> bool {
    true
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: String::new(),
            mode: ProxyMode::Global,
            proxy_domains: Vec::new(),
            bypass_domains: Vec::new(),
            username: None,
            password: None,
            bypass_localhost: default_bypass_localhost(),
        }
    }
}

impl ProxyConfig {
    /// Create a new ProxyConfig with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a proxy is effectively configured and enabled.
    /// Returns true only when `enabled` is true AND `url` is non-empty.
    pub fn is_configured(&self) -> bool {
        self.enabled && !self.url.is_empty()
    }

    /// Determine whether a connection to the given host should go through the proxy.
    ///
    /// - If proxy is not enabled or URL is empty, returns `false`.
    /// - In `Global` mode: returns `true` unless the host matches `bypass_domains` or is localhost
    ///   (when `bypass_localhost` is set).
    /// - In `Rules` mode: returns `true` only if the host matches a pattern in `proxy_domains`
    ///   and does not match `bypass_domains`.
    pub fn should_proxy(&self, host: &str) -> bool {
        if !self.is_configured() {
            return false;
        }

        // Always bypass localhost if configured
        if self.bypass_localhost && is_localhost(host) {
            return false;
        }

        // Check bypass_domains
        if matches_domain_pattern(host, &self.bypass_domains) {
            return false;
        }

        match self.mode {
            ProxyMode::Global => true,
            ProxyMode::Rules => matches_domain_pattern(host, &self.proxy_domains),
        }
    }

    /// Get the effective proxy URL if the proxy is configured.
    /// Returns the URL with credentials embedded if username/password are set.
    pub fn effective_url(&self) -> Option<String> {
        if !self.is_configured() {
            return None;
        }
        Some(self.url.clone())
    }

    /// Set the proxy enabled flag.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the proxy URL.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Set the proxy mode.
    pub fn with_mode(mut self, mode: ProxyMode) -> Self {
        self.mode = mode;
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
            write!(f, "Proxy: {} (mode: {}", self.url, self.mode)?;
            if !self.proxy_domains.is_empty() {
                write!(f, ", domains: {:?}", self.proxy_domains)?;
            }
            if !self.bypass_domains.is_empty() {
                write!(f, ", bypass: {:?}", self.bypass_domains)?;
            }
            write!(f, ")")
        } else {
            write!(f, "Proxy: disabled")
        }
    }
}

/// Check if a hostname refers to localhost.
fn is_localhost(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1" | "0.0.0.0")
        || host.ends_with(".localhost")
        || host.starts_with("127.")
}

/// Check if a host matches any pattern in the list.
/// Supports:
/// - Exact match: `"discord.gg"` matches `discord.gg`
/// - Wildcard prefix: `"*.discord.gg"` matches `gateway.discord.gg`, `cdn.discord.gg`
/// - Wildcard suffix: `"192.168.*"` matches `192.168.1.1`, `192.168.0.100`
/// - Wildcard both: `"*discord*"` matches any host containing "discord"
fn matches_domain_pattern(host: &str, patterns: &[String]) -> bool {
    let host_lower = host.to_lowercase();
    for pattern in patterns {
        let pattern_lower = pattern.to_lowercase();
        if pattern_lower == host_lower {
            return true;
        }
        if pattern_lower.starts_with("*.") {
            // *.discord.gg should match gateway.discord.gg but not discord.gg itself
            let suffix = &pattern_lower[1..]; // ".discord.gg"
            if host_lower.ends_with(suffix) {
                return true;
            }
            // Also match the base domain (discord.gg matches *.discord.gg)
            let base = &pattern_lower[2..]; // "discord.gg"
            if host_lower == base {
                return true;
            }
        } else if pattern_lower.ends_with(".*") {
            // 192.168.* should match 192.168.1.1
            let prefix = &pattern_lower[..pattern_lower.len() - 1]; // "192.168."
            if host_lower.starts_with(prefix) {
                return true;
            }
        } else if pattern_lower.contains('*') {
            // Generic wildcard: treat * as match-any
            if wildcard_match(&pattern_lower, &host_lower) {
                return true;
            }
        }
    }
    false
}

/// Simple wildcard matching: * matches any sequence of characters.
fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let mut dp = vec![vec![false; t.len() + 1]; p.len() + 1];
    dp[0][0] = true;
    for i in 1..=p.len() {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }
    }
    for i in 1..=p.len() {
        for j in 1..=t.len() {
            if p[i - 1] == '*' {
                dp[i][j] = dp[i - 1][j] || dp[i][j - 1] || dp[i - 1][j - 1];
            } else if p[i - 1] == '?' || p[i - 1] == t[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            }
        }
    }
    dp[p.len()][t.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_configured() {
        let mut config = ProxyConfig::default();
        assert!(!config.is_configured());

        config.url = "http://127.0.0.1:7890".to_string();
        // Not enabled, so not configured
        assert!(!config.is_configured());

        config.enabled = true;
        assert!(config.is_configured());
    }

    #[test]
    fn test_should_proxy_global() {
        let config = ProxyConfig {
            enabled: true,
            url: "http://127.0.0.1:7890".to_string(),
            mode: ProxyMode::Global,
            bypass_domains: vec!["oapi.dingtalk.com".to_string()],
            bypass_localhost: true,
            ..Default::default()
        };

        assert!(config.should_proxy("discord.gg"));
        assert!(config.should_proxy("gateway.discord.gg"));
        assert!(!config.should_proxy("oapi.dingtalk.com"));
        assert!(!config.should_proxy("localhost"));
        assert!(!config.should_proxy("127.0.0.1"));
    }

    #[test]
    fn test_should_proxy_rules() {
        let config = ProxyConfig {
            enabled: true,
            url: "socks5://127.0.0.1:1080".to_string(),
            mode: ProxyMode::Rules,
            proxy_domains: vec!["discord.gg".to_string(), "*.discord.com".to_string()],
            bypass_domains: vec![],
            bypass_localhost: true,
            ..Default::default()
        };

        // Only discord.gg and *.discord.com should be proxied
        assert!(config.should_proxy("discord.gg"));
        assert!(config.should_proxy("gateway.discord.com"));
        assert!(config.should_proxy("cdn.discord.com"));
        // discord.com itself should match via *.discord.com
        assert!(config.should_proxy("discord.com"));
        // Non-matching domains should not be proxied
        assert!(!config.should_proxy("oapi.dingtalk.com"));
        assert!(!config.should_proxy("api.openai.com"));
    }

    #[test]
    fn test_should_proxy_disabled() {
        let config = ProxyConfig {
            enabled: false,
            url: "http://127.0.0.1:7890".to_string(),
            ..Default::default()
        };

        assert!(!config.should_proxy("discord.gg"));
    }

    #[test]
    fn test_matches_domain_pattern() {
        assert!(matches_domain_pattern(
            "gateway.discord.gg",
            &["*.discord.gg".to_string()]
        ));
        assert!(matches_domain_pattern(
            "discord.gg",
            &["*.discord.gg".to_string()]
        ));
        assert!(!matches_domain_pattern(
            "notdiscord.gg",
            &["*.discord.gg".to_string()]
        ));
        assert!(matches_domain_pattern(
            "192.168.1.1",
            &["192.168.*".to_string()]
        ));
        assert!(matches_domain_pattern(
            "api.discord.gg",
            &["*discord*".to_string()]
        ));
    }
}

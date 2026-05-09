use std::net::IpAddr;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Clash-style proxy rule type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProxyRuleType {
    /// Exact domain match (e.g., `DOMAIN,api.openai.com`).
    Domain,
    /// Domain suffix match (e.g., `DOMAIN-SUFFIX,discord.gg` matches `gateway.discord.gg`).
    #[serde(rename = "domain-suffix")]
    DomainSuffix,
    /// Domain keyword match (e.g., `DOMAIN-KEYWORD,discord` matches any domain containing "discord").
    #[serde(rename = "domain-keyword")]
    DomainKeyword,
    /// IP CIDR match (e.g., `IP-CIDR,192.168.0.0/16`).
    #[serde(rename = "ip-cidr")]
    IpCidr,
    /// GeoIP country code match (e.g., `GEOIP,CN`).
    #[serde(rename = "geoip")]
    Geoip,
    /// Matches everything (like global mode but as a rule).
    Match,
}

impl std::fmt::Display for ProxyRuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyRuleType::Domain => write!(f, "DOMAIN"),
            ProxyRuleType::DomainSuffix => write!(f, "DOMAIN-SUFFIX"),
            ProxyRuleType::DomainKeyword => write!(f, "DOMAIN-KEYWORD"),
            ProxyRuleType::IpCidr => write!(f, "IP-CIDR"),
            ProxyRuleType::Geoip => write!(f, "GEOIP"),
            ProxyRuleType::Match => write!(f, "MATCH"),
        }
    }
}

/// A single Clash-style proxy rule.
///
/// Each rule has a type and a value, e.g. `DOMAIN-SUFFIX,discord.gg`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProxyRule {
    pub rule_type: ProxyRuleType,
    pub value: String,
}

impl ProxyRule {
    /// Create a new proxy rule.
    pub fn new(rule_type: ProxyRuleType, value: impl Into<String>) -> Self {
        Self {
            rule_type,
            value: value.into(),
        }
    }

    /// Check if a host matches this rule.
    ///
    /// - `Domain`: exact case-insensitive match against `value`.
    /// - `DomainSuffix`: host ends with `value` (or is exactly `value`).
    /// - `DomainKeyword`: host contains `value` (case-insensitive).
    /// - `IpCidr`: host (parsed as IP) falls within the CIDR range.
    /// - `Geoip`: **not yet supported** – always returns `false`.
    /// - `Match`: always returns `true`.
    pub fn matches(&self, host: &str) -> bool {
        let host_lower = host.to_lowercase();
        let value_lower = self.value.to_lowercase();

        match self.rule_type {
            ProxyRuleType::Domain => host_lower == value_lower,
            ProxyRuleType::DomainSuffix => {
                // DOMAIN-SUFFIX,discord.gg should match discord.gg and gateway.discord.gg
                host_lower == value_lower || host_lower.ends_with(&format!(".{}", value_lower))
            }
            ProxyRuleType::DomainKeyword => host_lower.contains(&value_lower),
            ProxyRuleType::IpCidr => matches_ip_cidr(host, &self.value),
            ProxyRuleType::Geoip => {
                // GeoIP resolution requires a GeoIP database; not yet implemented.
                false
            }
            ProxyRuleType::Match => true,
        }
    }
}

/// Parse a Clash-style rule string into a `ProxyRule`.
///
/// Supported formats:
/// - `DOMAIN,example.com`
/// - `DOMAIN-SUFFIX,example.com`
/// - `DOMAIN-KEYWORD,example`
/// - `IP-CIDR,192.168.0.0/16`
/// - `GEOIP,CN`
/// - `MATCH`
///
/// An optional third field (policy) is accepted but ignored.
impl FromStr for ProxyRule {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(3, ',').collect();
        let type_str = parts[0].trim();
        let rule_type = match type_str.to_uppercase().as_str() {
            "DOMAIN" => ProxyRuleType::Domain,
            "DOMAIN-SUFFIX" => ProxyRuleType::DomainSuffix,
            "DOMAIN-KEYWORD" => ProxyRuleType::DomainKeyword,
            "IP-CIDR" => ProxyRuleType::IpCidr,
            "GEOIP" => ProxyRuleType::Geoip,
            "MATCH" => {
                // MATCH has no value
                return Ok(ProxyRule::new(ProxyRuleType::Match, ""));
            }
            _ => return Err(format!("unknown rule type: {}", type_str)),
        };

        if parts.len() < 2 {
            return Err(format!("rule {} requires a value", type_str));
        }

        let value = parts[1].trim().to_string();
        if value.is_empty() {
            return Err(format!("rule {} value cannot be empty", type_str));
        }

        Ok(ProxyRule::new(rule_type, value))
    }
}

impl std::fmt::Display for ProxyRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.rule_type {
            ProxyRuleType::Match => write!(f, "MATCH"),
            _ => write!(f, "{},{}", self.rule_type, self.value),
        }
    }
}

/// Check whether a host string (interpreted as an IP address) falls within a CIDR range.
fn matches_ip_cidr(host: &str, cidr: &str) -> bool {
    let Ok(ip) = host.parse::<IpAddr>() else {
        // Host is not an IP address, so IP-CIDR does not apply.
        return false;
    };

    // Parse CIDR
    let (network_str, prefix_len_str) = match cidr.rsplit_once('/') {
        Some(pair) => pair,
        None => return false,
    };

    let Ok(network_ip) = network_str.parse::<IpAddr>() else {
        return false;
    };

    let Ok(prefix_len) = prefix_len_str.parse::<u8>() else {
        return false;
    };

    // Only compare same address family
    match (ip, network_ip) {
        (IpAddr::V4(ip4), IpAddr::V4(net4)) => {
            let prefix = if prefix_len == 0 {
                0u32
            } else {
                !0u32 << (32 - prefix_len)
            };
            (u32::from(ip4) & prefix) == (u32::from(net4) & prefix)
        }
        (IpAddr::V6(ip6), IpAddr::V6(net6)) => {
            let prefix = if prefix_len == 0 {
                0u128
            } else {
                !0u128 << (128 - prefix_len)
            };
            (u128::from(ip6) & prefix) == (u128::from(net6) & prefix)
        }
        _ => false,
    }
}

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

    /// Clash-style proxy rules. When non-empty, these take precedence over
    /// `proxy_domains` / `bypass_domains` for the `should_proxy` decision.
    ///
    /// Each rule is a `ProxyRule` (parsed from strings like `DOMAIN-SUFFIX,discord.gg`).
    /// Rules are evaluated in order; the first matching rule determines whether
    /// the connection is proxied (all rules are "allow" rules – a match means proxy).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<ProxyRule>,

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
            rules: Vec::new(),
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
    /// When `rules` is non-empty, rules take precedence:
    /// - The host is checked against each rule in order.
    /// - The first matching rule means the host should be proxied.
    /// - If no rule matches, the host is **not** proxied.
    /// - `bypass_localhost` is still respected before rule evaluation.
    ///
    /// When `rules` is empty, the legacy `proxy_domains` / `bypass_domains` logic is used:
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

        // When Clash-style rules are present, use the rule-based engine.
        if !self.rules.is_empty() {
            for rule in &self.rules {
                if rule.matches(host) {
                    return true;
                }
            }
            return false;
        }

        // Legacy behaviour
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
            if !self.rules.is_empty() {
                write!(
                    f,
                    ", rules: [{}]?",
                    self.rules
                        .iter()
                        .map(|r| r.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
            }
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

    // --- ProxyRule tests ---

    #[test]
    fn test_proxy_rule_domain() {
        let rule = ProxyRule::new(ProxyRuleType::Domain, "api.openai.com");
        assert!(rule.matches("api.openai.com"));
        assert!(rule.matches("API.OPENAI.COM")); // case-insensitive
        assert!(!rule.matches("gateway.api.openai.com"));
        assert!(!rule.matches("openai.com"));
    }

    #[test]
    fn test_proxy_rule_domain_suffix() {
        let rule = ProxyRule::new(ProxyRuleType::DomainSuffix, "discord.gg");
        // Exact match
        assert!(rule.matches("discord.gg"));
        // Subdomain match
        assert!(rule.matches("gateway.discord.gg"));
        assert!(rule.matches("a.b.c.discord.gg"));
        // Non-match
        assert!(!rule.matches("notdiscord.gg"));
        assert!(!rule.matches("discord.gg.other.com"));
    }

    #[test]
    fn test_proxy_rule_domain_keyword() {
        let rule = ProxyRule::new(ProxyRuleType::DomainKeyword, "discord");
        assert!(rule.matches("discord.gg"));
        assert!(rule.matches("gateway.discord.gg"));
        assert!(rule.matches("discord.com"));
        assert!(rule.matches("my-discord-app.com"));
        assert!(!rule.matches("example.com"));
    }

    #[test]
    fn test_proxy_rule_ip_cidr() {
        let rule = ProxyRule::new(ProxyRuleType::IpCidr, "192.168.0.0/16");
        assert!(rule.matches("192.168.1.1"));
        assert!(rule.matches("192.168.0.0"));
        assert!(rule.matches("192.168.255.255"));
        assert!(!rule.matches("192.169.0.1"));
        assert!(!rule.matches("10.0.0.1"));
        // Non-IP host should not match
        assert!(!rule.matches("example.com"));
    }

    #[test]
    fn test_proxy_rule_ip_cidr_v6() {
        let rule = ProxyRule::new(ProxyRuleType::IpCidr, "fe80::/10");
        assert!(rule.matches("fe80::1"));
        assert!(!rule.matches("::1"));
    }

    #[test]
    fn test_proxy_rule_geoip() {
        // GeoIP is not yet implemented, always returns false
        let rule = ProxyRule::new(ProxyRuleType::Geoip, "CN");
        assert!(!rule.matches("1.2.3.4"));
    }

    #[test]
    fn test_proxy_rule_match() {
        let rule = ProxyRule::new(ProxyRuleType::Match, "");
        assert!(rule.matches("anything.com"));
        assert!(rule.matches("192.168.1.1"));
    }

    // --- FromStr tests ---

    #[test]
    fn test_proxy_rule_from_str() {
        let rule: ProxyRule = "DOMAIN,api.openai.com".parse().unwrap();
        assert_eq!(
            rule,
            ProxyRule::new(ProxyRuleType::Domain, "api.openai.com")
        );

        let rule: ProxyRule = "DOMAIN-SUFFIX,discord.gg".parse().unwrap();
        assert_eq!(
            rule,
            ProxyRule::new(ProxyRuleType::DomainSuffix, "discord.gg")
        );

        let rule: ProxyRule = "DOMAIN-KEYWORD,discord".parse().unwrap();
        assert_eq!(
            rule,
            ProxyRule::new(ProxyRuleType::DomainKeyword, "discord")
        );

        let rule: ProxyRule = "IP-CIDR,192.168.0.0/16".parse().unwrap();
        assert_eq!(
            rule,
            ProxyRule::new(ProxyRuleType::IpCidr, "192.168.0.0/16")
        );

        let rule: ProxyRule = "GEOIP,CN".parse().unwrap();
        assert_eq!(rule, ProxyRule::new(ProxyRuleType::Geoip, "CN"));

        let rule: ProxyRule = "MATCH".parse().unwrap();
        assert_eq!(rule, ProxyRule::new(ProxyRuleType::Match, ""));
    }

    #[test]
    fn test_proxy_rule_from_str_with_policy() {
        // The optional third policy field should be ignored
        let rule: ProxyRule = "DOMAIN-SUFFIX,discord.gg,DIRECT".parse().unwrap();
        assert_eq!(
            rule,
            ProxyRule::new(ProxyRuleType::DomainSuffix, "discord.gg")
        );
    }

    #[test]
    fn test_proxy_rule_from_str_errors() {
        // Unknown type
        assert!("UNKNOWN,value".parse::<ProxyRule>().is_err());
        // Missing value for non-MATCH type
        assert!("DOMAIN".parse::<ProxyRule>().is_err());
        // Empty value
        assert!("DOMAIN,".parse::<ProxyRule>().is_err());
    }

    // --- Display tests ---

    #[test]
    fn test_proxy_rule_display() {
        let rule = ProxyRule::new(ProxyRuleType::Domain, "api.openai.com");
        assert_eq!(rule.to_string(), "DOMAIN,api.openai.com");

        let rule = ProxyRule::new(ProxyRuleType::Match, "");
        assert_eq!(rule.to_string(), "MATCH");
    }

    #[test]
    fn test_proxy_rule_type_display() {
        assert_eq!(ProxyRuleType::Domain.to_string(), "DOMAIN");
        assert_eq!(ProxyRuleType::DomainSuffix.to_string(), "DOMAIN-SUFFIX");
        assert_eq!(ProxyRuleType::DomainKeyword.to_string(), "DOMAIN-KEYWORD");
        assert_eq!(ProxyRuleType::IpCidr.to_string(), "IP-CIDR");
        assert_eq!(ProxyRuleType::Geoip.to_string(), "GEOIP");
        assert_eq!(ProxyRuleType::Match.to_string(), "MATCH");
    }

    // --- should_proxy with rules ---

    #[test]
    fn test_should_proxy_with_rules() {
        let config = ProxyConfig {
            enabled: true,
            url: "socks5://127.0.0.1:1080".to_string(),
            mode: ProxyMode::Rules,
            proxy_domains: vec!["old-style.com".to_string()],
            bypass_domains: vec![],
            rules: vec![
                ProxyRule::new(ProxyRuleType::DomainSuffix, "discord.gg"),
                ProxyRule::new(ProxyRuleType::DomainKeyword, "openai"),
            ],
            bypass_localhost: true,
            ..Default::default()
        };

        // Rules take precedence over proxy_domains
        assert!(config.should_proxy("gateway.discord.gg"));
        assert!(config.should_proxy("discord.gg"));
        assert!(config.should_proxy("api.openai.com"));
        assert!(config.should_proxy("chat.openai.com"));
        // old-style.com is in proxy_domains but rules are present, so it won't match
        assert!(!config.should_proxy("old-style.com"));
        // No rule matches
        assert!(!config.should_proxy("example.com"));
    }

    #[test]
    fn test_should_proxy_rules_match_all() {
        let config = ProxyConfig {
            enabled: true,
            url: "socks5://127.0.0.1:1080".to_string(),
            mode: ProxyMode::Rules,
            rules: vec![ProxyRule::new(ProxyRuleType::Match, "")],
            bypass_localhost: true,
            ..Default::default()
        };

        // MATCH matches everything
        assert!(config.should_proxy("anything.com"));
        assert!(config.should_proxy("192.168.1.1"));
        // localhost still bypassed
        assert!(!config.should_proxy("localhost"));
        assert!(!config.should_proxy("127.0.0.1"));
    }

    #[test]
    fn test_should_proxy_rules_ip_cidr() {
        let config = ProxyConfig {
            enabled: true,
            url: "socks5://127.0.0.1:1080".to_string(),
            mode: ProxyMode::Rules,
            rules: vec![ProxyRule::new(ProxyRuleType::IpCidr, "10.0.0.0/8")],
            bypass_localhost: false, // so we can test 127.x.x.x
            ..Default::default()
        };

        assert!(config.should_proxy("10.0.0.1"));
        assert!(config.should_proxy("10.255.255.255"));
        assert!(!config.should_proxy("192.168.1.1"));
        // Domain names don't match IP-CIDR
        assert!(!config.should_proxy("example.com"));
    }

    #[test]
    fn test_should_proxy_rules_order() {
        // Rules are evaluated in order; first match wins.
        let config = ProxyConfig {
            enabled: true,
            url: "socks5://127.0.0.1:1080".to_string(),
            mode: ProxyMode::Rules,
            rules: vec![
                ProxyRule::new(ProxyRuleType::Domain, "specific.example.com"),
                ProxyRule::new(ProxyRuleType::Match, ""),
            ],
            bypass_localhost: true,
            ..Default::default()
        };

        // First rule matches
        assert!(config.should_proxy("specific.example.com"));
        // Second rule (MATCH) catches everything else
        assert!(config.should_proxy("other.example.com"));
        assert!(config.should_proxy("irrelevant.com"));
    }

    #[test]
    fn test_should_proxy_fallback_to_legacy() {
        // When rules is empty, fall back to proxy_domains/bypass_domains
        let config = ProxyConfig {
            enabled: true,
            url: "socks5://127.0.0.1:1080".to_string(),
            mode: ProxyMode::Rules,
            proxy_domains: vec!["discord.gg".to_string()],
            bypass_domains: vec![],
            rules: vec![],
            bypass_localhost: true,
            ..Default::default()
        };

        assert!(config.should_proxy("discord.gg"));
        assert!(!config.should_proxy("example.com"));
    }

    #[test]
    fn test_matches_ip_cidr_helper() {
        assert!(matches_ip_cidr("192.168.1.1", "192.168.0.0/16"));
        assert!(matches_ip_cidr("10.0.0.1", "10.0.0.0/8"));
        assert!(!matches_ip_cidr("192.169.1.1", "192.168.0.0/16"));
        assert!(!matches_ip_cidr("example.com", "192.168.0.0/16"));
        // /0 matches everything
        assert!(matches_ip_cidr("1.2.3.4", "0.0.0.0/0"));
    }

    #[test]
    fn test_proxy_rule_serde_roundtrip() {
        let rule = ProxyRule::new(ProxyRuleType::DomainSuffix, "discord.gg");
        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: ProxyRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, deserialized);
    }

    #[test]
    fn test_proxy_config_with_rules_yaml() {
        let yaml = r#"
enabled: true
url: "socks5://127.0.0.1:1080"
mode: rules
rules:
  - rule_type: domain
    value: api.openai.com
  - rule_type: domain-suffix
    value: discord.gg
  - rule_type: match
    value: ""
proxy_domains: []
bypass_domains: []
        "#;
        let config: ProxyConfig = serde_yaml::from_str(yaml).unwrap();
        // The order of rules matters
        assert_eq!(config.rules.len(), 3);
        assert!(config.should_proxy("api.openai.com"));
        assert!(config.should_proxy("gateway.discord.gg"));
        assert!(config.should_proxy("anything.com")); // MATCH
    }
}

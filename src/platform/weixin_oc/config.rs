use serde::{Deserialize, Serialize};

/// Default API base URL for WeChat iLink.
pub const DEFAULT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
/// Default CDN base URL for WeChat media transfers.
pub const CDN_BASE_URL: &str = "https://novac2c.cdn.weixin.qq.com/c2c";

/// Configuration for a WeChat personal (ClawBot) adapter.
///
/// Example YAML:
/// ```yaml
/// platforms:
///   - type: weixin_oc
///     id: my-wechat-bot
///     enable: true
///     # token and account_id will be auto-saved after QR login
///     token: ""
///     account_id: ""
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeixinOcConfig {
    /// Bearer token obtained after QR code login.
    /// If empty, the adapter will start the QR login flow on startup.
    #[serde(default)]
    pub token: Option<String>,

    /// Account ID (ilink_bot_id) obtained after QR login.
    /// E.g. "b0f5860fdecb@im.bot"
    #[serde(default)]
    pub account_id: Option<String>,

    /// iLink API base URL.
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// CDN base URL for media uploads/downloads.
    #[serde(default = "default_cdn_base_url")]
    pub cdn_base_url: String,

    /// QR code polling interval in milliseconds.
    #[serde(default = "default_qr_poll_interval")]
    pub qr_poll_interval_ms: u64,

    /// Long-polling timeout for getUpdates in milliseconds.
    #[serde(default = "default_long_poll_timeout_ms")]
    pub long_poll_timeout_ms: u64,

    /// General API request timeout in milliseconds.
    #[serde(default = "default_api_timeout_ms")]
    pub api_timeout_ms: u64,

    /// Optional proxy URL for HTTP connections.
    #[serde(default)]
    pub proxy_url: Option<String>,
}

fn default_base_url() -> String {
    DEFAULT_BASE_URL.to_string()
}
fn default_cdn_base_url() -> String {
    CDN_BASE_URL.to_string()
}
fn default_qr_poll_interval() -> u64 {
    1000
}
fn default_long_poll_timeout_ms() -> u64 {
    35_000
}
fn default_api_timeout_ms() -> u64 {
    15_000
}

impl Default for WeixinOcConfig {
    fn default() -> Self {
        Self {
            token: None,
            account_id: None,
            base_url: default_base_url(),
            cdn_base_url: default_cdn_base_url(),
            qr_poll_interval_ms: default_qr_poll_interval(),
            long_poll_timeout_ms: default_long_poll_timeout_ms(),
            api_timeout_ms: default_api_timeout_ms(),
            proxy_url: None,
        }
    }
}

use serde::{Deserialize, Serialize};

/// Configuration for a DingTalk Stream-mode robot.
///
/// Example YAML:
/// ```yaml
/// platforms:
///   - type: dingtalk
///     id: my-dingtalk-bot
///     enable: true
///     client_id: "dingxxxxxxxxx"
///     client_secret: "xxxxxxxxx"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingtalkConfig {
    /// The appKey of the DingTalk robot (also called `client_id`).
    pub client_id: String,
    /// The appSecret of the DingTalk robot (also called `client_secret`).
    pub client_secret: String,
    /// Optional proxy URL for HTTP and WebSocket connections.
    /// Supports HTTP and SOCKS5 proxies (e.g., "http://127.0.0.1:7890", "socks5://127.0.0.1:1080").
    #[serde(default)]
    pub proxy_url: Option<String>,
}

/// DingTalk Stream API endpoints.
pub(crate) const ENDPOINT_ACCESS_TOKEN: &str = "https://api.dingtalk.com/v1.0/oauth2/accessToken";
pub(crate) const ENDPOINT_STREAM_OPEN: &str =
    "https://api.dingtalk.com/v1.0/gateway/connections/open";
pub(crate) const ENDPOINT_GROUP_MESSAGE_SEND: &str =
    "https://api.dingtalk.com/v1.0/robot/groupMessages/send";
pub(crate) const ENDPOINT_OTO_MESSAGE_SEND: &str =
    "https://api.dingtalk.com/v1.0/robot/oToMessages/batchSend";
/// DingTalk Stream message topic for bot messages.
pub(crate) const TOPIC_BOT_MESSAGE: &str = "/v1.0/im/bot/messages/get";

/// Subscribe topic for bot messages used when opening a stream connection.
pub(crate) const SUBSCRIPTION_BOT_MESSAGE: &str = "/v1.0/im/bot/messages/get";

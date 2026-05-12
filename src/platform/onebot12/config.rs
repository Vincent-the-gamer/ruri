use serde::{Deserialize, Serialize};

/// OneBot v12 platform adapter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneBot12Config {
    /// The platform name in OneBot standard (e.g., "qq", "telegram", "discord").
    /// See OneBot v12 glossary for format.
    pub platform: String,

    /// The bot's user ID on the platform.
    pub self_user_id: String,

    /// Access token for authentication.
    #[serde(default)]
    pub access_token: Option<String>,

    /// HTTP server configuration.
    #[serde(default)]
    pub http: Option<HttpConfig>,

    /// HTTP Webhook configuration.
    #[serde(default)]
    pub http_webhook: Option<HttpWebhookConfig>,

    /// Forward WebSocket server configuration.
    #[serde(default)]
    pub ws: Option<WsConfig>,

    /// Reverse WebSocket client configuration.
    #[serde(default)]
    pub ws_reverse: Option<WsReverseConfig>,
}

impl OneBot12Config {
    /// Validate the configuration.
    ///
    /// Returns an error if:
    /// - `platform` is empty
    /// - `self_user_id` is empty
    /// - No communication method is configured
    pub fn validate(&self) -> Result<(), String> {
        if self.platform.is_empty() {
            return Err("OneBot12 config: `platform` must not be empty".to_string());
        }
        if self.self_user_id.is_empty() {
            return Err("OneBot12 config: `self_user_id` must not be empty".to_string());
        }
        if self.http.is_none()
            && self.http_webhook.is_none()
            && self.ws.is_none()
            && self.ws_reverse.is_none()
        {
            return Err(
                "OneBot12 config: at least one communication method must be configured \
                 (http, http_webhook, ws, ws_reverse)"
                    .to_string(),
            );
        }
        Ok(())
    }
}

/// HTTP server configuration.
///
/// The OneBot implementation acts as an HTTP server. Action calls are made
/// via HTTP POST, and optionally events can be polled via HTTP GET.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Listen host.
    #[serde(default = "default_host")]
    pub host: String,

    /// Listen port.
    #[serde(default = "default_http_port")]
    pub port: u16,

    /// Whether event polling is enabled on this HTTP server.
    #[serde(default)]
    pub event_enabled: bool,

    /// Event buffer size (0 = unlimited).
    #[serde(default = "default_event_buffer_size")]
    pub event_buffer_size: usize,
}

/// HTTP Webhook configuration.
///
/// The OneBot implementation acts as an HTTP client, pushing events
/// to the configured URL via HTTP POST.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpWebhookConfig {
    /// URL to push events to.
    pub url: String,
}

/// Forward WebSocket server configuration.
///
/// The OneBot implementation acts as a WebSocket server, providing
/// both action and event service over a single WebSocket connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsConfig {
    /// Listen host.
    #[serde(default = "default_host")]
    pub host: String,

    /// Listen port.
    #[serde(default = "default_ws_port")]
    pub port: u16,
}

/// Reverse WebSocket client configuration.
///
/// The OneBot implementation acts as a WebSocket client, connecting
/// to the configured URL for both action and event service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsReverseConfig {
    /// URL to connect to.
    pub url: String,

    /// Reconnection interval in milliseconds.
    #[serde(default = "default_reconnect_interval")]
    pub reconnect_interval: u64,
}

// ---------------------------------------------------------------------------
// Default value helpers
// ---------------------------------------------------------------------------

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_http_port() -> u16 {
    6700
}

fn default_ws_port() -> u16 {
    6701
}

fn default_event_buffer_size() -> usize {
    0
}

fn default_reconnect_interval() -> u64 {
    3000
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ok() {
        let config = OneBot12Config {
            platform: "qq".to_string(),
            self_user_id: "123456".to_string(),
            access_token: None,
            http: Some(HttpConfig {
                host: default_host(),
                port: default_http_port(),
                event_enabled: false,
                event_buffer_size: 0,
            }),
            http_webhook: None,
            ws: None,
            ws_reverse: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_platform() {
        let config = OneBot12Config {
            platform: "".to_string(),
            self_user_id: "123456".to_string(),
            access_token: None,
            http: Some(HttpConfig {
                host: default_host(),
                port: default_http_port(),
                event_enabled: false,
                event_buffer_size: 0,
            }),
            http_webhook: None,
            ws: None,
            ws_reverse: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_empty_self_user_id() {
        let config = OneBot12Config {
            platform: "qq".to_string(),
            self_user_id: "".to_string(),
            access_token: None,
            http: Some(HttpConfig {
                host: default_host(),
                port: default_http_port(),
                event_enabled: false,
                event_buffer_size: 0,
            }),
            http_webhook: None,
            ws: None,
            ws_reverse: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_no_communication_method() {
        let config = OneBot12Config {
            platform: "qq".to_string(),
            self_user_id: "123456".to_string(),
            access_token: None,
            http: None,
            http_webhook: None,
            ws: None,
            ws_reverse: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_deserialize_defaults() {
        let yaml = r#"
platform: "qq"
self_user_id: "123456"
http: {}
"#;
        let config: OneBot12Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.platform, "qq");
        assert_eq!(config.self_user_id, "123456");
        let http = config.http.unwrap();
        assert_eq!(http.host, "0.0.0.0");
        assert_eq!(http.port, 6700);
        assert!(!http.event_enabled);
        assert_eq!(http.event_buffer_size, 0);
    }

    #[test]
    fn test_deserialize_full() {
        let yaml = r#"
platform: "qq"
self_user_id: "123456"
access_token: "mytoken"
http:
  host: "127.0.0.1"
  port: 8000
  event_enabled: true
  event_buffer_size: 10
ws:
  host: "0.0.0.0"
  port: 6701
ws_reverse:
  url: "ws://127.0.0.1:8080/onebot/ws"
  reconnect_interval: 5000
"#;
        let config: OneBot12Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.access_token.as_deref(), Some("mytoken"));
        let http = config.http.unwrap();
        assert_eq!(http.host, "127.0.0.1");
        assert_eq!(http.port, 8000);
        assert!(http.event_enabled);
        assert_eq!(http.event_buffer_size, 10);
        let ws = config.ws.unwrap();
        assert_eq!(ws.port, 6701);
        let ws_reverse = config.ws_reverse.unwrap();
        assert_eq!(ws_reverse.url, "ws://127.0.0.1:8080/onebot/ws");
        assert_eq!(ws_reverse.reconnect_interval, 5000);
        assert!(config.http_webhook.is_none());
    }
}

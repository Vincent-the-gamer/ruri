//! WeChat iLink API client.
//!
//! Implements all HTTP endpoints for the WeChat ClawBot protocol:
//! - getUpdates (long-poll for inbound messages)
//! - sendMessage (text/image/file/video)
//! - getUploadUrl (CDN upload pre-signed URL)
//! - getConfig (typing ticket)
//! - sendTyping (typing indicator)
//! - QR code login flow

use crate::platform::weixin_oc::config::WeixinOcConfig;
use crate::platform::weixin_oc::types::*;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Internal state shared between the adapter and API calls.
#[derive(Debug, Clone)]
pub struct ApiState {
    /// Bearer token obtained after QR login.
    pub token: Option<String>,
    /// Account ID (ilink_bot_id).
    pub account_id: Option<String>,
    /// Sync cursor for getUpdates long-poll.
    pub get_updates_buf: String,
    /// Base URL for the iLink API.
    pub base_url: String,
}

/// WeChat iLink API client.
pub struct WeixinApi {
    http: Client,
    config: WeixinOcConfig,
    state: Arc<RwLock<ApiState>>,
}

impl WeixinApi {
    /// Create a new API client.
    pub fn new(config: WeixinOcConfig) -> Result<Self, String> {
        let http = if let Some(ref proxy_url) = config.proxy_url {
            let proxy = reqwest::Proxy::all(proxy_url)
                .map_err(|e| format!("Invalid proxy URL '{}': {}", proxy_url, e))?;
            Client::builder()
                .proxy(proxy)
                .build()
                .map_err(|e| format!("Failed to build HTTP client with proxy: {}", e))?
        } else {
            Client::new()
        };

        let state = ApiState {
            token: config.token.clone(),
            account_id: config.account_id.clone(),
            get_updates_buf: String::new(),
            base_url: config.base_url.clone(),
        };

        Ok(Self {
            http,
            config,
            state: Arc::new(RwLock::new(state)),
        })
    }

    /// Get a clone of the shared state Arc.
    pub fn state(&self) -> Arc<RwLock<ApiState>> {
        self.state.clone()
    }

    // -----------------------------------------------------------------------
    // QR Login
    // -----------------------------------------------------------------------

    /// Step 1: Request a QR code for login.
    pub async fn qr_login_start(&self) -> anyhow::Result<QrCodeResponse> {
        let url = format!(
            "{}/ilink/bot/get_bot_qrcode?bot_type=3",
            self.config.base_url
        );
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("QR login start failed: status={}, body={}", status, body);
        }
        let qr_resp: QrCodeResponse = resp.json().await?;
        Ok(qr_resp)
    }

    /// Step 2: Poll QR code status until confirmed, expired, or error.
    pub async fn qr_login_wait(
        &self,
        qrcode: &str,
        timeout_ms: u64,
    ) -> anyhow::Result<QrStatusResponse> {
        let url = format!(
            "{}/ilink/bot/get_qrcode_status?qrcode={}",
            self.config.base_url,
            urlencoding::encode(qrcode)
        );
        let resp = self
            .http
            .get(&url)
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .send()
            .await;

        match resp {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    anyhow::bail!("QR status poll failed: body={}", body);
                }
                let status_resp: QrStatusResponse = resp.json().await?;
                Ok(status_resp)
            }
            Err(e) => {
                if e.is_timeout() {
                    // Timeout is normal for long-poll — return "wait"
                    Ok(QrStatusResponse {
                        status: "wait".to_string(),
                        bot_token: None,
                        ilink_bot_id: None,
                        baseurl: None,
                        ilink_user_id: None,
                        redirect_host: None,
                    })
                } else {
                    // Network error, treat as wait and retry
                    tracing::warn!("QR status poll network error, will retry: {}", e);
                    Ok(QrStatusResponse {
                        status: "wait".to_string(),
                        bot_token: None,
                        ilink_bot_id: None,
                        baseurl: None,
                        ilink_user_id: None,
                        redirect_host: None,
                    })
                }
            }
        }
    }

    /// Save the login credentials after successful QR login.
    pub async fn save_login(&self, token: String, account_id: String, base_url: Option<String>) {
        let mut state = self.state.write().await;
        state.token = Some(token);
        state.account_id = Some(account_id);
        if let Some(url) = base_url {
            state.base_url = url;
        }
    }

    // -----------------------------------------------------------------------
    // Core API endpoints
    // -----------------------------------------------------------------------

    /// Long-poll for new messages via getUpdates.
    pub async fn get_updates(&self) -> anyhow::Result<GetUpdatesResp> {
        let (token, base_url, buf) = {
            let state = self.state.read().await;
            (
                state.token.clone().unwrap_or_default(),
                state.base_url.clone(),
                state.get_updates_buf.clone(),
            )
        };

        let body = serde_json::json!({
            "get_updates_buf": buf,
            "base_info": { "channel_version": env!("CARGO_PKG_VERSION") }
        });

        let resp = self
            .post_json(&format!("{}/ilink/bot/getupdates", base_url), &token, &body)
            .timeout(std::time::Duration::from_millis(
                self.config.long_poll_timeout_ms,
            ))
            .send()
            .await;

        match resp {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    // Session timeout (-14) — return error so the caller can re-login
                    tracing::error!("getUpdates failed: status={}, body={}", status, body);
                    anyhow::bail!("getUpdates failed: status={}, body={}", status, body);
                }
                let updates: GetUpdatesResp = resp.json().await?;
                // Save the new sync cursor
                if let Some(ref new_buf) = updates.get_updates_buf {
                    let mut state = self.state.write().await;
                    state.get_updates_buf = new_buf.clone();
                }
                Ok(updates)
            }
            Err(e) => {
                if e.is_timeout() {
                    // Long-poll timeout is normal — return empty
                    let state = self.state.read().await;
                    Ok(GetUpdatesResp {
                        ret: Some(0),
                        msgs: Some(vec![]),
                        get_updates_buf: Some(state.get_updates_buf.clone()),
                        errcode: None,
                        errmsg: None,
                        longpolling_timeout_ms: None,
                    })
                } else {
                    Err(e.into())
                }
            }
        }
    }

    /// Send a text message to a user.
    pub async fn send_text_message(
        &self,
        to_user_id: &str,
        text: &str,
        context_token: Option<&str>,
    ) -> anyhow::Result<()> {
        let (token, base_url) = {
            let state = self.state.read().await;
            (
                state.token.clone().unwrap_or_default(),
                state.base_url.clone(),
            )
        };

        let req = SendMessageReq {
            msg: WeixinMessageSend {
                to_user_id: to_user_id.to_string(),
                context_token: context_token.map(|s| s.to_string()),
                item_list: vec![SendMessageItem::text(text)],
            },
            base_info: Some(BaseInfo {
                channel_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        };

        let body = serde_json::to_value(&req)?;
        let resp = self
            .post_json(
                &format!("{}/ilink/bot/sendmessage", base_url),
                &token,
                &body,
            )
            .timeout(std::time::Duration::from_millis(self.config.api_timeout_ms))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("sendMessage failed: status={}, body={}", status, body);
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Build a POST request with the common WeChat headers.
    fn post_json(
        &self,
        url: &str,
        token: &str,
        body: &serde_json::Value,
    ) -> reqwest::RequestBuilder {
        let uin = {
            let uint32: u32 = rand::random();
            BASE64.encode(uint32.to_string().as_bytes())
        };

        let mut builder = self
            .http
            .post(url)
            .header("Content-Type", "application/json")
            .header("AuthorizationType", "ilink_bot_token")
            .header("X-WECHAT-UIN", &uin)
            .json(body);

        if !token.is_empty() {
            builder = builder.header("Authorization", format!("Bearer {}", token));
        }

        builder
    }
}

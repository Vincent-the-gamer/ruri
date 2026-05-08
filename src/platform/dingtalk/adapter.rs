//! DingTalk platform adapter for Ruri.
//!
//! This adapter uses the DingTalk Stream (WebSocket) mode to receive messages,
//! and the REST API to send replies. It supports:
//! - Group messages and private (1-on-1) messages
//! - Text, markdown, and image replies
//! - Automatic reconnection with exponential backoff
//!
//! # Configuration
//!
//! Each DingTalk bot instance needs a YAML config block like:
//!
//! ```yaml
//! platforms:
//!   - type: dingtalk
//!     id: my-dingtalk-bot       # unique instance ID
//!     enable: true
//!     client_id: "dingxxxxxxxx"   # appKey
//!     client_secret: "xxxxxxxx"   # appSecret
//! ```

use crate::platform::dingtalk::config::{DingtalkConfig, *};
use crate::platform::trait_def::{Platform, PlatformEvent};
use crate::platform::types::{
    MessageType, OutboundContent, OutboundMessage, PlatformMessage, PlatformMetadata,
    PlatformStatus,
};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, watch};

/// DingTalk platform adapter.
pub struct DingtalkAdapter {
    config: DingtalkConfig,
    instance_id: String,
    http: Client,
    status: PlatformStatus,
    /// Access token cache (shared between send operations).
    access_token: Arc<Mutex<String>>,
    /// Channel to signal the stream task to shut down.
    shutdown_tx: Option<watch::Sender<bool>>,
}

impl DingtalkAdapter {
    /// Create a new adapter from an instance ID and JSON config.
    pub fn from_config(instance_id: String, extra: &serde_json::Value) -> Result<Self, String> {
        let config: DingtalkConfig = serde_json::from_value(extra.clone())
            .map_err(|e| format!("Invalid DingTalk config: {}", e))?;

        if config.client_id.is_empty() {
            return Err("DingTalk config missing `client_id`".into());
        }
        if config.client_secret.is_empty() {
            return Err("DingTalk config missing `client_secret`".into());
        }

        Ok(Self {
            config,
            instance_id,
            http: Client::new(),
            status: PlatformStatus::Pending,
            access_token: Arc::new(Mutex::new(String::new())),
            shutdown_tx: None,
        })
    }

    /// Get a cached or fresh access token.
    async fn get_access_token(&self) -> anyhow::Result<String> {
        {
            let token = self.access_token.lock().await;
            if !token.is_empty() {
                return Ok(token.clone());
            }
        }
        self.refresh_access_token().await
    }

    /// Force-refresh the access token.
    async fn refresh_access_token(&self) -> anyhow::Result<String> {
        let payload = serde_json::json!({
            "appKey": self.config.client_id,
            "appSecret": self.config.client_secret,
        });

        let resp = self
            .http
            .post(ENDPOINT_ACCESS_TOKEN)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "DingTalk access token request failed: status={}, body={}",
                status,
                body
            );
        }

        #[derive(serde::Deserialize)]
        struct TokenResp {
            #[serde(rename = "accessToken", default)]
            access_token: String,
            #[serde(rename = "expireIn", default)]
            expire_in: i64,
        }

        let data: TokenResp = resp.json().await?;
        tracing::debug!(
            expire_in = data.expire_in,
            "Refreshed DingTalk access token"
        );

        let mut token = self.access_token.lock().await;
        *token = data.access_token.clone();
        Ok(data.access_token)
    }

    /// Send a group message via the DingTalk REST API.
    async fn send_group_message(
        &self,
        open_conversation_id: &str,
        msg_key: &str,
        msg_param: serde_json::Value,
    ) -> anyhow::Result<()> {
        let access_token = self.get_access_token().await?;

        let payload = serde_json::json!({
            "msgKey": msg_key,
            "msgParam": msg_param.to_string(),
            "openConversationId": open_conversation_id,
            "robotCode": self.config.client_id,
        });

        let resp = self
            .http
            .post(ENDPOINT_GROUP_MESSAGE_SEND)
            .header("Content-Type", "application/json")
            .header("x-acs-dingtalk-access-token", &access_token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            // If token expired, refresh and retry once
            if status.as_u16() == 401 || status.as_u16() == 400 {
                tracing::warn!("DingTalk access token may be expired, refreshing and retrying");
                self.refresh_access_token().await?;
                let access_token = self.get_access_token().await?;
                let resp2 = self
                    .http
                    .post(ENDPOINT_GROUP_MESSAGE_SEND)
                    .header("Content-Type", "application/json")
                    .header("x-acs-dingtalk-access-token", &access_token)
                    .json(&payload)
                    .send()
                    .await?;
                if !resp2.status().is_success() {
                    let status2 = resp2.status();
                    let body2 = resp2.text().await.unwrap_or_default();
                    anyhow::bail!(
                        "DingTalk group message send failed after retry: status={}, body={}",
                        status2,
                        body2
                    );
                }
                return Ok(());
            }
            anyhow::bail!(
                "DingTalk group message send failed: status={}, body={}",
                status,
                body
            );
        }

        Ok(())
    }

    /// Send a private (1-on-1) message via the DingTalk REST API.
    async fn send_private_message(
        &self,
        staff_id: &str,
        msg_key: &str,
        msg_param: serde_json::Value,
    ) -> anyhow::Result<()> {
        let access_token = self.get_access_token().await?;

        let payload = serde_json::json!({
            "robotCode": self.config.client_id,
            "userIds": [staff_id],
            "msgKey": msg_key,
            "msgParam": msg_param.to_string(),
        });

        let resp = self
            .http
            .post(ENDPOINT_OTO_MESSAGE_SEND)
            .header("Content-Type", "application/json")
            .header("x-acs-dingtalk-access-token", &access_token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            // If token expired, refresh and retry once
            if status.as_u16() == 401 || status.as_u16() == 400 {
                tracing::warn!("DingTalk access token may be expired, refreshing and retrying");
                self.refresh_access_token().await?;
                let access_token = self.get_access_token().await?;
                let resp2 = self
                    .http
                    .post(ENDPOINT_OTO_MESSAGE_SEND)
                    .header("Content-Type", "application/json")
                    .header("x-acs-dingtalk-access-token", &access_token)
                    .json(&payload)
                    .send()
                    .await?;
                if !resp2.status().is_success() {
                    let status2 = resp2.status();
                    let body2 = resp2.text().await.unwrap_or_default();
                    anyhow::bail!(
                        "DingTalk private message send failed after retry: status={}, body={}",
                        status2,
                        body2
                    );
                }
                return Ok(());
            }
            anyhow::bail!(
                "DingTalk private message send failed: status={}, body={}",
                status,
                body
            );
        }

        Ok(())
    }
}

#[async_trait]
impl Platform for DingtalkAdapter {
    fn meta(&self) -> PlatformMetadata {
        PlatformMetadata {
            name: "dingtalk".to_string(),
            description: "钉钉机器人官方 Stream API 适配器".to_string(),
            id: self.instance_id.clone(),
            support_streaming_message: false,
        }
    }

    async fn run(&mut self, event_sender: mpsc::Sender<PlatformEvent>) -> anyhow::Result<()> {
        let config = self.config.clone();
        let instance_id = self.instance_id.clone();

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        self.status = PlatformStatus::Running;

        // The stream module produces PlatformMessage items;
        // we bridge them into PlatformEvent::Message before forwarding.
        let (msg_sender, mut msg_receiver) = mpsc::channel::<PlatformMessage>(256);

        // Spawn the bridge task that wraps PlatformMessage → PlatformEvent
        tokio::spawn(async move {
            while let Some(msg) = msg_receiver.recv().await {
                let event = PlatformEvent::Message(msg);
                if event_sender.send(event).await.is_err() {
                    break;
                }
            }
        });

        // Spawn the stream connection as a background task
        tokio::spawn(async move {
            if let Err(e) = crate::platform::dingtalk::stream::run_dingtalk_stream(
                instance_id.clone(),
                config,
                msg_sender,
                shutdown_rx,
            )
            .await
            {
                tracing::error!(
                    error = %e,
                    platform_id = %instance_id,
                    "DingTalk stream task errored"
                );
            }
        });

        tracing::info!(
            platform_id = %self.instance_id,
            "DingTalk adapter started in Stream mode"
        );

        Ok(())
    }

    async fn terminate(&mut self) -> anyhow::Result<()> {
        self.status = PlatformStatus::Stopped;
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        tracing::info!(platform_id = %self.instance_id, "DingTalk adapter terminated");
        Ok(())
    }

    async fn send_message(&self, message: OutboundMessage) -> anyhow::Result<()> {
        let (msg_key, msg_param) = match &message.content {
            OutboundContent::Text { content } => {
                ("sampleText", serde_json::json!({ "content": content }))
            }
            OutboundContent::Markdown { title, text } => (
                "sampleMarkdown",
                serde_json::json!({ "title": title, "text": text }),
            ),
            OutboundContent::Image { photo_url } => (
                "sampleImageMsg",
                serde_json::json!({ "photoURL": photo_url }),
            ),
            OutboundContent::File {
                media_id,
                file_name,
                file_type,
            } => (
                "sampleFile",
                serde_json::json!({
                    "mediaId": media_id,
                    "fileName": file_name,
                    "fileType": file_type,
                }),
            ),
        };

        match message.target_type {
            MessageType::GroupMessage => {
                self.send_group_message(&message.target_id, msg_key, msg_param)
                    .await
            }
            MessageType::FriendMessage => {
                self.send_private_message(&message.target_id, msg_key, msg_param)
                    .await
            }
        }
    }

    fn status(&self) -> PlatformStatus {
        self.status
    }

    fn platform_type(&self) -> &str {
        "dingtalk"
    }
}

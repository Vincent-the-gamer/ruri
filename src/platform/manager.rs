use crate::platform::dingtalk::DingtalkAdapter;
use crate::platform::discord::DiscordAdapter;
use crate::platform::trait_def::{Platform, PlatformEvent};
use crate::platform::types::{MessageType, OutboundMessage, PlatformStatus};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Configuration for a single platform instance.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlatformInstanceConfig {
    /// Platform type, e.g. "dingtalk".
    #[serde(rename = "type")]
    pub platform_type: String,
    /// Instance ID (unique across all platforms).
    #[serde(default)]
    pub id: String,
    /// Platform-specific configuration (passed to the adapter).
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Configuration file that can hold multiple platform instances.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PlatformConfigFile {
    /// List of platform instance configs.
    pub platforms: Vec<PlatformInstanceConfig>,
}

/// Manages multiple platform adapter instances.
///
/// Each instance is identified by its `id` and backed by a
/// configuration block. The manager:
/// - Starts / stops adapter instances
/// - Routes inbound [`PlatformEvent`]s through a single channel
/// - Provides status queries
pub struct PlatformManager {
    /// Active adapters, keyed by instance ID.
    adapters: HashMap<String, Box<dyn Platform>>,
    /// Shared sender — cloned into each adapter.
    /// This sender is stable and never changes, so dynamically added adapters
    /// can use the same channel as the initial ones.
    event_sender: mpsc::Sender<PlatformEvent>,
    /// The event receiver. Should be taken once at startup via [`take_event_receiver`].
    event_receiver: Option<mpsc::Receiver<PlatformEvent>>,
}

impl PlatformManager {
    /// Create a new, empty manager.
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::channel(256);
        Self {
            adapters: HashMap::new(),
            event_sender,
            event_receiver: Some(event_receiver),
        }
    }

    /// Build a platform adapter from a single instance config.
    pub fn build_adapter(config: &PlatformInstanceConfig) -> Result<Box<dyn Platform>, String> {
        match config.platform_type.as_str() {
            "dingtalk" => {
                let adapter = DingtalkAdapter::from_config(config.id.clone(), &config.extra)
                    .map_err(|e| format!("Failed to create Dingtalk adapter: {e}"))?;
                Ok(Box::new(adapter))
            }
            "discord" => {
                let adapter = DiscordAdapter::from_config(config.id.clone(), &config.extra)
                    .map_err(|e| format!("Failed to create Discord adapter: {e}"))?;
                Ok(Box::new(adapter))
            }
            other => Err(format!("Unknown platform type: {}", other)),
        }
    }

    /// Add and start a platform adapter from config.
    pub async fn add_platform(&mut self, config: PlatformInstanceConfig) -> Result<(), String> {
        let instance_id = if config.id.is_empty() {
            config.platform_type.clone()
        } else {
            config.id.clone()
        };

        let mut adapter = Self::build_adapter(&PlatformInstanceConfig {
            id: instance_id.clone(),
            ..config
        })?;

        tracing::info!(
            platform_id = %instance_id,
            platform_type = %adapter.platform_type(),
            "Starting platform adapter"
        );

        let sender = self.event_sender.clone();
        adapter
            .run(sender)
            .await
            .map_err(|e| format!("Failed to start platform {}: {}", instance_id, e))?;

        self.adapters.insert(instance_id, adapter);
        Ok(())
    }

    /// Load platforms from a YAML configuration file.
    pub async fn load_from_file(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let config: PlatformConfigFile = serde_yaml::from_str(&content)?;

        for platform_config in config.platforms {
            if let Err(e) = self.add_platform(platform_config).await {
                tracing::error!("Failed to add platform: {}", e);
            }
        }

        Ok(())
    }

    /// Reload all platforms from the config file, stopping existing ones first.
    /// This is used for hot-reload when the config file changes.
    pub async fn reload_from_file(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        tracing::info!("Reloading platform configs from: {:?}", path);

        // Stop all existing adapters
        self.shutdown_all().await;

        // Reload
        self.load_from_file(path).await?;

        tracing::info!("Platform configs reloaded successfully");
        Ok(())
    }

    /// Restart a platform adapter by stopping and then starting it.
    pub async fn restart_platform(&mut self, config: PlatformInstanceConfig) -> Result<(), String> {
        let instance_id = if config.id.is_empty() {
            config.platform_type.clone()
        } else {
            config.id.clone()
        };

        // Stop the existing adapter if it exists
        if self.adapters.contains_key(&instance_id) {
            if let Err(e) = self.remove_platform(&instance_id).await {
                tracing::warn!(
                    platform_id = %instance_id,
                    error = %e,
                    "Failed to stop platform during restart"
                );
            }
        }

        // Start the adapter with new config
        self.add_platform(config).await
    }

    /// Take the event receiver. This should be called once at startup.
    /// Returns `None` if the receiver has already been taken.
    pub fn take_event_receiver(&mut self) -> Option<mpsc::Receiver<PlatformEvent>> {
        self.event_receiver.take()
    }

    /// Get a clone of the event sender.
    pub fn event_sender(&self) -> mpsc::Sender<PlatformEvent> {
        self.event_sender.clone()
    }

    /// Get a reference to a platform adapter by instance ID.
    pub fn get_adapter(&self, id: &str) -> Option<&dyn Platform> {
        self.adapters.get(id).map(|b| b.as_ref())
    }

    /// Check whether a platform adapter with the given ID is running.
    pub fn is_running(&self, id: &str) -> bool {
        self.adapters.contains_key(id)
    }

    /// Send a message through a specific platform adapter.
    pub async fn send_to_platform(
        &self,
        platform_id: &str,
        message: OutboundMessage,
    ) -> anyhow::Result<()> {
        if let Some(adapter) = self.adapters.get(platform_id) {
            adapter.send_message(message).await
        } else {
            anyhow::bail!("Platform adapter '{}' not found", platform_id)
        }
    }

    /// Send a text reply through a specific platform adapter.
    pub async fn send_text_to_platform(
        &self,
        platform_id: &str,
        target_type: MessageType,
        target_id: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        if let Some(adapter) = self.adapters.get(platform_id) {
            adapter.send_text(target_type, target_id, text).await
        } else {
            anyhow::bail!("Platform adapter '{}' not found", platform_id)
        }
    }

    /// List all adapter statuses.
    pub fn statuses(&self) -> Vec<(String, PlatformStatus)> {
        self.adapters
            .iter()
            .map(|(id, adapter)| (id.clone(), adapter.status()))
            .collect()
    }

    /// Stop and remove a platform by ID.
    pub async fn remove_platform(&mut self, id: &str) -> anyhow::Result<()> {
        if let Some(mut adapter) = self.adapters.remove(id) {
            adapter.terminate().await?;
        }
        Ok(())
    }

    /// Stop all platforms.
    pub async fn shutdown_all(&mut self) {
        for (id, mut adapter) in self.adapters.drain() {
            tracing::info!(platform_id = %id, "Stopping platform adapter");
            if let Err(e) = adapter.terminate().await {
                tracing::error!(platform_id = %id, error = %e, "Failed to terminate platform adapter");
            }
        }
    }

    /// Number of active adapters.
    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    /// Whether there are no adapters.
    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}

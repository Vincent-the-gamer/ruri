use crate::provider::{Provider, ProviderError};
use crate::types::{ChatRequest, ChatResponse, StreamEvent};
use futures_util::stream::BoxStream;
use std::time::Duration;

/// Configuration for the HTTP transport layer.
#[derive(Debug, Clone)]
pub struct HttpTransportConfig {
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Base delay between retries in milliseconds (uses exponential backoff).
    pub retry_base_delay_ms: u64,
    /// Whether to retry on rate limit errors (429).
    pub retry_on_rate_limit: bool,
    /// Whether to retry on server errors (5xx).
    pub retry_on_server_error: bool,
}

impl Default for HttpTransportConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 120,
            max_retries: 3,
            retry_base_delay_ms: 1000,
            retry_on_rate_limit: true,
            retry_on_server_error: true,
        }
    }
}

/// HTTP transport layer that wraps a Provider with retry logic and timeout handling.
///
/// The transport layer sits between the Agent and the Provider,
/// handling cross-cutting concerns like:
/// - Request timeouts
/// - Automatic retries with exponential backoff
/// - Request/response logging
pub struct HttpTransport {
    provider: Box<dyn Provider>,
    config: HttpTransportConfig,
}

impl HttpTransport {
    pub fn with_default_config(provider: Box<dyn Provider>) -> Self {
        Self {
            provider,
            config: HttpTransportConfig::default(),
        }
    }

    /// Send a chat request through the transport layer.
    pub async fn send(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let delay = self.config.retry_base_delay_ms * 2u64.pow(attempt - 1);
                tracing::info!(
                    attempt = attempt,
                    delay_ms = delay,
                    "Retrying request after failure"
                );
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            match self.send_once(&request).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    tracing::warn!(
                        attempt = attempt,
                        error = %error,
                        "Request failed"
                    );

                    if self.should_retry(&error) {
                        last_error = Some(error);
                        continue;
                    }

                    return Err(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| ProviderError::Custom("All retries exhausted".into())))
    }

    /// Send a single request attempt with timeout.
    async fn send_once(&self, request: &ChatRequest) -> Result<ChatResponse, ProviderError> {
        let result = tokio::time::timeout(
            Duration::from_secs(self.config.timeout_secs),
            self.provider.chat(request.clone()),
        )
        .await;

        match result {
            Ok(response) => response,
            Err(_) => Err(ProviderError::Timeout),
        }
    }

    /// Determine whether an error should trigger a retry.
    fn should_retry(&self, error: &ProviderError) -> bool {
        match error {
            ProviderError::RateLimitExceeded => self.config.retry_on_rate_limit,
            ProviderError::HttpError(e) => {
                if e.is_timeout() {
                    return true;
                }
                if let Some(status) = e.status()
                    && status.is_server_error()
                {
                    return self.config.retry_on_server_error;
                }
                false
            }
            ProviderError::ApiError { status, .. } => {
                if *status == 429 {
                    return self.config.retry_on_rate_limit;
                }
                if *status >= 500 {
                    return self.config.retry_on_server_error;
                }
                false
            }
            ProviderError::Timeout => true,
            _ => false,
        }
    }

    /// Return the provider name.
    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    /// Return the default model for the underlying provider.
    pub fn default_model(&self) -> &str {
        self.provider.default_model()
    }

    /// Whether the underlying provider supports multimodal (image) content.
    pub fn supports_multimodal(&self) -> bool {
        self.provider.supports_multimodal()
    }

    /// Send a chat request and return a stream of events.
    ///
    /// Unlike [`send`], this does not use retry logic because the response
    /// is a long-lived SSE stream that must not be duplicated.
    pub fn send_stream(
        &self,
        request: ChatRequest,
    ) -> BoxStream<'_, Result<StreamEvent, ProviderError>> {
        self.provider.chat_stream(request)
    }
}

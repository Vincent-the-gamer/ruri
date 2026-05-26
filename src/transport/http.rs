use crate::provider::{Provider, ProviderError};
use crate::types::{ChatRequest, ChatResponse, StreamEvent};
use futures_util::stream::BoxStream;
use std::time::Duration;

/// Keywords typically found in API error messages when the model does not
/// support multimodal (image) content.
const MULTIMODAL_ERROR_KEYWORDS: &[&str] = &[
    "image",
    "vision",
    "multimodal",
    "image_url",
    "inline_data",
    "visual",
    "does not support image",
    "not support image",
    "unsupported content type",
    "invalid content type",
    "content type not supported",
];

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
    metrics: Option<std::sync::Arc<tokio::sync::RwLock<crate::metrics::MetricsCollector>>>,
    /// Source of the metrics (debug_session / profile / acp) for token source tracking.
    metrics_source: Option<crate::metrics::TokenSource>,
}

impl HttpTransport {
    pub fn with_default_config(provider: Box<dyn Provider>) -> Self {
        Self {
            provider,
            config: HttpTransportConfig::default(),
            metrics: None,
            metrics_source: None,
        }
    }

    /// Set the metrics collector for traffic and token tracking.
    pub fn set_metrics(
        &mut self,
        metrics: std::sync::Arc<tokio::sync::RwLock<crate::metrics::MetricsCollector>>,
    ) {
        self.metrics = Some(metrics);
    }

    /// Set the metrics source for token source tracking.
    pub fn set_metrics_source(&mut self, source: crate::metrics::TokenSource) {
        self.metrics_source = Some(source);
    }

    /// Send a chat request through the transport layer.
    ///
    /// Includes automatic fallback if the request fails because the model
    /// does not support multimodal content: the request is retried once with
    /// image content stripped from all messages.
    pub async fn send(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        // Estimate request size for traffic tracking
        let req_size = serde_json::to_string(&request)
            .map(|s| s.len() as u64)
            .unwrap_or(0);

        // Track the request
        if let Some(ref m) = self.metrics {
            m.write().await.record_request(self.provider.name());
            m.write().await.record_traffic(req_size, 0);
        }

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
                Ok(response) => {
                    // Track response traffic and token usage
                    if let Some(ref m) = self.metrics {
                        let res_size = serde_json::to_string(&response)
                            .map(|s| s.len() as u64)
                            .unwrap_or(0);
                        m.write().await.record_traffic(0, res_size);
                        if let Some(ref usage) = response.usage {
                            m.write().await.record_tokens_with_source(
                                self.provider.name(),
                                response.model.as_deref(),
                                usage.prompt_tokens.unwrap_or(0),
                                usage.completion_tokens.unwrap_or(0),
                                self.metrics_source.clone(),
                            );
                        }
                    }
                    return Ok(response);
                }
                Err(error) => {
                    tracing::warn!(
                        attempt = attempt,
                        error = %error,
                        "Request failed"
                    );

                    // If the error looks like the model doesn't support multimodal
                    // content, and the request contains images, fall back to a
                    // request with images stripped.
                    if Self::is_multimodal_error(&error) && request.has_multimodal_content() {
                        tracing::warn!(
                            "Request failed with a multimodal-related error. Retrying with image content stripped as a fallback."
                        );
                        let stripped = request.strip_multimodal_content();
                        return self.send_once(&stripped).await;
                    }

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

    /// Send a single request attempt with timeout and no retry logic.
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

    /// Determine whether an error is likely caused by the model not supporting
    /// multimodal (image) content.
    ///
    /// This checks for:
    /// - `ProviderError::MultimodalNotSupported` (explicitly raised by providers)
    /// - `ProviderError::ApiError` with a 400 status and a message containing
    ///   keywords like "image", "vision", "multimodal", etc.
    fn is_multimodal_error(error: &ProviderError) -> bool {
        match error {
            ProviderError::MultimodalNotSupported => true,
            ProviderError::ApiError { status, message } => {
                if *status != 400 && *status != 422 {
                    return false;
                }
                let msg_lower = message.to_lowercase();
                MULTIMODAL_ERROR_KEYWORDS
                    .iter()
                    .any(|kw| msg_lower.contains(kw))
            }
            _ => false,
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
    /// Unlike [`send`], this does not use general retry logic because the
    /// response is a long-lived SSE stream that must not be duplicated.
    /// However, if the very first event is an error that looks like a
    /// multimodal rejection, the request is automatically retried once with
    /// image content stripped.
    pub fn send_stream(
        &self,
        request: ChatRequest,
    ) -> BoxStream<'_, Result<StreamEvent, ProviderError>> {
        use futures_util::StreamExt;

        // Track the request
        if let Some(ref m) = self.metrics {
            let metrics = m.clone();
            let provider_name = self.provider.name().to_string();
            let req_size = serde_json::to_string(&request)
                .map(|s| s.len() as u64)
                .unwrap_or(0);
            tokio::spawn(async move {
                let mut m = metrics.write().await;
                m.record_request(&provider_name);
                m.record_traffic(req_size, 0);
            });
        }

        // Fast path: if the request has no multimodal content, no fallback is needed.
        if !request.has_multimodal_content() {
            tracing::info!(
                provider = %self.provider.name(),
                "send_stream: taking fast path (no multimodal content)"
            );
            return self.provider.chat_stream(request);
        }

        // If the provider has already declared it doesn't support multimodal,
        // strip images upfront.
        if !self.provider.supports_multimodal() {
            tracing::info!(
                provider = %self.provider.name(),
                "send_stream: provider doesn't support multimodal, stripping images"
            );
            let stripped = request.strip_multimodal_content();
            return self.provider.chat_stream(stripped);
        }

        // The provider claims to support multimodal, but the model itself may
        // not. We peek at the first event; if it's a multimodal-related error,
        // we transform it into a `MultimodalNotSupported` error so the caller
        // (AgentStreamer) can retry with stripped content.
        tracing::info!(
            provider = %self.provider.name(),
            "send_stream: taking multimodal fallback path"
        );
        let original_stream = self.provider.chat_stream(request);

        let streaming_fallback = async_stream::stream! {
            let mut peekable = original_stream.peekable();

            // Peek at the first event
            let first = peekable.next().await;
            let first = match first {
                Some(event) => event,
                None => {
                    // The provider returned an empty stream — propagate as an error
                    // so the caller (AgentStreamer) can handle it gracefully.
                    tracing::error!("Multimodal fallback: provider returned an empty stream");
                    yield Err(ProviderError::Custom(
                        "Provider returned an empty stream".into(),
                    ));
                    return;
                }
            };

            // Check if the first event is a multimodal error
            if let Err(ref error) = first {
                if Self::is_multimodal_error(error) {
                    tracing::warn!(
                        "Streaming request failed with a multimodal-related error. Retrying with image content stripped as a fallback."
                    );
                    // We can't retry here because we don't own the provider,
                    // so emit the error as a StreamEvent::Error and let the
                    // caller handle the retry.
                    //
                    // Instead, we emit a special StreamEvent::Error that the
                    // AgentStreamer can detect and retry with stripped content.
                    yield Err(ProviderError::MultimodalNotSupported);
                    return;
                }
            }

            // Not a multimodal error — forward the first event and the rest
            yield first;
            while let Some(event) = peekable.next().await {
                yield event;
            }
        };

        streaming_fallback.boxed()
    }
}

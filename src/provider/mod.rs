pub mod anthropic;
pub mod gemini;
pub mod openai;

use crate::types::{ChatRequest, ChatResponse, StreamEvent};
use async_stream;
use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;

/// Error type for provider operations.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Request timeout")]
    Timeout,

    /// The provider rejected the request because it does not support multimodal
    /// (image) content. The caller may retry after stripping image content.
    #[error("Multimodal content not supported by the model")]
    MultimodalNotSupported,

    #[error("Custom error: {0}")]
    Custom(String),
}

/// A provider that can send chat completion requests to an AI model API.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Send a chat completion request and return the response.
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;

    /// Return the name of this provider.
    fn name(&self) -> &str;

    /// Return the default model used by this provider.
    fn default_model(&self) -> &str;

    /// Whether this provider supports multimodal (image) content.
    ///
    /// Returns `true` by default. Providers backed by servers that don't
    /// support multimodal processing (e.g., llama.cpp without `--multimodal`)
    /// should override this to return `false` so that image content parts are
    /// stripped from requests before they are sent.
    fn supports_multimodal(&self) -> bool {
        true
    }

    /// Set an HTTP proxy for this provider's client.
    ///
    /// If the proxy URL is invalid, the existing client is left unchanged.
    /// When both `username` and `password` are provided, proxy basic auth is configured.
    fn set_proxy(&mut self, _proxy_url: &str, _username: Option<&str>, _password: Option<&str>) {}

    /// Send a chat completion request and return a stream of events.
    ///
    /// The default implementation falls back to the non-streaming [`chat`] method
    /// and emits a single `ContentDelta` followed by `Done`.
    /// Providers that support streaming should override this to send
    /// incremental `ContentDelta` events as tokens arrive.
    fn chat_stream<'a>(
        &'a self,
        request: ChatRequest,
    ) -> BoxStream<'a, Result<StreamEvent, ProviderError>> {
        // Default: use non-streaming chat and emit the full response as a single delta
        let stream = async_stream::stream! {
            match self.chat(request).await {
                Ok(response) => {
                    // Extract content from the first choice
                    if let Some(choice) = response.choices.first() {
                        if let Some(content) = &choice.message.content {
                            let text = content.as_text_full().unwrap_or_default();
                            if !text.is_empty() {
                                yield Ok(StreamEvent::ContentDelta { delta: text });
                            }
                        }
                    }

                    yield Ok(StreamEvent::Done {
                        usage: response.usage.map(|u| crate::types::StreamUsage {
                            prompt_tokens: u.prompt_tokens.unwrap_or(0),
                            completion_tokens: u.completion_tokens.unwrap_or(0),
                        }),
                    });
                }
                Err(e) => {
                    yield Err(e);
                }
            }
        };
        stream.boxed()
    }
}

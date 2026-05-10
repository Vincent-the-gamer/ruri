pub mod anthropic;
pub mod custom;
pub mod openai;

use crate::types::{ChatRequest, ChatResponse};
use async_trait::async_trait;

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
}

use crate::provider::{Provider, ProviderError};
use crate::types::{ChatRequest, ChatResponse};
use async_trait::async_trait;

/// LM Studio provider for local LLM inference.
///
/// LM Studio exposes an OpenAI-compatible API server, typically running on
/// `http://localhost:1234/v1`. This provider wraps the OpenAI-compatible API
/// with LM Studio-specific defaults and configuration options.
///
/// # Example
///
/// ```rust,ignore
/// use ruri::provider::lm_studio::LmStudioProvider;
///
/// // Use builder pattern
/// let provider = LmStudioProvider::builder()
///     .port(8080)
///     .default_model("llama-3.1-8b")
///     .build();
///
/// // With API key (if LM Studio is configured to require one)
/// let provider = LmStudioProvider::builder()
///     .api_key("my-secret-key")
///     .default_model("llama-3.1-8b")
///     .build();
/// ```
pub struct LmStudioProvider {
    /// Internal OpenAI-compatible provider.
    inner: crate::provider::openai::OpenAIProvider,
}

/// Builder for `LmStudioProvider`.
pub struct LmStudioProviderBuilder {
    host: String,
    port: u16,
    api_key: Option<String>,
    default_model: String,
}

impl LmStudioProviderBuilder {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self {
            host: "localhost".into(),
            port: 1234,
            api_key: None,
            default_model: "local-model".into(),
        }
    }

    /// Set the host for the LM Studio server.
    ///
    /// Default: `localhost`
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Set the port for the LM Studio server.
    ///
    /// Default: `1234`
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the API key (optional).
    ///
    /// LM Studio does not require an API key by default, but you can configure
    /// one in LM Studio's settings.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the default model name.
    ///
    /// This should match the model name as shown in LM Studio.
    /// Default: `local-model`
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Build the `LmStudioProvider`.
    pub fn build(self) -> LmStudioProvider {
        let base_url = format!("http://{}:{}", self.host, self.port);

        let inner = crate::provider::openai::OpenAIProvider::new(
            &base_url,
            self.api_key,
            &self.default_model,
        );

        LmStudioProvider { inner }
    }
}

impl Default for LmStudioProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LmStudioProvider {
    /// Create a builder for `LmStudioProvider`.
    pub fn builder() -> LmStudioProviderBuilder {
        LmStudioProviderBuilder::new()
    }
}

#[async_trait]
impl Provider for LmStudioProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        self.inner.chat(request).await
    }

    fn name(&self) -> &str {
        "lm_studio"
    }

    fn default_model(&self) -> &str {
        self.inner.default_model()
    }
}

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
/// // Use defaults (localhost:1234)
/// let provider = LmStudioProvider::new("llama-3.1-8b");
///
/// // Custom port
/// let provider = LmStudioProvider::builder()
///     .port(8080)
///     .default_model("mistral-7b")
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
    /// Configured port for LM Studio server.
    port: u16,
}

/// Builder for `LmStudioProvider`.
pub struct LmStudioProviderBuilder {
    host: String,
    port: u16,
    api_key: Option<String>,
    default_model: String,
    extra_headers: Vec<(String, String)>,
}

impl LmStudioProviderBuilder {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self {
            host: "localhost".into(),
            port: 1234,
            api_key: None,
            default_model: "local-model".into(),
            extra_headers: Vec::new(),
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

    /// Add a custom header to every request.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.push((key.into(), value.into()));
        self
    }

    /// Build the `LmStudioProvider`.
    pub fn build(self) -> LmStudioProvider {
        let base_url = format!("http://{}:{}", self.host, self.port);

        let mut inner = crate::provider::openai::OpenAIProvider::new(
            &base_url,
            self.api_key,
            &self.default_model,
        );

        for (key, value) in self.extra_headers {
            inner = inner.with_header(key, value);
        }

        LmStudioProvider {
            inner,
            port: self.port,
        }
    }
}

impl Default for LmStudioProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LmStudioProvider {
    /// Create a new `LmStudioProvider` with the default model.
    ///
    /// Uses the default host (`localhost`) and port (`1234`).
    pub fn new(default_model: impl Into<String>) -> Self {
        Self::builder().default_model(default_model).build()
    }

    /// Create a new `LmStudioProvider` with a custom port.
    pub fn with_port(default_model: impl Into<String>, port: u16) -> Self {
        Self::builder()
            .port(port)
            .default_model(default_model)
            .build()
    }

    /// Create a new `LmStudioProvider` with a custom host and port.
    pub fn with_host_and_port(
        default_model: impl Into<String>,
        host: impl Into<String>,
        port: u16,
    ) -> Self {
        Self::builder()
            .host(host)
            .port(port)
            .default_model(default_model)
            .build()
    }

    /// Create a builder for `LmStudioProvider`.
    pub fn builder() -> LmStudioProviderBuilder {
        LmStudioProviderBuilder::new()
    }

    /// Get the configured port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get a reference to the inner OpenAI-compatible provider.
    pub fn inner(&self) -> &crate::provider::openai::OpenAIProvider {
        &self.inner
    }

    /// Check if the LM Studio server is reachable.
    ///
    /// Sends a lightweight request to the `/v1/models` endpoint to verify
    /// that the server is running and accessible.
    pub async fn ping(&self) -> Result<bool, ProviderError> {
        let url = format!("http://localhost:{}/v1/models", self.port);
        let client = reqwest::Client::new();

        match client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// List available models from the LM Studio server.
    pub async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let url = format!("http://localhost:{}/v1/models", self.port);
        let client = reqwest::Client::new();

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ProviderError::ApiError {
                status: response.status().as_u16(),
                message: "Failed to list models".into(),
            });
        }

        let body: serde_json::Value = response.json().await?;

        body.get("data")
            .and_then(|data| data.as_array())
            .map(|models| {
                models
                    .iter()
                    .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                    .collect()
            })
            .ok_or_else(|| ProviderError::Custom("Failed to parse model list".into()))
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

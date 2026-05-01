use crate::provider::{Provider, ProviderError};
use crate::types::{ChatRequest, ChatResponse};
use async_trait::async_trait;

/// OpenAI-compatible API provider.
///
/// Works with OpenAI, Azure OpenAI, and any OpenAI-compatible endpoint
/// (e.g., LocalAI, Ollama, Together, etc.)
pub struct OpenAIProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    default_model: String,
    /// Optional custom headers to send with every request.
    extra_headers: Vec<(String, String)>,
}

impl OpenAIProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: Option<String>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key,
            default_model: default_model.into(),
            extra_headers: Vec::new(),
        }
    }

    /// Create a provider for the official OpenAI API.
    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            ProviderError::ConfigError("OPENAI_API_KEY environment variable not set".into())
        })?;
        Ok(Self::new(
            "https://api.openai.com/v1",
            Some(api_key),
            "gpt-4o",
        ))
    }

    /// Add a custom header to every request.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.push((key.into(), value.into()));
        self
    }

    /// Build the full endpoint URL.
    fn chat_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{}/chat/completions", base)
    }

    /// Convert our unified request into the OpenAI-specific JSON body.
    fn build_request_body(&self, request: ChatRequest) -> serde_json::Value {
        let mut body = serde_json::to_value(&request).unwrap_or(serde_json::Value::Null);

        // Ensure model is set
        if body.get("model").is_none() || body["model"].is_null() {
            body["model"] = serde_json::Value::String(self.default_model.clone());
        }

        // OpenAI uses "max_tokens" directly — no transformation needed since our
        // ChatRequest already uses this field name.

        body
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let url = self.chat_url();
        let body = self.build_request_body(request);

        let mut req_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(ref api_key) = self.api_key {
            req_builder = req_builder.bearer_auth(api_key);
        }

        for (key, value) in &self.extra_headers {
            req_builder = req_builder.header(key.as_str(), value.as_str());
        }

        let response = req_builder.json(&body).send().await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".into());
            return Err(ProviderError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let chat_response: ChatResponse = response.json().await?;
        Ok(chat_response)
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }
}

use serde::{Deserialize, Serialize};

/// Configuration for an embedding provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingProviderConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    /// Vector dimension (e.g., 1024 for bge-m3)
    pub dimension: usize,
}

/// A single embedding vector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub index: usize,
    pub vector: Vec<f32>,
}

/// Response from embedding API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub model: String,
    pub data: Vec<Embedding>,
    pub usage: Option<EmbeddingUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

/// Error type for embedding operations
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("No embeddings returned")]
    NoEmbeddings,
}

/// OpenAI-compatible embedding provider.
///
/// Works with OpenAI, SiliconFlow, and any OpenAI-compatible embedding endpoint.
pub struct EmbeddingProvider {
    client: reqwest::Client,
    config: EmbeddingProviderConfig,
}

impl EmbeddingProvider {
    pub fn new(config: EmbeddingProviderConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Build the full endpoint URL.
    fn embeddings_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        format!("{}/embeddings", base)
    }

    /// Embed multiple texts, returning full response with usage info.
    pub async fn embed(&self, texts: Vec<String>) -> Result<EmbeddingResponse, EmbeddingError> {
        let url = self.embeddings_url();

        let body = serde_json::json!({
            "model": self.config.model,
            "input": texts,
            "encoding_format": "float"
        });

        let mut req_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(ref api_key) = self.config.api_key {
            req_builder = req_builder.bearer_auth(api_key);
        }

        tracing::info!(
            model = %self.config.model,
            input_count = texts.len(),
            "Sending embedding request"
        );

        let response = req_builder.json(&body).send().await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".into());
            tracing::error!(
                status = status.as_u16(),
                "Embedding API error: {}",
                error_text
            );
            return Err(EmbeddingError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let raw: serde_json::Value = response.json().await?;
        let embedding_response = self.convert_response(raw);

        tracing::info!(
            model = %embedding_response.model,
            count = embedding_response.data.len(),
            "Embedding request completed"
        );

        Ok(embedding_response)
    }

    /// Embed a single text, returning just the vector.
    pub async fn embed_single(&self, text: String) -> Result<Vec<f32>, EmbeddingError> {
        let response = self.embed(vec![text]).await?;
        response
            .data
            .into_iter()
            .next()
            .map(|e| e.vector)
            .ok_or(EmbeddingError::NoEmbeddings)
    }

    /// Convert raw JSON response into `EmbeddingResponse`.
    ///
    /// Handles the OpenAI-compatible format where each element in the `data`
    /// array has an `embedding` field (array of floats) and an `index` field.
    fn convert_response(&self, raw: serde_json::Value) -> EmbeddingResponse {
        let model = raw
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.config.model)
            .to_string();

        let usage = raw.get("usage").and_then(|u| {
            Some(EmbeddingUsage {
                prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()),
                total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()),
            })
        });

        let data = raw
            .get("data")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let index = item.get("index").and_then(|v| v.as_u64())? as usize;
                        let vector = item
                            .get("embedding")
                            .and_then(|v| v.as_array())?
                            .iter()
                            .filter_map(|f| f.as_f64().map(|v| v as f32))
                            .collect::<Vec<f32>>();
                        Some(Embedding { index, vector })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        EmbeddingResponse { model, data, usage }
    }
}

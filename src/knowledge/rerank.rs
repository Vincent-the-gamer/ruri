use serde::{Deserialize, Serialize};

/// Configuration for a rerank provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankProviderConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    pub index: usize,
    pub relevance_score: f64,
    pub document: Option<RerankDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankDocument {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponse {
    pub model: String,
    pub results: Vec<RerankResult>,
    pub usage: Option<RerankUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankUsage {
    pub prompt_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

/// Error type for rerank operations
#[derive(Debug, thiserror::Error)]
pub enum RerankError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Reranking provider for re-scoring documents by relevance to a query.
///
/// Works with SiliconFlow, Jina, Cohere, and other reranking APIs.
pub struct RerankProvider {
    client: reqwest::Client,
    config: RerankProviderConfig,
}

impl RerankProvider {
    pub fn new(config: RerankProviderConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Build the full endpoint URL.
    fn rerank_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        format!("{}/rerank", base)
    }

    /// Rerank documents by relevance to a query.
    pub async fn rerank(
        &self,
        query: &str,
        documents: Vec<String>,
        top_n: Option<usize>,
    ) -> Result<RerankResponse, RerankError> {
        let url = self.rerank_url();

        let mut body = serde_json::json!({
            "model": self.config.model,
            "query": query,
            "documents": documents,
        });

        if let Some(n) = top_n {
            body["top_n"] = serde_json::Value::Number(n.into());
        }

        let mut req_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(ref api_key) = self.config.api_key {
            req_builder = req_builder.bearer_auth(api_key);
        }

        tracing::info!(
            model = %self.config.model,
            doc_count = documents.len(),
            top_n = ?top_n,
            "Sending rerank request"
        );

        let response = req_builder.json(&body).send().await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".into());
            tracing::error!(status = status.as_u16(), "Rerank API error: {}", error_text);
            return Err(RerankError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let raw: serde_json::Value = response.json().await?;
        let rerank_response = self.convert_response(raw);

        tracing::info!(
            model = %rerank_response.model,
            result_count = rerank_response.results.len(),
            "Rerank request completed"
        );

        Ok(rerank_response)
    }

    /// Convert raw JSON response into `RerankResponse`.
    fn convert_response(&self, raw: serde_json::Value) -> RerankResponse {
        let model = raw
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.config.model)
            .to_string();

        let usage = raw.get("usage").and_then(|u| {
            Some(RerankUsage {
                prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()),
                total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()),
            })
        });

        let results = raw
            .get("results")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let index = item.get("index").and_then(|v| v.as_u64())? as usize;
                        let relevance_score = item
                            .get("relevance_score")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);

                        let document = item.get("document").and_then(|doc| {
                            doc.get("text")
                                .and_then(|t| t.as_str())
                                .map(|s| RerankDocument {
                                    text: s.to_string(),
                                })
                        });

                        Some(RerankResult {
                            index,
                            relevance_score,
                            document,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        RerankResponse {
            model,
            results,
            usage,
        }
    }
}

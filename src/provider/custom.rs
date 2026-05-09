use crate::provider::{Provider, ProviderError};
use crate::types::{ChatRequest, ChatResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a custom API provider.
///
/// This allows connecting to any API by specifying:
/// - The endpoint URL
/// - How to map request fields to the API's expected format
/// - How to extract the response from the API's format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProviderConfig {
    /// Base URL of the API.
    pub base_url: String,
    /// Path appended to base_url for chat completions (e.g., "/v1/chat").
    pub chat_path: String,
    /// HTTP method (POST, GET, etc.). Defaults to POST.
    #[serde(default = "default_method")]
    pub method: String,
    /// Header name for the API key (e.g., "Authorization", "X-API-Key").
    pub auth_header: Option<String>,
    /// Prefix for the auth header value (e.g., "Bearer ").
    #[serde(default = "default_auth_prefix")]
    pub auth_prefix: String,
    /// Additional static headers.
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    /// Request body template. Use {{field}} placeholders for dynamic values.
    /// If not set, the request body is sent as-is in OpenAI format.
    #[serde(default)]
    pub request_template: Option<serde_json::Value>,
    /// Response path to extract the assistant's text content
    /// (e.g., "data.response" or "choices.0.message.content").
    #[serde(default)]
    pub response_content_path: Option<String>,
    /// Response path to extract tool calls.
    #[serde(default)]
    pub response_tool_calls_path: Option<String>,
    /// Response path to extract the model name.
    #[serde(default)]
    pub response_model_path: Option<String>,
    /// Response path to extract the finish reason.
    #[serde(default)]
    pub response_finish_reason_path: Option<String>,
    /// Default model name.
    pub default_model: String,
    /// Whether to send the body as OpenAI-compatible format directly.
    #[serde(default = "default_true")]
    pub use_openai_format: bool,
}

fn default_method() -> String {
    "POST".into()
}

fn default_auth_prefix() -> String {
    "Bearer ".into()
}

fn default_true() -> bool {
    true
}

/// A custom API provider that can adapt to different API formats.
pub struct CustomProvider {
    client: reqwest::Client,
    config: CustomProviderConfig,
    api_key: Option<String>,
}

impl CustomProvider {
    pub fn new(config: CustomProviderConfig, api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            api_key,
        }
    }

    fn chat_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        let path = self.config.chat_path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }

    /// Build the request body — either use OpenAI format or apply the template.
    fn build_request_body(&self, request: &ChatRequest) -> serde_json::Value {
        let mut body = if self.config.use_openai_format {
            serde_json::to_value(request).unwrap_or(serde_json::Value::Null)
        } else if let Some(ref template) = self.config.request_template {
            self.apply_template(template, request)
        } else {
            serde_json::to_value(request).unwrap_or(serde_json::Value::Null)
        };

        // Ensure model is set
        if (body.get("model").is_none() || body["model"].is_null())
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert(
                "model".into(),
                serde_json::Value::String(self.config.default_model.clone()),
            );
        }

        body
    }

    /// Apply simple template substitution on a JSON value.
    fn apply_template(
        &self,
        template: &serde_json::Value,
        request: &ChatRequest,
    ) -> serde_json::Value {
        let request_json = serde_json::to_value(request).unwrap_or(serde_json::Value::Null);

        match template {
            serde_json::Value::String(s) => {
                // Replace {{field}} placeholders with values from the request
                let mut result = s.clone();
                if let Some(obj) = request_json.as_object() {
                    for (key, value) in obj {
                        let placeholder = format!("{{{{{}}}}}", key);
                        if let Some(str_val) = value.as_str() {
                            result = result.replace(&placeholder, str_val);
                        } else if value.is_null() {
                            result = result.replace(&placeholder, "null");
                        } else {
                            result = result.replace(&placeholder, &value.to_string());
                        }
                    }
                }
                serde_json::Value::String(result)
            }
            serde_json::Value::Object(map) => {
                let new_map: serde_json::Map<String, serde_json::Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), self.apply_template(v, request)))
                    .collect();
                serde_json::Value::Object(new_map)
            }
            serde_json::Value::Array(arr) => serde_json::Value::Array(
                arr.iter()
                    .map(|v| self.apply_template(v, request))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// Extract a value from a JSON structure using a dot-separated path.
    fn extract_path<'a>(
        &self,
        value: &'a serde_json::Value,
        path: &str,
    ) -> Option<&'a serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in parts {
            if let Ok(index) = part.parse::<usize>() {
                current = current.get(index)?;
            } else {
                current = current.get(part)?;
            }
        }

        Some(current)
    }

    /// Convert the custom API response into our unified ChatResponse.
    fn convert_response(&self, raw: serde_json::Value) -> ChatResponse {
        // Extract text content
        let content = self
            .config
            .response_content_path
            .as_ref()
            .and_then(|path| self.extract_path(&raw, path))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Extract tool calls (if path is configured)
        let tool_calls = self
            .config
            .response_tool_calls_path
            .as_ref()
            .and_then(|path| self.extract_path(&raw, path))
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        // Extract model
        let model = self
            .config
            .response_model_path
            .as_ref()
            .and_then(|path| self.extract_path(&raw, path))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Extract finish reason
        let finish_reason = self
            .config
            .response_finish_reason_path
            .as_ref()
            .and_then(|path| self.extract_path(&raw, path))
            .and_then(|v| v.as_str())
            .map(String::from);

        let message = if let Some(tc) = tool_calls {
            crate::types::ChatMessage::assistant_with_tool_calls(
                if content.is_empty() {
                    None
                } else {
                    Some(content)
                },
                tc,
            )
        } else {
            crate::types::ChatMessage::assistant(content)
        };

        ChatResponse {
            id: raw.get("id").and_then(|v| v.as_str()).map(String::from),
            object: Some("chat.completion".into()),
            model,
            choices: vec![crate::types::Choice {
                index: 0,
                message,
                finish_reason,
            }],
            usage: None,
            extra: serde_json::Map::new(),
        }
    }
}

#[async_trait]
impl Provider for CustomProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let url = self.chat_url();
        let body = self.build_request_body(&request);

        let mut req_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        // Apply authentication
        if let Some(ref api_key) = self.api_key
            && let Some(ref auth_header) = self.config.auth_header
        {
            let header_value = format!("{}{}", self.config.auth_prefix, api_key);
            req_builder = req_builder.header(auth_header.as_str(), header_value);
        }

        // Apply extra headers
        for (key, value) in &self.config.extra_headers {
            req_builder = req_builder.header(key.as_str(), value.as_str());
        }

        // Use the configured HTTP method
        let response = if self.config.method.to_uppercase() == "GET" {
            req_builder.send().await?
        } else {
            req_builder.json(&body).send().await?
        };

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

        let raw_response: serde_json::Value = response.json().await?;
        Ok(self.convert_response(raw_response))
    }

    fn name(&self) -> &str {
        "custom"
    }

    fn default_model(&self) -> &str {
        &self.config.default_model
    }
}

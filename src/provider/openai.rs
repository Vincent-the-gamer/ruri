use crate::provider::{Provider, ProviderError};
use crate::types::{
    ChatMessage, ChatRequest, ChatResponse, Choice, MessageContent, MessageRole, Usage,
};
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
    /// Whether the backend supports multimodal (image) content.
    ///
    /// Set to `false` when using a self-hosted server (e.g., llama.cpp) that
    /// hasn't been started with the `--multimodal` flag.
    supports_multimodal: bool,
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
            supports_multimodal: true,
        }
    }

    /// Set whether this provider's backend supports multimodal content.
    pub fn with_multimodal_support(mut self, enabled: bool) -> Self {
        self.supports_multimodal = enabled;
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

        // Convert inline image data parts to OpenAI's image_url format with data URLs
        Self::convert_image_parts_to_openai_format(&mut body);

        body
    }

    /// Walk through the request body and convert any `ContentPartType::Image`
    /// parts (serialized as `{"type":"image", "image_data":{...}}`) to
    /// OpenAI's `image_url` format with a data URL:
    ///
    /// ```json
    /// {
    ///   "type": "image_url",
    ///   "image_url": {
    ///     "url": "data:{media_type};base64,{data}"
    ///   }
    /// }
    /// ```
    ///
    /// Parts with `"type": "image_url"` or `"type": "text"` are left as-is.
    fn convert_image_parts_to_openai_format(body: &mut serde_json::Value) {
        let messages = match body.get_mut("messages").and_then(|m| m.as_array_mut()) {
            Some(msgs) => msgs,
            None => return,
        };

        for message in messages.iter_mut() {
            let content = match message.get_mut("content") {
                Some(c) => c,
                None => continue,
            };

            // Content can be a string or an array of content parts
            let parts = match content.as_array_mut() {
                Some(arr) => arr,
                None => continue,
            };

            for part in parts.iter_mut() {
                let part_type = match part.get("type").and_then(|t| t.as_str()) {
                    Some(t) => t,
                    None => continue,
                };

                if part_type != "image" {
                    continue;
                }

                // Extract image_data fields
                let image_data = match part.get("image_data") {
                    Some(id) => id,
                    None => continue,
                };

                let data = image_data
                    .get("data")
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                let media_type = image_data
                    .get("media_type")
                    .and_then(|m| m.as_str())
                    .unwrap_or("image/png");

                // Build the data URL
                let data_url = format!("data:{};base64,{}", media_type, data);

                // Replace the content part with OpenAI's image_url format
                *part = serde_json::json!({
                    "type": "image_url",
                    "image_url": {
                        "url": data_url
                    }
                });
            }
        }
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

        let raw_response: serde_json::Value = response.json().await?;
        let chat_response = self.convert_response(raw_response);
        Ok(chat_response)
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn supports_multimodal(&self) -> bool {
        self.supports_multimodal
    }
}

impl OpenAIProvider {
    /// Convert a raw JSON response into our unified `ChatResponse`.
    ///
    /// This handles non-standard responses from OpenAI-compatible APIs
    /// (e.g., kimi-k2.5 may return `"content": null` when tool_calls are present).
    fn convert_response(&self, raw: serde_json::Value) -> ChatResponse {
        // Try direct deserialization first (fast path for standard responses)
        if let Ok(resp) = serde_json::from_value::<ChatResponse>(raw.clone()) {
            return resp;
        }

        // Fallback: manually extract fields from the raw JSON
        tracing::warn!("Standard deserialization failed, attempting manual conversion");

        let id = raw.get("id").and_then(|v| v.as_str()).map(String::from);
        let object = raw.get("object").and_then(|v| v.as_str()).map(String::from);
        let model = raw.get("model").and_then(|v| v.as_str()).map(String::from);

        let usage = raw.get("usage").and_then(|u| {
            Some(Usage {
                prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()),
                completion_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()),
                total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()),
            })
        });

        let choices = raw
            .get("choices")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .filter_map(|(i, choice)| {
                        let message_val = choice.get("message")?;
                        let role_str = message_val
                            .get("role")
                            .and_then(|r| r.as_str())
                            .unwrap_or("assistant");
                        let role = match role_str {
                            "system" => MessageRole::System,
                            "user" => MessageRole::User,
                            "assistant" => MessageRole::Assistant,
                            "tool" => MessageRole::Tool,
                            _ => MessageRole::Assistant,
                        };

                        // Handle content that may be null, string, or array
                        let content = match message_val.get("content") {
                            Some(val) if !val.is_null() => {
                                serde_json::from_value::<MessageContent>(val.clone()).ok()
                            }
                            _ => None,
                        };

                        let tool_calls = message_val
                            .get("tool_calls")
                            .and_then(|v| serde_json::from_value(v.clone()).ok());

                        let tool_call_id = message_val
                            .get("tool_call_id")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        let name = message_val
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        let message = ChatMessage {
                            role,
                            content,
                            name,
                            tool_calls,
                            tool_call_id,
                        };

                        let finish_reason = choice
                            .get("finish_reason")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        Some(Choice {
                            index: i as u64,
                            message,
                            finish_reason,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        ChatResponse {
            id,
            object,
            model,
            choices,
            usage,
            extra: serde_json::Map::new(),
        }
    }
}

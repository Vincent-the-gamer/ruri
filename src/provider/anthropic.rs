use crate::provider::{Provider, ProviderError};
use crate::types::*;
use async_trait::async_trait;
use serde_json::json;

/// Anthropic API provider (Claude models).
///
/// Anthropic's API uses a different format from OpenAI:
/// - `system` is a top-level field, not a message
/// - Messages use `content` as an array of content blocks
/// - Tool calls use a different structure
pub struct AnthropicProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: String,
    /// Anthropic API version header.
    api_version: String,
    /// Optional custom headers.
    extra_headers: Vec<(String, String)>,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>, default_model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.anthropic.com".into(),
            api_key: api_key.into(),
            default_model: default_model.into(),
            api_version: "2023-06-01".into(),
            extra_headers: Vec::new(),
        }
    }

    pub fn from_env() -> Result<Self, ProviderError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
            ProviderError::ConfigError("ANTHROPIC_API_KEY environment variable not set".into())
        })?;
        Ok(Self::new(api_key, "claude-sonnet-4-20250514"))
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.push((key.into(), value.into()));
        self
    }

    fn chat_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{}/v1/messages", base)
    }

    /// Convert our unified `ChatRequest` into an Anthropic-specific JSON body.
    fn build_request_body(&self, request: &ChatRequest) -> serde_json::Value {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        // Separate system messages from the rest.
        let mut system_parts = Vec::new();
        let mut messages = Vec::new();

        for msg in &request.messages {
            if msg.role == MessageRole::System {
                system_parts.push(json!({
                    "type": "text",
                    "text": msg.content.as_text().unwrap_or(""),
                }));
            } else {
                messages.push(self.convert_message(msg));
            }
        }

        let mut body = json!({
            "model": model,
            "messages": messages,
        });

        // System prompt
        if !system_parts.is_empty() {
            if system_parts.len() == 1 {
                body["system"] = json!(system_parts[0]["text"].as_str().unwrap_or(""));
            } else {
                body["system"] = json!(system_parts);
            }
        }

        // Max tokens — Anthropic requires this field
        body["max_tokens"] = json!(request.max_tokens.unwrap_or(4096));

        // Temperature
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        // Stop sequences
        if let Some(ref stop) = request.stop {
            body["stop_sequences"] = json!(stop);
        }

        // Tools
        if let Some(ref tools) = request.tools {
            let anthropic_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.function.name,
                        "description": t.function.description,
                        "input_schema": t.function.parameters,
                    })
                })
                .collect();
            body["tools"] = json!(anthropic_tools);
        }

        // Extra fields
        for (key, value) in &request.extra {
            body[key] = value.clone();
        }

        body
    }

    /// Convert a `ChatMessage` into Anthropic's message format.
    fn convert_message(&self, msg: &ChatMessage) -> serde_json::Value {
        let role = match msg.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "user", // Anthropic puts tool results in user messages
            _ => "user",
        };

        // Handle tool result messages
        if msg.role == MessageRole::Tool {
            return json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": msg.tool_call_id,
                    "content": msg.content.as_text().unwrap_or(""),
                }]
            });
        }

        // Handle assistant messages with tool calls
        if msg.role == MessageRole::Assistant {
            if let Some(ref tool_calls) = msg.tool_calls {
                let mut content: Vec<serde_json::Value> = Vec::new();

                // Text content first
                let text = msg.content.as_text().unwrap_or("");
                if !text.is_empty() {
                    content.push(json!({
                        "type": "text",
                        "text": text,
                    }));
                }

                // Then tool use blocks
                for tc in tool_calls {
                    content.push(json!({
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.function.name,
                        "input": serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                            .unwrap_or(json!({})),
                    }));
                }

                return json!({
                    "role": role,
                    "content": content,
                });
            }
        }

        // Simple text messages
        match &msg.content {
            MessageContent::Text(text) => json!({
                "role": role,
                "content": text,
            }),
            MessageContent::Parts(parts) => {
                let content: Vec<serde_json::Value> = parts
                    .iter()
                    .map(|p| match p.part_type {
                        ContentPartType::Text => json!({
                            "type": "text",
                            "text": p.text,
                        }),
                        ContentPartType::ImageUrl => {
                            // Anthropic uses base64 image content
                            json!({
                                "type": "image",
                                "source": {
                                    "type": "url",
                                    "url": p.image_url.as_ref().map(|u| u.url.as_str()).unwrap_or(""),
                                },
                            })
                        }
                    })
                    .collect();
                json!({
                    "role": role,
                    "content": content,
                })
            }
        }
    }

    /// Convert Anthropic's response into our unified `ChatResponse`.
    fn convert_response(&self, response: serde_json::Value) -> ChatResponse {
        let id = response
            .get("id")
            .and_then(|v| v.as_str())
            .map(String::from);
        let model = response
            .get("model")
            .and_then(|v| v.as_str())
            .map(String::from);

        let usage = response.get("usage").map(|u| Usage {
            prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()),
            completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()),
            total_tokens: None,
        });

        // Parse content blocks
        let content_blocks = response
            .get("content")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();
        let stop_reason = response
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .map(String::from);

        for block in &content_blocks {
            let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match block_type {
                "text" => {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        text_parts.push(text.to_string());
                    }
                }
                "tool_use" => {
                    tool_calls.push(ToolCall {
                        id: block
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        call_type: ToolCallType::Function,
                        function: FunctionCall {
                            name: block
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            arguments: block
                                .get("input")
                                .and_then(|v| serde_json::to_string(v).ok())
                                .unwrap_or_else(|| "{}".into()),
                        },
                    });
                }
                _ => {}
            }
        }

        let message = if tool_calls.is_empty() {
            ChatMessage::assistant(text_parts.join(""))
        } else {
            ChatMessage::assistant_with_tool_calls(
                if text_parts.is_empty() {
                    None
                } else {
                    Some(text_parts.join(""))
                },
                tool_calls,
            )
        };

        ChatResponse {
            id,
            object: Some("chat.completion".into()),
            model,
            choices: vec![Choice {
                index: 0,
                message,
                finish_reason: stop_reason,
            }],
            usage,
            extra: serde_json::Map::new(),
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let url = self.chat_url();
        let body = self.build_request_body(&request);

        let mut req_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.api_version);

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

            if status.as_u16() == 401 {
                return Err(ProviderError::AuthFailed(error_text));
            }
            if status.as_u16() == 429 {
                return Err(ProviderError::RateLimitExceeded);
            }

            return Err(ProviderError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let raw_response: serde_json::Value = response.json().await?;
        Ok(self.convert_response(raw_response))
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }
}

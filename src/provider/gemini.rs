use crate::provider::{Provider, ProviderError};
use crate::types::*;
use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use serde_json::json;
use std::time::Duration;

/// Helper struct to accumulate tool call data across streaming chunks.
struct StreamingFunctionCall {
    name: String,
    args: String,
}

/// Google Gemini API provider.
///
/// Gemini's API uses a different format from OpenAI:
/// - System messages go into a top-level `systemInstruction` field
/// - "assistant" role becomes "model"
/// - Tool calls use `functionCall` parts
/// - Tool results use `functionResponse` parts with role "user"
/// - Authentication is via `?key=API_KEY` query parameter
pub struct GeminiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: String,
    supports_multimodal: bool,
}

impl GeminiProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            default_model: default_model.into(),
            supports_multimodal: true,
        }
    }

    /// Set whether this provider's backend supports multimodal content.
    pub fn with_multimodal_support(mut self, enabled: bool) -> Self {
        self.supports_multimodal = enabled;
        self
    }

    /// Build the non-streaming endpoint URL.
    fn chat_url(&self, model: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!(
            "{}/models/{}:generateContent?key={}",
            base, model, self.api_key
        )
    }

    /// Build the streaming endpoint URL (SSE).
    fn stream_url(&self, model: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!(
            "{}/models/{}:streamGenerateContent?alt=sse&key={}",
            base, model, self.api_key
        )
    }

    /// Convert our unified `ChatRequest` into a Gemini-specific JSON body.
    fn build_request_body(&self, request: &ChatRequest) -> serde_json::Value {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        // Separate system messages from conversation messages.
        let mut system_instruction_parts = Vec::new();
        let mut contents = Vec::new();

        for msg in &request.messages {
            if msg.role == MessageRole::System {
                // System messages go into systemInstruction
                let text = msg
                    .content
                    .as_ref()
                    .and_then(|c| c.as_text_full())
                    .unwrap_or_default();
                system_instruction_parts.push(json!({
                    "text": text,
                }));
            } else {
                contents.push(self.convert_message(msg));
            }
        }

        let mut body = json!({
            "contents": contents,
        });

        // System instruction
        if !system_instruction_parts.is_empty() {
            body["systemInstruction"] = json!({
                "parts": system_instruction_parts,
            });
        }

        // Generation config
        let mut generation_config = serde_json::Map::new();

        if let Some(temp) = request.temperature {
            generation_config.insert("temperature".to_string(), json!(temp));
        }

        if let Some(max_tokens) = request.max_tokens {
            generation_config.insert("maxOutputTokens".to_string(), json!(max_tokens));
        }

        if let Some(ref stop) = request.stop {
            generation_config.insert("stopSequences".to_string(), json!(stop));
        }

        if !generation_config.is_empty() {
            body["generationConfig"] = json!(generation_config);
        }

        // Tools (function declarations)
        if let Some(ref tools) = request.tools {
            let function_declarations: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    let mut func_decl = json!({
                        "name": t.function.name,
                    });

                    if let Some(ref desc) = t.function.description {
                        func_decl["description"] = json!(desc);
                    }

                    if let Some(ref params) = t.function.parameters {
                        // Convert our ToolParameters to Gemini's JSON Schema format.
                        // Gemini expects a JSON Schema compatible `parameters` object.
                        func_decl["parameters"] = serde_json::to_value(params).unwrap_or(json!({}));
                    }

                    func_decl
                })
                .collect();

            body["tools"] = json!([{
                "functionDeclarations": function_declarations,
            }]);

            // Convert tool_choice to Gemini format
            // See: https://ai.google.dev/gemini-api/docs/function-calling#tool_selection
            if let Some(ref choice) = request.tool_choice {
                match choice {
                    ToolChoice::String(ToolChoiceString::Auto) => {
                        body["toolConfig"] = json!({
                            "functionCallingConfig": {
                                "mode": "AUTO"
                            }
                        });
                    }
                    ToolChoice::String(ToolChoiceString::None) => {
                        body["toolConfig"] = json!({
                            "functionCallingConfig": {
                                "mode": "NONE"
                            }
                        });
                    }
                    ToolChoice::String(ToolChoiceString::Required) => {
                        body["toolConfig"] = json!({
                            "functionCallingConfig": {
                                "mode": "ANY"
                            }
                        });
                    }
                    ToolChoice::Function(func) => {
                        body["toolConfig"] = json!({
                            "functionCallingConfig": {
                                "mode": "ANY",
                                "allowedFunctionNames": [func.function.name]
                            }
                        });
                    }
                }
            }
        }

        // Extra fields
        for (key, value) in &request.extra {
            body[key] = value.clone();
        }

        // Store model for URL construction
        body["_model"] = json!(model);

        body
    }

    /// Convert a `ChatMessage` into Gemini's content format.
    fn convert_message(&self, msg: &ChatMessage) -> serde_json::Value {
        let role = match msg.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "model",
            MessageRole::Tool => "user", // Tool results go in user role with functionResponse
            MessageRole::System => "user", // Fallback; system messages should be filtered out
        };

        // Handle tool result messages
        if msg.role == MessageRole::Tool {
            let tool_call_id = msg.tool_call_id.as_deref().unwrap_or("");
            let result_text = msg
                .content
                .as_ref()
                .and_then(|c| c.as_text_full())
                .unwrap_or_default();

            // Try to parse the result text as JSON; if it fails, wrap it in a
            // simple object so Gemini gets a valid `response` value.
            let response_value: serde_json::Value =
                serde_json::from_str(&result_text).unwrap_or(json!({
                    "result": result_text,
                }));

            return json!({
                "role": "user",
                "parts": [{
                    "functionResponse": {
                        "name": tool_call_id,
                        "response": response_value,
                    }
                }]
            });
        }

        // Handle assistant messages with tool calls
        if msg.role == MessageRole::Assistant {
            if let Some(ref tool_calls) = msg.tool_calls {
                let mut parts = Vec::new();

                // Text content first (if any)
                if let Some(ref content) = msg.content {
                    let text = content.as_text_full().unwrap_or_default();
                    if !text.is_empty() {
                        parts.push(json!({
                            "text": text,
                        }));
                    }
                }

                // Then function call parts
                for tc in tool_calls {
                    let args: serde_json::Value =
                        serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
                    parts.push(json!({
                        "functionCall": {
                            "name": tc.function.name,
                            "args": args,
                        }
                    }));
                }

                return json!({
                    "role": role,
                    "parts": parts,
                });
            }
        }

        // Regular messages (user or assistant without tool calls)
        match &msg.content {
            Some(MessageContent::Text(text)) => json!({
                "role": role,
                "parts": [{"text": text}],
            }),
            Some(MessageContent::Parts(parts)) => {
                let content_parts: Vec<serde_json::Value> =
                    parts.iter().map(|p| self.convert_content_part(p)).collect();
                json!({
                    "role": role,
                    "parts": content_parts,
                })
            }
            None => json!({
                "role": role,
                "parts": [{"text": ""}],
            }),
        }
    }

    /// Convert a content part to Gemini's format.
    fn convert_content_part(&self, part: &ContentPart) -> serde_json::Value {
        match part.part_type {
            ContentPartType::Text => {
                json!({
                    "text": part.text.as_deref().unwrap_or(""),
                })
            }
            ContentPartType::Image => {
                // Convert our ImageData to Gemini's inlineData format
                if let Some(ref img_data) = part.image_data {
                    json!({
                        "inlineData": {
                            "mimeType": img_data.media_type,
                            "data": img_data.data,
                        }
                    })
                } else {
                    json!({
                        "text": "",
                    })
                }
            }
            ContentPartType::ImageUrl => {
                // Convert image URL to Gemini's inlineData format if it's a data URL,
                // or fileData format for regular URLs
                if let Some(ref iu) = part.image_url {
                    let url = &iu.url;
                    if url.starts_with("data:") {
                        // Parse data URL: data:{media_type};base64,{data}
                        if let Some(semicolon) = url.find(';') {
                            let media_type = url[5..semicolon].to_string();
                            let rest = &url[semicolon + 1..];
                            if let Some(comma) = rest.find(',') {
                                let data = &rest[comma + 1..];
                                json!({
                                    "inlineData": {
                                        "mimeType": media_type,
                                        "data": data,
                                    }
                                })
                            } else {
                                // Fallback: just use text
                                json!({
                                    "text": format!("[image: {}]", url),
                                })
                            }
                        } else {
                            json!({
                                "text": format!("[image: {}]", url),
                            })
                        }
                    } else {
                        // Regular URL — Gemini doesn't support image URLs directly
                        // in the same way; we'll try fileData as a fallback
                        json!({
                            "fileData": {
                                "fileUri": url,
                            }
                        })
                    }
                } else {
                    json!({
                        "text": "",
                    })
                }
            }
        }
    }

    /// Convert Gemini's response into our unified `ChatResponse`.
    fn convert_response(&self, raw: serde_json::Value) -> ChatResponse {
        let model = raw
            .get("modelVersion")
            .and_then(|v| v.as_str())
            .map(String::from);

        let usage = raw.get("usageMetadata").map(|u| Usage {
            prompt_tokens: u.get("promptTokenCount").and_then(|v| v.as_u64()),
            completion_tokens: u.get("candidatesTokenCount").and_then(|v| v.as_u64()),
            total_tokens: u.get("totalTokenCount").and_then(|v| v.as_u64()),
        });

        // Parse candidates
        let candidates = raw
            .get("candidates")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();
        let mut finish_reason = None;

        if let Some(candidate) = candidates.first() {
            // Extract finish reason
            finish_reason = candidate
                .get("finishReason")
                .and_then(|v| v.as_str())
                .map(String::from);

            // Extract content parts
            if let Some(content) = candidate.get("content") {
                if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                    for (idx, part) in parts.iter().enumerate() {
                        // Text part
                        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                            text_parts.push(text.to_string());
                        }

                        // Function call part
                        if let Some(func_call) = part.get("functionCall") {
                            let name = func_call
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("")
                                .to_string();
                            let args = func_call
                                .get("args")
                                .and_then(|a| serde_json::to_string(a).ok())
                                .unwrap_or_else(|| "{}".into());

                            tool_calls.push(ToolCall {
                                id: format!("call_{}", idx),
                                call_type: ToolCallType::Function,
                                function: FunctionCall {
                                    name,
                                    arguments: args,
                                },
                            });
                        }
                    }
                }
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
            id: None,
            object: Some("chat.completion".into()),
            model,
            choices: vec![Choice {
                index: 0,
                message,
                finish_reason,
            }],
            usage,
            extra: serde_json::Map::new(),
        }
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let body = self.build_request_body(&request);

        // Extract model from the body (we stashed it there) and remove it
        let model = body["_model"]
            .as_str()
            .unwrap_or(&self.default_model)
            .to_string();
        let mut body = body;
        if let Some(obj) = body.as_object_mut() {
            obj.remove("_model");
        }

        let url = self.chat_url(&model);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".into());

            if status.as_u16() == 401 || status.as_u16() == 403 {
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
        "gemini"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    fn supports_multimodal(&self) -> bool {
        self.supports_multimodal
    }

    fn set_proxy(&mut self, proxy_url: &str, username: Option<&str>, password: Option<&str>) {
        if let Ok(mut proxy) = reqwest::Proxy::all(proxy_url) {
            if let (Some(u), Some(p)) = (username, password) {
                proxy = proxy.basic_auth(u, p);
            }
            self.client = reqwest::Client::builder()
                .proxy(proxy)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
        }
    }

    fn chat_stream<'a>(
        &'a self,
        request: ChatRequest,
    ) -> BoxStream<'a, Result<StreamEvent, ProviderError>> {
        let body = self.build_request_body(&request);

        // Extract model from the body (we stashed it there) and remove it
        let model = body["_model"]
            .as_str()
            .unwrap_or(&self.default_model)
            .to_string();
        let mut body = body;
        if let Some(obj) = body.as_object_mut() {
            obj.remove("_model");
        }

        let url = self.stream_url(&model);
        let client = self.client.clone();

        let stream = async_stream::stream! {
            // Timeout the initial HTTP connection to prevent indefinite hangs
            // when the upstream server becomes unresponsive.
            let response = match tokio::time::timeout(
                Duration::from_secs(120),
                client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send(),
            )
            .await
            {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    yield Err(ProviderError::HttpError(e));
                    return;
                }
                Err(_elapsed) => {
                    yield Err(ProviderError::Timeout);
                    return;
                }
            };

            let status = response.status();
            if status.is_client_error() || status.is_server_error() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".into());
                yield Err(ProviderError::ApiError {
                    status: status.as_u16(),
                    message: error_text,
                });
                return;
            }

            // Parse SSE stream
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            // Accumulated function calls across chunks
            let mut function_calls: Vec<StreamingFunctionCall> = Vec::new();
            // Final usage (typically in the last chunk)
            let mut final_usage: Option<StreamUsage> = None;

            while let Some(chunk_result) = futures_util::StreamExt::next(&mut stream).await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(ProviderError::HttpError(e));
                        return;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                // Process complete SSE lines
                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim().to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line.is_empty() || line.starts_with(':') {
                        continue;
                    }

                    if let Some(data) = line.strip_prefix("data: ") {
                        // Parse the SSE data as JSON
                        let event: serde_json::Value = match serde_json::from_str(data) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };

                        // Extract usage metadata if present
                        if let Some(usage_meta) = event.get("usageMetadata") {
                            let prompt = usage_meta
                                .get("promptTokenCount")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            let completion = usage_meta
                                .get("candidatesTokenCount")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            final_usage = Some(StreamUsage {
                                prompt_tokens: prompt,
                                completion_tokens: completion,
                            });
                        }

                        // Extract content from candidates
                        if let Some(candidates) =
                            event.get("candidates").and_then(|c| c.as_array())
                        {
                            if let Some(candidate) = candidates.first() {
                                if let Some(content) = candidate.get("content") {
                                    if let Some(parts) =
                                        content.get("parts").and_then(|p| p.as_array())
                                    {
                                        for part in parts {
                                            // Text delta
                                            if let Some(text) =
                                                part.get("text").and_then(|t| t.as_str())
                                            {
                                                if !text.is_empty() {
                                                    yield Ok(StreamEvent::ContentDelta {
                                                        delta: text.to_string(),
                                                    });
                                                }
                                            }

                                            // Function call delta
                                            if let Some(func_call) = part.get("functionCall") {
                                                let name = func_call
                                                    .get("name")
                                                    .and_then(|n| n.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                let args = func_call
                                                    .get("args")
                                                    .and_then(|a| {
                                                        serde_json::to_string(a).ok()
                                                    })
                                                    .unwrap_or_else(|| "{}".into());

                                                // Gemini sends complete function calls,
                                                // not incremental deltas like OpenAI.
                                                // We generate start + delta + end events.
                                                let call_id =
                                                    format!("call_{}", function_calls.len());
                                                function_calls
                                                    .push(StreamingFunctionCall {
                                                        name: name.clone(),
                                                        args: args.clone(),
                                                    });

                                                yield Ok(StreamEvent::ToolCallStart {
                                                    tool_call_id: call_id.clone(),
                                                    function_name: name,
                                                });
                                                yield Ok(StreamEvent::ToolCallDelta {
                                                    tool_call_id: call_id.clone(),
                                                    arguments_delta: args,
                                                });
                                                yield Ok(StreamEvent::ToolCallEnd {
                                                    tool_call_id: call_id,
                                                    function_name: function_calls
                                                        .last()
                                                        .unwrap()
                                                        .name
                                                        .clone(),
                                                    arguments: function_calls
                                                        .last()
                                                        .unwrap()
                                                        .args
                                                        .clone(),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            yield Ok(StreamEvent::Done { usage: final_usage });
        };

        stream.boxed()
    }
}

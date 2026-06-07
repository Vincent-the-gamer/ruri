use crate::types::ToolDefinition;
use serde::{Deserialize, Serialize};

/// Role of a chat message sender.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A unified chat message that works across all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    /// The content of the message. Can be `null` when tool calls are present
    /// (e.g., kimi-k2.5 returns `"content": null` when tool_calls exist).
    #[serde(default)]
    pub content: Option<MessageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Content of a message — can be simple text or a list of content parts.
///
/// Note: Some API providers (e.g., Kimi) may return `null` for the content field
/// when tool calls are present. This is handled via custom deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

/// A single content part within a multipart message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub part_type: ContentPartType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<ImageUrl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_data: Option<ImageData>,
}

/// Type of a content part.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContentPartType {
    Text,
    ImageUrl,
    Image,
}

/// Inline image data (base64-encoded).
///
/// Compatible with:
/// - OpenAI: `{ "type": "image_url", "image_url": { "url": "data:image/png;base64,..." } }`
/// - Anthropic: `{ "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": "..." } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    /// The base64-encoded image data (without the data URL prefix).
    pub data: String,
    /// MIME type of the image (e.g., "image/png", "image/jpeg").
    pub media_type: String,
}

/// An image URL within a content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// A tool call made by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: ToolCallType,
    pub function: FunctionCall,
}

/// Type of tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolCallType {
    Function,
}

/// A function call within a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Result returned by a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    /// Whether the tool execution succeeded.
    pub ok: bool,
}

// ─── Chat Request / Response ────────────────────────────────────────

/// Controls which tool the model should call.
///
/// Follows the OpenAI / Qwen Function Calling specification:
/// - `"auto"` – the model decides whether to call a tool (default)
/// - `"none"`  – the model will **not** call any tool
/// - `"required"` – the model **must** call at least one tool
/// - `{"type": "function", "function": {"name": "..."}}` – force a specific tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Simple string variants: "auto", "none", "required"
    String(ToolChoiceString),
    /// Force the model to call a specific function by name.
    Function(ToolChoiceFunction),
}

/// String variants for `tool_choice`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoiceString {
    Auto,
    None,
    Required,
}

/// A specific function to force-call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoiceFunction {
    #[serde(rename = "type")]
    pub choice_type: ToolChoiceType,
    pub function: ToolChoiceFunctionName,
}

/// The type field in a tool_choice function object (always "function").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoiceType {
    Function,
}

/// The function name within a tool_choice object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoiceFunctionName {
    pub name: String,
}

impl Default for ToolChoice {
    fn default() -> Self {
        Self::String(ToolChoiceString::Auto)
    }
}

/// A chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// Controls which (if any) tool the model should call.
    ///
    /// Supported values: `"auto"` (default), `"none"`, `"required"`,
    /// or `{"type": "function", "function": {"name": "<tool_name>"}}`.
    ///
    /// See: <https://help.aliyun.com/zh/model-studio/qwen-function-calling>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// When `true`, the model may return multiple tool calls in a single response
    /// so that independent tools can be invoked in parallel.
    ///
    /// Defaults to `true` for models that support it.
    ///
    /// See: <https://help.aliyun.com/zh/model-studio/qwen-function-calling>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: Option<String>,
    pub object: Option<String>,
    pub model: Option<String>,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A single choice in a chat response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u64,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

// ─── Builder helpers ────────────────────────────────────────────────

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: Some(MessageContent::Text(content.into())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: Some(MessageContent::Text(content.into())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: Some(MessageContent::Text(content.into())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tool_calls(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.map(|c| MessageContent::Text(c)),
            name: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: Some(MessageContent::Text(content.into())),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

impl ChatRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            model: None,
            temperature: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            stream: Some(false),
            stop: None,
            extra: serde_json::Map::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the `tool_choice` parameter.
    ///
    /// Controls which (if any) tool the model should call.
    /// Defaults to `"auto"` when tools are provided.
    pub fn with_tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    /// Set whether the model may return multiple tool calls in a single response.
    pub fn with_parallel_tool_calls(mut self, enabled: bool) -> Self {
        self.parallel_tool_calls = Some(enabled);
        self
    }

    /// Whether any message in this request contains multimodal (image) content.
    pub fn has_multimodal_content(&self) -> bool {
        self.messages
            .iter()
            .any(|msg| msg.content.as_ref().map_or(false, |c| c.has_images()))
    }

    /// Return a new request with all image content parts stripped from messages.
    ///
    /// Messages that only contained images (no text) are converted to a
    /// placeholder text message so that the conversation structure is preserved.
    /// This is useful as a fallback when a provider rejects multimodal content.
    pub fn strip_multimodal_content(&self) -> ChatRequest {
        let mut had_images = false;
        let messages = self.messages.iter().map(|msg| {
            let Some(ref content) = msg.content else {
                return msg.clone();
            };

            match content {
                MessageContent::Text(_) => msg.clone(),
                MessageContent::Parts(parts) => {
                    let has_images = parts.iter().any(|p|
                        p.part_type == ContentPartType::ImageUrl
                        || p.part_type == ContentPartType::Image
                    );

                    if !has_images {
                        return msg.clone();
                    }

                    had_images = true;

                    // Keep only text parts
                    let text_parts: Vec<&ContentPart> = parts
                        .iter()
                        .filter(|p| p.part_type == ContentPartType::Text)
                        .collect();

                    let new_content = if text_parts.is_empty() {
                        // No text parts remaining — use a placeholder
                        Some(MessageContent::Text(
                            "[Image content was removed: the active provider does not support multimodal]"
                                .to_string(),
                        ))
                    } else if text_parts.len() == 1 {
                        // Single text part — simplify to plain text
                        Some(MessageContent::Text(
                            text_parts[0].text.clone().unwrap_or_default(),
                        ))
                    } else {
                        // Multiple text parts — keep them
                        Some(MessageContent::Parts(
                            text_parts.into_iter().cloned().collect(),
                        ))
                    };

                    ChatMessage {
                        content: new_content,
                        ..msg.clone()
                    }
                }
            }
        }).collect();

        let result = ChatRequest {
            messages,
            model: self.model.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            tools: self.tools.clone(),
            tool_choice: self.tool_choice.clone(),
            parallel_tool_calls: self.parallel_tool_calls,
            stream: self.stream,
            stop: self.stop.clone(),
            extra: self.extra.clone(),
        };

        if had_images {
            tracing::warn!(
                "Stripped image content from chat request because the active model does not support multimodal. \
                 Set supports_multimodal to false on the provider to avoid this warning."
            );
        }

        result
    }
}

/// A streaming event emitted during a chat completion stream.
///
/// These events are sent over SSE (Server-Sent Events) to the WebUI client
/// so that the assistant's response is displayed incrementally.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    /// A content delta (partial text) from the assistant.
    #[serde(rename = "content_delta")]
    ContentDelta {
        /// The text fragment appended to the assistant's message.
        delta: String,
    },
    /// A tool is being executed (sent immediately before the tool runs).
    /// This gives the user immediate feedback that work is happening.
    #[serde(rename = "tool_executing")]
    ToolExecuting {
        tool_call_id: String,
        tool_name: String,
        /// Human-readable preview of the arguments (truncated if long).
        arguments_preview: String,
    },
    /// A tool call is being started by the assistant.
    #[serde(rename = "tool_call_start")]
    ToolCallStart {
        tool_call_id: String,
        function_name: String,
    },
    /// Arguments delta for a tool call.
    #[serde(rename = "tool_call_delta")]
    ToolCallDelta {
        tool_call_id: String,
        arguments_delta: String,
    },
    /// A tool call has completed.
    #[serde(rename = "tool_call_end")]
    ToolCallEnd {
        tool_call_id: String,
        function_name: String,
        arguments: String,
    },
    /// A tool has been executed and the result is available.
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        content: String,
        /// Whether the tool execution succeeded.
        ok: bool,
    },
    /// A segment of content for segmented (multi-message) reply mode.
    /// When segmented reply is enabled, the full reply is split into
    /// multiple segments, each sent as a separate event so the frontend
    /// can create a new message bubble for each one.
    #[serde(rename = "segmented_content_delta")]
    SegmentedContentDelta {
        /// Index of this segment (0-based).
        segment_index: usize,
        /// Total number of segments in the reply.
        total_segments: usize,
        /// The text content of this segment.
        delta: String,
    },
    /// The stream has completed.
    #[serde(rename = "done")]
    Done {
        /// Token usage statistics, if available.
        usage: Option<StreamUsage>,
    },
    /// An error occurred during streaming.
    #[serde(rename = "error")]
    Error { error: String },
}

/// Token usage statistics for a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

/// Split text into segments for segmented reply.
///
/// Strategy (in priority order):
/// 1. Preserve fenced code blocks (``` ... ```) — never split inside them.
/// 2. Split on paragraph boundaries (double newlines) first.
/// 3. If a paragraph is still too long, split on sentence boundaries.
/// 4. Keep markdown headings together with their following content.
pub fn split_text_into_segments(text: &str) -> Vec<String> {
    // Maximum characters per segment before we try harder to split.
    const MAX_SEGMENT_LEN: usize = 1500;

    // Step 1: Extract code blocks so we never split inside them.
    // Replace code blocks with placeholders, then restore after splitting.
    let mut code_blocks: Vec<String> = Vec::new();
    let mut processed = String::with_capacity(text.len());
    let mut in_code_block = false;
    let mut code_buf = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_code_block {
                // End of code block
                code_buf.push_str(line);
                code_buf.push('\n');
                let placeholder = format!("\n<!--CODEBLOCK{}-->\n", code_blocks.len());
                code_blocks.push(code_buf.clone());
                processed.push_str(&placeholder);
                code_buf.clear();
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
                code_buf.push_str(line);
                code_buf.push('\n');
            }
        } else if in_code_block {
            code_buf.push_str(line);
            code_buf.push('\n');
        } else {
            processed.push_str(line);
            processed.push('\n');
        }
    }
    // If we ended while still in a code block (malformed), treat the rest as code
    if in_code_block && !code_buf.is_empty() {
        let placeholder = format!("\n<!--CODEBLOCK{}-->\n", code_blocks.len());
        code_blocks.push(code_buf);
        processed.push_str(&placeholder);
    }

    // Step 2: Split by paragraph boundaries (one or more blank lines)
    let paragraphs: Vec<&str> = processed
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    // Step 3: Build segments, merging short paragraphs and splitting long ones
    let mut segments: Vec<String> = Vec::new();
    let mut current = String::new();

    for para in paragraphs {
        let para_len = para.chars().count();

        if current.is_empty() {
            current = para.to_string();
        } else if current.chars().count() + para_len < MAX_SEGMENT_LEN {
            // Merge with current segment
            current.push_str("\n\n");
            current.push_str(para);
        } else {
            // Current segment is full — push it and start a new one
            segments.push(current);
            current = para.to_string();
        }

        // If a single paragraph is still too long, split it by sentences
        if current.chars().count() > MAX_SEGMENT_LEN {
            let mut parts: Vec<String> = Vec::new();
            let mut part = String::new();
            for ch in current.chars() {
                part.push(ch);
                if matches!(ch, '。' | '！' | '？' | '!' | '?' | '\n')
                    && part.chars().count() >= 300
                {
                    let trimmed = part.trim().to_string();
                    if !trimmed.is_empty() {
                        parts.push(trimmed);
                    }
                    part.clear();
                }
            }
            let trimmed = part.trim().to_string();
            if !trimmed.is_empty() {
                parts.push(trimmed);
            }
            if parts.len() > 1 {
                // Push all but the last, keep last as current
                if let Some(last) = parts.pop() {
                    for p in parts {
                        segments.push(p);
                    }
                    current = last;
                }
            }
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }

    // If no segments were produced, return the original
    if segments.is_empty() {
        return vec![text.to_string()];
    }

    // If only one segment, no need to restore code blocks — return as-is
    if segments.len() <= 1 && code_blocks.is_empty() {
        return segments;
    }

    // Step 4: Restore code blocks in each segment
    for segment in &mut segments {
        for (i, code) in code_blocks.iter().enumerate() {
            let placeholder = format!("\n<!--CODEBLOCK{}-->\n", i);
            *segment = segment.replace(&placeholder, code);
        }
    }

    segments
}

impl MessageContent {
    /// Whether this content contains any image parts.
    pub fn has_images(&self) -> bool {
        match self {
            MessageContent::Text(_) => false,
            MessageContent::Parts(parts) => parts.iter().any(|p| {
                p.part_type == ContentPartType::ImageUrl || p.part_type == ContentPartType::Image
            }),
        }
    }

    /// 提取所有文本部分并合并为单个字符串。
    /// 如果没有找到文本部分，则返回 None。
    pub fn as_text_full(&self) -> Option<String> {
        match self {
            MessageContent::Text(t) => Some(t.clone()),
            MessageContent::Parts(parts) => {
                let texts: Vec<String> = parts
                    .iter()
                    .filter_map(|p| {
                        if p.part_type == ContentPartType::Text {
                            p.text.clone()
                        } else {
                            None
                        }
                    })
                    .collect();
                if texts.is_empty() {
                    None
                } else {
                    Some(texts.join("\n"))
                }
            }
        }
    }
}

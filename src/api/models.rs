use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types;

// ─── Provider Models ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderConfigDto {
    Openai(OpenAIProviderConfigDto),
    Anthropic(AnthropicProviderConfigDto),
    Gemini(GeminiProviderConfigDto),
    Custom(Box<CustomProviderConfigDto>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIProviderConfigDto {
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    /// Whether this provider's backend supports multimodal (image) content.
    ///
    /// Defaults to `true` for the standard OpenAI API. Set to `false` when
    /// using a self-hosted server (e.g., llama.cpp) that hasn't been started
    /// with the `--multimodal` flag.
    #[serde(default = "default_true")]
    pub supports_multimodal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiProviderConfigDto {
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    /// Whether this provider's backend supports multimodal (image) content.
    ///
    /// Defaults to `true` because Gemini's API always supports images.
    #[serde(default = "default_true")]
    pub supports_multimodal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicProviderConfigDto {
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    pub api_version: String,
    /// Whether this provider's backend supports multimodal (image) content.
    ///
    /// Defaults to `true` because Anthropic's cloud API always supports images.
    #[serde(default = "default_true")]
    pub supports_multimodal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProviderConfigDto {
    pub base_url: String,
    pub chat_path: String,
    #[serde(default = "default_method")]
    pub method: String,
    pub auth_header: Option<String>,
    #[serde(default = "default_auth_prefix")]
    pub auth_prefix: String,
    /// Optional API key for custom providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    pub request_template: Option<serde_json::Value>,
    pub response_content_path: Option<String>,
    pub response_tool_calls_path: Option<String>,
    pub response_model_path: Option<String>,
    pub response_finish_reason_path: Option<String>,
    pub default_model: String,
    #[serde(default = "default_true")]
    pub use_openai_format: bool,
    /// Whether this provider's backend supports multimodal (image) content.
    ///
    /// Defaults to `false` because custom providers are often self-hosted
    /// servers (e.g., llama.cpp) that may not have multimodal processing
    /// enabled.
    #[serde(default)]
    pub supports_multimodal: bool,
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

fn default_command_prefix_dto() -> String {
    "/".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDto {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub config: ProviderConfigDto,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub provider_type: String,
    pub config: ProviderConfigDto,
}

// ─── Skill Models ────────────────────────────────────────────────

/// Parsed skill from a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSkill {
    pub name: String,
    pub description: String,
    pub skill_type: String,
    pub config: serde_json::Value,
    pub version: String,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDto {
    pub name: String,
    pub description: String,
    pub skill_type: String,
    pub config: serde_json::Value,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSkillRequest {
    pub skill_type: String,
    pub config: serde_json::Value,
}

/// Response for skill package upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadSkillPackageResponse {
    pub skill: SkillDto,
    pub parsed: ParsedSkill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleSkillRequest {
    pub is_active: bool,
}

// ─── Tool Models ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDto {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameterDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameterDto {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ─── Chat Models ─────────────────────────────────────────────────

/// An attached file sent with a chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachedFileDto {
    pub name: String,
    /// MIME type of the file.
    pub mime_type: String,
    /// File content: plain text for text files, or base64 data-URL for binary files.
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequestDto {
    pub message: String,
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub files: Vec<AttachedFileDto>,
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    #[serde(default)]
    pub knowledge_base_ids: Vec<String>,
    /// Custom error message override for this chat session.
    /// If set, takes priority over the config profile's `custom_error_message`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_error_message: Option<String>,
    /// Controls which (if any) tool the model should call.
    ///
    /// Supported values: `"auto"` (default), `"none"`, `"required"`,
    /// or `{"type": "function", "function": {"name": "<tool_name>"}}`.
    ///
    /// See: <https://help.aliyun.com/zh/model-studio/qwen-function-calling>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<crate::types::ToolChoice>,
    /// When `true`, the model may return multiple tool calls in a single
    /// response so that independent tools can be invoked in parallel.
    ///
    /// See: <https://help.aliyun.com/zh/model-studio/qwen-function-calling>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponseDto {
    pub message: ChatMessageDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<ToolResultDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDto {
    pub role: String,
    pub content: serde_json::Value, // Supports both string and array of content parts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDto {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunctionDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunctionDto {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultDto {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDto {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

// ─── Agent Status ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusDto {
    pub status: String,
    pub active_provider: Option<String>,
    pub active_model: Option<String>,
    pub skills_count: usize,
    pub tools_count: usize,
    pub uptime_secs: u64,
    pub message_count: usize,
}

// ─── ACP Config Models ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpProviderOptionDto {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub default_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSkillOptionDto {
    pub name: String,
    pub description: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpConfigDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_provider_id: Option<String>,
    pub active_skill_names: Vec<String>,
    #[serde(default)]
    pub active_knowledge_base_ids: Vec<String>,
    pub available_providers: Vec<AcpProviderOptionDto>,
    pub available_skills: Vec<AcpSkillOptionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAcpConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_skill_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_knowledge_base_ids: Option<Vec<String>>,
}

// ─── Persona Models ──────────────────────────────────────────────

/// DTO for embedded persona configuration.
/// Each config profile and debug session owns its persona data inline —
/// no global persona references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedPersonaDto {
    /// The display name of the persona
    pub name: String,
    /// A short description of the persona's role.
    pub description: String,
    /// The full system prompt that defines the persona's behavior.
    pub prompt: String,
}

impl From<&crate::api::state::EmbeddedPersona> for EmbeddedPersonaDto {
    fn from(p: &crate::api::state::EmbeddedPersona) -> Self {
        Self {
            name: p.name.clone(),
            description: p.description.clone(),
            prompt: p.prompt.clone(),
        }
    }
}

impl From<EmbeddedPersonaDto> for crate::api::state::EmbeddedPersona {
    fn from(dto: EmbeddedPersonaDto) -> Self {
        Self {
            name: dto.name,
            description: dto.description,
            prompt: dto.prompt,
        }
    }
}

impl From<&EmbeddedPersonaDto> for crate::api::state::EmbeddedPersona {
    fn from(dto: &EmbeddedPersonaDto) -> Self {
        Self {
            name: dto.name.clone(),
            description: dto.description.clone(),
            prompt: dto.prompt.clone(),
        }
    }
}

// ─── Persona Library Models ────────────────────────────────────────

/// Persona template returned to the client (from the persona library).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaDto {
    /// Unique identifier for this persona template.
    pub id: String,
    /// The display name of the persona (e.g., "Assistant", "Coder", "Teacher").
    pub name: String,
    /// A short description of the persona's role.
    pub description: String,
    /// The full system prompt that defines the persona's behavior.
    pub prompt: String,
}

/// Request body for creating a new persona template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePersonaRequest {
    /// The display name of the persona.
    pub name: String,
    /// A short description of the persona's role.
    pub description: String,
    /// The full system prompt that defines the persona's behavior.
    pub prompt: String,
}

/// Request body for partially updating a persona template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePersonaRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

// ─── Computer Use Config Models ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerUseConfigDto {
    pub runtime: String,
    pub require_admin: bool,
    pub admin_ids: Vec<String>,
    pub allowed_paths: Vec<String>,
    /// Per-command admin requirement overrides.
    /// Key: command name, Value: true = admin required, false = open to all.
    pub command_admin_required: HashMap<String, bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aio_sandbox_config: Option<AioSandboxConfigDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AioSandboxConfigDto {
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateComputerUseConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_admin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_admin_required: Option<HashMap<String, bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aio_sandbox_config: Option<AioSandboxConfigDto>,
}

// ─── Conversions ─────────────────────────────────────────────────

impl From<&types::ChatMessage> for ChatMessageDto {
    fn from(msg: &types::ChatMessage) -> Self {
        let content_value = match &msg.content {
            Some(types::MessageContent::Text(t)) => serde_json::Value::String(t.clone()),
            Some(types::MessageContent::Parts(parts)) => {
                let parts_json: Vec<serde_json::Value> = parts
                    .iter()
                    .map(|p| {
                        match &p.part_type {
                            types::ContentPartType::Text => serde_json::json!({
                                "type": "text",
                                "text": p.text.as_deref().unwrap_or(""),
                            }),
                            types::ContentPartType::ImageUrl => serde_json::json!({
                                "type": "image_url",
                                "image_url": {
                                    "url": p.image_url.as_ref().map(|iu| &iu.url).unwrap_or(&String::new()),
                                },
                            }),
                            types::ContentPartType::Image => serde_json::json!({
                                "type": "image",
                                "image_data": {
                                    "data": p.image_data.as_ref().map(|d| &d.data).unwrap_or(&String::new()),
                                    "media_type": p.image_data.as_ref().map(|d| &d.media_type).unwrap_or(&String::new()),
                                },
                            }),
                        }
                    })
                    .collect();
                serde_json::Value::Array(parts_json)
            }
            None => serde_json::Value::String(String::new()),
        };
        Self {
            role: match msg.role {
                types::MessageRole::System => "system",
                types::MessageRole::User => "user",
                types::MessageRole::Assistant => "assistant",
                types::MessageRole::Tool => "tool",
            }
            .to_string(),
            content: content_value,
            tool_calls: msg.tool_calls.as_ref().map(|calls| {
                calls
                    .iter()
                    .map(|tc| ToolCallDto {
                        id: tc.id.clone(),
                        call_type: "function".to_string(),
                        function: ToolCallFunctionDto {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect()
            }),
            tool_call_id: msg.tool_call_id.clone(),
        }
    }
}

impl From<&types::ToolDefinition> for ToolDto {
    fn from(def: &types::ToolDefinition) -> Self {
        Self {
            name: def.function.name.clone(),
            description: def.function.description.clone().unwrap_or_default(),
            parameters: def
                .function
                .parameters
                .as_ref()
                .map(|p| {
                    p.properties
                        .as_ref()
                        .map(|props| {
                            props
                                .iter()
                                .map(|(name, param)| ToolParameterDto {
                                    name: name.clone(),
                                    param_type: match param.param_type {
                                        types::ParameterType::String => "string",
                                        types::ParameterType::Number => "number",
                                        types::ParameterType::Integer => "integer",
                                        types::ParameterType::Boolean => "boolean",
                                        types::ParameterType::Array => "array",
                                        types::ParameterType::Object => "object",
                                    }
                                    .to_string(),
                                    required: p
                                        .required
                                        .as_ref()
                                        .map(|r| r.contains(name))
                                        .unwrap_or(false),
                                    description: param.description.clone(),
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .unwrap_or_default(),
        }
    }
}

impl From<&crate::computer_use::ComputerUseConfig> for ComputerUseConfigDto {
    fn from(config: &crate::computer_use::ComputerUseConfig) -> Self {
        Self {
            runtime: match config.runtime {
                crate::computer_use::ComputerUseRuntime::None => "none",
                crate::computer_use::ComputerUseRuntime::Local => "local",
                crate::computer_use::ComputerUseRuntime::AioSandbox => "aio_sandbox",
            }
            .to_string(),
            require_admin: config.require_admin,
            admin_ids: config.admin_ids.clone(),
            allowed_paths: config.allowed_paths.clone(),
            command_admin_required: config.command_admin_required.clone(),
            aio_sandbox_config: config
                .aio_sandbox_config
                .as_ref()
                .map(|s| AioSandboxConfigDto {
                    endpoint: s.endpoint.clone(),
                }),
        }
    }
}

// ─── config profile Models ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigProfileDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enable: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    /// Persona ID reference to the persona library (hot-reload enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_persona: Option<EmbeddedPersonaDto>,
    pub web_search_enabled: bool,
    pub computer_use_enabled: bool,
    pub active_skill_names: Vec<String>,
    #[serde(default)]
    pub active_knowledge_base_ids: Vec<String>,
    #[serde(default)]
    pub proxy_config: crate::types::ProxyConfig,
    #[serde(default = "default_command_prefix_dto")]
    pub command_prefix: String,
    /// List of enabled built-in command names for this profile.
    #[serde(default)]
    pub enabled_commands: Vec<String>,
    /// Per-command admin requirement overrides for this profile.
    #[serde(default)]
    pub command_admin_required: HashMap<String, bool>,
    /// Custom error message to show users when a tool call or API request fails.
    /// If not set, the raw error message is returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_error_message: Option<String>,
    /// Platform instance IDs that this profile is associated with.
    #[serde(default)]
    pub platform_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConfigProfileRequest {
    pub name: String,
    pub description: String,
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    /// Persona ID reference to the persona library (hot-reload enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_persona: Option<EmbeddedPersonaDto>,
    pub web_search_enabled: bool,
    pub computer_use_enabled: bool,
    #[serde(default)]
    pub active_skill_names: Vec<String>,
    #[serde(default)]
    pub active_knowledge_base_ids: Vec<String>,
    #[serde(default)]
    pub proxy_config: crate::types::ProxyConfig,
    #[serde(default = "default_command_prefix_dto")]
    pub command_prefix: String,
    #[serde(default)]
    pub enabled_commands: Vec<String>,
    #[serde(default)]
    pub command_admin_required: HashMap<String, bool>,
    /// Custom error message to show users when a tool call or API request fails.
    /// If not set, the raw error message is returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_error_message: Option<String>,
    /// Platform instance IDs that this profile is associated with.
    #[serde(default)]
    pub platform_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfigProfileRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<Option<String>>,
    /// Persona ID reference to the persona library (hot-reload enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_persona: Option<Option<EmbeddedPersonaDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computer_use_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_skill_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_knowledge_base_ids: Option<Vec<String>>,
    pub proxy_config: Option<crate::types::ProxyConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_commands: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_admin_required: Option<HashMap<String, bool>>,
    /// Custom error message to show users when a tool call or API request fails.
    /// If not set, the raw error message is returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_error_message: Option<Option<String>>,
    /// Platform instance IDs that this profile is associated with.
    #[serde(default)]
    pub platform_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigProfileProviderResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderDto>,
}

// ─── Web Search Config Models ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfigDto {
    pub search_engine: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    pub max_results: usize,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWebSearchConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

impl From<&types::WebSearchConfig> for WebSearchConfigDto {
    fn from(config: &types::WebSearchConfig) -> Self {
        Self {
            search_engine: match config.search_engine {
                types::SearchEngine::DuckDuckGo => "duckduckgo",
                types::SearchEngine::Tavily => "tavily",
                types::SearchEngine::BoCha => "bocha",
                types::SearchEngine::Baidu => "baidu",
                types::SearchEngine::Brave => "brave",
            }
            .to_string(),
            api_key: config.api_key.clone(),
            max_results: config.max_results,
            enabled: config.enabled,
        }
    }
}

// ─── Conversation DTOs ─────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ConversationDto {
    pub id: String,
    pub bot_name: String,
    pub chat_type: String,
    pub chat_id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct MessageDto {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListConversationsRequest {
    pub bot_name: Option<String>,
    pub chat_type: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequestDto {
    pub bot_name: String,
    pub chat_type: String,
    pub chat_id: String,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddMessageRequestDto {
    pub role: String,
    pub content: String,
}

// ─── MCP Server Models ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMcpServerRequest {
    pub name: String,
    pub transport_type: crate::mcp::types::TransportType,
    pub transport_config: crate::mcp::types::TransportConfig,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMcpServerRequest {
    pub name: Option<String>,
    pub transport_type: Option<crate::mcp::types::TransportType>,
    pub transport_config: Option<crate::mcp::types::TransportConfig>,
    pub enabled: Option<bool>,
}

// ─── Platform Types ──────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct PlatformInstanceDto {
    pub id: String,
    pub platform_type: String,
    pub enable: bool,
    pub config: serde_json::Value,
    pub status: String,
}

#[derive(Deserialize)]
pub struct CreatePlatformRequest {
    pub id: String,
    #[serde(rename = "type")]
    pub platform_type: String,
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(flatten)]
    pub config: serde_json::Value,
}

#[derive(Deserialize)]
pub struct UpdatePlatformRequest {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub platform_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    #[serde(flatten)]
    pub config: Option<serde_json::Value>,
}

// ─── Knowledge Base Types ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingProviderConfigDto {
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    pub model: String,
    pub dimension: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankProviderConfigDto {
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub embedding_provider_config: EmbeddingProviderConfigDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_provider_config: Option<RerankProviderConfigDto>,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub document_count: usize,
    pub chunk_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKnowledgeBaseRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub embedding_provider_config: EmbeddingProviderConfigDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_provider_config: Option<RerankProviderConfigDto>,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,
}

fn default_chunk_size() -> usize {
    512
}
fn default_chunk_overlap() -> usize {
    64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateKnowledgeBaseRequestDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_provider_config: Option<Option<RerankProviderConfigDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_overlap: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbDocumentDto {
    pub id: String,
    pub knowledge_base_id: String,
    pub filename: String,
    pub file_size: i64,
    pub file_type: String,
    pub content_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    pub chunk_count: usize,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

fn default_top_k() -> usize {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultDto {
    pub content: String,
    pub score: f64,
    pub source: String,
    pub chunk_index: usize,
    /// Expanded context including neighboring chunks, if available.
    pub context: Option<String>,
}

// ─── Knowledge Base From impls ──────────────────────────────────

impl From<crate::knowledge::EmbeddingProviderConfig> for EmbeddingProviderConfigDto {
    fn from(config: crate::knowledge::EmbeddingProviderConfig) -> Self {
        Self {
            base_url: config.base_url,
            api_key: config.api_key,
            model: config.model,
            dimension: config.dimension,
        }
    }
}

impl From<EmbeddingProviderConfigDto> for crate::knowledge::EmbeddingProviderConfig {
    fn from(dto: EmbeddingProviderConfigDto) -> Self {
        Self {
            base_url: dto.base_url,
            api_key: dto.api_key,
            model: dto.model,
            dimension: dto.dimension,
        }
    }
}

impl From<crate::knowledge::RerankProviderConfig> for RerankProviderConfigDto {
    fn from(config: crate::knowledge::RerankProviderConfig) -> Self {
        Self {
            base_url: config.base_url,
            api_key: config.api_key,
            model: config.model,
        }
    }
}

impl From<RerankProviderConfigDto> for crate::knowledge::RerankProviderConfig {
    fn from(dto: RerankProviderConfigDto) -> Self {
        Self {
            base_url: dto.base_url,
            api_key: dto.api_key,
            model: dto.model,
        }
    }
}

impl From<crate::knowledge::KnowledgeBase> for KnowledgeBaseDto {
    fn from(kb: crate::knowledge::KnowledgeBase) -> Self {
        Self {
            id: kb.id,
            name: kb.name,
            description: kb.description,
            embedding_provider_config: kb.embedding_provider_config.into(),
            rerank_provider_config: kb.rerank_provider_config.map(Into::into),
            chunk_size: kb.chunk_size,
            chunk_overlap: kb.chunk_overlap,
            document_count: kb.document_count,
            chunk_count: kb.chunk_count,
            created_at: kb.created_at,
            updated_at: kb.updated_at,
        }
    }
}

impl From<crate::knowledge::KbDocument> for KbDocumentDto {
    fn from(doc: crate::knowledge::KbDocument) -> Self {
        Self {
            id: doc.id,
            knowledge_base_id: doc.knowledge_base_id,
            filename: doc.filename,
            file_size: doc.file_size,
            file_type: doc.file_type,
            content_hash: doc.content_hash,
            tags: doc.tags,
            chunk_count: doc.chunk_count,
            status: doc.status,
            error_message: doc.error_message,
            created_at: doc.created_at,
            updated_at: doc.updated_at,
        }
    }
}

impl From<crate::knowledge::SearchResult> for SearchResultDto {
    fn from(result: crate::knowledge::SearchResult) -> Self {
        Self {
            content: result.chunk.content,
            score: result.score as f64,
            source: result.document_filename,
            chunk_index: result.chunk.chunk_index,
            context: result.context,
        }
    }
}

// ─── Debug Session Models ───────────────────────────────────────

/// DTO for embedded provider in debug session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedProviderDto {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub config: ProviderConfigDto,
}

/// DTO for embedded skill in debug session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedSkillDto {
    pub name: String,
    pub description: String,
    pub skill_type: String,
    pub config: serde_json::Value,
}

/// Response DTO for debug session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSessionDto {
    /// Persona ID reference to the persona library (hot-reload enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_persona: Option<EmbeddedPersonaDto>,
    #[serde(default)]
    pub providers: Vec<EmbeddedProviderDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u64>,
    #[serde(default)]
    pub custom_error_message: Option<String>,
    #[serde(default)]
    pub knowledge_base_ids: Vec<String>,
    #[serde(default)]
    pub skills: Vec<EmbeddedSkillDto>,
    #[serde(default)]
    pub active_skill_names: Vec<String>,
    /// Built-in command prefix for debug session (default: "/").
    #[serde(default = "default_command_prefix_dto")]
    pub command_prefix: String,
    /// List of enabled built-in command names.
    #[serde(default)]
    pub enabled_commands: Vec<String>,
}

/// Request DTO for updating debug session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDebugSessionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_persona: Option<Option<EmbeddedPersonaDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<Vec<EmbeddedProviderDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_provider: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<Option<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<Option<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_error_message: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_base_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<EmbeddedSkillDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_skill_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_commands: Option<Vec<String>>,
}

impl From<&crate::api::state::EmbeddedProvider> for EmbeddedProviderDto {
    fn from(p: &crate::api::state::EmbeddedProvider) -> Self {
        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            provider_type: p.provider_type.clone(),
            config: serde_json::from_value(p.config_json.clone()).unwrap_or_else(|_| {
                // Fallback to a basic OpenAI config if deserialization fails
                ProviderConfigDto::Openai(OpenAIProviderConfigDto {
                    base_url: String::new(),
                    api_key: String::new(),
                    default_model: String::new(),
                    supports_multimodal: true,
                })
            }),
        }
    }
}

impl From<&EmbeddedProviderDto> for crate::api::state::EmbeddedProvider {
    fn from(dto: &EmbeddedProviderDto) -> Self {
        Self {
            id: dto.id.clone(),
            name: dto.name.clone(),
            provider_type: dto.provider_type.clone(),
            config_json: serde_json::to_value(&dto.config).unwrap_or(serde_json::Value::Null),
        }
    }
}

impl From<&crate::api::state::EmbeddedSkill> for EmbeddedSkillDto {
    fn from(s: &crate::api::state::EmbeddedSkill) -> Self {
        Self {
            name: s.name.clone(),
            description: s.description.clone(),
            skill_type: s.skill_type.clone(),
            config: s.config.clone(),
        }
    }
}

impl From<&EmbeddedSkillDto> for crate::api::state::EmbeddedSkill {
    fn from(dto: &EmbeddedSkillDto) -> Self {
        Self {
            name: dto.name.clone(),
            description: dto.description.clone(),
            skill_type: dto.skill_type.clone(),
            config: dto.config.clone(),
        }
    }
}

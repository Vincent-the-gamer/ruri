use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types;

// ─── Provider Models ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderConfigDto {
    Openai(OpenAIProviderConfigDto),
    Anthropic(AnthropicProviderConfigDto),
    Custom(CustomProviderConfigDto),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIProviderConfigDto {
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicProviderConfigDto {
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    pub api_version: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequestDto {
    pub message: String,
    pub provider_id: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
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
    pub content: String,
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

// ─── Conversions ─────────────────────────────────────────────────

impl From<&types::ChatMessage> for ChatMessageDto {
    fn from(msg: &types::ChatMessage) -> Self {
        Self {
            role: match msg.role {
                types::MessageRole::System => "system",
                types::MessageRole::User => "user",
                types::MessageRole::Assistant => "assistant",
                types::MessageRole::Tool => "tool",
            }
            .to_string(),
            content: msg.content.as_text().unwrap_or("").to_string(),
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

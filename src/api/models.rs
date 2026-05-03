use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types;

// ─── Provider Models ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderConfigDto {
    Openai(OpenAIProviderConfigDto),
    Anthropic(AnthropicProviderConfigDto),
    Custom(Box<CustomProviderConfigDto>),
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

/// Skill Package Manifest - defines a skill package structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPackageManifest {
    /// Unique identifier for the skill
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Version of the skill package (e.g., "1.0.0")
    pub version: String,
    /// Author information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Configuration schema for this skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
    /// Default configuration values
    #[serde(default)]
    pub default_config: serde_json::Value,
    /// Type identifier for the skill
    #[serde(default = "default_skill_type")]
    pub skill_type: String,
}

fn default_skill_type() -> String {
    "custom".to_string()
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequestDto {
    pub message: String,
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
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
    pub available_providers: Vec<AcpProviderOptionDto>,
    pub available_skills: Vec<AcpSkillOptionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAcpConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_skill_names: Option<Vec<String>>,
}

// ─── Computer Use Config Models ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerUseConfigDto {
    pub runtime: String,
    pub require_admin: bool,
    pub admin_ids: Vec<String>,
    pub allowed_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_config: Option<SandboxConfigDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfigDto {
    pub driver: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    pub ttl_secs: u64,
    pub enable_browser: bool,
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
    pub sandbox_config: Option<SandboxConfigDto>,
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
            content: msg
                .content
                .as_ref()
                .and_then(|c| c.as_text())
                .unwrap_or("")
                .to_string(),
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
                crate::computer_use::ComputerUseRuntime::Sandbox => "sandbox",
            }
            .to_string(),
            require_admin: config.require_admin,
            admin_ids: config.admin_ids.clone(),
            allowed_paths: config.allowed_paths.clone(),
            sandbox_config: config.sandbox_config.as_ref().map(|s| SandboxConfigDto {
                driver: s.driver.clone(),
                endpoint: s.endpoint.clone(),
                profile: s.profile.clone(),
                ttl_secs: s.ttl_secs,
                enable_browser: s.enable_browser,
            }),
        }
    }
}

impl ComputerUseConfigDto {
    /// Convert to ComputerUseConfig
    pub fn to_config(&self) -> Result<crate::computer_use::ComputerUseConfig, String> {
        let runtime = match self.runtime.as_str() {
            "none" => crate::computer_use::ComputerUseRuntime::None,
            "local" => crate::computer_use::ComputerUseRuntime::Local,
            "sandbox" => crate::computer_use::ComputerUseRuntime::Sandbox,
            _ => return Err(format!("Invalid runtime: {}", self.runtime)),
        };

        Ok(crate::computer_use::ComputerUseConfig {
            runtime,
            require_admin: self.require_admin,
            admin_ids: self.admin_ids.clone(),
            allowed_paths: self.allowed_paths.clone(),
            sandbox_config: self.sandbox_config.as_ref().map(|s| {
                crate::computer_use::SandboxConfig {
                    driver: s.driver.clone(),
                    endpoint: s.endpoint.clone(),
                    profile: s.profile.clone(),
                    ttl_secs: s.ttl_secs,
                    enable_browser: s.enable_browser,
                }
            }),
        })
    }
}

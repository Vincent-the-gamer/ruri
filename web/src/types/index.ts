// ─── Provider Types ──────────────────────────────────────────────

export interface OpenAIProviderConfig {
  type: 'openai'
  base_url: string
  api_key: string
  default_model: string
}

export interface AnthropicProviderConfig {
  type: 'anthropic'
  base_url: string
  api_key: string
  default_model: string
  api_version: string
}

export interface CustomProviderConfig {
  type: 'custom'
  base_url: string
  chat_path: string
  method: string
  auth_header: string | null
  auth_prefix: string
  api_key?: string | null
  extra_headers: Record<string, string>
  request_template: unknown | null
  response_content_path: string | null
  response_tool_calls_path: string | null
  response_model_path: string | null
  response_finish_reason_path: string | null
  default_model: string
  use_openai_format: boolean
}

export type ProviderConfig = OpenAIProviderConfig | AnthropicProviderConfig | CustomProviderConfig

export type ProviderType = 'openai' | 'anthropic' | 'custom'

export interface Provider {
  id: string
  name: string
  provider_type: ProviderType
  config: ProviderConfig
  is_active: boolean
  created_at: string
}

export interface CreateProviderRequest {
  name: string
  provider_type: ProviderType
  config: ProviderConfig
}

// ─── Skill Types ─────────────────────────────────────────────────

export interface Skill {
  name: string
  description: string
  skill_type: string
  config: Record<string, unknown>
  is_active: boolean
}

export interface CreateSkillRequest {
  skill_type: string
  config: Record<string, unknown>
}

// ─── Tool Types ──────────────────────────────────────────────────

export interface ToolParameter {
  name: string
  param_type: string
  required: boolean
  description?: string
}

export interface Tool {
  name: string
  description: string
  parameters: ToolParameter[]
}

// ─── Chat Types ──────────────────────────────────────────────────

export interface ToolCallFunction {
  name: string
  arguments: string
}

export interface ToolCall {
  id: string
  type: string
  function: ToolCallFunction
}

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant' | 'tool'
  content: string
  tool_calls?: ToolCall[]
  tool_call_id?: string
}

export interface ChatRequest {
  message: string
  provider_id?: string
  temperature?: number
  max_tokens?: number
}

export interface ToolResult {
  tool_call_id: string
  tool_name: string
  content: string
}

export interface ChatResponse {
  message: ChatMessage
  tool_results?: ToolResult[]
  usage?: {
    prompt_tokens: number
    completion_tokens: number
  }
}

// ─── Agent Status ────────────────────────────────────────────────

export interface AgentStatus {
  status: 'running' | 'stopped' | 'error'
  active_provider: string | null
  active_model: string | null
  skills_count: number
  tools_count: number
  uptime_secs: number
  message_count: number
}

// ─── API Response ────────────────────────────────────────────────

export interface ApiResponse<T = unknown> {
  success: boolean
  data?: T
  error?: string
}

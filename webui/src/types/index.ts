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

export interface SkillPackageManifest {
  name: string
  description: string
  version: string
  author?: string
  config_schema?: Record<string, unknown>
  default_config: Record<string, unknown>
  skill_type: string
}

export interface ParsedSkill {
  name: string
  description: string
  skill_type: string
  config: Record<string, unknown>
  version: string
  author?: string
}

export interface UploadSkillPackageResponse {
  skill: Skill
  parsed: ParsedSkill
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
  persona_id?: string
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

// ─── ACP Types ───────────────────────────────────────────────────

export interface AcpProviderOption {
  id: string
  name: string
  provider_type: ProviderType
  default_model: string
}

export interface AcpSkillOption {
  name: string
  description: string
  is_active: boolean
}

export interface AcpConfig {
  active_provider_id: string | null
  active_skill_names: string[]
  available_providers: AcpProviderOption[]
  available_skills: AcpSkillOption[]
}

export interface UpdateAcpConfigRequest {
  active_provider_id: string | null
  active_skill_names: string[]
}

// ─── Log Types ────────────────────────────────────────────────────────

export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error'

export interface LogEntry {
  timestamp: number
  level: LogLevel
  target: string
  message: string
  file?: string
  line?: number
}

// ─── Computer Use Types ────────────────────────────────────────────

export type ComputerUseRuntime = 'none' | 'local' | 'sandbox'

export type SandboxDriver = 'shipyard_neo' | 'cua'

export interface SandboxConfig {
  driver: SandboxDriver

  // Shipyard Neo 配置
  endpoint?: string
  access_token?: string
  profile?: string
  ttl_secs?: number

  // CUA 配置
  cua_image?: string
  cua_os_type?: string
  cua_sandbox_ttl?: number
  cua_telemetry_enabled?: boolean
  cua_local_runtime?: boolean
  cua_api_key?: string

  // 通用配置
  enable_browser?: boolean
}

export interface ComputerUseConfig {
  runtime: ComputerUseRuntime
  require_admin: boolean
  admin_ids: string[]
  allowed_paths: string[]
  sandbox_config?: SandboxConfig
}

export interface UpdateComputerUseConfigRequest {
  runtime?: ComputerUseRuntime
  require_admin?: boolean
  admin_ids?: string[]
  allowed_paths?: string[]
  sandbox_config?: SandboxConfig
}

// ─── Web Search Types ────────────────────────────────────────────

export type SearchEngine = 'duckduckgo' | 'tavily' | 'bocha' | 'baidu' | 'brave'

export interface WebSearchConfig {
  search_engine: SearchEngine
  api_key?: string | null
  max_results: number
  enabled: boolean
}

export interface UpdateWebSearchConfigRequest {
  search_engine?: SearchEngine
  api_key?: string | null
  max_results?: number
  enabled?: boolean
}

// ─── Persona Types ───────────────────────────────────────────────

export interface Persona {
  id: string
  name: string
  description: string
  prompt: string
  is_active: boolean
}

export interface CreatePersonaRequest {
  name: string
  description: string
  prompt: string
  is_active?: boolean
}

export interface UpdatePersonaRequest {
  name?: string
  description?: string
  prompt?: string
  is_active?: boolean
}

// ─── Config Profile Types ───────────────────────────────────────

export interface ConfigProfile {
  id: string
  name: string
  description: string
  is_active: boolean
  created_at: string
  updated_at: string
  // 关联的配置
  provider_id: string | null
  persona_id: string | null
  web_search_enabled: boolean
  computer_use_enabled: boolean
  acp_enabled: boolean
  // 技能配置
  active_skill_names: string[]
  // 平台配置
  active_platform_ids: string[]
  // 代理配置
  proxy_config: ProxyConfig
}

export interface CreateConfigProfileRequest {
  name: string
  description: string
  provider_id: string | null
  persona_id: string | null
  web_search_enabled: boolean
  computer_use_enabled: boolean
  acp_enabled: boolean
  active_skill_names: string[]
  active_platform_ids: string[]
  proxy_config: ProxyConfig
}

export interface UpdateConfigProfileRequest {
  name?: string
  description?: string
  provider_id?: string | null
  persona_id?: string | null
  web_search_enabled?: boolean
  computer_use_enabled?: boolean
  acp_enabled?: boolean
  active_skill_names?: string[]
  active_platform_ids?: string[]
  proxy_config?: ProxyConfig
}

// ─── MCP Types ──────────────────────────────────────────────────

export type TransportType = 'stdio' | 'sse' | 'websocket' | 'http'

export interface StdioTransportConfig {
  type: 'stdio'
  command: string
  args?: string[]
  env?: Record<string, string>
}

export interface SSETransportConfig {
  type: 'sse'
  url: string
  headers?: Record<string, string>
}

export interface WebSocketTransportConfig {
  type: 'websocket'
  url: string
  headers?: Record<string, string>
}

export interface HttpTransportConfig {
  type: 'http'
  url: string
  headers?: Record<string, string>
}

export type TransportConfig = StdioTransportConfig | SSETransportConfig | WebSocketTransportConfig | HttpTransportConfig

export interface McpServerConfig {
  id: string
  name: string
  transport_type: TransportType
  transport_config: TransportConfig
  enabled?: boolean
  created_at: string
  updated_at: string
}

export interface CreateMcpServerRequest {
  name: string
  transport_type: TransportType
  transport_config: TransportConfig
  enabled?: boolean
}

export interface UpdateMcpServerRequest {
  name?: string
  transport_type?: TransportType
  transport_config?: TransportConfig
  enabled?: boolean
}

export interface McpServerStatus {
  id: string
  name: string
  connected: boolean
  tools_count?: number
  error?: string
}

// ─── API Response ────────────────────────────────────────────────

export interface ApiResponse<T = unknown> {
  success: boolean
  data?: T
  error?: string
}

// ─── Platform Types ──────────────────────────────────────────

export type PlatformType = 'dingtalk' | 'discord'

export type PlatformStatus = 'running' | 'stopped' | 'pending' | 'error'

export interface DingtalkPlatformConfig {
  client_id: string
  client_secret: string
}

export interface DiscordPlatformConfig {
  token: string
  pre_response_reactions?: boolean
  reaction_emojis?: string[]
}

export type PlatformConfig = DingtalkPlatformConfig | DiscordPlatformConfig

export interface PlatformInstance {
  id: string
  platform_type: PlatformType
  config: PlatformConfig
  status: PlatformStatus
}

export interface CreatePlatformRequest {
  id: string
  type: PlatformType
  // DingTalk fields
  client_id?: string
  client_secret?: string
  // Discord fields
  token?: string
  pre_response_reactions?: boolean
  reaction_emojis?: string[]
}

export interface UpdatePlatformRequest {
  id?: string
  type?: PlatformType
  // DingTalk fields
  client_id?: string
  client_secret?: string
  // Discord fields
  token?: string
  pre_response_reactions?: boolean
  reaction_emojis?: string[]
}

// ─── Proxy Config Types ──────────────────────────────────────────

export interface ProxyConfig {
  url: string
  username?: string | null
  password?: string | null
  bypass_localhost: boolean
  bypass_hosts: string[]
}



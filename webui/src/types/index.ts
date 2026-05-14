// ─── Provider Types ──────────────────────────────────────────────

export interface OpenAIProviderConfig {
  type: 'openai'
  base_url: string
  api_key: string
  default_model: string
  supports_multimodal?: boolean
}

export interface AnthropicProviderConfig {
  type: 'anthropic'
  base_url: string
  api_key: string
  default_model: string
  api_version: string
  supports_multimodal?: boolean
}

export interface GeminiProviderConfig {
  type: 'gemini'
  base_url: string
  api_key: string
  default_model: string
  supports_multimodal?: boolean
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
  supports_multimodal?: boolean
}

export type ProviderConfig = OpenAIProviderConfig | AnthropicProviderConfig | GeminiProviderConfig | CustomProviderConfig

export type ProviderType = 'openai' | 'anthropic' | 'gemini' | 'custom'

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

export interface ContentPart {
  type: 'text' | 'image_url'
  text?: string
  image_url?: {
    url: string
    detail?: string
  }
}

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant' | 'tool'
  content: string | ContentPart[]
  tool_calls?: ToolCall[]
  tool_call_id?: string
}

export interface AttachedFile {
  name: string
  mime_type: string  // mime type
  content: string  // text content or base64 data URL
}

/**
 * Controls which (if any) tool the model should call.
 *
 * - `"auto"`     — the model decides (default)
 * - `"none"`     — the model will not call any tool
 * - `"required"` — the model must call at least one tool
 * - `{ type: "function", function: { name: string } }` — force a specific tool
 */
export type ToolChoice = 'auto' | 'none' | 'required' | {
  type: 'function'
  function: { name: string }
}

export interface ChatRequest {
  message: string
  images?: string[]  // base64 data URLs or HTTP URLs
  files?: AttachedFile[]  // attached files
  provider_id?: string
  persona_id?: string
  temperature?: number
  max_tokens?: number
  knowledge_base_ids?: string[]
  custom_error_message?: string
  /** The authenticated user's ID from the auth store. */
  user_id?: string
  /** Controls which (if any) tool the model should call. */
  tool_choice?: ToolChoice
  /** Whether the model may return multiple tool calls in parallel. */
  parallel_tool_calls?: boolean
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

// ─── Streaming Types ─────────────────────────────────────────────

export interface StreamUsage {
  prompt_tokens: number
  completion_tokens: number
}

export type StreamEvent =
  | { type: 'content_delta'; delta: string }
  | { type: 'tool_call_start'; tool_call_id: string; function_name: string }
  | { type: 'tool_call_delta'; tool_call_id: string; arguments_delta: string }
  | { type: 'tool_call_end'; tool_call_id: string; function_name: string; arguments: string }
  | { type: 'tool_result'; tool_call_id: string; tool_name: string; content: string }
  | { type: 'done'; usage: StreamUsage | null }
  | { type: 'error'; error: string }

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
  active_knowledge_base_ids: string[]
  available_providers: AcpProviderOption[]
  available_skills: AcpSkillOption[]
}

export interface UpdateAcpConfigRequest {
  active_provider_id: string | null
  active_skill_names: string[]
  active_knowledge_base_ids?: string[]
}

// ─── Log Types ────────────────────────────────────────────────────────

export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error'

export interface LogEntry {
  id: number
  timestamp: number
  level: LogLevel
  target: string
  message: string
  module_path?: string
  file?: string
  line?: number
  fields?: Record<string, string>
}

/** WebSocket command to set log level filter */
export interface WsFilterCommand {
  type: 'filter'
  level: LogLevel
}

/** WebSocket command to request logs since a timestamp */
export interface WsGetSinceCommand {
  type: 'get_since'
  timestamp: number
}

// ─── Computer Use Types ────────────────────────────────────────────

export type ComputerUseRuntime = 'none' | 'local' | 'aio_sandbox'

export interface AioSandboxConfig {
  endpoint: string
}

export interface ComputerUseConfig {
  runtime: ComputerUseRuntime
  require_admin: boolean
  admin_ids: string[]
  allowed_paths: string[]
  command_admin_required: Record<string, boolean>
  aio_sandbox_config?: AioSandboxConfig
}

export interface UpdateComputerUseConfigRequest {
  runtime?: ComputerUseRuntime
  require_admin?: boolean
  admin_ids?: string[]
  allowed_paths?: string[]
  command_admin_required?: Record<string, boolean>
  aio_sandbox_config?: AioSandboxConfig
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
}

export interface CreatePersonaRequest {
  name: string
  description: string
  prompt: string
}

export interface UpdatePersonaRequest {
  name?: string
  description?: string
  prompt?: string
}

// ─── Config Profile Types ───────────────────────────────────────

export interface ConfigProfile {
  id: string
  name: string
  description: string
  enable: boolean
  is_active: boolean
  created_at: string
  updated_at: string
  // 关联的配置
  provider_id: string | null
  persona_id: string | null
  web_search_enabled: boolean
  computer_use_enabled: boolean
  // 技能配置
  active_skill_names: string[]
  active_knowledge_base_ids: string[]
  // 内置指令前缀
  command_prefix: string
  // 启用的内置指令列表
  enabled_commands: string[]
  // 每个指令的管理员权限覆盖
  command_admin_required: Record<string, boolean>
  // 自定义错误信息
  custom_error_message?: string
  // 关联的平台实例
  platform_ids: string[]
  // 代理配置
  proxy_config: ProxyConfig
}

export interface CreateConfigProfileRequest {
  name: string
  description: string
  enable: boolean
  provider_id: string | null
  persona_id: string | null
  web_search_enabled: boolean
  computer_use_enabled: boolean
  active_skill_names: string[]
  active_knowledge_base_ids: string[]
  command_prefix: string
  enabled_commands?: string[]
  command_admin_required?: Record<string, boolean>
  custom_error_message?: string
  platform_ids?: string[]
  proxy_config: ProxyConfig
}

export interface UpdateConfigProfileRequest {
  name?: string
  description?: string
  enable?: boolean
  provider_id?: string | null
  persona_id?: string | null
  web_search_enabled?: boolean
  computer_use_enabled?: boolean
  active_skill_names?: string[]
  active_knowledge_base_ids?: string[]
  command_prefix?: string
  enabled_commands?: string[]
  command_admin_required?: Record<string, boolean>
  custom_error_message?: string | null
  platform_ids?: string[]
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

export type PlatformType = 'dingtalk' | 'discord' | 'weixin_oc' | 'onebot12'

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

export interface WeixinOcPlatformConfig {
  token?: string
  account_id?: string
  base_url?: string
  cdn_base_url?: string
  proxy_url?: string
}

export interface OneBot12PlatformConfig {
  platform: string
  self_user_id: string
  access_token?: string
  http?: {
    host?: string
    port?: number
    event_enabled?: boolean
    event_buffer_size?: number
  }
  http_webhook?: {
    url: string
  }
  ws?: {
    host?: string
    port?: number
  }
  ws_reverse?: {
    url: string
    reconnect_interval?: number
  }
}

export type PlatformConfig = DingtalkPlatformConfig | DiscordPlatformConfig | WeixinOcPlatformConfig | OneBot12PlatformConfig

export interface PlatformInstance {
  id: string
  platform_type: PlatformType
  enable: boolean
  config: PlatformConfig
  status: PlatformStatus
}

export interface CreatePlatformRequest {
  id: string
  type: PlatformType
  enable?: boolean
  // DingTalk fields
  client_id?: string
  client_secret?: string
  // Discord fields
  token?: string
  pre_response_reactions?: boolean
  reaction_emojis?: string[]
  // WeChat (weixin_oc) fields
  account_id?: string
  base_url?: string
  cdn_base_url?: string
  proxy_url?: string
  // OneBot12 fields
  platform?: string
  self_user_id?: string
  access_token?: string
  http?: {
    host?: string
    port?: number
    event_enabled?: boolean
    event_buffer_size?: number
  }
  http_webhook?: {
    url: string
  }
  ws?: {
    host?: string
    port?: number
  }
  ws_reverse?: {
    url: string
    reconnect_interval?: number
  }
}

export interface UpdatePlatformRequest {
  id?: string
  type?: PlatformType
  enable?: boolean
  // DingTalk fields
  client_id?: string
  client_secret?: string
  // Discord fields
  token?: string
  pre_response_reactions?: boolean
  reaction_emojis?: string[]
  // WeChat (weixin_oc) fields
  account_id?: string
  base_url?: string
  cdn_base_url?: string
  proxy_url?: string
  // OneBot12 fields
  platform?: string
  self_user_id?: string
  access_token?: string
  http?: {
    host?: string
    port?: number
    event_enabled?: boolean
    event_buffer_size?: number
  }
  http_webhook?: {
    url: string
  }
  ws?: {
    host?: string
    port?: number
  }
  ws_reverse?: {
    url: string
    reconnect_interval?: number
  }
}

// ─── Proxy Config Types ──────────────────────────────────────────

export type ProxyMode = 'global' | 'rules'

/** Clash-style proxy rule type */
export type ProxyRuleType = 'domain' | 'domain-suffix' | 'domain-keyword' | 'ip-cidr' | 'geoip' | 'match'

/** Display label mapping for ProxyRuleType */
export const ProxyRuleTypeLabels: Record<ProxyRuleType, string> = {
  'domain': 'DOMAIN',
  'domain-suffix': 'DOMAIN-SUFFIX',
  'domain-keyword': 'DOMAIN-KEYWORD',
  'ip-cidr': 'IP-CIDR',
  'geoip': 'GEOIP',
  'match': 'MATCH',
}

/** A single Clash-style proxy rule (e.g. DOMAIN-SUFFIX,discord.gg) */
export interface ProxyRule {
  rule_type: ProxyRuleType
  value: string
}

export interface ProxyConfig {
  enabled: boolean
  url: string
  mode: ProxyMode
  proxy_domains: string[]
  bypass_domains: string[]
  username?: string | null
  password?: string | null
  bypass_localhost: boolean
  /** Clash-style rules (preferred over proxy_domains/bypass_domains when non-empty) */
  rules: ProxyRule[]
}

// ─── Knowledge Base Types ──────────────────────────────────────

export interface EmbeddingProviderConfig {
  base_url: string
  api_key?: string | null
  model: string
  dimension: number
}

export interface RerankProviderConfig {
  base_url: string
  api_key?: string | null
  model: string
}

export interface KnowledgeBase {
  id: string
  name: string
  description: string
  embedding_provider_config: EmbeddingProviderConfig
  rerank_provider_config: RerankProviderConfig | null
  chunk_size: number
  chunk_overlap: number
  document_count: number
  chunk_count: number
  created_at: string
  updated_at: string
}

export interface CreateKnowledgeBaseRequest {
  name: string
  description?: string
  embedding_provider_config: EmbeddingProviderConfig
  rerank_provider_config?: RerankProviderConfig | null
  chunk_size?: number
  chunk_overlap?: number
}

export interface UpdateKnowledgeBaseRequest {
  name?: string
  description?: string
  rerank_provider_config?: RerankProviderConfig | null
  chunk_size?: number
  chunk_overlap?: number
}

export interface KbDocument {
  id: string
  knowledge_base_id: string
  filename: string
  file_size: number
  file_type: string
  content_hash: string
  chunk_count: number
  status: string
  error_message: string | null
  created_at: string
  updated_at: string
}

export interface SearchResult {
  content: string
  score: number
  source: string
  chunk_index: number
}

export interface SearchRequest {
  query: string
  top_k?: number
}

// ─── Built-in Command Types ──────────────────────────────────────

export interface BuiltinCommand {
  name: string
  description: string
  usage: string
  /** Current effective admin requirement (may be overridden via config). */
  require_admin: boolean
  /** Built-in default admin requirement for this command. */
  default_require_admin: boolean
  hidden: boolean
  /** Whether this command is enabled in the active config profile. */
  enabled: boolean
}

// ─── Auth Types ─────────────────────────────────────────────────

export interface UserInfo {
  id: string
  username: string
  must_change_password: boolean
  avatar_url?: string
}

export interface LoginRequest {
  username: string
  password: string
  remember_me?: boolean
}

export interface LoginResponse {
  token: string
  user: UserInfo
}

export interface ChangePasswordRequest {
  old_password: string
  new_password: string
}

// ─── Persona Mode Types ──────────────────────────────────────────

/** Controls how persona is resolved for a debug session.
 * - 'default': use the existing fallback chain (embedded persona → profile persona → global persona)
 * - 'none': never inject any persona, regardless of profile or global settings
 * - 'custom': use the embedded persona from the debug session only
 */
export type PersonaMode = 'default' | 'none' | 'custom'

// ─── Debug Session Types ─────────────────────────────────────────

export interface EmbeddedPersona {
  name: string
  description: string
  prompt: string
}

export interface EmbeddedProvider {
  id: string
  name: string
  provider_type: string
  config: ProviderConfig
}

export interface EmbeddedSkill {
  name: string
  description: string
  skill_type: string
  config: Record<string, unknown>
}

export interface DebugSession {
  persona_mode: PersonaMode
  persona: EmbeddedPersona | null
  providers: EmbeddedProvider[]
  active_provider: string | null
  provider_id: string | null
  temperature: number | null
  max_tokens: number | null
  custom_error_message: string | null
  knowledge_base_ids: string[]
  skills: EmbeddedSkill[]
  active_skill_names: string[]
  command_prefix: string
  enabled_commands: string[]
}

export interface UpdateDebugSessionRequest {
  persona_mode?: PersonaMode
  persona?: EmbeddedPersona | null
  providers?: EmbeddedProvider[]
  active_provider?: string | null
  provider_id?: string | null
  temperature?: number | null
  max_tokens?: number | null
  custom_error_message?: string | null
  knowledge_base_ids?: string[]
  skills?: EmbeddedSkill[]
  active_skill_names?: string[]
  command_prefix?: string
  enabled_commands?: string[]
}



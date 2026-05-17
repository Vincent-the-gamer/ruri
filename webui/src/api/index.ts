import axios from 'axios'
import type {
  AgentStatus,
  AcpConfig,
  BuiltinCommand,
  ChangePasswordRequest,
  ChatRequest,
  ChatResponse,
  ChatMessage,
  ComputerUseConfig,
  ConfigProfile,
  CreateConfigProfileRequest,
  CreateKnowledgeBaseRequest,
  CreateMcpServerRequest,
  CreatePersonaRequest,
  CreatePlatformRequest,
  CreateProviderRequest,
  CreateSkillRequest,
  DebugSession,
  FetchModelsResponse,
  KbDocument,
  KnowledgeBase,
  LogEntry,
  LogLevel,
  LoginRequest,
  LoginResponse,
  McpServerConfig,
  Persona,
  PlatformInstance,
  Provider,
  SearchResult,
  SearchRequest,
  Skill,
  StreamEvent,
  Tool,
  UpdateAcpConfigRequest,
  UpdateComputerUseConfigRequest,
  UpdateConfigProfileRequest,
  UpdateDebugSessionRequest,
  UpdateKnowledgeBaseRequest,
  UpdateMcpServerRequest,
  UpdatePersonaRequest,
  UpdatePlatformRequest,
  UpdateWebSearchConfigRequest,
  UploadSkillPackageResponse,
  UserInfo,
  WsFilterCommand,
  WsGetSinceCommand,
  WebSearchConfig,
} from '../types'

const client = axios.create({
  baseURL: '',
  withCredentials: true,
  // Remove default Content-Type to allow FormData multipart uploads to work correctly
  // headers: {
  //   'Content-Type': 'application/json',
  // },
})

// Response interceptor for error handling
client.interceptors.response.use(
  (response) => response,
  (error) => {
    const message = error.response?.data?.error || error.message || 'Unknown error'
    console.error('[API Error]', message)

    // If we get a 401 Unauthorized response, the session has expired
    // Clear local auth state and redirect to login
    if (error.response?.status === 401) {
      // Import guards to avoid circular dependency - use direct localStorage manipulation
      localStorage.removeItem('auth_token')
      localStorage.removeItem('auth_user')
      // Only redirect if not already on login page
      if (!window.location.pathname.startsWith('/login') && !window.location.pathname.startsWith('/change-password')) {
        window.location.href = '/login'
      }
    }

    return Promise.reject(new Error(message))
  },
)

// ─── Providers ───────────────────────────────────────────────────

export async function getProviders(): Promise<Provider[]> {
  const res = await client.get('/api/providers')
  return res.data
}

export async function getProvider(id: string): Promise<Provider> {
  const res = await client.get(`/api/providers/${id}`)
  return res.data
}

export async function createProvider(data: CreateProviderRequest): Promise<Provider> {
  const res = await client.post('/api/providers', data)
  return res.data
}

export async function updateProvider(id: string, data: Partial<CreateProviderRequest>): Promise<Provider> {
  const res = await client.put(`/api/providers/${id}`, data)
  return res.data
}

export async function deleteProvider(id: string): Promise<void> {
  await client.delete(`/api/providers/${id}`)
}

export async function activateProvider(id: string): Promise<Provider> {
  const res = await client.post(`/api/providers/${id}/activate`)
  return res.data
}

export async function fetchProviderModels(data: {
  provider_type: string
  base_url: string
  api_key: string
}): Promise<FetchModelsResponse> {
  const res = await client.post('/api/providers/models', data)
  return res.data
}

// ─── Skills ──────────────────────────────────────────────────────

export async function getSkills(): Promise<Skill[]> {
  const res = await client.get('/api/skills')
  return res.data
}

export async function addSkill(data: CreateSkillRequest): Promise<Skill> {
  const res = await client.post('/api/skills', data)
  return res.data
}

export async function uploadSkillPackage(file: File): Promise<UploadSkillPackageResponse> {
  const formData = new FormData()
  formData.append('file', file)
  const res = await client.post('/api/skills/upload', formData)
  return res.data
}

export async function removeSkill(name: string): Promise<void> {
  await client.delete(`/api/skills/${name}`)
}

export async function toggleSkill(name: string, isActive: boolean): Promise<Skill> {
  const res = await client.patch(`/api/skills/${name}`, { is_active: isActive })
  return res.data
}

// ─── Tools ───────────────────────────────────────────────────────

export async function getTools(): Promise<Tool[]> {
  const res = await client.get('/api/tools')
  return res.data
}

// ─── Chat ────────────────────────────────────────────────────────

export async function sendMessage(data: ChatRequest): Promise<ChatResponse> {
  const res = await client.post('/api/chat', data)
  return res.data
}

/**
 * Send a chat message and receive streaming SSE events.
 * Returns an async generator that yields StreamEvent objects.
 */
export async function* sendMessageStream(
  data: ChatRequest,
): AsyncGenerator<StreamEvent> {
  const protocol = window.location.protocol === 'https:' ? 'https:' : 'http:'
  const host = window.location.host
  const url = `${protocol}//${host}/api/chat/stream`

  // Get the auth cookie (axios sends it automatically with withCredentials)
  // We use fetch for SSE streaming support
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    credentials: 'include',
    body: JSON.stringify(data),
  })

  if (!response.ok) {
    const errorText = await response.text()
    let errorMsg = `Stream request failed: ${response.status}`
    try {
      const errorJson = JSON.parse(errorText)
      if (errorJson.error) errorMsg = errorJson.error
    } catch {
      // Use default error message
    }
    throw new Error(errorMsg)
  }

  const reader = response.body?.getReader()
  if (!reader) {
    throw new Error('No response body')
  }

  const decoder = new TextDecoder()
  let buffer = ''

  while (true) {
    const { done, value } = await reader.read()
    if (done) break

    buffer += decoder.decode(value, { stream: true })

    // Process complete SSE events (separated by double newlines)
    const lines = buffer.split('\n')
    // Keep the last incomplete line in the buffer
    buffer = lines.pop() || ''

    for (const line of lines) {
      const trimmed = line.trim()
      if (trimmed.startsWith('data:')) {
        const data = trimmed.slice(5).trim()
        if (!data) continue

        try {
          const event: StreamEvent = JSON.parse(data)
          yield event
        } catch {
          // Skip malformed events
        }
      }
    }
  }

  // Process any remaining data in buffer
  if (buffer.trim().startsWith('data:')) {
    const data = buffer.trim().slice(5).trim()
    if (data) {
      try {
        const event: StreamEvent = JSON.parse(data)
        yield event
      } catch {
        // Skip malformed events
      }
    }
  }
}

export async function getChatHistory(): Promise<ChatMessage[]> {
  const res = await client.get('/api/chat/history')
  return res.data
}

export async function clearChatHistory(): Promise<void> {
  await client.delete('/api/chat/history')
}

export async function stopChatGeneration(sessionId?: string): Promise<{ stopped: boolean; session_id: string }> {
  const res = await client.post('/api/chat/stop', { session_id: sessionId })
  return res.data
}

// ─── ACP ──────────────────────────────────────────────────────────

export async function getAcpConfig(): Promise<AcpConfig> {
  const res = await client.get('/api/acp/config')
  return res.data
}

export async function updateAcpConfig(data: UpdateAcpConfigRequest): Promise<AcpConfig> {
  const res = await client.put('/api/acp/config', data)
  return res.data
}

// ─── Computer Use ───────────────────────────────────────────────────

export async function getComputerUseConfig(): Promise<ComputerUseConfig> {
  const res = await client.get('/api/computer-use/config')
  return res.data
}

export async function updateComputerUseConfig(data: UpdateComputerUseConfigRequest): Promise<ComputerUseConfig> {
  const res = await client.put('/api/computer-use/config', data)
  return res.data
}

// ─── Agent ───────────────────────────────────────────────────────

export async function getAgentStatus(): Promise<AgentStatus> {
  const res = await client.get('/api/agent/status')
  return res.data
}

// ─── Logs ─────────────────────────────────────────────────────────

export async function getLogs(): Promise<LogEntry[]> {
  const res = await client.get('/api/logs')
  return res.data
}

export async function clearLogs(): Promise<void> {
  await client.delete('/api/logs')
}

/**
 * Open a WebSocket connection to the log stream.
 * Supports reconnection by sending `get_since` command after connect,
 * and level filtering via `filter` command.
 */
export function openLogsStream(): WebSocket {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const host = window.location.host
  return new WebSocket(`${protocol}//${host}/api/logs/stream`)
}

/** Send a filter command to the log WebSocket */
export function sendLogFilter(ws: WebSocket, level: LogLevel): void {
  if (ws.readyState === WebSocket.OPEN) {
    const cmd: WsFilterCommand = { type: 'filter', level }
    ws.send(JSON.stringify(cmd))
  }
}

/** Send a get_since command to fill gap after reconnection */
export function sendGetSince(ws: WebSocket, timestamp: number): void {
  if (ws.readyState === WebSocket.OPEN) {
    const cmd: WsGetSinceCommand = { type: 'get_since', timestamp }
    ws.send(JSON.stringify(cmd))
  }
}

// ─── Web Search ───────────────────────────────────────────────────

export async function getWebSearchConfig(): Promise<WebSearchConfig> {
  const res = await client.get('/api/web-search/config')
  return res.data
}

export async function updateWebSearchConfig(data: UpdateWebSearchConfigRequest): Promise<WebSearchConfig> {
  const res = await client.put('/api/web-search/config', data)
  return res.data
}

// ─── Persona Library ──────────────────────────────────────────────

export async function getPersonas(): Promise<Persona[]> {
  const res = await client.get('/api/personas')
  return res.data
}

export async function getPersona(id: string): Promise<Persona> {
  const res = await client.get(`/api/personas/${id}`)
  return res.data
}

export async function createPersona(data: CreatePersonaRequest): Promise<Persona> {
  const res = await client.post('/api/personas', data)
  return res.data
}

export async function updatePersona(id: string, data: UpdatePersonaRequest): Promise<Persona> {
  const res = await client.put(`/api/personas/${id}`, data)
  return res.data
}

export async function deletePersona(id: string): Promise<void> {
  await client.delete(`/api/personas/${id}`)
}

// ─── Config Profiles ─────────────────────────────────────────────

export async function getConfigProfiles(): Promise<ConfigProfile[]> {
  const res = await client.get('/api/config-profiles')
  return res.data
}

export async function getConfigProfile(id: string): Promise<ConfigProfile> {
  const res = await client.get(`/api/config-profiles/${id}`)
  return res.data
}

export async function createConfigProfile(data: CreateConfigProfileRequest): Promise<ConfigProfile> {
  const res = await client.post('/api/config-profiles', data)
  return res.data
}

export async function updateConfigProfile(id: string, data: UpdateConfigProfileRequest): Promise<ConfigProfile> {
  const res = await client.put(`/api/config-profiles/${id}`, data)
  return res.data
}

export async function deleteConfigProfile(id: string): Promise<void> {
  await client.delete(`/api/config-profiles/${id}`)
}

export async function activateConfigProfile(id: string): Promise<ConfigProfile> {
  const res = await client.post(`/api/config-profiles/${id}/activate`)
  return res.data
}

export async function deactivateConfigProfile(id: string): Promise<ConfigProfile> {
  const res = await client.post(`/api/config-profiles/${id}/deactivate`)
  return res.data
}

export async function getConfigProfileProvider(profileId: string): Promise<Provider | null> {
  const res = await client.get(`/api/config-profiles/${profileId}/provider`)
  return res.data.provider || null
}

// ─── MCP ─────────────────────────────────────────────────────────

export async function getMcpServers(): Promise<McpServerConfig[]> {
  const res = await client.get('/api/mcp/servers')
  return res.data
}

export async function getMcpServer(id: string): Promise<McpServerConfig> {
  const res = await client.get(`/api/mcp/servers/${id}`)
  return res.data
}

export async function createMcpServer(data: CreateMcpServerRequest): Promise<McpServerConfig> {
  const res = await client.post('/api/mcp/servers', data)
  return res.data
}

export async function updateMcpServer(id: string, data: UpdateMcpServerRequest): Promise<McpServerConfig> {
  const res = await client.put(`/api/mcp/servers/${id}`, data)
  return res.data
}

export async function deleteMcpServer(id: string): Promise<void> {
  await client.delete(`/api/mcp/servers/${id}`)
}

export async function toggleMcpServer(id: string): Promise<McpServerConfig> {
  const res = await client.post(`/api/mcp/servers/${id}/toggle`)
  return res.data
}

// ─── Platforms ──────────────────────────────────────────────────

export async function getPlatforms(): Promise<PlatformInstance[]> {
  const res = await client.get('/api/platforms')
  return res.data
}

export async function getPlatform(id: string): Promise<PlatformInstance> {
  const res = await client.get(`/api/platforms/${id}`)
  return res.data
}

export async function createPlatform(data: CreatePlatformRequest): Promise<PlatformInstance> {
  const res = await client.post('/api/platforms', data)
  return res.data
}

export async function updatePlatform(id: string, data: UpdatePlatformRequest): Promise<PlatformInstance> {
  const res = await client.put(`/api/platforms/${id}`, data)
  return res.data
}

export async function deletePlatform(id: string): Promise<void> {
  await client.delete(`/api/platforms/${id}`)
}

export async function restartPlatform(id: string): Promise<PlatformInstance> {
  const res = await client.post(`/api/platforms/${id}/restart`)
  return res.data
}

export async function weixinQrLoginStart(id: string): Promise<{ qrcode: string; qrcode_img_content: string }> {
  const res = await client.post(`/api/platforms/${id}/weixin-qr-login`)
  return res.data
}

export async function weixinQrLoginStatus(id: string, qrcode: string): Promise<{
  status: string
  bot_token?: string
  ilink_bot_id?: string
  baseurl?: string
  ilink_user_id?: string
  redirect_host?: string
}> {
  const res = await client.get(`/api/platforms/${id}/weixin-qr-status`, {
    params: { qrcode },
  })
  return res.data
}

// ─── System ─────────────────────────────────────────────

export async function restartSystem(): Promise<{ message: string }> {
  const res = await client.post('/api/system/restart')
  return res.data
}

// ─── Knowledge Base ──────────────────────────────────────────

export async function getKnowledgeBases(): Promise<KnowledgeBase[]> {
  const res = await client.get('/api/knowledge-bases')
  return res.data
}

export async function getKnowledgeBase(id: string): Promise<KnowledgeBase> {
  const res = await client.get(`/api/knowledge-bases/${id}`)
  return res.data
}

export async function createKnowledgeBase(data: CreateKnowledgeBaseRequest): Promise<KnowledgeBase> {
  const res = await client.post('/api/knowledge-bases', data)
  return res.data
}

export async function updateKnowledgeBase(id: string, data: UpdateKnowledgeBaseRequest): Promise<KnowledgeBase> {
  const res = await client.put(`/api/knowledge-bases/${id}`, data)
  return res.data
}

export async function deleteKnowledgeBase(id: string): Promise<void> {
  await client.delete(`/api/knowledge-bases/${id}`)
}

export async function getKbDocuments(kbId: string): Promise<KbDocument[]> {
  const res = await client.get(`/api/knowledge-bases/${kbId}/documents`)
  return res.data
}

export async function uploadKbDocument(kbId: string, files: File[]): Promise<KbDocument[]> {
  const formData = new FormData()
  for (const file of files) {
    formData.append('files', file)
  }
  const res = await client.post(`/api/knowledge-bases/${kbId}/documents`, formData)
  return res.data
}

export async function deleteKbDocument(kbId: string, docId: string): Promise<void> {
  await client.delete(`/api/knowledge-bases/${kbId}/documents/${docId}`)
}

export async function searchKnowledgeBase(kbId: string, data: SearchRequest): Promise<SearchResult[]> {
  const res = await client.post(`/api/knowledge-bases/${kbId}/search`, data)
  return res.data
}

// ─── Built-in Commands ──────────────────────────────────────────

export async function getBuiltinCommands(): Promise<BuiltinCommand[]> {
  const res = await client.get('/api/commands')
  return res.data
}

export async function toggleCommandAdmin(commandName: string, requireAdmin: boolean): Promise<{ command: string; require_admin: boolean }> {
  const res = await client.patch(`/api/commands/${commandName}/admin`, { require_admin: requireAdmin })
  return res.data
}

// ─── Auth ──────────────────────────────────────────────────────

export async function login(data: LoginRequest): Promise<LoginResponse> {
  const res = await client.post('/api/auth/login', data)
  return res.data
}

export async function logout(): Promise<{ message: string }> {
  const res = await client.post('/api/auth/logout')
  return res.data
}

export async function getCurrentUser(): Promise<UserInfo> {
  const res = await client.get('/api/auth/me')
  return res.data
}

export async function changePassword(data: ChangePasswordRequest): Promise<{ message: string }> {
  const res = await client.post('/api/auth/change-password', data)
  return res.data
}

export async function updateUsername(data: { new_username: string }): Promise<{ message: string }> {
  const res = await client.post('/api/auth/update-username', data)
  return res.data
}

export async function uploadAvatar(file: File): Promise<{ avatar_url: string }> {
  const formData = new FormData()
  formData.append('avatar', file)
  const res = await client.post('/api/auth/upload-avatar', formData, {
    headers: {
      'Content-Type': 'multipart/form-data',
    },
  })
  return res.data
}

// ─── Debug Session ──────────────────────────────────────────────

export async function getDebugSession(): Promise<DebugSession> {
  const res = await client.get('/api/debug-session')
  return res.data
}

export async function updateDebugSession(data: UpdateDebugSessionRequest): Promise<DebugSession> {
  const res = await client.put('/api/debug-session', data)
  return res.data
}

import axios from 'axios'
import type {
  AgentStatus,
  AcpConfig,
  ChatRequest,
  ChatResponse,
  ChatMessage,
  ComputerUseConfig,
  ConfigProfile,
  CreateConfigProfileRequest,
  CreateMcpServerRequest,
  CreateProviderRequest,
  CreateSkillRequest,
  CreatePersonaRequest,
  LogEntry,
  McpServerConfig,
  Persona,
  Provider,
  Skill,
  Tool,
  UpdateAcpConfigRequest,
  UpdateComputerUseConfigRequest,
  UpdateConfigProfileRequest,
  UpdateMcpServerRequest,
  UpdatePersonaRequest,
  UpdateWebSearchConfigRequest,
  UploadSkillPackageResponse,
  WebSearchConfig,
} from '../types'

const client = axios.create({
  baseURL: '',
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

export async function getChatHistory(): Promise<ChatMessage[]> {
  const res = await client.get('/api/chat/history')
  return res.data
}

export async function clearChatHistory(): Promise<void> {
  await client.delete('/api/chat/history')
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

export function openLogsStream(): WebSocket {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const host = window.location.host
  return new WebSocket(`${protocol}//${host}/api/logs/stream`)
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

// ─── Personas ─────────────────────────────────────────────────────

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

export async function activatePersona(id: string): Promise<Persona> {
  const res = await client.patch(`/api/personas/${id}/activate`)
  return res.data
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

export async function getConfigProfileProvider(profileId: string): Promise<Provider | null> {
  const res = await client.get(`/api/config-profiles/${profileId}/provider`)
  return res.data.provider || null
}

export async function getConfigProfilePersona(profileId: string): Promise<Persona | null> {
  const res = await client.get(`/api/config-profiles/${profileId}/persona`)
  return res.data.persona || null
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

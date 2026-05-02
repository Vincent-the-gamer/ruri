import axios from 'axios'
import type {
  AgentStatus,
  AcpConfig,
  ChatRequest,
  ChatResponse,
  ChatMessage,
  CreateProviderRequest,
  CreateSkillRequest,
  Provider,
  Skill,
  Tool,
  UpdateAcpConfigRequest,
  UploadSkillPackageResponse,
} from '../types'

const client = axios.create({
  baseURL: '',
  timeout: 60000,
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

// ─── Agent ───────────────────────────────────────────────────────

export async function getAgentStatus(): Promise<AgentStatus> {
  const res = await client.get('/api/agent/status')
  return res.data
}

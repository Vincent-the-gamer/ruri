import axios, { type AxiosResponse } from 'axios'

export interface Conversation {
  id: string
  bot_name: string
  chat_type: 'group' | 'private'
  chat_id: string
  title: string | null
  created_at: string
  updated_at: string
}

export interface Message {
  id: string
  conversation_id: string
  role: string
  content: string
  created_at: string
}

export interface ConversationFilter {
  bot_name?: string
  chat_type?: 'group' | 'private'
  keyword?: string
}

export interface CreateConversationRequest {
  bot_name: string
  chat_type: 'group' | 'private'
  chat_id: string
  title?: string
}

export interface AddMessageRequest {
  role: string
  content: string
}

// 获取对话列表
export async function listConversations(
  filter?: ConversationFilter
): Promise<AxiosResponse<Conversation[]>> {
  const params = new URLSearchParams()
  if (filter?.bot_name) {
    params.append('bot_name', filter.bot_name)
  }
  if (filter?.chat_type) {
    params.append('chat_type', filter.chat_type)
  }
  if (filter?.keyword) {
    params.append('keyword', filter.keyword)
  }
  return axios.get<Conversation[]>(`/api/conversations${params.toString() ? `?${params.toString()}` : ''}`)
}

// 创建新对话
export async function createConversation(
  data: CreateConversationRequest
): Promise<AxiosResponse<Conversation>> {
  return axios.post<Conversation>('/api/conversations', data)
}

// 获取单个对话
export async function getConversation(
  id: string
): Promise<AxiosResponse<Conversation>> {
  return axios.get<Conversation>(`/api/conversations/${id}`)
}

// 删除对话
export async function deleteConversation(id: string): Promise<void> {
  return axios.delete(`/api/conversations/${id}`)
}

// 添加消息
export async function addMessage(
  conversationId: string,
  data: AddMessageRequest
): Promise<AxiosResponse<Message>> {
  return axios.post<Message>(`/api/conversations/${conversationId}/messages`, data)
}

// 获取对话的所有消息
export async function getConversationMessages(
  conversationId: string
): Promise<AxiosResponse<Message[]>> {
  return axios.get<Message[]>(`/api/conversations/${conversationId}/messages`)
}

export default {
  listConversations,
  createConversation,
  getConversation,
  deleteConversation,
  addMessage,
  getConversationMessages,
}

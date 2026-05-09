import axios from 'axios'

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
): Promise<Conversation[]> {
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
  const res = await axios.get<Conversation[]>(`/api/conversations${params.toString() ? `?${params.toString()}` : ''}`)
  return res.data
}

// 创建新对话
export async function createConversation(
  data: CreateConversationRequest
): Promise<Conversation> {
  const res = await axios.post<Conversation>('/api/conversations', data)
  return res.data
}

// 获取单个对话
export async function getConversation(
  id: string
): Promise<Conversation> {
  const res = await axios.get<Conversation>(`/api/conversations/${id}`)
  return res.data
}

// 删除对话
export async function deleteConversation(id: string): Promise<void> {
  await axios.delete(`/api/conversations/${id}`)
}

// 添加消息
export async function addMessage(
  conversationId: string,
  data: AddMessageRequest
): Promise<Message> {
  const res = await axios.post<Message>(`/api/conversations/${conversationId}/messages`, data)
  return res.data
}

// 获取对话的所有消息
export async function getConversationMessages(
  conversationId: string
): Promise<Message[]> {
  const res = await axios.get<Message[]>(`/api/conversations/${conversationId}/messages`)
  return res.data
}

export default {
  listConversations,
  createConversation,
  getConversation,
  deleteConversation,
  addMessage,
  getConversationMessages,
}

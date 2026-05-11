import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { ChatMessage, ChatRequest, ContentPart } from '../types'
import * as api from '../api'

export const useChatStore = defineStore('chat', () => {
  const messages = ref<ChatMessage[]>([])
  const loading = ref(false)
  const sending = ref(false)
  const error = ref<string | null>(null)

  // Computed: true when the agent is actively processing a message
  const isThinking = computed(() => sending.value)

  // Always fetch chat history from database, no local caching
  async function fetchHistory() {
    loading.value = true
    error.value = null
    try {
      const serverHistory = await api.getChatHistory()
      messages.value = serverHistory || []
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch chat history'
      messages.value = []
    } finally {
      loading.value = false
    }
  }

  async function sendMessage(req: ChatRequest) {
    // Build display content for the optimistic user message
    const contentParts: ContentPart[] = [];

    if (req.images && req.images.length > 0) {
      for (const img of req.images) {
        contentParts.push({
          type: 'image_url',
          image_url: { url: img }
        });
      }
    }

    // Show attached files as text placeholders in the message
    if (req.files && req.files.length > 0) {
      for (const file of req.files) {
        // For text files, show content; for binary, just show filename
        const isText = file.mime_type.startsWith('text/') || !file.content.startsWith('data:')
        contentParts.push({
          type: 'text',
          text: isText
            ? `--- File: ${file.name} ---\n${file.content.length > 2000 ? file.content.slice(0, 2000) + '\n... (truncated)' : file.content}`
            : `📎 ${file.name}`
        });
      }
    }

    contentParts.push({ type: 'text', text: req.message });

    const hasMultiContent = (req.images && req.images.length > 0) || (req.files && req.files.length > 0)
    const userMessage: ChatMessage = {
      role: 'user',
      content: hasMultiContent ? contentParts : req.message,
    }
    messages.value.push(userMessage)

    sending.value = true
    error.value = null
    try {
      const response = await api.sendMessage(req)
      // Add assistant response
      messages.value.push(response.message)
      // Add tool result messages if any
      if (response.tool_results) {
        for (const tr of response.tool_results) {
          messages.value.push({
            role: 'tool',
            content: tr.content,
            tool_call_id: tr.tool_call_id,
          })
        }
      }
      return response
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to send message'
      // Add error as assistant message
      messages.value.push({
        role: 'assistant',
        content: `⚠️ Error: ${error.value}`,
      })
      throw e
    } finally {
      sending.value = false
    }
  }

  async function clearHistory() {
    loading.value = true
    error.value = null
    try {
      await api.clearChatHistory()
      messages.value = []
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to clear chat history'
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    messages,
    loading,
    sending,
    isThinking,
    error,
    fetchHistory,
    sendMessage,
    clearHistory,
  }
})

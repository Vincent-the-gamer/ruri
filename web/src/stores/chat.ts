import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import type { ChatMessage, ChatRequest } from '../types'
import * as api from '../api'

const CHAT_HISTORY_KEY = 'ruri_chat_history'

export const useChatStore = defineStore('chat', () => {
  const messages = ref<ChatMessage[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Load from localStorage on initialization
  function loadFromLocalStorage() {
    try {
      const saved = localStorage.getItem(CHAT_HISTORY_KEY)
      if (saved) {
        messages.value = JSON.parse(saved)
      }
    } catch (e) {
      console.error('Failed to load chat history from localStorage:', e)
    }
  }

  // Save to localStorage whenever messages change
  watch(messages, (newMessages) => {
    try {
      localStorage.setItem(CHAT_HISTORY_KEY, JSON.stringify(newMessages))
    } catch (e) {
      console.error('Failed to save chat history to localStorage:', e)
    }
  }, { deep: true })

  // Load persisted messages on store initialization
  loadFromLocalStorage()

  async function fetchHistory() {
    loading.value = true
    error.value = null
    try {
      const serverHistory = await api.getChatHistory()
      // Merge with local history, preferring more recent/complete data
      if (serverHistory && serverHistory.length > 0) {
        messages.value = serverHistory
      }
      // If server returns empty but we have local history, keep it
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch chat history'
      // Don't clear local messages on fetch error
    } finally {
      loading.value = false
    }
  }

  async function sendMessage(req: ChatRequest) {
    // Add user message optimistically
    const userMessage: ChatMessage = {
      role: 'user',
      content: req.message,
    }
    messages.value.push(userMessage)

    loading.value = true
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
      loading.value = false
    }
  }

  async function clearHistory() {
    loading.value = true
    error.value = null
    try {
      await api.clearChatHistory()
      messages.value = []
      localStorage.removeItem(CHAT_HISTORY_KEY)
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
    error,
    fetchHistory,
    sendMessage,
    clearHistory,
  }
})

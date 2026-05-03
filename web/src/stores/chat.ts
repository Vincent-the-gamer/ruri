import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import type { ChatMessage, ChatRequest } from '../types'
import * as api from '../api'

const CHAT_HISTORY_KEY = 'ruri_chat_history'
const CHAT_LOADING_KEY = 'ruri_chat_loading'

export const useChatStore = defineStore('chat', () => {
  const messages = ref<ChatMessage[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Persist loading state so thinking indicator survives page switches
  function persistLoading() {
    try {
      localStorage.setItem(CHAT_LOADING_KEY, JSON.stringify(loading.value))
    } catch { /* ignore */ }
  }

  function restoreLoading() {
    try {
      const saved = localStorage.getItem(CHAT_LOADING_KEY)
      if (saved) {
        loading.value = JSON.parse(saved)
      }
    } catch { /* ignore */ }
  }

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

  // Restore loading state from persistence
  restoreLoading()

  // Save to localStorage whenever messages change
  watch(messages, (newMessages) => {
    try {
      localStorage.setItem(CHAT_HISTORY_KEY, JSON.stringify(newMessages))
    } catch (e) {
      console.error('Failed to save chat history to localStorage:', e)
    }
  }, { deep: true })

  // Persist loading state whenever it changes
  watch(loading, persistLoading)

  // Load persisted messages on store initialization
  loadFromLocalStorage()

  // Clear loading persistence when loading finishes
  function clearLoadingPersistence() {
    try {
      localStorage.removeItem(CHAT_LOADING_KEY)
    } catch { /* ignore */ }
  }

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
      clearLoadingPersistence()
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
      clearLoadingPersistence()
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
      clearLoadingPersistence()
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

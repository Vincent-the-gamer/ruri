import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import type { ChatMessage, ChatRequest, ContentPart, StreamEvent } from '../types'
import * as api from '../api'

// ── sessionStorage cache helpers (cleared when browser closes) ──────────
const CHAT_CACHE_KEY = 'ruri_chat_messages_cache'

function saveMessagesToCache(messages: ChatMessage[]) {
  try {
    sessionStorage.setItem(CHAT_CACHE_KEY, JSON.stringify(messages))
  } catch {
    // sessionStorage might be full or unavailable, ignore
  }
}

function loadMessagesFromCache(): ChatMessage[] | null {
  try {
    const cached = sessionStorage.getItem(CHAT_CACHE_KEY)
    if (cached) {
      return JSON.parse(cached)
    }
  } catch {
    // ignore parse errors
  }
  return null
}

function clearMessagesCache() {
  try {
    sessionStorage.removeItem(CHAT_CACHE_KEY)
  } catch {
    // ignore
  }
}

export const useChatStore = defineStore('chat', () => {
  const messages = ref<ChatMessage[]>([])
  const loading = ref(false)
  const sending = ref(false)
  const error = ref<string | null>(null)
  /** The streaming assistant message content being built up in real-time */
  const streamingContent = ref<string>('')
  /** Whether we are currently in a streaming response */
  const isStreaming = ref(false)

  // Computed: true when the agent is actively processing a message
  const isThinking = computed(() => sending.value)

  // Cache-first strategy: read from sessionStorage for instant display,
  // then sync with database for consistency
  async function fetchHistory() {
    loading.value = true
    error.value = null

    // 1. Try to load from cache first for instant display
    const cached = loadMessagesFromCache()
    if (cached && cached.length > 0) {
      messages.value = cached
    }

    try {
      // 2. Always fetch from database for consistency
      const serverHistory = await api.getChatHistory()
      messages.value = serverHistory || []

      // 3. Update cache with latest data from database
      saveMessagesToCache(messages.value)
    } catch (e: unknown) {
      // If database fetch fails but we have cache, keep the cached data
      if (!cached || cached.length === 0) {
        error.value = e instanceof Error ? e.message : 'Failed to fetch chat history'
        messages.value = []
      }
      // Otherwise keep the cached data – better stale than nothing
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
        const isAudio = file.mime_type.startsWith('audio/') || file.name.match(/\.(mp3|wav|ogg|flac|aac|m4a|wma|webm|opus)$/i)
        const isText = file.mime_type.startsWith('text/') || !file.content.startsWith('data:')
        if (isAudio) {
          contentParts.push({
            type: 'text',
            text: `🎵 ${file.name}`
          });
        } else {
          contentParts.push({
            type: 'text',
            text: isText
              ? `--- File: ${file.name} ---\n${file.content.length > 2000 ? file.content.slice(0, 2000) + '\n... (truncated)' : file.content}`
              : `📎 ${file.name}`
          });
        }
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
    streamingContent.value = ''
    isStreaming.value = true

    try {
      // Use streaming API
      for await (const event of api.sendMessageStream(req)) {
        handleStreamEvent(event)
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to send message'
      // If we have partial streaming content, finalize it as an error
      if (streamingContent.value) {
        // Remove the partial streaming message and add error
        messages.value.push({
          role: 'assistant',
          content: streamingContent.value + `\n\n⚠️ Error: ${error.value}`,
        })
      } else {
        // Add error as assistant message
        messages.value.push({
          role: 'assistant',
          content: `⚠️ Error: ${error.value}`,
        })
      }
      streamingContent.value = ''
      isStreaming.value = false
      throw e
    } finally {
      sending.value = false
      isStreaming.value = false
      // Refresh history from database after streaming completes to ensure
      // the DB-stored messages are in sync with what was displayed.
      // This is done asynchronously so it doesn't block the UI.
      api.getChatHistory().then((serverHistory) => {
        if (serverHistory && serverHistory.length > 0) {
          messages.value = serverHistory
          saveMessagesToCache(messages.value)
        }
      }).catch(() => {
        // Ignore errors — the optimistic UI state is already shown
      })
    }
  }

  /** Handle a single SSE stream event */
  function handleStreamEvent(event: StreamEvent) {
    switch (event.type) {
      case 'content_delta': {
        streamingContent.value += event.delta
        // Update or create the assistant message in the messages array
        updateOrAddStreamingMessage()
        break
      }
      case 'tool_call_start': {
        // Tool call is starting - we may still be streaming content
        // Just log it, the UI can show a subtle indicator
        break
      }
      case 'tool_call_delta': {
        // Arguments being streamed for a tool call - not shown in chat
        break
      }
      case 'tool_call_end': {
        // A tool call completed
        break
      }
      case 'tool_result': {
        // A tool was executed - add as a tool message
        // If we have streaming content, finalize it first
        if (streamingContent.value) {
          finalizeStreamingMessage()
        }
        messages.value.push({
          role: 'tool',
          content: `🔧 **${event.tool_name}**: ${truncateContent(event.content, 500)}`,
          tool_call_id: event.tool_call_id,
        })
        break
      }
      case 'done': {
        // Stream completed
        finalizeStreamingMessage()
        break
      }
      case 'error': {
        if (streamingContent.value) {
          // Append error to existing content
          streamingContent.value += `\n\n⚠️ Error: ${event.error}`
          finalizeStreamingMessage()
        } else {
          messages.value.push({
            role: 'assistant',
            content: `⚠️ Error: ${event.error}`,
          })
        }
        break
      }
    }
  }

  /** Update the last assistant message with current streaming content, or add one */
  function updateOrAddStreamingMessage() {
    const lastMsg = messages.value[messages.value.length - 1]
    if (lastMsg && lastMsg.role === 'assistant' && isStreaming.value) {
      // Update existing streaming message
      lastMsg.content = streamingContent.value
    } else {
      // Add new assistant message
      messages.value.push({
        role: 'assistant',
        content: streamingContent.value,
      })
    }
  }

  /** Finalize the current streaming message and reset state */
  function finalizeStreamingMessage() {
    if (streamingContent.value) {
      const lastMsg = messages.value[messages.value.length - 1]
      if (lastMsg && lastMsg.role === 'assistant') {
        lastMsg.content = streamingContent.value
      } else {
        messages.value.push({
          role: 'assistant',
          content: streamingContent.value,
        })
      }
    }
    streamingContent.value = ''
  }

  /** Truncate content for display in tool result messages */
  function truncateContent(content: string, maxLength: number): string {
    if (content.length <= maxLength) return content
    return content.slice(0, maxLength) + '\n... (truncated)'
  }

  // Auto-sync messages to sessionStorage cache on every change
  watch(messages, (newMessages) => {
    saveMessagesToCache(newMessages)
  }, { deep: true })

  async function clearHistory() {
    loading.value = true
    error.value = null
    try {
      await api.clearChatHistory()
      messages.value = []
      clearMessagesCache()
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
    isStreaming,
    streamingContent,
    error,
    fetchHistory,
    sendMessage,
    clearHistory,
  }
})

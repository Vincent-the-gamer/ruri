import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import type { ChatMessage, ChatRequest, ContentPart, StreamEvent, ToolCall } from '../types'
import * as api from '../api'

// ── localStorage cache helpers (persists across page navigations & tab refreshes) ──────
const CHAT_CACHE_KEY = 'ruri_chat_messages_cache'

function saveMessagesToCache(messages: ChatMessage[]) {
  try {
    localStorage.setItem(CHAT_CACHE_KEY, JSON.stringify(messages))
  } catch {
    // localStorage might be full or unavailable, ignore
  }
}

function loadMessagesFromCache(): ChatMessage[] | null {
  try {
    const cached = localStorage.getItem(CHAT_CACHE_KEY)
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
    localStorage.removeItem(CHAT_CACHE_KEY)
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
  /** Whether we have done at least one successful database fetch (initial sync) */
  const _syncedWithDb = ref(false)
  /** Accumulated tool calls during the current streaming session */
  const _accumulatedToolCalls = ref<ToolCall[]>([])
  /** Whether the stream received a 'done' event (normal completion) */
  const _streamDoneReceived = ref(false)

  // Computed: true when the agent is actively processing a message
  const isThinking = computed(() => sending.value)

  /**
   * Initial load: cache-first strategy.
   * 1. Restore from localStorage for instant display (no loading spinner)
   * 2. Fetch from database in background to ensure consistency
   * 3. Silently update messages when database data arrives
   */
  async function fetchHistory() {
    error.value = null

    // 1. Try to restore from persistent cache first for instant display
    const cached = loadMessagesFromCache()
    if (cached && cached.length > 0) {
      messages.value = cached
    }

    // 2. Always fetch from database for consistency (background, non-blocking)
    try {
      const serverHistory = await api.getChatHistory()
      const serverMessages = serverHistory || []

      // Only replace if server actually returned data OR if we had no cache
      // This prevents wiping out optimistic messages that haven't been persisted yet
      if (serverMessages.length > 0 || messages.value.length === 0) {
        messages.value = serverMessages
      }

      // 3. Update cache with latest data from database
      saveMessagesToCache(messages.value)
      _syncedWithDb.value = true
    } catch (e: unknown) {
      // If database fetch fails but we have cache, keep the cached data
      if (!cached || cached.length === 0) {
        error.value = e instanceof Error ? e.message : 'Failed to fetch chat history'
        messages.value = []
      }
      // Otherwise keep the cached data – better stale than nothing
    }
  }

  /**
   * Gentle re-sync: called when the component is re-activated (keep-alive).
   * Unlike fetchHistory, this does NOT show a loading state and does NOT
   * replace messages if the store already has data. It only silently syncs
   * in the background if the data might be stale.
   */
  async function syncWithDatabase() {
    // Don't sync while streaming — the optimistic UI state is authoritative
    if (isStreaming.value || sending.value) return

    try {
      const serverHistory = await api.getChatHistory()
      const serverMessages = serverHistory || []

      // Replace only if server data differs or we have no current data
      if (serverMessages.length > 0) {
        messages.value = serverMessages
        saveMessagesToCache(messages.value)
      } else if (messages.value.length > 0) {
        // Server returned empty but we have local data —
        // this means the conversation was cleared externally
        // Only clear if we were previously synced (not optimistic)
        if (_syncedWithDb.value) {
          messages.value = []
          saveMessagesToCache(messages.value)
        }
      }
    } catch {
      // Silent sync failure — keep whatever we have
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

    // Guard: don't create an empty user message with no attachments
    const hasMessageText = req.message && req.message.trim().length > 0;
    const hasImages = req.images && req.images.length > 0;
    const hasFiles = req.files && req.files.length > 0;
    if (!hasMessageText && !hasImages && !hasFiles) return;

    const hasMultiContent = hasImages || hasFiles
    const userMessage: ChatMessage = {
      role: 'user',
      content: hasMultiContent ? contentParts : req.message,
    }
    messages.value.push(userMessage)

    sending.value = true
    error.value = null
    streamingContent.value = ''
    isStreaming.value = true
    _accumulatedToolCalls.value = []
    _streamDoneReceived.value = false

    try {
      // Use streaming API
      for await (const event of api.sendMessageStream(req)) {
        handleStreamEvent(event)
      }

      // If the stream ended without a done event, any content, or an error,
      // something went wrong (e.g. the connection was closed silently or the agent panicked).
      // A stream can legitimately have no text content if it was purely tool calls.
      if (!_streamDoneReceived.value && !streamingContent.value && !error.value) {
        messages.value.push({
          role: 'assistant',
          content: '⚠️ No response received from the model. The stream ended unexpectedly.',
        })
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
      _accumulatedToolCalls.value = []
      // Save current local messages to cache (they are the authoritative
      // view after a successful stream). Do NOT replace local state with
      // server history — the streaming response is the source of truth.
      // Server sync via getChatHistory is deferred to the next page load
      // or activation to avoid race conditions and content format mismatches.
      if (!error.value) {
        saveMessagesToCache(messages.value)
      } else {
        // On error, keep local state (includes error messages).
        // Still attempt to save for cache consistency.
        saveMessagesToCache(messages.value)
      }
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
        // Tool call is starting — ensure there's an assistant message to attach
        // tool_calls to, even if no text content has been streamed yet.
        // This guarantees tool calls appear before the response text in the UI.
        if (!streamingContent.value) {
          // Create a placeholder assistant message for the tool calls
          const lastMsg = messages.value[messages.value.length - 1]
          if (!lastMsg || lastMsg.role !== 'assistant' || !isStreaming.value) {
            messages.value.push({
              role: 'assistant',
              content: '',
              tool_calls: [],
            })
          }
        }
        break
      }
      case 'tool_executing': {
        // ── Natural-language tool progress ──
        // Instead of creating a separate tool block that feels cold and
        // disconnected, integrate the tool call notification into the
        // assistant's message flow as inline text. This makes the agent
        // feel like it's naturally telling the user what it's doing.
        const toolDesc = event.tool_name.replace(/_/g, ' ');
        const argsPreview = event.arguments_preview
          ? ` with \`${event.arguments_preview.length > 80 ? event.arguments_preview.slice(0, 80) + '...' : event.arguments_preview}\``
          : '';
        const statusMsg = `\n\n> 🔧 正在使用 \`${toolDesc}\`${argsPreview}...\n\n`;

        // Append the tool status inline to the current streaming content —
        // don't finalize or create a separate message. The status is part
        // of the assistant's natural response flow.
        streamingContent.value += statusMsg;
        updateOrAddStreamingMessage();

        // Also add a lightweight status marker so we can clean it up later
        // when tool_result arrives.
        const existingExecutingIdx = messages.value.findIndex(
          (m: any) => m._executing === true
        );
        if (existingExecutingIdx === -1) {
          messages.value.push({
            role: 'tool',
            content: '',
            tool_call_id: event.tool_call_id,
            tool_name: event.tool_name,
            _executing: true,
            _inline: true,  // mark as inline (no separate display)
          } as any);
        }
        break;
      }
      case 'tool_call_delta': {
        // Arguments being streamed for a tool call - not shown in chat
        break
      }
      case 'tool_call_end': {
        // A tool call completed — accumulate it for attachment to the
        // assistant message and add it to the current streaming message
        const toolCall: ToolCall = {
          id: event.tool_call_id,
          type: 'function',
          function: {
            name: event.function_name,
            arguments: event.arguments,
          },
        }
        _accumulatedToolCalls.value.push(toolCall)
        // Attach accumulated tool_calls to the current streaming assistant message
        attachToolCallsToStreamingMessage()
        break
      }
      case 'tool_result': {
        // Remove the executing status marker(s) — both inline markers
        // and any legacy separate tool messages.
        for (let i = messages.value.length - 1; i >= 0; i--) {
          const msg = messages.value[i] as any;
          if (msg.role === 'tool' && msg.tool_call_id === event.tool_call_id) {
            if (msg._executing) {
              // If this was an inline status marker (no content), just remove it.
              // If it had content (legacy style), remove the whole message.
              messages.value.splice(i, 1);
            }
            break;
          }
        }

        // Strip the inline tool status text from the assistant's streaming
        // content so the user sees the final result cleanly.
        if (streamingContent.value) {
          const statusPattern = /\n*> 🔧 正在使用 `[^`]+`[^\n]*\.\.\.\n*\n*/g;
          streamingContent.value = streamingContent.value.replace(statusPattern, '');
        }

        // If we have streaming content, finalize it
        if (streamingContent.value) {
          finalizeStreamingMessage();
        }

        // Skip empty tool results — they would create empty dialog boxes.
        // But if the tool result is non-empty, we append it as a footnote
        // to the last assistant message rather than as a separate block,
        // keeping the conversational flow natural.
        const resultContent = (event.content || '').trim();
        if (resultContent) {
          // Append tool result as a collapsible footnote to the last
          // assistant message rather than a separate tool block.
          const lastAssistant = findLastAssistantMessage();
          if (lastAssistant) {
            const preview = resultContent.length > 300
              ? resultContent.slice(0, 300) + '...'
              : resultContent;
            (lastAssistant as any)._tool_results = (lastAssistant as any)._tool_results || [];
            (lastAssistant as any)._tool_results.push({
              tool_name: event.tool_name,
              tool_call_id: event.tool_call_id,
              content: resultContent,
              preview,
            });
          } else {
            messages.value.push({
              role: 'tool',
              content: event.content,
              tool_call_id: event.tool_call_id,
              tool_name: event.tool_name,
            });
          }
        }
        break
      }
      case 'done': {
        // Stream completed — finalize any remaining streaming content
        // and attach accumulated tool_calls to the last assistant message
        _streamDoneReceived.value = true
        finalizeStreamingMessage()
        attachToolCallsToLastAssistantMessage()
        cleanupEmptyPlaceholders()
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

  /** Attach accumulated tool_calls to the current streaming assistant message */
  function attachToolCallsToStreamingMessage() {
    if (_accumulatedToolCalls.value.length === 0) return
    const lastMsg = messages.value[messages.value.length - 1]
    if (lastMsg && lastMsg.role === 'assistant') {
      lastMsg.tool_calls = [..._accumulatedToolCalls.value]
    }
  }

  /** Attach accumulated tool_calls to the last assistant message (called at stream end) */
  function attachToolCallsToLastAssistantMessage() {
    if (_accumulatedToolCalls.value.length === 0) return
    // Find the last assistant message
    for (let i = messages.value.length - 1; i >= 0; i--) {
      if (messages.value[i].role === 'assistant') {
        messages.value[i].tool_calls = [..._accumulatedToolCalls.value]
        break
      }
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

  /** Find the last assistant message in the list */
  function findLastAssistantMessage(): ChatMessage | undefined {
    for (let i = messages.value.length - 1; i >= 0; i--) {
      if (messages.value[i].role === 'assistant') {
        return messages.value[i]
      }
    }
    return undefined
  }

  /** Remove empty placeholder assistant messages created by tool_call_start
   *  that never got populated with content or tool_calls */
  function cleanupEmptyPlaceholders() {
    for (let i = messages.value.length - 1; i >= 0; i--) {
      const msg = messages.value[i]
      if (msg.role === 'assistant') {
        const hasContent = typeof msg.content === 'string'
          ? msg.content.length > 0
          : (Array.isArray(msg.content) && msg.content.length > 0)
        const hasToolCalls = msg.tool_calls && msg.tool_calls.length > 0
        if (!hasContent && !hasToolCalls) {
          messages.value.splice(i, 1)
          continue
        }
        break
      }
      // Also clean up inline status markers
      if (msg.role === 'tool' && (msg as any)._inline) {
        messages.value.splice(i, 1)
        continue
      }
      if (msg.role !== 'tool' && msg.role !== 'user') break
    }
  }

  // Auto-sync messages to localStorage cache on every change
  watch(messages, (newMessages) => {
    saveMessagesToCache(newMessages)
  }, { deep: true })

  /** Stop the currently running generation */
  async function stopGeneration() {
    if (!sending.value && !isStreaming.value) return
    try {
      await api.stopChatGeneration('webui')
    } catch {
      // Even if the API call fails, we should still update the local state
      // The server might have already finished or the connection might be broken
    }
    // Finalize any partial streaming content
    if (streamingContent.value) {
      finalizeStreamingMessage()
    }
    // Attach any accumulated tool calls
    attachToolCallsToLastAssistantMessage()
    cleanupEmptyPlaceholders()
    sending.value = false
    isStreaming.value = false
    _accumulatedToolCalls.value = []
  }

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
    syncWithDatabase,
    sendMessage,
    stopGeneration,
    clearHistory,
  }
})

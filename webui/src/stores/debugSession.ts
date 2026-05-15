import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { DebugSession, UpdateDebugSessionRequest } from '../types'
import * as api from '../api'

export const useDebugSessionStore = defineStore('debugSession', () => {
  const debugSession = ref<DebugSession | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Computed accessors
  const temperature = computed(() => debugSession.value?.temperature ?? null)
  const maxTokens = computed(() => debugSession.value?.max_tokens ?? null)
  const customErrorMessage = computed(() => debugSession.value?.custom_error_message ?? null)
  const knowledgeBaseIds = computed(() => debugSession.value?.knowledge_base_ids ?? [])
  const activeProvider = computed(() => debugSession.value?.active_provider ?? null)
  const providerId = computed(() => debugSession.value?.provider_id ?? null)
  const personaId = computed(() => debugSession.value?.persona_id ?? null)
  const embeddedPersona = computed(() => debugSession.value?.embedded_persona ?? null)
  const commandPrefix = computed(() => debugSession.value?.command_prefix ?? '/')
  const enabledCommands = computed(() => debugSession.value?.enabled_commands ?? [])
  const webSearchEnabled = computed(() => debugSession.value?.web_search_enabled ?? false)
  const computerUseEnabled = computed(() => debugSession.value?.computer_use_enabled ?? false)
  const proxyConfig = computed(() => debugSession.value?.proxy_config)
  const commandAdminRequired = computed(() => debugSession.value?.command_admin_required ?? {})

  async function fetchDebugSession() {
    loading.value = true
    error.value = null
    try {
      debugSession.value = await api.getDebugSession()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch debug session'
      console.warn('Debug session not available:', error.value)
    } finally {
      loading.value = false
    }
  }

  async function updateDebugSessionConfig(data: UpdateDebugSessionRequest) {
    loading.value = true
    error.value = null
    try {
      const updated = await api.updateDebugSession(data)
      debugSession.value = updated
      return updated
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update debug session'
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    debugSession,
    loading,
    error,
    temperature,
    maxTokens,
    customErrorMessage,
    knowledgeBaseIds,
    activeProvider,
    providerId,
    personaId,
    embeddedPersona,
    commandPrefix,
    enabledCommands,
    webSearchEnabled,
    computerUseEnabled,
    proxyConfig,
    commandAdminRequired,
    fetchDebugSession,
    updateDebugSessionConfig,
  }
})

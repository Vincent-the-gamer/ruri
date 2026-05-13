import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { DebugSession, UpdateDebugSessionRequest, PersonaMode } from '../types'
import * as api from '../api'

export const useDebugSessionStore = defineStore('debugSession', () => {
  const debugSession = ref<DebugSession | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Computed accessors
  const personaMode = computed<PersonaMode>(() => debugSession.value?.persona_mode ?? 'default')
  const temperature = computed(() => debugSession.value?.temperature ?? null)
  const maxTokens = computed(() => debugSession.value?.max_tokens ?? null)
  const customErrorMessage = computed(() => debugSession.value?.custom_error_message ?? null)
  const knowledgeBaseIds = computed(() => debugSession.value?.knowledge_base_ids ?? [])
  const activeProvider = computed(() => debugSession.value?.active_provider ?? null)
  const providerId = computed(() => debugSession.value?.provider_id ?? null)

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
    personaMode,
    temperature,
    maxTokens,
    customErrorMessage,
    knowledgeBaseIds,
    activeProvider,
    providerId,
    fetchDebugSession,
    updateDebugSessionConfig,
  }
})

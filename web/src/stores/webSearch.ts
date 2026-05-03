import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { WebSearchConfig, UpdateWebSearchConfigRequest } from '../types'
import * as api from '../api'

export const useWebSearchStore = defineStore('webSearch', () => {
  const config = ref<WebSearchConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchConfig() {
    loading.value = true
    error.value = null
    try {
      config.value = await api.getWebSearchConfig()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch Web Search config'
    } finally {
      loading.value = false
    }
  }

  async function updateConfig(data: UpdateWebSearchConfigRequest) {
    loading.value = true
    error.value = null
    try {
      config.value = await api.updateWebSearchConfig(data)
      return config.value
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update Web Search config'
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    config,
    loading,
    error,
    fetchConfig,
    updateConfig,
  }
})

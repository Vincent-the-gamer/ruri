import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { AcpConfig, UpdateAcpConfigRequest } from '../types'
import * as api from '../api'

export const useAcpStore = defineStore('acp', () => {
  const config = ref<AcpConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchConfig() {
    loading.value = true
    error.value = null
    try {
      config.value = await api.getAcpConfig()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch ACP config'
    } finally {
      loading.value = false
    }
  }

  async function updateConfig(data: UpdateAcpConfigRequest) {
    loading.value = true
    error.value = null
    try {
      config.value = await api.updateAcpConfig(data)
      return config.value
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update ACP config'
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

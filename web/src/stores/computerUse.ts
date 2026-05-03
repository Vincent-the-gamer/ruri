import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { ComputerUseConfig, UpdateComputerUseConfigRequest } from '../types'
import * as api from '../api'

export const useComputerUseStore = defineStore('computerUse', () => {
  const config = ref<ComputerUseConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchConfig() {
    loading.value = true
    error.value = null
    try {
      config.value = await api.getComputerUseConfig()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch Computer Use config'
    } finally {
      loading.value = false
    }
  }

  async function updateConfig(data: UpdateComputerUseConfigRequest) {
    loading.value = true
    error.value = null
    try {
      config.value = await api.updateComputerUseConfig(data)
      return config.value
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update Computer Use config'
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

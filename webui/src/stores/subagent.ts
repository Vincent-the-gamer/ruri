import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { SubAgentOrchestratorConfig } from '../types'
import * as api from '../api'

export const useSubAgentStore = defineStore('subagent', () => {
  const config = ref<SubAgentOrchestratorConfig>({
    main_enable: false,
    remove_main_duplicate_tools: false,
    router_system_prompt: '',
    agents: [],
  })
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchConfig() {
    loading.value = true
    error.value = null
    try {
      config.value = await api.getSubAgentConfig()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch sub-agent config'
    } finally {
      loading.value = false
    }
  }

  async function updateConfig(data: SubAgentOrchestratorConfig) {
    loading.value = true
    error.value = null
    try {
      await api.updateSubAgentConfig(data)
      config.value = data
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update sub-agent config'
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

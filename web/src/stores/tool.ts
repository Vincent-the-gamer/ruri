import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Tool } from '../types'
import * as api from '../api'

export const useToolStore = defineStore('tool', () => {
  const tools = ref<Tool[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchTools() {
    loading.value = true
    error.value = null
    try {
      tools.value = await api.getTools()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch tools'
    } finally {
      loading.value = false
    }
  }

  return {
    tools,
    loading,
    error,
    fetchTools,
  }
})

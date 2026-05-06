import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { AgentStatus } from '../types'
import * as api from '../api'

export const useAgentStore = defineStore('agent', () => {
  const status = ref<AgentStatus>({
    status: 'stopped',
    active_provider: null,
    active_model: null,
    skills_count: 0,
    tools_count: 0,
    uptime_secs: 0,
    message_count: 0,
  })
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchStatus() {
    loading.value = true
    error.value = null
    try {
      status.value = await api.getAgentStatus()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch agent status'
    } finally {
      loading.value = false
    }
  }

  function formatUptime(secs: number): string {
    if (secs < 60) return `${secs}s`
    if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`
    const h = Math.floor(secs / 3600)
    const m = Math.floor((secs % 3600) / 60)
    return `${h}h ${m}m`
  }

  return {
    status,
    loading,
    error,
    fetchStatus,
    formatUptime,
  }
})

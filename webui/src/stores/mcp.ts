import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { McpServerConfig, CreateMcpServerRequest, UpdateMcpServerRequest } from '../types'
import * as api from '../api'

export const useMcpStore = defineStore('mcp', () => {
  const servers = ref<McpServerConfig[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchServers() {
    loading.value = true
    error.value = null
    try {
      servers.value = await api.getMcpServers()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch MCP servers'
    } finally {
      loading.value = false
    }
  }

  async function createServer(data: CreateMcpServerRequest) {
    loading.value = true
    error.value = null
    try {
      const newServer = await api.createMcpServer(data)
      servers.value.push(newServer)
      return newServer
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to create MCP server'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function updateServer(id: string, data: UpdateMcpServerRequest) {
    loading.value = true
    error.value = null
    try {
      const updated = await api.updateMcpServer(id, data)
      const index = servers.value.findIndex(s => s.id === id)
      if (index !== -1) {
        servers.value[index] = updated
      }
      return updated
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update MCP server'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteServer(id: string) {
    loading.value = true
    error.value = null
    try {
      await api.deleteMcpServer(id)
      servers.value = servers.value.filter(s => s.id !== id)
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to delete MCP server'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function toggleServer(id: string) {
    loading.value = true
    error.value = null
    try {
      const updated = await api.toggleMcpServer(id)
      const index = servers.value.findIndex(s => s.id === id)
      if (index !== -1) {
        servers.value[index] = updated
      }
      return updated
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to toggle MCP server'
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    servers,
    loading,
    error,
    fetchServers,
    createServer,
    updateServer,
    deleteServer,
    toggleServer,
  }
})

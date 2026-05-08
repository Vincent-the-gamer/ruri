import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { PlatformInstance, CreatePlatformRequest, UpdatePlatformRequest } from '../types'
import * as api from '../api'

export const usePlatformStore = defineStore('platform', () => {
  const instances = ref<PlatformInstance[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchInstances() {
    loading.value = true
    error.value = null
    try {
      instances.value = await api.getPlatforms()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch platform instances'
    } finally {
      loading.value = false
    }
  }

  async function createInstance(data: CreatePlatformRequest) {
    loading.value = true
    error.value = null
    try {
      const newInstance = await api.createPlatform(data)
      instances.value.push(newInstance)
      return newInstance
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to create platform instance'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function updateInstance(id: string, data: UpdatePlatformRequest) {
    loading.value = true
    error.value = null
    try {
      const updated = await api.updatePlatform(id, data)
      const index = instances.value.findIndex(s => s.id === id)
      if (index !== -1) {
        instances.value[index] = updated
      }
      return updated
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update platform instance'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteInstance(id: string) {
    loading.value = true
    error.value = null
    try {
      await api.deletePlatform(id)
      instances.value = instances.value.filter(s => s.id !== id)
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to delete platform instance'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function toggleInstance(id: string) {
    loading.value = true
    error.value = null
    try {
      const updated = await api.togglePlatform(id)
      const index = instances.value.findIndex(s => s.id === id)
      if (index !== -1) {
        instances.value[index] = updated
      }
      return updated
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to toggle platform instance'
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    instances,
    loading,
    error,
    fetchInstances,
    createInstance,
    updateInstance,
    deleteInstance,
    toggleInstance,
  }
})

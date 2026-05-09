import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Persona, CreatePersonaRequest, UpdatePersonaRequest } from '../types'
import * as api from '../api'
import { useConfigStore } from './config'

export const usePersonaStore = defineStore('persona', () => {
  const personas = ref<Persona[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Derive active persona from the active config profile's persona_id
  const activePersona = computed(() => {
    const configStore = useConfigStore()
    const personaId = configStore.activePersonaId
    if (!personaId) return null
    return personas.value.find(p => p.id === personaId) || null
  })

  async function fetchPersonas() {
    loading.value = true
    error.value = null
    try {
      personas.value = await api.getPersonas()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch personas'
    } finally {
      loading.value = false
    }
  }

  async function createPersona(data: CreatePersonaRequest) {
    loading.value = true
    error.value = null
    try {
      const newPersona = await api.createPersona(data)
      personas.value.push(newPersona)
      return newPersona
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to create persona'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function updatePersona(id: string, data: UpdatePersonaRequest) {
    loading.value = true
    error.value = null
    try {
      const updated = await api.updatePersona(id, data)
      const index = personas.value.findIndex(p => p.id === id)
      if (index !== -1) {
        personas.value[index] = updated
      }
      return updated
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update persona'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deletePersona(id: string) {
    loading.value = true
    error.value = null
    try {
      await api.deletePersona(id)
      personas.value = personas.value.filter(p => p.id !== id)
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to delete persona'
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    personas,
    activePersona,
    loading,
    error,
    fetchPersonas,
    createPersona,
    updatePersona,
    deletePersona,
  }
})
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Persona, CreatePersonaRequest, UpdatePersonaRequest } from '../types'
import * as api from '../api'

export const usePersonaStore = defineStore('persona', () => {
  const personas = ref<Persona[]>([])
  const activePersona = ref<Persona | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchPersonas() {
    loading.value = true
    error.value = null
    try {
      personas.value = await api.getPersonas()
      activePersona.value = personas.value.find(p => p.is_active) || null
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
      if (newPersona.is_active) {
        activePersona.value = newPersona
      }
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
      if (updated.is_active) {
        activePersona.value = updated
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
      if (activePersona.value?.id === id) {
        activePersona.value = personas.value.find(p => p.is_active) || null
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to delete persona'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function activatePersona(id: string) {
    loading.value = true
    error.value = null
    try {
      const updated = await api.activatePersona(id)
      personas.value = personas.value.map(p => ({
        ...p,
        is_active: p.id === id,
      }))
      activePersona.value = updated
      return updated
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to activate persona'
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
    activatePersona,
  }
})
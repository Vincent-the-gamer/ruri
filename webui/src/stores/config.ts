import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { ConfigProfile, CreateConfigProfileRequest, UpdateConfigProfileRequest } from '../types'
import * as api from '../api'

export const useConfigStore = defineStore('config', () => {
  const configProfiles = ref<ConfigProfile[]>([])
  const activeConfigProfile = ref<ConfigProfile | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Computed properties for easier access
  const activeProfileId = computed(() => activeConfigProfile.value?.id || null)

  async function fetchConfigProfiles() {
    loading.value = true
    error.value = null
    try {
      configProfiles.value = await api.getConfigProfiles()
      activeConfigProfile.value = configProfiles.value.find(p => p.is_active) || null
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch config profiles'
      // Silently fail if endpoint doesn't exist yet (backend not implemented)
      console.warn('Config profiles not available:', error.value)
    } finally {
      loading.value = false
    }
  }

  async function createConfigProfile(data: CreateConfigProfileRequest) {
    loading.value = true
    error.value = null
    try {
      const newProfile = await api.createConfigProfile(data)
      configProfiles.value.push(newProfile)
      if (newProfile.is_active) {
        activeConfigProfile.value = newProfile
      }
      return newProfile
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to create config profile'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function updateConfigProfile(id: string, data: UpdateConfigProfileRequest) {
    loading.value = true
    error.value = null
    try {
      const updated = await api.updateConfigProfile(id, data)
      const index = configProfiles.value.findIndex(p => p.id === id)
      if (index !== -1) {
        configProfiles.value[index] = updated
      }
      if (updated.is_active) {
        activeConfigProfile.value = updated
      } else if (activeConfigProfile.value?.id === id) {
        // The previously active profile was deactivated, find another active one
        activeConfigProfile.value = configProfiles.value.find(p => p.is_active) || null
      }
      return updated
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update config profile'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteConfigProfile(id: string) {
    loading.value = true
    error.value = null
    try {
      await api.deleteConfigProfile(id)
      configProfiles.value = configProfiles.value.filter(p => p.id !== id)
      if (activeConfigProfile.value?.id === id) {
        activeConfigProfile.value = configProfiles.value.find(p => p.is_active) || null
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to delete config profile'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function activateConfigProfile(id: string) {
    loading.value = true
    error.value = null
    try {
      const updated = await api.activateConfigProfile(id)
      configProfiles.value = configProfiles.value.map(p => ({
        ...p,
        is_active: p.id === id,
      }))
      activeConfigProfile.value = updated
      return updated
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to activate config profile'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function getConfigProfileProvider(profileId: string) {
    try {
      return await api.getConfigProfileProvider(profileId)
    } catch (e: unknown) {
      console.warn('Failed to get config profile provider:', e)
      return null
    }
  }

  async function getConfigProfilePersona(profileId: string) {
    try {
      return await api.getConfigProfilePersona(profileId)
    } catch (e: unknown) {
      console.warn('Failed to get config profile persona:', e)
      return null
    }
  }

  // Get active configuration values
  const activeProviderId = computed(() => activeConfigProfile.value?.provider_id || null)
  const activePersonaId = computed(() => activeConfigProfile.value?.persona_id || null)
  const webSearchEnabled = computed(() => activeConfigProfile.value?.web_search_enabled ?? false)
  const computerUseEnabled = computed(() => activeConfigProfile.value?.computer_use_enabled ?? false)
  const acpEnabled = computed(() => activeConfigProfile.value?.acp_enabled ?? false)
  const activeSkillNames = computed(() => activeConfigProfile.value?.active_skill_names ?? [])
  const activePlatformIds = computed(() => activeConfigProfile.value?.active_platform_ids ?? [])
  const commandPrefix = computed(() => activeConfigProfile.value?.command_prefix ?? '/')

  return {
    configProfiles,
    activeConfigProfile,
    activeProfileId,
    activeProviderId,
    activePersonaId,
    webSearchEnabled,
    computerUseEnabled,
    acpEnabled,
    activeSkillNames,
    activePlatformIds,
    commandPrefix,
    loading,
    error,
    fetchConfigProfiles,
    createConfigProfile,
    updateConfigProfile,
    deleteConfigProfile,
    activateConfigProfile,
    getConfigProfileProvider,
    getConfigProfilePersona,
  }
})

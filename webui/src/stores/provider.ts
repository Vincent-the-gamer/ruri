import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Provider, CreateProviderRequest, ProviderType, ProviderConfig } from '../types'
import * as api from '../api'

export const useProviderStore = defineStore('provider', () => {
  const providers = ref<Provider[]>([])
  const activeProvider = ref<Provider | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchProviders() {
    loading.value = true
    error.value = null
    try {
      providers.value = await api.getProviders()
      activeProvider.value = providers.value.find(p => p.is_active) || null
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch providers'
    } finally {
      loading.value = false
    }
  }

  async function createProvider(data: CreateProviderRequest) {
    loading.value = true
    error.value = null
    try {
      const provider = await api.createProvider(data)
      providers.value.push(provider)
      if (provider.is_active) {
        activeProvider.value = provider
      }
      return provider
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to create provider'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function updateProvider(id: string, data: Partial<CreateProviderRequest>) {
    loading.value = true
    error.value = null
    try {
      const provider = await api.updateProvider(id, data)
      const idx = providers.value.findIndex(p => p.id === id)
      if (idx !== -1) {
        providers.value[idx] = provider
      }
      if (provider.is_active) {
        activeProvider.value = provider
      }
      return provider
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update provider'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteProvider(id: string) {
    loading.value = true
    error.value = null
    try {
      await api.deleteProvider(id)
      providers.value = providers.value.filter(p => p.id !== id)
      if (activeProvider.value?.id === id) {
        activeProvider.value = providers.value.find(p => p.is_active) || null
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to delete provider'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function activateProvider(id: string) {
    loading.value = true
    error.value = null
    try {
      const provider = await api.activateProvider(id)
      providers.value = providers.value.map(p => ({
        ...p,
        is_active: p.id === id,
      }))
      activeProvider.value = provider
      return provider
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to activate provider'
      throw e
    } finally {
      loading.value = false
    }
  }

  function getConfigByType(type: ProviderType): ProviderConfig {
    switch (type) {
      case 'openai':
        return {
          type: 'openai',
          base_url: 'https://api.openai.com/v1',
          api_key: '',
          default_model: 'gpt-4o',
          supports_multimodal: true,
        }
      case 'siliconflow':
        return {
          type: 'siliconflow',
          base_url: 'https://api.siliconflow.cn/v1',
          api_key: '',
          default_model: 'deepseek-ai/DeepSeek-V3',
          supports_multimodal: true,
        }
      case 'deepseek':
        return {
          type: 'deepseek',
          base_url: 'https://api.deepseek.com',
          api_key: '',
          default_model: 'deepseek-chat',
          supports_multimodal: false,
        }
      case 'anthropic':
        return {
          type: 'anthropic',
          base_url: 'https://api.anthropic.com',
          api_key: '',
          default_model: 'claude-sonnet-4-20250514',
          api_version: '2023-06-01',
          supports_multimodal: true,
        }
      case 'gemini':
        return {
          type: 'gemini',
          base_url: 'https://generativelanguage.googleapis.com/v1beta',
          api_key: '',
          default_model: 'gemini-2.0-flash',
          supports_multimodal: true,
        }
    }
  }

  return {
    providers,
    activeProvider,
    loading,
    error,
    fetchProviders,
    createProvider,
    updateProvider,
    deleteProvider,
    activateProvider,
    getConfigByType,
  }
})

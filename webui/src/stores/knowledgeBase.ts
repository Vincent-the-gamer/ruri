import { defineStore } from 'pinia'
import { ref } from 'vue'
import type {
  KnowledgeBase,
  CreateKnowledgeBaseRequest,
  UpdateKnowledgeBaseRequest,
  KbDocument,
  SearchResult,
  SearchRequest,
} from '../types'
import * as api from '../api'

export const useKnowledgeBaseStore = defineStore('knowledgeBase', () => {
  const knowledgeBases = ref<KnowledgeBase[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchKnowledgeBases() {
    loading.value = true
    error.value = null
    try {
      knowledgeBases.value = await api.getKnowledgeBases()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch knowledge bases'
    } finally {
      loading.value = false
    }
  }

  async function createKnowledgeBase(data: CreateKnowledgeBaseRequest) {
    loading.value = true
    error.value = null
    try {
      const kb = await api.createKnowledgeBase(data)
      knowledgeBases.value.push(kb)
      return kb
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to create knowledge base'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function updateKnowledgeBase(id: string, data: UpdateKnowledgeBaseRequest) {
    loading.value = true
    error.value = null
    try {
      const kb = await api.updateKnowledgeBase(id, data)
      const idx = knowledgeBases.value.findIndex(k => k.id === id)
      if (idx !== -1) {
        knowledgeBases.value[idx] = kb
      }
      return kb
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update knowledge base'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteKnowledgeBase(id: string) {
    loading.value = true
    error.value = null
    try {
      await api.deleteKnowledgeBase(id)
      knowledgeBases.value = knowledgeBases.value.filter(k => k.id !== id)
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to delete knowledge base'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchDocuments(kbId: string): Promise<KbDocument[]> {
    try {
      return await api.getKbDocuments(kbId)
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch documents'
      return []
    }
  }

  async function uploadDocument(kbId: string, files: File[]): Promise<KbDocument[]> {
    loading.value = true
    error.value = null
    try {
      const docs = await api.uploadKbDocument(kbId, files)
      return docs
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to upload document'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteDocument(kbId: string, docId: string) {
    try {
      await api.deleteKbDocument(kbId, docId)
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to delete document'
      throw e
    }
  }

  async function search(kbId: string, data: SearchRequest): Promise<SearchResult[]> {
    try {
      return await api.searchKnowledgeBase(kbId, data)
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to search knowledge base'
      return []
    }
  }

  return {
    knowledgeBases,
    loading,
    error,
    fetchKnowledgeBases,
    createKnowledgeBase,
    updateKnowledgeBase,
    deleteKnowledgeBase,
    fetchDocuments,
    uploadDocument,
    deleteDocument,
    search,
  }
})

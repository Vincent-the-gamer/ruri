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
    console.log('[KB Store] Fetching knowledge bases...')
    try {
      const result = await api.getKnowledgeBases()
      console.log('[KB Store] Fetched knowledge bases:', result)
      knowledgeBases.value = result
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to fetch knowledge bases'
      error.value = errorMsg
      console.error('[KB Store] Failed to fetch knowledge bases:', errorMsg, e)
    } finally {
      loading.value = false
    }
  }

  async function createKnowledgeBase(data: CreateKnowledgeBaseRequest) {
    loading.value = true
    error.value = null
    console.log('[KB Store] Creating knowledge base:', data)
    try {
      const kb = await api.createKnowledgeBase(data)
      console.log('[KB Store] Created knowledge base:', kb)
      knowledgeBases.value.push(kb)
      return kb
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to create knowledge base'
      error.value = errorMsg
      console.error('[KB Store] Failed to create knowledge base:', errorMsg)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function updateKnowledgeBase(id: string, data: UpdateKnowledgeBaseRequest) {
    loading.value = true
    error.value = null
    console.log('[KB Store] Updating knowledge base:', id, data)
    try {
      const kb = await api.updateKnowledgeBase(id, data)
      console.log('[KB Store] Updated knowledge base:', kb)
      const idx = knowledgeBases.value.findIndex(k => k.id === id)
      if (idx !== -1) {
        knowledgeBases.value[idx] = kb
      }
      return kb
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to update knowledge base'
      error.value = errorMsg
      console.error('[KB Store] Failed to update knowledge base:', errorMsg)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteKnowledgeBase(id: string) {
    loading.value = true
    error.value = null
    console.log('[KB Store] Deleting knowledge base:', id)
    try {
      await api.deleteKnowledgeBase(id)
      console.log('[KB Store] Deleted knowledge base:', id)
      knowledgeBases.value = knowledgeBases.value.filter(k => k.id !== id)
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to delete knowledge base'
      error.value = errorMsg
      console.error('[KB Store] Failed to delete knowledge base:', errorMsg)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchDocuments(kbId: string): Promise<KbDocument[]> {
    console.log('[KB Store] Fetching documents for KB:', kbId)
    try {
      const docs = await api.getKbDocuments(kbId)
      console.log('[KB Store] Fetched documents:', docs)
      return docs
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to fetch documents'
      error.value = errorMsg
      console.error('[KB Store] Failed to fetch documents:', errorMsg)
      return []
    }
  }

  async function uploadDocument(kbId: string, files: File[]): Promise<KbDocument[]> {
    loading.value = true
    error.value = null
    console.log('[KB Store] Uploading documents to KB:', kbId, files)
    try {
      const docs = await api.uploadKbDocument(kbId, files)
      console.log('[KB Store] Uploaded documents:', docs)
      return docs
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to upload document'
      error.value = errorMsg
      console.error('[KB Store] Failed to upload document:', errorMsg)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteDocument(kbId: string, docId: string) {
    console.log('[KB Store] Deleting document:', kbId, docId)
    try {
      await api.deleteKbDocument(kbId, docId)
      console.log('[KB Store] Deleted document:', kbId, docId)
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to delete document'
      error.value = errorMsg
      console.error('[KB Store] Failed to delete document:', errorMsg)
      throw e
    }
  }

  async function search(kbId: string, data: SearchRequest): Promise<SearchResult[]> {
    console.log('[KB Store] Searching KB:', kbId, data)
    try {
      const results = await api.searchKnowledgeBase(kbId, data)
      console.log('[KB Store] Search results:', results)
      return results
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to search knowledge base'
      error.value = errorMsg
      console.error('[KB Store] Failed to search knowledge base:', errorMsg)
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

import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Skill, CreateSkillRequest } from '../types'
import * as api from '../api'

export const useSkillStore = defineStore('skill', () => {
  const skills = ref<Skill[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchSkills() {
    loading.value = true
    error.value = null
    try {
      skills.value = await api.getSkills()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch skills'
    } finally {
      loading.value = false
    }
  }

  async function addSkill(data: CreateSkillRequest) {
    loading.value = true
    error.value = null
    try {
      const skill = await api.addSkill(data)
      skills.value.push(skill)
      return skill
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to add skill'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function removeSkill(name: string) {
    loading.value = true
    error.value = null
    try {
      await api.removeSkill(name)
      skills.value = skills.value.filter(s => s.name !== name)
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to remove skill'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function toggleSkill(name: string, isActive: boolean) {
    loading.value = true
    error.value = null
    try {
      const skill = await api.toggleSkill(name, isActive)
      const idx = skills.value.findIndex(s => s.name === name)
      if (idx !== -1) {
        skills.value[idx] = skill
      }
      return skill
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to toggle skill'
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    skills,
    loading,
    error,
    fetchSkills,
    addSkill,
    removeSkill,
    toggleSkill,
  }
})

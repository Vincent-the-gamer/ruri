import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { UserInfo, LoginRequest, ChangePasswordRequest } from '../types'

export interface UpdateUsernameRequest {
  new_username: string
}
import * as api from '../api'

export const useAuthStore = defineStore('auth', () => {
  // State
  const token = ref<string | null>(localStorage.getItem('auth_token'))
  const user = ref<UserInfo | null>(JSON.parse(localStorage.getItem('auth_user') || 'null'))
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Computed
  const isLoggedIn = computed(() => !!token.value)
  const mustChangePassword = computed(() => user.value?.must_change_password ?? false)
  const username = computed(() => user.value?.username ?? '')

  // Actions
  async function login(credentials: LoginRequest) {
    loading.value = true
    error.value = null

    try {
      const res = await api.login(credentials)
      token.value = res.token
      user.value = res.user

      // Persist to localStorage
      localStorage.setItem('auth_token', res.token)
      localStorage.setItem('auth_user', JSON.stringify(res.user))

      return res
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Login failed'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function logout() {
    loading.value = true
    error.value = null

    try {
      await api.logout()
    } catch (e: unknown) {
      // Silently fail logout on server side, but clear local state
      console.warn('Logout API call failed:', e)
    } finally {
      // Always clear local state
      token.value = null
      user.value = null
      localStorage.removeItem('auth_token')
      localStorage.removeItem('auth_user')
      loading.value = false
    }
  }

  async function getCurrentUser() {
    loading.value = true
    error.value = null

    try {
      const userData = await api.getCurrentUser()
      user.value = userData
      localStorage.setItem('auth_user', JSON.stringify(userData))
      return userData
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to get user info'
      // If not authenticated, clear local state
      if (e instanceof Error && e.message.includes('Not authenticated')) {
        token.value = null
        user.value = null
        localStorage.removeItem('auth_token')
        localStorage.removeItem('auth_user')
      }
      throw e
    } finally {
      loading.value = false
    }
  }

  async function changePassword(data: ChangePasswordRequest) {
    loading.value = true
    error.value = null

    try {
      await api.changePassword(data)
      // Update the must_change_password flag
      if (user.value) {
        user.value.must_change_password = false
        localStorage.setItem('auth_user', JSON.stringify(user.value))
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to change password'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function updateUsername(data: UpdateUsernameRequest) {
    loading.value = true
    error.value = null

    try {
      await api.updateUsername(data)
      // Update the username in local state
      if (user.value) {
        user.value.username = data.new_username
        localStorage.setItem('auth_user', JSON.stringify(user.value))
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : 'Failed to update username'
      throw e
    } finally {
      loading.value = false
    }
  }

  // Initialize auth state from localStorage
  function initAuth() {
    if (token.value) {
      // Restore from localStorage
      const storedUser = localStorage.getItem('auth_user')
      if (storedUser) {
        try {
          user.value = JSON.parse(storedUser)
        } catch {
          // Invalid stored user, clear everything
          token.value = null
          user.value = null
          localStorage.removeItem('auth_token')
          localStorage.removeItem('auth_user')
        }
      }
    }
  }

  // Call init on store creation
  initAuth()

  return {
    token,
    user,
    loading,
    error,
    isLoggedIn,
    mustChangePassword,
    username,
    login,
    logout,
    getCurrentUser,
    changePassword,
    updateUsername,
  }
})
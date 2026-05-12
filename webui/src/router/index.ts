import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/login',
      name: 'Login',
      component: () => import('../views/Login.vue'),
      meta: { requiresGuest: true },
    },
    {
      path: '/change-password',
      name: 'ChangePassword',
      component: () => import('../views/ChangePassword.vue'),
      meta: { requiresAuth: true, requiresPasswordChange: true },
    },
    {
      path: '/',
      name: 'Home',
      component: () => import('../views/Home.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/dashboard',
      name: 'Dashboard',
      component: () => import('../views/Dashboard.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/providers',
      name: 'Providers',
      component: () => import('../views/Providers.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/skills',
      name: 'Skills',
      component: () => import('../views/Skills.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/tools',
      name: 'Tools',
      component: () => import('../views/Tools.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/chat',
      name: 'Chat',
      component: () => import('../views/Chat.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/personas',
      name: 'Personas',
      component: () => import('../views/Personas.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/acp-config',
      name: 'ACP Config',
      component: () => import('../views/AcpConfig.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/computer-use-config',
      name: 'Computer Use Config',
      component: () => import('../views/ComputerUseConfig.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/web-search-config',
      name: 'Web Search Config',
      component: () => import('../views/WebSearchConfig.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/knowledge-base',
      name: 'KnowledgeBase',
      component: () => import('../views/KnowledgeBase.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/api-test',
      name: 'API Test',
      component: () => import('../views/APITest.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/logs',
      name: 'Logs',
      component: () => import('../views/Logs.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/mcp-config',
      name: 'MCP Config',
      component: () => import('../views/McpConfig.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/platform-config',
      name: 'Platform Config',
      component: () => import('../views/PlatformConfig.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/configs',
      name: 'Configs',
      component: () => import('../views/Configs.vue'),
      meta: { requiresAuth: true },
    },

    {
      path: '/conversation-history',
      name: 'ConversationHistory',
      component: () => import('../views/ConversationHistory.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/builtin-commands',
      name: 'BuiltinCommands',
      component: () => import('../views/BuiltinCommands.vue'),
      meta: { requiresAuth: true },
    },
  ],
})

// Navigation guard for authentication
router.beforeEach(async (to) => {
  const authStore = useAuthStore()

  // Check if route requires authentication
  const requiresAuth = to.matched.some((record) => record.meta.requiresAuth)
  const requiresGuest = to.matched.some((record) => record.meta.requiresGuest)

  // If route requires guest (not logged in) and user is logged in, redirect to home
  if (requiresGuest && authStore.isLoggedIn) {
    // If user must change password, redirect there
    if (authStore.mustChangePassword) {
      return { name: 'ChangePassword' }
    }
    return { name: 'Home' }
  }

  // If route requires auth and user is not logged in, redirect to login
  if (requiresAuth && !authStore.isLoggedIn) {
    return { name: 'Login' }
  }

  // If user is logged in and must change password, redirect to change password page
  // unless they're already on the change password page or login page
  if (
    authStore.isLoggedIn &&
    authStore.mustChangePassword &&
    to.name !== 'ChangePassword' &&
    to.name !== 'Login'
  ) {
    return { name: 'ChangePassword' }
  }
})

export default router
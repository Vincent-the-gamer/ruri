import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'Home',
      component: () => import('../views/Home.vue'),
    },
    {
      path: '/dashboard',
      name: 'Dashboard',
      component: () => import('../views/Dashboard.vue'),
    },
    {
      path: '/providers',
      name: 'Providers',
      component: () => import('../views/Providers.vue'),
    },
    {
      path: '/skills',
      name: 'Skills',
      component: () => import('../views/Skills.vue'),
    },
    {
      path: '/tools',
      name: 'Tools',
      component: () => import('../views/Tools.vue'),
    },
    {
      path: '/chat',
      name: 'Chat',
      component: () => import('../views/Chat.vue'),
      meta: { keepAlive: true },
    },
    {
      path: '/acp-config',
      name: 'ACP Config',
      component: () => import('../views/AcpConfig.vue'),
    },
    {
      path: '/computer-use-config',
      name: 'Computer Use Config',
      component: () => import('../views/ComputerUseConfig.vue'),
    },
    {
      path: '/web-search-config',
      name: 'Web Search Config',
      component: () => import('../views/WebSearchConfig.vue'),
    },
    {
      path: '/api-test',
      name: 'API Test',
      component: () => import('../views/APITest.vue'),
    },
    {
      path: '/logs',
      name: 'Logs',
      component: () => import('../views/Logs.vue'),
    },
  ],
})

export default router

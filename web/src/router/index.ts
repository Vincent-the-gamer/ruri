import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/dashboard',
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
    },
    {
      path: '/api-test',
      name: 'API Test',
      component: () => import('../views/APITest.vue'),
    },
  ],
})

export default router

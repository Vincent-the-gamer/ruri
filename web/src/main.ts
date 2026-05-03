import { createApp } from 'vue'
import { createPinia } from 'pinia'
import router from './router'
import App from './App.vue'
import i18n from './locales'

// UnoCSS
import '@unocss/reset/tailwind.css'
import 'uno.css'
import './style.css'

// Initialize dark mode before app mounts
import { initDarkMode } from './composables/useDarkMode'
initDarkMode()

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(i18n)
app.mount('#app')

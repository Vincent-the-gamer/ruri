import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import unocss from 'unocss/vite'

export default defineConfig({
  plugins: [
    vue(),
    unocss(),
  ],
  base: '/',
  build: {
    outDir: '../src/web_dist',
    emptyOutDir: true,
  },
  server: {
    port: 8080,
    proxy: {
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      },
    },
  },
})

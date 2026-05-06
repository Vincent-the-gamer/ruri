<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Icon } from '@iconify/vue'

const isDark = ref(false)

// Toggle dark mode
const toggleDark = () => {
  isDark.value = !isDark.value
  updateDarkMode()
}

// Update dark mode
const updateDarkMode = () => {
  if (isDark.value) {
    document.documentElement.classList.add('dark')
    localStorage.setItem('ruri-color-mode', 'dark')
  } else {
    document.documentElement.classList.remove('dark')
    localStorage.setItem('ruri-color-mode', 'light')
  }
}

// Initialize dark mode from localStorage
onMounted(() => {
  const savedMode = localStorage.getItem('ruri-color-mode')
  if (savedMode === 'dark') {
    isDark.value = true
  } else if (savedMode === 'light') {
    isDark.value = false
  } else {
    // Auto mode - use system preference
    isDark.value = window.matchMedia('(prefers-color-scheme: dark)').matches
  }
})
</script>

<template>
  <button
    class="h-9 w-9 flex items-center justify-center rounded-lg bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors duration-200"
    @click="toggleDark"
    :aria-label="isDark ? 'Switch to light mode' : 'Switch to dark mode'"
  >
    <Icon
      :icon="isDark ? 'lucide:sun' : 'lucide:moon'"
      class="text-lg"
    />
  </button>
</template>

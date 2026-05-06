import { usePreferredDark, useStorage } from '@vueuse/core'
import { computed, watch } from 'vue'

export type ColorMode = 'light' | 'dark' | 'auto'

// 使用 VueUse 的 useStorage 来持久化主题设置
const colorMode = useStorage<ColorMode>('ruri-color-mode', 'auto')

// 获取系统偏好
const prefersDark = usePreferredDark()

// 计算当前实际使用的主题
const isDark = computed(() => {
  if (colorMode.value === 'auto') {
    return prefersDark.value
  }
  return colorMode.value === 'dark'
})

// 更新 DOM
const updateDOM = (dark: boolean) => {
  const html = document.documentElement

  if (dark) {
    html.classList.add('dark')
  } else {
    html.classList.remove('dark')
  }
}

// 监听主题变化
watch(isDark, (dark) => {
  updateDOM(dark)
}, { immediate: true })

// 切换主题
export function toggleDarkMode() {
  if (colorMode.value === 'auto') {
    // 如果是 auto，切换到当前系统偏好的相反模式
    colorMode.value = prefersDark.value ? 'light' : 'dark'
  } else {
    // 如果是 light 或 dark，切换到相反模式
    colorMode.value = colorMode.value === 'light' ? 'dark' : 'light'
  }
}

// 设置特定主题
export function setColorMode(mode: ColorMode) {
  colorMode.value = mode
}

// composable hook
export function useDarkMode() {
  return {
    colorMode,
    isDark,
    toggleDarkMode,
    setColorMode,
  }
}

// 初始化函数（在 main.ts 中调用）
export function initDarkMode() {
  updateDOM(isDark.value)
}

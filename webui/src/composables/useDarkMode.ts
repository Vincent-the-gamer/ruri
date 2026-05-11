import { useStorage } from '@vueuse/core'
import { computed, ref, watch } from 'vue'

export type ColorMode = 'light' | 'dark' | 'auto'

// 使用 VueUse 的 useStorage 来持久化主题设置
const colorMode = useStorage<ColorMode>('ruri-color-mode', 'auto')

// 基于时间的暗色模式判断
const currentHour = ref(new Date().getHours())

// Update currentHour every minute
let timeInterval: ReturnType<typeof setInterval> | null = null

export function startTimeCheck() {
  timeInterval = setInterval(() => {
    currentHour.value = new Date().getHours()
  }, 60_000) // check every minute
}

export function stopTimeCheck() {
  if (timeInterval) {
    clearInterval(timeInterval)
    timeInterval = null
  }
}

// 计算当前实际使用的主题
const isDark = computed(() => {
  if (colorMode.value === 'auto') {
    // Time-based: dark mode from 18:00 to 6:00
    return currentHour.value >= 18 || currentHour.value < 6
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

// 切换主题：light → dark → auto → light
export function toggleDarkMode() {
  if (colorMode.value === 'light') {
    colorMode.value = 'dark'
  } else if (colorMode.value === 'dark') {
    colorMode.value = 'auto'
  } else {
    colorMode.value = 'light'
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
  startTimeCheck()
  updateDOM(isDark.value)
}

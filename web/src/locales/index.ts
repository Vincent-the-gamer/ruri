import { createI18n } from 'vue-i18n'
import zhCN from './zh-CN'
import enUS from './en-US'

export type Locale = 'zh-CN' | 'en-US'

// 从 localStorage 获取保存的语言设置，如果没有则使用默认语言
function getDefaultLocale(): Locale {
  const savedLocale = localStorage.getItem('ruri-locale')
  if (savedLocale && ['zh-CN', 'en-US'].includes(savedLocale)) {
    return savedLocale as Locale
  }
  return 'zh-CN' // 默认中文
}

const messages = {
  'zh-CN': zhCN,
  'en-US': enUS,
}

const i18n = createI18n({
  legacy: false, // 使用 Composition API 模式
  locale: getDefaultLocale(),
  fallbackLocale: 'zh-CN',
  messages,
})

export default i18n

// 导出工具函数
export function setLocale(locale: Locale): void {
  localStorage.setItem('ruri-locale', locale)
  i18n.global.locale.value = locale
}

export function getLocale(): Locale {
  return i18n.global.locale.value as Locale
}

export function getAvailableLocales(): Array<{ code: Locale; name: string }> {
  return [
    { code: 'zh-CN', name: '中文' },
    { code: 'en-US', name: 'English' },
  ]
}

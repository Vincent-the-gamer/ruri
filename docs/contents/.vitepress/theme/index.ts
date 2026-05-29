import type { EnhanceAppContext } from 'vitepress'
import Teek, { teekConfigContext } from 'vitepress-theme-teek'
import { h, provide } from 'vue'
import TeekLayoutProvider from '../components/TeekLayoutProvider.vue'

import 'vitepress-theme-teek/index.css'
import 'virtual:uno.css'
import 'vitepress-theme-teek/theme-chalk/tk-code-block-mobile.css'
import 'vitepress-theme-teek/theme-chalk/tk-sidebar.css'
import 'vitepress-theme-teek/theme-chalk/tk-aside.css'
import 'vitepress-theme-teek/theme-chalk/tk-nav.css'
import 'vitepress-theme-teek/theme-chalk/tk-doc-h1-gradient.css'
import 'vitepress-theme-teek/theme-chalk/tk-doc-fade-in.css'
import 'vitepress-theme-teek/theme-chalk/tk-banner-desc-gradient.css'
import 'vitepress-theme-teek/theme-chalk/tk-banner-full-img-scale.css'
import 'vitepress-theme-teek/theme-chalk/tk-fade-up-animation.css'
import 'vitepress-theme-teek/theme-chalk/tk-home-card-hover.css'
import './hero-gradient.css'
import './code-fix.css'

provide(teekConfigContext, {})

export default {
  extends: Teek,
  Layout: () =>
    h('div', null, [
      // 使用国际化布局组件
      h(TeekLayoutProvider),
    ]),
  enhanceApp(ctx: EnhanceAppContext) {
    const { app } = ctx
    // TwoslashFloatingVue removed: not compatible with VitePress 1.x + @shikijs/vitepress-twoslash v3
    // Re-add when upgrading to a compatible version
  },
}

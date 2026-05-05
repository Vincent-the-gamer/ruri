<script setup lang="ts">
import Teek, { en, zhCn } from 'vitepress-theme-teek'
import { useData } from 'vitepress'
import { computed } from 'vue'

const { lang } = useData()

const locale = computed(() => {
  if (lang.value === 'zh_hans' || lang.value === 'zh-CN') {
    return zhCn
  }
  return en
})
</script>

<template>
  <Teek.Layout :locale />
</template>
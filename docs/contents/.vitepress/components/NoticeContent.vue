<!-- .vitepress/theme/components/NoticeContent.vue -->
<script setup lang="ts" name="NoticeContent">
import { useData } from "vitepress";
import { computed } from "vue";

const { lang } = useData();
const namespace = "notice";

const noticeText = computed(() => {
  if (lang.value === "zh_hans" || lang.value === "zh-CN") {
    return "VitePress Starter 现在使用";
  }
  return "VitePress Starter now using";
});
</script>

<template>
  <div :class="namespace">
    <p>
      {{ noticeText }}
      <a
        color-red
        underline
        href="https://github.com/Kele-Bingtang/vitepress-theme-teek"
        target="_blank"
        >Teek Theme</a
      >
    </p>
  </div>
</template>

<style scoped>
.notice {
  img {
    width: 100%;
    height: 120px;
    object-fit: contain;
  }
  p {
    text-align: center;
    font-size: 14px;
    padding: 10px 0;
  }
}
</style>

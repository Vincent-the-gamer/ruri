<script setup lang="ts">
import Sidebar from "./components/Sidebar.vue";
</script>

<template>
    <div class="app-container">
        <Sidebar />
        <main class="main-content">
            <router-view v-slot="{ Component }">
                <transition name="page" mode="out-in">
                    <component :is="Component" />
                </transition>
            </router-view>
        </main>
    </div>
</template>

<style scoped>
.app-container {
    display: flex;
    min-height: 100vh;
    min-height: 100dvh;
    background: var(--color-bg);
    position: relative;
}

.main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
}

/* 页面过渡动画 */
.page-enter-active,
.page-leave-active {
    transition: all var(--transition-normal);
}

.page-enter-from {
    opacity: 0;
    transform: translateY(8px);
}

.page-leave-to {
    opacity: 0;
    transform: translateY(-4px);
}
</style>

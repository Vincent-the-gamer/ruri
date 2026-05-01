<script setup lang="ts">
import Sidebar from "./components/Sidebar.vue";
</script>

<template>
    <div class="app-layout">
        <Sidebar />
        <main class="app-main">
            <router-view v-slot="{ Component }">
                <transition name="page" mode="out-in">
                    <component :is="Component" />
                </transition>
            </router-view>
        </main>
    </div>
</template>

<style scoped>
.app-layout {
    display: flex;
    min-height: 100vh;
    min-height: 100dvh;
    background: var(--color-bg, #0f0f11);
}

.app-main {
    flex: 1;
    overflow: auto;
    border-left: 1px solid rgba(255, 255, 255, 0.06);
}

.page-enter-active,
.page-leave-active {
    transition:
        opacity 0.2s ease,
        transform 0.2s ease;
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

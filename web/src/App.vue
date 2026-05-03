<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRoute } from "vue-router";
import Sidebar from "./components/Sidebar.vue";
import ThemeToggle from "./components/ThemeToggle.vue";
import LocaleSwitcher from "./components/LocaleSwitcher.vue";

const route = useRoute();
const isHome = ref(false);

// Hide loading screen when app is ready
onMounted(() => {
    const loadingScreen = document.getElementById("loading-screen");
    if (loadingScreen) {
        setTimeout(() => {
            loadingScreen.classList.add("hidden");
            setTimeout(() => {
                loadingScreen.remove();
            }, 300);
        }, 1200);
    }

    // Check if current route is home
    isHome.value = route.path === "/";
});
</script>

<template>
    <div class="min-h-screen flex flex-col bg-background font-sans-rounded">
        <!-- Header - Inspired by airi's elegant navigation -->
        <header
            class="sticky top-0 z-20 h-[68px] w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 transition-all duration-300"
        >
            <div
                class="max-w-[1440px] mx-auto h-full flex items-center justify-between px-6"
            >
                <!-- Logo -->
                <router-link
                    to="/"
                    class="flex items-center gap-2 text-xl font-bold text-foreground hover:opacity-80 transition-opacity"
                >
                    <span class="text-primary">◈</span>
                    <span>Ruri</span>
                </router-link>

                <!-- Right side: Locale & Theme Toggle -->
                <div class="flex items-center gap-3">
                    <LocaleSwitcher />
                    <ThemeToggle />
                </div>
            </div>
        </header>

        <!-- Main Content Area -->
        <div class="flex flex-1">
            <!-- Sidebar - Left Navigation -->
            <Sidebar />

            <!-- Main Content -->
            <main class="flex-1 overflow-auto">
                <div class="max-w-[1440px] mx-auto w-full p-6">
                    <router-view v-slot="{ Component }">
                        <transition name="fade" mode="out-in">
                            <component :is="Component" />
                        </transition>
                    </router-view>
                </div>
            </main>
        </div>
    </div>
</template>

<style scoped>
/* Page transition */
.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.2s ease-in-out;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}
</style>

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
    <div
        class="app-container min-h-screen flex flex-col bg-background font-sans-rounded"
    >
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
/* App Container with gradient background */
.app-container {
    position: relative;
    min-height: 100vh;
}

.app-container::before {
    content: "";
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background:
        radial-gradient(
            ellipse 80% 50% at 20% -10%,
            hsl(var(--primary) / 0.08) 0%,
            transparent 50%
        ),
        radial-gradient(
            ellipse 60% 40% at 80% 100%,
            hsl(280 70% 60% / 0.06) 0%,
            transparent 50%
        ),
        radial-gradient(
            ellipse 40% 30% at 50% 50%,
            hsl(var(--primary) / 0.03) 0%,
            transparent 70%
        );
    pointer-events: none;
    z-index: 0;
}

/* Dark mode gradient */
:global(.dark) .app-container::before {
    background:
        radial-gradient(
            ellipse 80% 50% at 20% -10%,
            hsl(var(--primary) / 0.12) 0%,
            transparent 50%
        ),
        radial-gradient(
            ellipse 60% 40% at 80% 100%,
            hsl(280 70% 60% / 0.08) 0%,
            transparent 50%
        ),
        radial-gradient(
            ellipse 40% 30% at 50% 50%,
            hsl(var(--primary) / 0.05) 0%,
            transparent 70%
        );
}

.app-container > * {
    position: relative;
    z-index: 1;
}

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

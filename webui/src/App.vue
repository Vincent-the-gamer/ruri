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
        class="app-container h-screen flex flex-col bg-background font-sans-rounded"
    >
        <!-- Floating orbs background animation -->
        <div class="bg-orbs">
            <div class="orb orb-1"></div>
            <div class="orb orb-2"></div>
            <div class="orb orb-3"></div>
        </div>

        <!-- Main Content Area -->
        <div class="flex flex-1 overflow-hidden flex-col">
            <!-- Header -->
            <header
                class="sticky top-0 h-[68px] w-full border-b border-border/30 bg-background/30 backdrop-blur-xl supports-[backdrop-filter]:bg-background/20 transition-all duration-300"
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

            <!-- Sidebar and Main Content -->
            <div class="flex flex-1 overflow-hidden">
                <!-- Sidebar - Left Navigation -->
                <Sidebar />

                <!-- Main Content -->
                <main class="flex-1 overflow-y-auto">
                    <div class="max-w-[1440px] mx-auto w-full p-6">
                        <router-view v-slot="{ Component, route }">
                            <keep-alive v-if="route.meta.keepAlive">
                                <transition name="fade" mode="out-in">
                                    <component :is="Component" />
                                </transition>
                            </keep-alive>
                            <template v-else>
                                <transition name="fade" mode="out-in">
                                    <component :is="Component" />
                                </transition>
                            </template>
                        </router-view>
                    </div>
                </main>
            </div>
        </div>
    </div>
</template>

<style scoped>
/* App Container with gradient background */
.app-container {
    position: relative;
    min-height: 100vh;
}

/* Full-page gradient background - lowest layer */
.app-container::before {
    content: "";
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    width: 100%;
    height: 100%;
    background:
        linear-gradient(
            135deg,
            hsl(var(--background)) 0%,
            hsl(var(--primary) / 0.12) 20%,
            hsl(280 70% 60% / 0.1) 40%,
            hsl(var(--primary) / 0.08) 60%,
            hsl(200 70% 60% / 0.08) 80%,
            hsl(var(--background)) 100%
        ),
        radial-gradient(
            ellipse 60% 50% at 10% 0%,
            hsl(var(--primary) / 0.15) 0%,
            transparent 60%
        ),
        radial-gradient(
            ellipse 50% 40% at 90% 100%,
            hsl(280 70% 60% / 0.12) 0%,
            transparent 60%
        ),
        radial-gradient(
            ellipse 40% 30% at 50% 50%,
            hsl(var(--primary) / 0.06) 0%,
            transparent 70%
        );
    pointer-events: none;
    z-index: 0;
}

/* Dark mode gradient - more vibrant */
:global(.dark) .app-container::before {
    background:
        linear-gradient(
            135deg,
            hsl(var(--background)) 0%,
            hsl(var(--primary) / 0.2) 20%,
            hsl(280 70% 60% / 0.18) 40%,
            hsl(var(--primary) / 0.15) 60%,
            hsl(200 70% 60% / 0.12) 80%,
            hsl(var(--background)) 100%
        ),
        radial-gradient(
            ellipse 60% 50% at 10% 0%,
            hsl(var(--primary) / 0.25) 0%,
            transparent 60%
        ),
        radial-gradient(
            ellipse 50% 40% at 90% 100%,
            hsl(280 70% 60% / 0.2) 0%,
            transparent 60%
        ),
        radial-gradient(
            ellipse 40% 30% at 50% 50%,
            hsl(var(--primary) / 0.1) 0%,
            transparent 70%
        );
}

.app-container > .flex:first-child {
    position: relative;
    z-index: 1;
}

/* Floating orbs background animation */
.bg-orbs {
    position: fixed;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 0;
    overflow: hidden;
}

.orb {
    position: absolute;
    border-radius: 50%;
    filter: blur(80px);
    opacity: 0.3;
    animation: orb-float 25s ease-in-out infinite;
}

.orb-1 {
    width: 500px;
    height: 500px;
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(280 70% 60%));
    top: -100px;
    right: -100px;
    animation-delay: 0s;
}

.orb-2 {
    width: 400px;
    height: 400px;
    background: linear-gradient(135deg, hsl(320 70% 60%), hsl(var(--primary)));
    bottom: -80px;
    left: -80px;
    animation-delay: -8s;
}

.orb-3 {
    width: 350px;
    height: 350px;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.7),
        hsl(200 70% 70%)
    );
    top: 40%;
    left: 20%;
    animation-delay: -16s;
}

@keyframes orb-float {
    0%,
    100% {
        transform: translate(0, 0) scale(1);
    }
    33% {
        transform: translate(40px, -40px) scale(1.05);
    }
    66% {
        transform: translate(-30px, 30px) scale(0.95);
    }
}

/* Sidebar frosted glass enhancement via global style */
:global(.sidebar-container) {
    backdrop-filter: blur(20px) saturate(1.5);
    -webkit-backdrop-filter: blur(20px) saturate(1.5);
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

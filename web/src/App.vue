<script setup lang="ts">
import { onMounted } from "vue";
import Sidebar from "./components/Sidebar.vue";
import SakuraRain from "./components/effects/SakuraRain.vue";
import SparkleParticles from "./components/effects/SparkleParticles.vue";

// Hide loading screen when app is ready
onMounted(() => {
    const loadingScreen = document.getElementById("loading-screen");
    if (loadingScreen) {
        setTimeout(() => {
            loadingScreen.classList.add("hidden");
            setTimeout(() => {
                loadingScreen.remove();
            }, 500);
        }, 1800);
    }
});
</script>

<template>
    <div class="app-container">
        <!-- ═══════════════════════════════════════════════════════════════
         *  ✨ 二次元特效层 - Anime Style Effects Layer
         *  樱花飘落 + 闪光粒子 + 背景光球
         * ═══════════════════════════════════════════════════════════════ -->
        <SakuraRain count="25" speed="normal" />
        <SparkleParticles count="40" />

        <!-- Animated background orbs for Raycast-inspired depth -->
        <div class="bg-orb bg-orb-1"></div>
        <div class="bg-orb bg-orb-2"></div>
        <div class="bg-orb bg-orb-3"></div>
        <div class="bg-orb bg-orb-4"></div>

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
    position: relative;
    overflow: hidden;
}

/* ═══════════════════════════════════════════════════════════════
 *  ✨ Raycast-inspired animated background orbs
 *  模糊光球创造深度感和高端氛围
 * ═══════════════════════════════════════════════════════════════ */

.bg-orb {
    position: fixed;
    border-radius: 50%;
    filter: blur(80px);
    opacity: 0.4;
    pointer-events: none;
    z-index: 0;
}

.bg-orb-1 {
    width: 500px;
    height: 500px;
    top: -100px;
    right: -100px;
    background: radial-gradient(
        circle,
        rgba(236, 72, 153, 0.3) 0%,
        transparent 70%
    );
    animation: orbFloat1 12s ease-in-out infinite;
}

.bg-orb-2 {
    width: 600px;
    height: 600px;
    bottom: -150px;
    left: -150px;
    background: radial-gradient(
        circle,
        rgba(139, 92, 246, 0.25) 0%,
        transparent 70%
    );
    animation: orbFloat2 15s ease-in-out infinite;
}

.bg-orb-3 {
    width: 400px;
    height: 400px;
    top: 40%;
    left: 30%;
    background: radial-gradient(
        circle,
        rgba(59, 130, 246, 0.15) 0%,
        transparent 70%
    );
    animation: orbFloat3 18s ease-in-out infinite;
}

.bg-orb-4 {
    width: 350px;
    height: 350px;
    top: 20%;
    right: 20%;
    background: radial-gradient(
        circle,
        rgba(168, 85, 247, 0.2) 0%,
        transparent 70%
    );
    animation: orbFloat4 20s ease-in-out infinite;
}

@keyframes orbFloat1 {
    0%,
    100% {
        transform: translate(0, 0) scale(1);
    }
    33% {
        transform: translate(-40px, 30px) scale(1.05);
    }
    66% {
        transform: translate(20px, -20px) scale(0.95);
    }
}

@keyframes orbFloat2 {
    0%,
    100% {
        transform: translate(0, 0) scale(1);
    }
    33% {
        transform: translate(30px, -40px) scale(1.08);
    }
    66% {
        transform: translate(-20px, 20px) scale(0.92);
    }
}

@keyframes orbFloat3 {
    0%,
    100% {
        transform: translate(0, 0) scale(1);
    }
    50% {
        transform: translate(50px, -30px) scale(1.1);
    }
}

@keyframes orbFloat4 {
    0%,
    100% {
        transform: translate(0, 0) scale(1);
    }
    50% {
        transform: translate(-30px, 40px) scale(1.06);
    }
}

.main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    z-index: 1;
    background: rgba(250, 245, 255, 0.3);
    backdrop-filter: blur(2px);
    -webkit-backdrop-filter: blur(2px);
}

/* 页面过渡动画 - 带微妙模糊效果 */
.page-enter-active,
.page-leave-active {
    transition: all var(--transition-normal) cubic-bezier(0.25, 0.1, 0.25, 1);
}

.page-enter-from {
    opacity: 0;
    transform: translateY(12px) scale(0.98);
    filter: blur(2px);
}

.page-leave-to {
    opacity: 0;
    transform: translateY(-8px) scale(0.99);
    filter: blur(1px);
}

/* ═══════════════════════════════════════════════════════════════
 *  ✨ 响应式调整
 * ═══════════════════════════════════════════════════════════════ */
</style>

<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";
import { computed } from "vue";
import { useAgentStore } from "../stores/agent";

const route = useRoute();
const router = useRouter();
const agentStore = useAgentStore();

const navItems = [
    { path: "/dashboard", label: "仪表盘", icon: "dashboard" },
    { path: "/providers", label: "模型供应商", icon: "server" },
    { path: "/skills", label: "技能", icon: "zap" },
    { path: "/tools", label: "工具", icon: "wrench" },
    { path: "/chat", label: "对话", icon: "message" },
    { path: "/api-test", label: "接口测试", icon: "flask" },
    { path: "/logs", label: "系统日志", icon: "file-text" },
    { path: "/acp-config", label: "ACP 配置", icon: "terminal" },
];

const isActive = (path: string) => route.path === path;

const statusDotClass = computed(() => {
    switch (agentStore.status.status) {
        case "running":
            return "status-dot-success";
        case "error":
            return "status-dot-danger";
        default:
            return "status-dot-muted";
    }
});

const statusLabel = computed(() => {
    switch (agentStore.status.status) {
        case "running":
            return "运行中";
        case "error":
            return "异常";
        default:
            return "已停止";
    }
});
</script>

<template>
    <aside class="sidebar glass">
        <!-- Logo 区域 -->
        <div class="sidebar-header">
            <div class="logo-container">
                <!-- 可爱的少女风格 Logo SVG -->
                <svg
                    class="logo-icon"
                    viewBox="0 0 100 100"
                    fill="none"
                    xmlns="http://www.w3.org/2000/svg"
                >
                    <!-- 背景光晕 -->
                    <defs>
                        <radialGradient
                            id="gradient-bg"
                            cx="50%"
                            cy="50%"
                            r="50%"
                        >
                            <stop offset="0%" stop-color="#faf5ff" />
                            <stop offset="100%" stop-color="#f3e8ff" />
                        </radialGradient>
                        <linearGradient
                            id="gradient-pink"
                            x1="0%"
                            y1="0%"
                            x2="100%"
                            y2="100%"
                        >
                            <stop offset="0%" stop-color="#ec4899" />
                            <stop offset="100%" stop-color="#8b5cf6" />
                        </linearGradient>
                        <filter id="glow">
                            <feGaussianBlur
                                stdDeviation="2"
                                result="coloredBlur"
                            />
                            <feMerge>
                                <feMergeNode in="coloredBlur" />
                                <feMergeNode in="SourceGraphic" />
                            </feMerge>
                        </filter>
                    </defs>
                    <!-- 背景圆 -->
                    <circle
                        cx="50"
                        cy="50"
                        r="45"
                        fill="url(#gradient-bg)"
                        stroke="url(#gradient-pink)"
                        stroke-width="2"
                    />
                    <!-- 水晶形状 -->
                    <path
                        d="M50 15 L75 45 L50 85 L25 45 Z"
                        fill="url(#gradient-pink)"
                        filter="url(#glow)"
                    />
                    <!-- 高光 -->
                    <path
                        d="M50 15 L60 35 L50 25 L40 35 Z"
                        fill="rgba(255,255,255,0.6)"
                    />
                    <!-- 内部光泽 -->
                    <ellipse
                        cx="50"
                        cy="45"
                        rx="12"
                        ry="16"
                        fill="rgba(255,255,255,0.3)"
                    />
                    <!-- 闪光 -->
                    <circle
                        cx="35"
                        cy="35"
                        r="3"
                        fill="white"
                        class="sparkle sparkle-1"
                    />
                    <circle
                        cx="65"
                        cy="55"
                        r="2"
                        fill="white"
                        class="sparkle sparkle-2"
                    />
                    <circle
                        cx="50"
                        cy="70"
                        r="2"
                        fill="white"
                        class="sparkle sparkle-3"
                    />
                </svg>
            </div>
            <div class="logo-text">
                <h1 class="brand-name text-gradient">琉璃</h1>
                <p class="brand-tag">Ruri AI</p>
            </div>
        </div>

        <!-- 激活状态指示器 -->
        <div class="sidebar-status">
            <div class="status-item">
                <span
                    class="status-dot pulse-dot"
                    :class="statusDotClass"
                ></span>
                <span class="status-text">{{ statusLabel }}</span>
                <span
                    v-if="agentStore.status.active_provider"
                    class="badge badge-accent"
                >
                    {{ agentStore.status.active_provider }}
                </span>
            </div>
        </div>

        <!-- 导航菜单 -->
        <nav class="sidebar-nav">
            <div v-for="item in navItems" :key="item.path">
                <button
                    @click="router.push(item.path)"
                    class="nav-item"
                    :class="{ active: isActive(item.path) }"
                    :title="item.label"
                >
                    <!-- Dashboard Icon -->
                    <svg
                        v-if="item.icon === 'dashboard'"
                        class="nav-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <rect x="3" y="3" width="7" height="7" rx="1" />
                        <rect x="14" y="3" width="7" height="7" rx="1" />
                        <rect x="3" y="14" width="7" height="7" rx="1" />
                        <rect x="14" y="14" width="7" height="7" rx="1" />
                    </svg>

                    <!-- Server Icon -->
                    <svg
                        v-else-if="item.icon === 'server'"
                        class="nav-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <rect x="2" y="2" width="20" height="8" rx="2" />
                        <rect x="2" y="14" width="20" height="8" rx="2" />
                        <circle cx="6" cy="6" r="1" fill="currentColor" />
                        <circle cx="6" cy="18" r="1" fill="currentColor" />
                    </svg>

                    <!-- Zap Icon -->
                    <svg
                        v-else-if="item.icon === 'zap'"
                        class="nav-icon"
                        viewBox="0 0 24 24"
                        fill="currentColor"
                    >
                        <polygon
                            points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"
                        />
                    </svg>

                    <!-- Wrench Icon -->
                    <svg
                        v-else-if="item.icon === 'wrench'"
                        class="nav-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path
                            d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"
                        />
                    </svg>

                    <!-- Message Icon -->
                    <svg
                        v-else-if="item.icon === 'message'"
                        class="nav-icon"
                        viewBox="0 0 24 24"
                        fill="currentColor"
                    >
                        <path
                            d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"
                        />
                    </svg>

                    <!-- Terminal Icon -->
                    <svg
                        v-else-if="item.icon === 'terminal'"
                        class="nav-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <rect x="2" y="3" width="20" height="18" rx="2" />
                        <polyline points="7 10 10 13 7 16" />
                        <line x1="13" y1="16" x2="17" y2="16" />
                    </svg>

                    <!-- Flask Icon -->
                    <svg
                        v-else-if="item.icon === 'flask'"
                        class="nav-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path
                            d="M6 2h12M8 2v6.39A4.39 4.39 0 0 1 5.82 15l-.82.52a2 2 0 0 0-1 1.74V19a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-.74a2 2 0 0 0-1-1.74l-.82-.52A4.39 4.39 0 0 1 16 8.39V2"
                        />
                        <line x1="9" y1="11" x2="15" y2="11" />
                        <line x1="9" y1="15" x2="12" y2="15" />
                    </svg>

                    <!-- File Text Icon -->
                    <svg
                        v-else-if="item.icon === 'file-text'"
                        class="nav-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path
                            d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                        />
                        <polyline points="14 2 14 8 20 8" />
                        <line x1="16" y1="13" x2="8" y2="13" />
                        <line x1="16" y1="17" x2="8" y2="17" />
                        <polyline points="10 9 9 9 8 9" />
                    </svg>

                    <span class="nav-label">{{ item.label }}</span>

                    <!-- 激活指示器 -->
                    <span
                        v-if="isActive(item.path)"
                        class="active-indicator"
                    ></span>
                </button>
            </div>
        </nav>

        <!-- 页脚 -->
        <div class="sidebar-footer">
            <div class="footer-info">
                <span class="version">v0.1.0</span>
                <span class="separator">·</span>
                <span class="tech">Rust</span>
            </div>
        </div>
    </aside>
</template>

<style scoped>
/* ═══════════════════════════════════════════════════════════════
 * Raycast-inspired Sidebar
 * 多层毛玻璃 + 微妙边框 + 悬浮阴影
 * ═══════════════════════════════════════════════════════════════ */
.sidebar {
    width: 240px;
    height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 0;
    position: sticky;
    top: 0;
    padding: 0;
    z-index: 100;
    /* 增强毛玻璃效果 */
    background: linear-gradient(
        180deg,
        rgba(255, 255, 255, 0.88) 0%,
        rgba(250, 245, 255, 0.82) 50%,
        rgba(255, 255, 255, 0.85) 100%
    );
    backdrop-filter: blur(24px) saturate(180%);
    -webkit-backdrop-filter: blur(24px) saturate(180%);
    border-right: 1px solid rgba(216, 180, 254, 0.25);
    box-shadow:
        4px 0 24px rgba(139, 92, 246, 0.06),
        2px 0 12px rgba(236, 72, 153, 0.04),
        inset -1px 0 0 rgba(255, 255, 255, 0.5);
}

/* Header - 带微妙分隔线 */
.sidebar-header {
    padding: 1.5rem 1.25rem 1rem;
    display: flex;
    align-items: center;
    gap: 0.875rem;
    position: relative;
}
.sidebar-header::after {
    content: "";
    position: absolute;
    bottom: 0;
    left: 1.25rem;
    right: 1.25rem;
    height: 1px;
    background: linear-gradient(
        90deg,
        transparent,
        rgba(216, 180, 254, 0.3),
        transparent
    );
}

.logo-container {
    flex-shrink: 0;
}

.logo-icon {
    width: 44px;
    height: 44px;
    filter: drop-shadow(0 2px 6px rgba(139, 92, 246, 0.2));
    transition: transform var(--transition-spring);
}
.logo-container:hover .logo-icon {
    transform: scale(1.05) rotate(-2deg);
}

.logo-icon .sparkle {
    animation: sparkle 1.5s ease-in-out infinite;
}

.sparkle-1 {
    animation-delay: 0s;
}

.sparkle-2 {
    animation-delay: 0.5s;
}

.sparkle-3 {
    animation-delay: 1s;
}

@keyframes sparkle {
    0%,
    100% {
        opacity: 0.6;
    }
    50% {
        opacity: 1;
    }
}

.logo-text {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    transition: opacity var(--transition-fast);
}

.brand-name {
    font-size: 1.125rem;
    font-weight: 700;
    line-height: 1;
    letter-spacing: -0.01em;
    /* 文字发光效果 */
    text-shadow: 0 1px 3px rgba(139, 92, 246, 0.15);
}

.brand-tag {
    font-size: 0.6875rem;
    font-weight: 500;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

/* Status - 带玻璃效果 */
.sidebar-status {
    padding: 0 1.25rem 1rem;
}

.status-item {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    background: rgba(255, 255, 255, 0.6);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    padding: 0.5rem 0.75rem;
    border-radius: var(--radius-sm);
    border: 1px solid rgba(216, 180, 254, 0.2);
    box-shadow: 0 2px 8px rgba(139, 92, 246, 0.04);
    transition: all var(--transition-fast);
}
.status-item:hover {
    background: rgba(255, 255, 255, 0.75);
    border-color: rgba(216, 180, 254, 0.35);
    box-shadow: 0 2px 12px rgba(139, 92, 246, 0.08);
}

.status-dot {
    flex-shrink: 0;
}

.status-text {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-text-secondary);
    flex: 1;
    letter-spacing: 0.01em;
}

/* Navigation - Raycast 风格导航项 */
.sidebar-nav {
    flex: 1;
    padding: 0.5rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    overflow-y: auto;
}

.nav-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.875rem;
    padding: 0.75rem 1rem;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
    position: relative;
    text-align: left;
    /* 微妙的光泽效果 */
    overflow: hidden;
}
.nav-item::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.5) 0%,
        rgba(250, 245, 255, 0.3) 100%
    );
    opacity: 0;
    transition: opacity var(--transition-fast);
    border-radius: inherit;
}

.nav-item:hover {
    background: rgba(255, 255, 255, 0.5);
    border-color: rgba(216, 180, 254, 0.2);
    box-shadow: 0 2px 8px rgba(139, 92, 246, 0.06);
}
.nav-item:hover::before {
    opacity: 1;
}

.nav-item.active {
    background: linear-gradient(
        135deg,
        rgba(236, 72, 153, 0.1) 0%,
        rgba(139, 92, 246, 0.08) 50%,
        rgba(168, 85, 247, 0.06) 100%
    );
    border: 1px solid rgba(236, 72, 153, 0.25);
    box-shadow:
        0 2px 12px rgba(236, 72, 153, 0.1),
        0 4px 16px rgba(139, 92, 246, 0.08),
        inset 0 1px 0 rgba(255, 255, 255, 0.4);
}
.nav-item.active::before {
    opacity: 1;
}

.nav-item.active .nav-icon {
    color: var(--color-accent);
    filter: drop-shadow(0 0 4px rgba(236, 72, 153, 0.3));
}

.nav-item.active .nav-label {
    color: var(--color-accent);
    font-weight: 700;
    text-shadow: 0 1px 2px rgba(236, 72, 153, 0.15);
}

.nav-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    color: var(--color-text-muted);
    transition: all var(--transition-fast);
}
.nav-item:hover .nav-icon {
    color: var(--color-text-secondary);
    transform: scale(1.05);
}

.nav-label {
    flex: 1;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-text-secondary);
    transition: all var(--transition-fast);
}
.nav-item:hover .nav-label {
    color: var(--color-text);
}

.active-indicator {
    flex-shrink: 0;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: linear-gradient(
        135deg,
        var(--color-accent) 0%,
        var(--color-primary) 100%
    );
    box-shadow: 0 0 8px rgba(236, 72, 153, 0.6);
    animation: activePulse 2s ease-in-out infinite;
}
@keyframes activePulse {
    0%,
    100% {
        box-shadow: 0 0 6px rgba(236, 72, 153, 0.4);
        transform: scale(1);
    }
    50% {
        box-shadow: 0 0 10px rgba(236, 72, 153, 0.7);
        transform: scale(1.1);
    }
}

/* Footer - 带微妙分隔线 */
.sidebar-footer {
    padding: 1rem 1.25rem 1.5rem;
    border-top: 1px solid rgba(216, 180, 254, 0.2);
    position: relative;
}
.sidebar-footer::before {
    content: "";
    position: absolute;
    top: 0;
    left: 1.25rem;
    right: 1.25rem;
    height: 1px;
    background: linear-gradient(
        90deg,
        transparent,
        rgba(216, 180, 254, 0.3),
        transparent
    );
}

.footer-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.6875rem;
    color: var(--color-text-muted);
    background: rgba(255, 255, 255, 0.4);
    padding: 0.375rem 0.625rem;
    border-radius: var(--radius-sm);
    border: 1px solid rgba(216, 180, 254, 0.15);
}

.version {
    font-weight: 600;
}

.separator {
    opacity: 0.4;
}

.tech {
    font-weight: 500;
}

/* Status dot colors */
.status-dot-success {
    background: var(--color-success);
}

.status-dot-danger {
    background: var(--color-danger);
}

.status-dot-muted {
    background: var(--color-text-muted);
}

/* Scrollbar */
.sidebar-nav::-webkit-scrollbar {
    width: 4px;
}

.sidebar-nav::-webkit-scrollbar-track {
    background: transparent;
}

.sidebar-nav::-webkit-scrollbar-thumb {
    background: var(--color-border);
    border-radius: 4px;
}

.sidebar-nav::-webkit-scrollbar-thumb:hover {
    background: var(--color-border-hover);
}

/* Responsive */
@media (max-width: 768px) {
    .sidebar {
        width: 64px;
    }

    .sidebar-header {
        padding: 1rem 0.5rem;
        justify-content: center;
    }

    .logo-text {
        display: none;
    }

    .sidebar-status {
        padding: 0 0.5rem 1rem;
    }

    .status-item {
        padding: 0.5rem;
    }

    .status-text,
    .badge {
        display: none;
    }

    .sidebar-nav {
        padding: 0 0.5rem;
    }

    .nav-label {
        display: none;
    }

    .nav-item {
        justify-content: center;
        padding: 0.75rem;
    }

    .active-indicator {
        display: none;
    }

    .sidebar-footer {
        padding: 1rem 0.5rem 1.5rem;
        justify-content: center;
        display: flex;
    }

    .separator,
    .tech {
        display: none;
    }

    .version {
        margin: 0 auto;
    }
}
</style>

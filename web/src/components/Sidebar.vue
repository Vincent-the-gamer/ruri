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
];

const isActive = (path: string) => route.path === path;

const statusDotClass = computed(() => {
    switch (agentStore.status.status) {
        case "running":
            return "status-dot-running";
        case "error":
            return "status-dot-error";
        default:
            return "status-dot-stopped";
    }
});

const statusLabel = computed(() => {
    switch (agentStore.status.status) {
        case "running":
            return "运行中";
        case "error":
            return "错误";
        default:
            return "已停止";
    }
});
</script>

<template>
    <aside
        class="w-56 min-h-screen flex flex-col"
        style="
            background: var(--color-bg-soft);
            border-right: 1px solid var(--color-border);
        "
    >
        <!-- Logo -->
        <div class="p-4 border-b" style="border-color: var(--color-border)">
            <div class="flex items-center gap-2.5">
                <svg
                    width="24"
                    height="24"
                    viewBox="0 0 24 24"
                    fill="none"
                    xmlns="http://www.w3.org/2000/svg"
                    class="flex-shrink-0"
                >
                    <path
                        d="M13 2L4 14h7l-2 8 9-12h-7l2-8z"
                        fill="var(--color-accent)"
                        stroke="var(--color-accent)"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    />
                </svg>
                <div>
                    <h1
                        class="text-base font-semibold"
                        style="color: var(--color-text)"
                    >
                        Ruri
                    </h1>
                    <p class="text-xs" style="color: var(--color-text-muted)">
                        AI 智能体控制台
                    </p>
                </div>
            </div>
        </div>

        <!-- Status -->
        <div
            class="px-4 py-2.5 border-b"
            style="border-color: var(--color-border)"
        >
            <div class="flex items-center gap-2">
                <span
                    class="status-dot w-1.5 h-1.5 rounded-full flex-shrink-0"
                    :class="statusDotClass"
                ></span>
                <span class="text-xs" style="color: var(--color-text-muted)">{{
                    statusLabel
                }}</span>
                <span
                    v-if="agentStore.status.active_provider"
                    class="text-xs ml-auto"
                    style="color: var(--color-accent)"
                >
                    {{ agentStore.status.active_provider }}
                </span>
            </div>
        </div>

        <!-- Navigation -->
        <nav class="flex-1 py-2">
            <button
                v-for="item in navItems"
                :key="item.path"
                @click="router.push(item.path)"
                class="w-full flex items-center gap-2.5 px-4 py-2 text-sm transition-colors relative"
                :class="
                    isActive(item.path)
                        ? 'nav-item-active'
                        : 'nav-item-inactive'
                "
            >
                <span
                    v-if="isActive(item.path)"
                    class="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-4 rounded-r"
                    style="background: var(--color-accent)"
                ></span>

                <!-- Dashboard Icon -->
                <svg
                    v-if="item.icon === 'dashboard'"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="flex-shrink-0"
                >
                    <rect x="3" y="3" width="7" height="7" rx="1" />
                    <rect x="14" y="3" width="7" height="7" rx="1" />
                    <rect x="3" y="14" width="7" height="7" rx="1" />
                    <rect x="14" y="14" width="7" height="7" rx="1" />
                </svg>

                <!-- Server Icon -->
                <svg
                    v-else-if="item.icon === 'server'"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="flex-shrink-0"
                >
                    <rect x="2" y="2" width="20" height="8" rx="2" ry="2" />
                    <rect x="2" y="14" width="20" height="8" rx="2" ry="2" />
                    <circle cx="6" cy="6" r="1" fill="currentColor" />
                    <circle cx="6" cy="18" r="1" fill="currentColor" />
                </svg>

                <!-- Zap Icon -->
                <svg
                    v-else-if="item.icon === 'zap'"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="flex-shrink-0"
                >
                    <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
                </svg>

                <!-- Wrench Icon -->
                <svg
                    v-else-if="item.icon === 'wrench'"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="flex-shrink-0"
                >
                    <path
                        d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"
                    />
                </svg>

                <!-- Message Icon -->
                <svg
                    v-else-if="item.icon === 'message'"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="flex-shrink-0"
                >
                    <path
                        d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"
                    />
                </svg>

                <!-- Flask Icon -->
                <svg
                    v-else-if="item.icon === 'flask'"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="flex-shrink-0"
                >
                    <path
                        d="M6 2h12M8 2v6.39A4.39 4.39 0 0 1 5.82 15l-.82.52a2 2 0 0 0-1 1.74V19a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-.74a2 2 0 0 0-1-1.74l-.82-.52A4.39 4.39 0 0 1 16 8.39V2"
                    />
                    <line x1="9" y1="11" x2="15" y2="11" />
                    <line x1="9" y1="15" x2="12" y2="15" />
                </svg>

                <span>{{ item.label }}</span>
            </button>
        </nav>

        <!-- Footer -->
        <div class="p-3 border-t" style="border-color: var(--color-border)">
            <div
                class="text-center text-xs flex items-center justify-center gap-1.5"
                style="color: var(--color-text-dim)"
            >
                <span>v0.1.0</span>
                <span style="color: var(--color-border)">·</span>
                <svg
                    width="12"
                    height="12"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <path
                        d="M11 2a7 7 0 0 1 7 7c0 2.38-1.19 4.47-3 5.74V17a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2v-2.26C5.19 13.47 4 11.38 4 9a7 7 0 0 1 7-7z"
                    />
                    <path d="M9 22h6" />
                    <path d="M10 22v-2" />
                    <path d="M14 22v-2" />
                </svg>
                <span>Rust 驱动</span>
            </div>
        </div>
    </aside>
</template>

<style scoped>
.nav-item-active {
    background: var(--color-accent-soft);
    color: var(--color-accent);
}

.nav-item-inactive {
    color: var(--color-text-secondary);
    background: transparent;
}

.nav-item-inactive:hover {
    color: var(--color-text);
    background: var(--color-accent-soft);
}

.status-dot-running {
    background: var(--color-success);
}

.status-dot-error {
    background: var(--color-danger);
}

.status-dot-stopped {
    background: var(--color-text-muted);
}
</style>

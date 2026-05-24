<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
import { Icon } from "@iconify/vue";

import packageJson from "../../package.json";

const route = useRoute();
const { t } = useI18n();

// Get version from package.json
const appVersion = packageJson.version;

// Navigation groups with items
const navGroups = computed(() => [
    {
        key: "general",
        label: t("nav.groups.general"),
        icon: "lucide:compass",
        items: [
            {
                path: "/",
                label: t("nav.home"),
                icon: "lucide:home",
            },
            {
                path: "/dashboard",
                label: t("nav.dashboard"),
                icon: "lucide:layout-dashboard",
            },
        ],
    },
    {
        key: "conversation",
        label: t("nav.groups.conversation"),
        icon: "lucide:message-square",
        items: [
            {
                path: "/chat",
                label: t("nav.chat"),
                icon: "lucide:message-circle",
            },
            {
                path: "/conversation-history",
                label: t("nav.conversationHistory"),
                icon: "lucide:history",
            },
        ],
    },
    {
        key: "config",
        label: t("nav.groups.config"),
        icon: "lucide:settings-2",
        items: [
            {
                path: "/configs",
                label: t("nav.configs"),
                icon: "lucide:layers",
            },
            {
                path: "/builtin-commands",
                label: t("nav.builtinCommands"),
                icon: "lucide:terminal",
            },
            {
                path: "/providers",
                label: t("nav.providers"),
                icon: "lucide:server",
            },
            {
                path: "/personas",
                label: t("nav.personas"),
                icon: "lucide:user-circle",
            },
        ],
    },
    {
        key: "capabilities",
        label: t("nav.groups.capabilities"),
        icon: "lucide:puzzle",
        items: [
            {
                path: "/platform-config",
                label: t("nav.platformConfig"),
                icon: "lucide:radio-tower",
            },
            {
                path: "/skills",
                label: t("nav.skills"),
                icon: "lucide:sparkles",
            },
            {
                path: "/tools",
                label: t("nav.tools"),
                icon: "lucide:wrench",
            },
            {
                path: "/acp-config",
                label: t("nav.agentConfig"),
                icon: "lucide:bot",
            },
            {
                path: "/mcp-config",
                label: t("nav.mcpConfig"),
                icon: "lucide:hard-drive",
            },
            {
                path: "/computer-use-config",
                label: t("nav.computerUse"),
                icon: "lucide:monitor",
            },
            {
                path: "/web-search-config",
                label: t("nav.webSearch"),
                icon: "lucide:search",
            },
            {
                path: "/knowledge-base",
                label: t("nav.knowledgeBase", "Knowledge Base"),
                icon: "lucide:book-open",
            },
        ],
    },
    {
        key: "debug",
        label: t("nav.groups.debug"),
        icon: "lucide:bug",
        items: [
            {
                path: "/logs",
                label: t("nav.logs"),
                icon: "lucide:scroll-text",
            },
            {
                path: "/network-monitor",
                label: t("nav.networkMonitor", "Network Monitor"),
                icon: "lucide:activity",
            },
            {
                path: "/api-test",
                label: t("nav.apiTest"),
                icon: "lucide:flask-conical",
            },
        ],
    },
    {
        key: "system",
        label: t("nav.groups.system"),
        icon: "lucide:cpu",
        items: [
            {
                path: "/system",
                label: t("nav.system"),
                icon: "lucide:settings",
            },
        ],
    },
]);

// Track which groups are collapsed (collapsed = true means hidden)
const collapsedGroups = ref<Record<string, boolean>>({});

// Initialize: all groups expanded by default
navGroups.value.forEach((group) => {
    collapsedGroups.value[group.key] = false;
});

// Watch route changes and auto-expand the group containing the active route
watch(
    () => route.path,
    (newPath) => {
        for (const group of navGroups.value) {
            if (group.items.some((item) => item.path === newPath)) {
                collapsedGroups.value[group.key] = false;
                break;
            }
        }
    },
    { immediate: true },
);

// Toggle group collapse
const toggleGroup = (key: string) => {
    collapsedGroups.value[key] = !collapsedGroups.value[key];
};

// Check if a nav item is active
const isActive = (path: string) => {
    return route.path === path;
};

// Check if a group has an active item
const isGroupActive = (group: (typeof navGroups.value)[0]) => {
    return group.items.some((item) => isActive(item.path));
};
</script>

<template>
    <aside
        class="sidebar-container w-64 flex-shrink-0 border-r border-border/30 bg-background/25 backdrop-blur-xl supports-[backdrop-filter]:bg-background/15 flex flex-col min-h-0 overflow-hidden"
    >
        <!-- Navigation -->
        <nav class="flex-1 min-h-0 p-3 space-y-1 overflow-y-auto scroll-hover">
            <div v-for="group in navGroups" :key="group.key" class="nav-group">
                <!-- Group Header -->
                <button
                    class="nav-group-header w-full flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-bold uppercase tracking-wider transition-all duration-200"
                    :class="[
                        isGroupActive(group)
                            ? 'nav-group-header-active'
                            : 'nav-group-header-inactive',
                    ]"
                    @click="toggleGroup(group.key)"
                >
                    <Icon
                        :icon="group.icon"
                        class="text-sm flex-shrink-0 nav-group-icon"
                    />
                    <span class="nav-group-label flex-1 text-left">{{
                        group.label
                    }}</span>
                    <Icon
                        :icon="
                            collapsedGroups[group.key]
                                ? 'lucide:chevron-right'
                                : 'lucide:chevron-down'
                        "
                        class="text-sm flex-shrink-0 nav-chevron transition-transform duration-200"
                    />
                </button>

                <!-- Group Items -->
                <Transition name="collapse">
                    <div
                        v-show="!collapsedGroups[group.key]"
                        class="nav-group-items"
                    >
                        <router-link
                            v-for="item in group.items"
                            :key="item.path"
                            :to="item.path"
                            class="nav-item flex items-center gap-3 pl-9 pr-3 py-2 rounded-lg text-sm font-semibold transition-all duration-200 relative overflow-hidden group"
                            :class="[
                                isActive(item.path)
                                    ? 'nav-item-active'
                                    : 'nav-item-inactive',
                            ]"
                        >
                            <Icon
                                :icon="item.icon"
                                class="text-lg flex-shrink-0 nav-icon transition-transform duration-200 group-hover:scale-110"
                            />
                            <span class="nav-label">{{ item.label }}</span>
                        </router-link>
                    </div>
                </Transition>
            </div>
        </nav>

        <!-- Footer -->
        <div class="sidebar-footer p-3 border-t border-border/40 flex-shrink-0">
            <div
                class="px-3 py-2 text-xs text-muted-foreground flex items-center gap-2"
            >
                <span class="font-semibold text-foreground">Ruri</span>
                <span class="text-muted-foreground/50">•</span>
                <span
                    class="version-badge px-2 py-0.5 rounded-full bg-primary/10 text-primary text-xs font-semibold"
                    >v{{ appVersion }}</span
                >
            </div>
        </div>
    </aside>
</template>

<style scoped>
/* Sidebar Container - frosted glass */
.sidebar-container {
    background: linear-gradient(
        180deg,
        hsl(var(--background) / 0.3) 0%,
        hsl(var(--background) / 0.2) 100%
    );
    backdrop-filter: blur(20px) saturate(1.5);
    -webkit-backdrop-filter: blur(20px) saturate(1.5);
}

/* Navigation Group */
.nav-group {
    margin-bottom: 2px;
}

/* Group Header */
.nav-group-header {
    border: 1px solid transparent;
    cursor: pointer;
    user-select: none;
    letter-spacing: 0.05em;
}

.nav-group-header-inactive {
    color: hsl(var(--muted-foreground) / 0.8);
    background-color: transparent;
}

.nav-group-header-inactive:hover {
    color: hsl(var(--foreground) / 0.8);
    background-color: hsl(var(--secondary) / 0.3);
}

.nav-group-header-active {
    color: hsl(var(--primary) / 0.9);
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.08) 0%,
        hsl(var(--primary) / 0.03) 100%
    );
}

.nav-group-header-active:hover {
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.12) 0%,
        hsl(var(--primary) / 0.06) 100%
    );
}

.nav-group-icon {
    transition: all 0.2s ease;
}

.nav-group-header-active .nav-group-icon {
    filter: drop-shadow(0 0 3px hsl(var(--primary) / 0.4));
}

.nav-chevron {
    opacity: 0.5;
    transition: all 0.2s ease;
}

.nav-group-header:hover .nav-chevron {
    opacity: 0.8;
}

/* Group Items */
.nav-group-items {
    overflow: hidden;
}

/* Collapse Transition */
.collapse-enter-active,
.collapse-leave-active {
    transition: all 0.25s ease;
    overflow: hidden;
}

.collapse-enter-from,
.collapse-leave-to {
    opacity: 0;
    max-height: 0;
}

.collapse-enter-to,
.collapse-leave-from {
    opacity: 1;
    max-height: 500px;
}

/* Navigation Items */
.nav-item {
    position: relative;
    font-weight: 600;
}

.nav-item::before {
    content: "";
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%) scaleY(0);
    width: 3px;
    height: 60%;
    border-radius: 0 2px 2px 0;
    background: linear-gradient(
        180deg,
        hsl(var(--primary)),
        hsl(var(--primary) / 0.7)
    );
    transition: transform 0.2s ease;
}

/* Inactive State */
.nav-item-inactive {
    color: hsl(var(--muted-foreground));
    background-color: transparent;
    border: 1px solid transparent;
}

.nav-item-inactive:hover {
    color: hsl(var(--foreground));
    background-color: hsl(var(--secondary) / 0.5);
    border-color: hsl(var(--border) / 0.5);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.nav-item-inactive:hover::before {
    transform: translateY(-50%) scaleY(0.5);
    background: hsl(var(--primary) / 0.3);
}

/* Active State */
.nav-item-active {
    color: hsl(var(--primary));
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.15) 0%,
        hsl(var(--primary) / 0.08) 100%
    );
    border: 1px solid hsl(var(--primary) / 0.3);
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.1);
}

.nav-item-active::before {
    transform: translateY(-50%) scaleY(1);
}

.nav-item-active:hover {
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.2) 0%,
        hsl(var(--primary) / 0.12) 100%
    );
    border-color: hsl(var(--primary) / 0.5);
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.15);
}

/* Icon */
.nav-icon {
    transition: all 0.2s ease;
}

.nav-item-active .nav-icon {
    filter: drop-shadow(0 0 4px hsl(var(--primary) / 0.5));
}

/* Label */
.nav-label {
    position: relative;
    z-index: 1;
}

/* Footer */
.sidebar-footer {
    background: hsl(var(--background) / 0.15);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
}

.version-badge {
    animation: pulss 2s ease-in-out infinite;
}

@keyframes pulss {
    0%,
    100% {
        opacity: 1;
    }
    50% {
        opacity: 0.7;
    }
}

/* Scrollbar styling - handled by scroll-hover class globally */
nav::-webkit-scrollbar {
    width: 4px;
}

nav::-webkit-scrollbar-track {
    background: transparent;
}

nav::-webkit-scrollbar-thumb {
    background: transparent;
    border-radius: 2px;
    transition: background 0.3s ease;
}

nav:hover::-webkit-scrollbar-thumb {
    background: hsl(var(--muted-foreground) / 0.2);
}

nav::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--muted-foreground) / 0.4);
}

/* Responsive */
@media (max-width: 768px) {
    aside {
        width: 64px;
    }

    .nav-label,
    .nav-group-label,
    .nav-chevron {
        display: none;
    }

    .nav-item {
        justify-content: center;
        padding-left: 0;
        padding-right: 0;
    }

    .nav-item::before {
        display: none;
    }

    .nav-group-header {
        justify-content: center;
        padding-left: 0;
        padding-right: 0;
    }
}
</style>

<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
import { Icon } from "@iconify/vue";
import packageJson from "../../package.json";

const route = useRoute();
const { t } = useI18n();

// Get version from package.json
const appVersion = packageJson.version;

// Navigation items
const navItems = computed(() => [
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
    {
        path: "/chat",
        label: t("nav.chat"),
        icon: "lucide:message-square",
    },
    {
        path: "/skills",
        label: t("nav.skills"),
        icon: "lucide:sparkles",
    },
    {
        path: "/providers",
        label: t("nav.providers"),
        icon: "lucide:server",
    },
    {
        path: "/tools",
        label: t("nav.tools"),
        icon: "lucide:wrench",
    },
    {
        path: "/logs",
        label: t("nav.logs"),
        icon: "lucide:scroll-text",
    },
    {
        path: "/acp-config",
        label: t("nav.agentConfig"),
        icon: "lucide:bot",
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
        path: "/api-test",
        label: t("nav.apiTest"),
        icon: "lucide:flask-conical",
    },
]);

// Check if nav item is active
const isActive = (path: string) => {
    return route.path === path;
};
</script>

<template>
    <aside
        class="sidebar-container w-64 border-r border-border/30 bg-background/25 backdrop-blur-xl supports-[backdrop-filter]:bg-background/15 flex flex-col transition-all duration-300"
    >
        <!-- Navigation -->
        <nav class="flex-1 p-3 space-y-1.5 overflow-y-auto">
            <router-link
                v-for="item in navItems"
                :key="item.path"
                :to="item.path"
                class="nav-item flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-semibold transition-all duration-200 relative overflow-hidden group"
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
        </nav>

        <!-- Footer -->
        <div class="sidebar-footer p-3 border-t border-border/40">
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

/* Scrollbar styling */
nav::-webkit-scrollbar {
    width: 6px;
}

nav::-webkit-scrollbar-track {
    background: transparent;
}

nav::-webkit-scrollbar-thumb {
    background: hsl(var(--muted));
    border-radius: 3px;
}

nav::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--muted-foreground) / 0.5);
}

/* Responsive */
@media (max-width: 768px) {
    aside {
        width: 64px;
    }

    .nav-label {
        display: none;
    }

    .nav-item {
        justify-content: center;
        padding: 0.625rem;
    }

    .nav-item::before {
        display: none;
    }
}
</style>

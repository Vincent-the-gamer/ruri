<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
import { Icon } from "@iconify/vue";

const route = useRoute();
const { t } = useI18n();

// Navigation items
const navItems = computed(() => [
    {
        path: "/",
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
        label: t("nav.settings"),
        icon: "lucide:settings",
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
        class="w-64 border-r border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 flex flex-col transition-all duration-300"
    >
        <!-- Navigation -->
        <nav class="flex-1 p-3 space-y-1 overflow-y-auto">
            <router-link
                v-for="item in navItems"
                :key="item.path"
                :to="item.path"
                class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-semibold text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors duration-200"
                :class="[
                    isActive(item.path)
                        ? 'bg-primary/10 text-primary hover:bg-primary/15 hover:text-primary'
                        : '',
                ]"
            >
                <Icon :icon="item.icon" class="text-lg flex-shrink-0" />
                <span>{{ item.label }}</span>
            </router-link>
        </nav>

        <!-- Footer -->
        <div class="p-3 border-t border-border/40">
            <div class="px-3 py-2 text-xs text-muted-foreground">
                <span class="font-semibold">Ruri</span>
                <span class="mx-2">•</span>
                <span>v1.0.0</span>
            </div>
        </div>
    </aside>
</template>

<style scoped>
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

    nav a span {
        display: none;
    }

    nav a {
        justify-content: center;
        padding: 0.625rem;
    }

    .footer-info span:not(:first-child) {
        display: none;
    }
}
</style>

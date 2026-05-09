<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useConfigStore } from "../stores/config";
import { getBuiltinCommands } from "../api";
import type { BuiltinCommand } from "../types";

const { t } = useI18n();
const configStore = useConfigStore();

const commands = ref<BuiltinCommand[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

const commandPrefix = computed(() => configStore.commandPrefix);

const visibleCommands = computed(() =>
    commands.value.filter((c) => !c.hidden),
);

onMounted(async () => {
    loading.value = true;
    error.value = null;
    try {
        commands.value = await getBuiltinCommands();
    } catch (e: unknown) {
        error.value =
            e instanceof Error ? e.message : "Failed to load commands";
    } finally {
        loading.value = false;
    }
});
</script>

<template>
    <div class="page">
        <div class="page-header">
            <div class="header-content">
                <div class="header-icon">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path d="M8 9l3 3-3 3" />
                        <line x1="14" y1="15" x2="18" y2="15" />
                        <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
                        <path
                            d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"
                        />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">{{ t("builtinCommands.title") }}</h1>
                    <p class="header-desc">{{ t("builtinCommands.subtitle") }}</p>
                </div>
            </div>
            <div v-if="commandPrefix" class="prefix-badge">
                <span class="prefix-label">{{ t("builtinCommands.prefix") }}:</span>
                <span class="prefix-value">{{ commandPrefix }}</span>
            </div>
        </div>

        <div v-if="loading" class="loading-state">
            <div class="loading-spinner"></div>
            <span class="loading-text">Loading...</span>
        </div>

        <div v-else-if="error" class="error-banner">
            <span>{{ error }}</span>
        </div>

        <div v-else-if="visibleCommands.length === 0" class="empty-state">
            <div class="empty-illustration">
                <div class="empty-icon-wrapper">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path d="M8 9l3 3-3 3" />
                        <line x1="14" y1="15" x2="18" y2="15" />
                        <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
                        <path
                            d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"
                        />
                    </svg>
                </div>
            </div>
            <h3 class="empty-title">{{ t("builtinCommands.noCommands") }}</h3>
            <p class="empty-desc">{{ t("builtinCommands.noCommandsDesc") }}</p>
        </div>

        <div v-else class="commands-list">
            <div
                v-for="cmd in visibleCommands"
                :key="cmd.name"
                class="command-card"
            >
                <div class="command-header">
                    <span class="command-name">{{ commandPrefix }}{{ cmd.name }}</span>
                    <span v-if="cmd.require_admin" class="admin-badge">
                        🔒 {{ t("builtinCommands.requireAdmin") }}
                    </span>
                </div>
                <p class="command-desc">{{ cmd.description }}</p>
                <p v-if="cmd.usage" class="command-usage">
                    <span class="usage-label">{{ t("builtinCommands.usage") }}:</span>
                    <code class="usage-code">{{ cmd.usage }}</code>
                </p>
            </div>
        </div>
    </div>
</template>

<style scoped>
.page {
    max-width: 800px;
}

.page-header {
    margin-bottom: 2rem;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 1rem;
}

.header-content {
    display: flex;
    align-items: center;
    gap: 1rem;
}

.header-icon {
    width: 48px;
    height: 48px;
    border-radius: 12px;
    background: hsl(var(--primary) / 0.1);
    border: 1px solid hsl(var(--primary) / 0.2);
    display: flex;
    align-items: center;
    justify-content: center;
    color: hsl(var(--primary));
    flex-shrink: 0;
}

.header-icon svg {
    width: 24px;
    height: 24px;
}

.header-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: hsl(var(--foreground));
    margin: 0;
}

.header-desc {
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
    margin: 0.25rem 0 0;
}

.prefix-badge {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
    font-size: 0.875rem;
}

.prefix-label {
    color: hsl(var(--muted-foreground));
    font-weight: 500;
}

.prefix-value {
    color: hsl(var(--primary));
    font-weight: 700;
    font-family: monospace;
    font-size: 1rem;
}

/* Loading / Error */
.loading-state {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 3rem;
    justify-content: center;
}

.loading-spinner {
    width: 24px;
    height: 24px;
    border: 3px solid hsl(var(--border));
    border-top-color: hsl(var(--primary));
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

.loading-text {
    color: hsl(var(--muted-foreground));
    font-size: 0.875rem;
}

.error-banner {
    padding: 1rem;
    background: hsl(0 84% 60% / 0.1);
    border: 1px solid hsl(0 84% 60% / 0.3);
    border-radius: 8px;
    color: hsl(0 84% 45%);
    font-size: 0.875rem;
}

/* Empty State */
.empty-state {
    text-align: center;
    padding: 4rem 2rem;
}

.empty-illustration {
    margin-bottom: 1rem;
}

.empty-icon-wrapper {
    width: 64px;
    height: 64px;
    margin: 0 auto;
    border-radius: 16px;
    background: hsl(var(--secondary));
    display: flex;
    align-items: center;
    justify-content: center;
    color: hsl(var(--muted-foreground));
}

.empty-icon-wrapper svg {
    width: 32px;
    height: 32px;
}

.empty-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0 0 0.5rem;
}

.empty-desc {
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
    margin: 0;
}

/* Commands List */
.commands-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}

.command-card {
    padding: 1rem 1.25rem;
    background: hsl(var(--card) / 0.6);
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    transition: all 0.2s ease;
}

.command-card:hover {
    border-color: hsl(var(--primary) / 0.3);
    background: hsl(var(--card));
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.05);
}

.command-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.375rem;
}

.command-name {
    font-family: monospace;
    font-size: 1rem;
    font-weight: 700;
    color: hsl(var(--primary));
}

.admin-badge {
    font-size: 0.6875rem;
    font-weight: 600;
    padding: 0.125rem 0.5rem;
    background: hsl(38 92% 50% / 0.15);
    color: hsl(38 92% 40%);
    border-radius: 999px;
}

.dark .admin-badge {
    color: hsl(38 92% 70%);
}

.command-desc {
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
    margin: 0 0 0.5rem;
}

.command-usage {
    font-size: 0.8125rem;
    margin: 0;
    display: flex;
    align-items: center;
    gap: 0.375rem;
}

.usage-label {
    color: hsl(var(--muted-foreground));
    font-weight: 500;
}

.usage-code {
    font-family: monospace;
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
    padding: 0.125rem 0.5rem;
    border-radius: 4px;
    color: hsl(var(--foreground));
    font-size: 0.8125rem;
}

@media (max-width: 640px) {
    .page-header {
        flex-direction: column;
    }
}
</style>

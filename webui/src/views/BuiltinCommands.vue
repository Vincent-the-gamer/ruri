<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useConfigStore } from "../stores/config";
import { getBuiltinCommands, toggleCommandAdmin } from "../api";
import type { BuiltinCommand } from "../types";

const { t } = useI18n();
const configStore = useConfigStore();

const commands = ref<BuiltinCommand[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const toggling = ref<string | null>(null);

const commandPrefix = computed(() => configStore.commandPrefix);

const visibleCommands = computed(() => commands.value.filter((c) => !c.hidden));

const enabledCount = computed(
    () => visibleCommands.value.filter((c) => c.enabled).length,
);

async function handleToggleAdmin(cmd: BuiltinCommand) {
    if (toggling.value) return;
    toggling.value = cmd.name;
    try {
        const newAdminRequired = !cmd.require_admin;
        await toggleCommandAdmin(cmd.name, newAdminRequired);
        // Update local state
        const target = commands.value.find((c) => c.name === cmd.name);
        if (target) {
            target.require_admin = newAdminRequired;
        }
    } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        error.value = t("builtinCommands.toggleFailed") + ": " + msg;
        setTimeout(() => {
            error.value = null;
        }, 3000);
    } finally {
        toggling.value = null;
    }
}

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
                    <h1 class="header-title">
                        {{ t("builtinCommands.title") }}
                    </h1>
                    <p class="header-desc">
                        {{ t("builtinCommands.subtitle") }}
                    </p>
                </div>
            </div>
        </div>

        <!-- Status summary -->
        <div class="status-bar">
            <div class="status-item">
                <span class="status-label">{{
                    t("builtinCommands.prefix")
                }}</span>
                <span class="status-value prefix-value">{{
                    commandPrefix
                }}</span>
            </div>
            <div class="status-divider"></div>
            <div class="status-item">
                <span class="status-label">{{
                    t("builtinCommands.totalCommands")
                }}</span>
                <span class="status-value">{{ visibleCommands.length }}</span>
            </div>
            <div class="status-divider"></div>
            <div class="status-item">
                <span class="status-label">{{
                    t("builtinCommands.enabledCommands")
                }}</span>
                <span class="status-value enabled-count">{{
                    enabledCount
                }}</span>
            </div>
        </div>

        <!-- Info tip -->
        <div class="info-tip">
            <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            >
                <circle cx="12" cy="12" r="10" />
                <line x1="12" y1="16" x2="12" y2="12" />
                <line x1="12" y1="8" x2="12.01" y2="8" />
            </svg>
            <span>{{ t("builtinCommands.readOnlyTip") }}</span>
        </div>

        <div v-if="loading" class="loading-state">
            <div class="loading-spinner"></div>
            <span class="loading-text">Loading...</span>
        </div>

        <div v-else-if="error && commands.length === 0" class="error-banner">
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
                :class="{ 'command-card--disabled': !cmd.enabled }"
            >
                <div class="command-main">
                    <div class="command-info">
                        <div class="command-header">
                            <span class="command-name"
                                >{{ commandPrefix }}{{ cmd.name }}</span
                            >
                            <span
                                class="status-badge"
                                :class="
                                    cmd.enabled
                                        ? 'status-badge--enabled'
                                        : 'status-badge--disabled'
                                "
                            >
                                {{
                                    cmd.enabled
                                        ? t("builtinCommands.enabled")
                                        : t("builtinCommands.disabled")
                                }}
                            </span>
                            <span
                                v-if="cmd.require_admin"
                                class="admin-badge clickable"
                                :class="{ toggling: toggling === cmd.name }"
                                :title="t('builtinCommands.toggleOpen')"
                                @click="handleToggleAdmin(cmd)"
                            >
                                <span
                                    v-if="toggling === cmd.name"
                                    class="toggle-spinner-inline"
                                ></span>
                                <template v-else
                                    >🔒
                                    {{
                                        t("builtinCommands.requireAdmin")
                                    }}</template
                                >
                            </span>
                            <span
                                v-else
                                class="open-badge clickable"
                                :class="{ toggling: toggling === cmd.name }"
                                :title="t('builtinCommands.toggleAdmin')"
                                @click="handleToggleAdmin(cmd)"
                            >
                                <span
                                    v-if="toggling === cmd.name"
                                    class="toggle-spinner-inline"
                                ></span>
                                <template v-else
                                    >🌐
                                    {{
                                        t("builtinCommands.openToAll")
                                    }}</template
                                >
                            </span>
                        </div>
                        <p class="command-desc">{{ cmd.description }}</p>
                        <p v-if="cmd.usage" class="command-usage">
                            <span class="usage-label"
                                >{{ t("builtinCommands.usage") }}:</span
                            >
                            <code class="usage-code">{{ cmd.usage }}</code>
                        </p>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.page {
    max-width: 800px;
}

.page-header {
    margin-bottom: 1.5rem;
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

/* Status Bar */
.status-bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1.25rem;
    background: hsl(var(--card) / 0.6);
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    margin-bottom: 1rem;
    flex-wrap: wrap;
}

.status-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.status-label {
    font-size: 0.8125rem;
    color: hsl(var(--muted-foreground));
    font-weight: 500;
}

.status-value {
    font-size: 0.875rem;
    font-weight: 700;
    color: hsl(var(--foreground));
}

.prefix-value {
    font-family: monospace;
    color: hsl(var(--primary));
}

.enabled-count {
    color: hsl(142 76% 36%);
}

.dark .enabled-count {
    color: hsl(142 76% 60%);
}

.status-divider {
    width: 1px;
    height: 20px;
    background: hsl(var(--border));
}

/* Info Tip */
.info-tip {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: hsl(var(--primary) / 0.06);
    border: 1px solid hsl(var(--primary) / 0.15);
    border-radius: 8px;
    margin-bottom: 1rem;
    font-size: 0.8125rem;
    color: hsl(var(--muted-foreground));
    line-height: 1.5;
}

.info-tip svg {
    flex-shrink: 0;
    margin-top: 1px;
    color: hsl(var(--primary));
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
    gap: 0.5rem;
}

.command-card {
    padding: 0.875rem 1.25rem;
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

.command-card--disabled {
    opacity: 0.55;
}

.command-card--disabled:hover {
    opacity: 0.75;
}

.command-main {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
}

.command-info {
    flex: 1;
    min-width: 0;
}

.command-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.375rem;
    flex-wrap: wrap;
}

.command-name {
    font-family: monospace;
    font-size: 1rem;
    font-weight: 700;
    color: hsl(var(--primary));
}

/* Status Badge */
.status-badge {
    font-size: 0.6875rem;
    font-weight: 600;
    padding: 0.125rem 0.5rem;
    border-radius: 999px;
}

.status-badge--enabled {
    background: hsl(142 76% 36% / 0.12);
    color: hsl(142 76% 30%);
}

.dark .status-badge--enabled {
    color: hsl(142 76% 70%);
}

.status-badge--disabled {
    background: hsl(var(--muted-foreground) / 0.12);
    color: hsl(var(--muted-foreground));
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

.open-badge {
    font-size: 0.6875rem;
    font-weight: 600;
    padding: 0.125rem 0.5rem;
    background: hsl(142 76% 36% / 0.12);
    color: hsl(142 76% 30%);
    border-radius: 999px;
}

.dark .open-badge {
    color: hsl(142 76% 70%);
}

.clickable {
    cursor: pointer;
    transition: all 0.15s ease;
    user-select: none;
}

.clickable:hover {
    filter: brightness(0.9);
    transform: scale(1.05);
}

.clickable:active {
    transform: scale(0.95);
}

.clickable.toggling {
    pointer-events: none;
    opacity: 0.6;
}

.toggle-spinner-inline {
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 2px solid hsl(var(--border));
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    vertical-align: middle;
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

    .command-main {
        flex-direction: column;
    }

    .status-bar {
        flex-direction: column;
        align-items: flex-start;
        gap: 0.5rem;
    }

    .status-divider {
        display: none;
    }
}
</style>

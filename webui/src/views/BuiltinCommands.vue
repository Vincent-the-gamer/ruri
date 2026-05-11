<script setup lang="ts">
import { ref, onMounted, computed, reactive } from "vue";
import { useI18n } from "vue-i18n";
import { useConfigStore } from "../stores/config";
import { getBuiltinCommands, updateCommandAdminRequired } from "../api";
import type { BuiltinCommand } from "../types";

const { t } = useI18n();
const configStore = useConfigStore();

const commands = ref<BuiltinCommand[]>([]);
const loading = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);
const successMessage = ref<string | null>(null);

const commandPrefix = computed(() => configStore.commandPrefix);

const visibleCommands = computed(() => commands.value.filter((c) => !c.hidden));

// Build admin overrides map from loaded commands
const adminOverrides = reactive<Record<string, boolean>>({});

function syncOverrides() {
    // Clear and rebuild from current commands
    for (const key of Object.keys(adminOverrides)) {
        delete adminOverrides[key];
    }
    for (const cmd of commands.value) {
        adminOverrides[cmd.name] = cmd.require_admin;
    }
}

function toggleAdminRequired(cmdName: string) {
    adminOverrides[cmdName] = !adminOverrides[cmdName];
}

function isOverridden(cmd: BuiltinCommand): boolean {
    return cmd.require_admin !== cmd.default_require_admin;
}

async function saveOverrides() {
    saving.value = true;
    error.value = null;
    successMessage.value = null;
    try {
        const result = await updateCommandAdminRequired({ ...adminOverrides });
        // Update local commands state from result
        for (const cmd of commands.value) {
            if (result.command_admin_required[cmd.name] !== undefined) {
                cmd.require_admin = result.command_admin_required[cmd.name];
            }
        }
        successMessage.value = t("builtinCommands.saveSuccess");
        setTimeout(() => {
            successMessage.value = null;
        }, 3000);
    } catch (e: unknown) {
        error.value = e instanceof Error ? e.message : "Failed to save";
        // Revert overrides on failure
        syncOverrides();
    } finally {
        saving.value = false;
    }
}

onMounted(async () => {
    loading.value = true;
    error.value = null;
    try {
        commands.value = await getBuiltinCommands();
        syncOverrides();
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
            <div class="header-actions">
                <div v-if="commandPrefix" class="prefix-badge">
                    <span class="prefix-label"
                        >{{ t("builtinCommands.prefix") }}:</span
                    >
                    <span class="prefix-value">{{ commandPrefix }}</span>
                </div>
                <button
                    class="btn-save"
                    :disabled="saving"
                    @click="saveOverrides"
                >
                    <svg
                        v-if="!saving"
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path
                            d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"
                        />
                        <polyline points="17 21 17 13 7 13 7 21" />
                        <polyline points="7 3 7 8 15 8" />
                    </svg>
                    <svg
                        v-else
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        class="spin-icon"
                    >
                        <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                    </svg>
                    {{
                        saving
                            ? t("builtinCommands.saving")
                            : t("builtinCommands.save")
                    }}
                </button>
            </div>
        </div>

        <!-- Success Banner -->
        <div v-if="successMessage" class="success-banner">
            <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            >
                <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                <polyline points="22 4 12 14.01 9 11.01" />
            </svg>
            <span>{{ successMessage }}</span>
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
                <span>{{ t("builtinCommands.adminTip") }}</span>
            </div>

            <div
                v-for="cmd in visibleCommands"
                :key="cmd.name"
                class="command-card"
            >
                <div class="command-main">
                    <div class="command-info">
                        <div class="command-header">
                            <span class="command-name"
                                >{{ commandPrefix }}{{ cmd.name }}</span
                            >
                            <span
                                v-if="adminOverrides[cmd.name]"
                                class="admin-badge"
                            >
                                🔒 {{ t("builtinCommands.requireAdmin") }}
                            </span>
                            <span v-else class="open-badge">
                                🌐 {{ t("builtinCommands.openToAll") }}
                            </span>
                            <span
                                v-if="isOverridden(cmd)"
                                class="override-badge"
                                :title="t('builtinCommands.overrideHint')"
                            >
                                {{ t("builtinCommands.customized") }}
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

                    <div class="command-toggle">
                        <label class="toggle-label">
                            <span class="toggle-text">{{
                                t("builtinCommands.adminOnly")
                            }}</span>
                            <button
                                class="toggle-switch"
                                :class="{ active: adminOverrides[cmd.name] }"
                                @click="toggleAdminRequired(cmd.name)"
                                role="switch"
                                :aria-checked="adminOverrides[cmd.name]"
                            >
                                <span class="toggle-thumb"></span>
                            </button>
                        </label>
                        <span
                            v-if="
                                cmd.default_require_admin &&
                                !adminOverrides[cmd.name]
                            "
                            class="default-notice"
                        >
                            {{ t("builtinCommands.defaultAdminOpened") }}
                        </span>
                        <span
                            v-else-if="
                                !cmd.default_require_admin &&
                                adminOverrides[cmd.name]
                            "
                            class="default-notice"
                        >
                            {{ t("builtinCommands.defaultOpenLocked") }}
                        </span>
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

.header-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
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

.btn-save {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 1rem;
    font-size: 0.8125rem;
    font-weight: 600;
    color: hsl(var(--primary-foreground));
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(280 70% 60%));
    border: none;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.2);
}

.btn-save:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.3);
}

.btn-save:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}

.spin-icon {
    animation: spin 0.8s linear infinite;
}

/* Success Banner */
.success-banner {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    margin-bottom: 1rem;
    background: hsl(142 76% 36% / 0.1);
    border: 1px solid hsl(142 76% 36% / 0.3);
    border-radius: 8px;
    color: hsl(142 76% 30%);
    font-size: 0.8125rem;
    font-weight: 500;
    animation: fadeIn 0.3s ease;
}

.dark .success-banner {
    color: hsl(142 76% 70%);
}

@keyframes fadeIn {
    from {
        opacity: 0;
        transform: translateY(-4px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
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

.override-badge {
    font-size: 0.625rem;
    font-weight: 600;
    padding: 0.0625rem 0.375rem;
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
    border-radius: 999px;
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

/* Toggle Switch */
.command-toggle {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.375rem;
    flex-shrink: 0;
    padding-top: 0.125rem;
}

.toggle-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
}

.toggle-text {
    font-size: 0.75rem;
    font-weight: 500;
    color: hsl(var(--muted-foreground));
    white-space: nowrap;
}

.toggle-switch {
    position: relative;
    width: 36px;
    height: 20px;
    border-radius: 10px;
    border: none;
    background: hsl(var(--border));
    cursor: pointer;
    transition: all 0.2s ease;
    padding: 0;
}

.toggle-switch.active {
    background: hsl(var(--primary));
}

.toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: white;
    transition: transform 0.2s ease;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
}

.toggle-switch.active .toggle-thumb {
    transform: translateX(16px);
}

.default-notice {
    font-size: 0.6875rem;
    color: hsl(38 92% 50%);
    white-space: nowrap;
}

.dark .default-notice {
    color: hsl(38 92% 70%);
}

@media (max-width: 640px) {
    .page-header {
        flex-direction: column;
    }

    .command-main {
        flex-direction: column;
    }

    .command-toggle {
        flex-direction: row;
        align-items: center;
        justify-content: space-between;
        width: 100%;
        padding-top: 0.5rem;
        border-top: 1px solid hsl(var(--border) / 0.5);
        margin-top: 0.25rem;
    }
}
</style>

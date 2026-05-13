<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { Icon } from "@iconify/vue";
import { useConfigStore } from "../stores/config";
import { useProviderStore } from "../stores/provider";
import { usePersonaStore } from "../stores/persona";
import { useSkillStore } from "../stores/skill";
import { usePlatformStore } from "../stores/platform";
import { useKnowledgeBaseStore } from "../stores/knowledgeBase";
import ConfigForm from "../components/ConfigForm.vue";
import type { ConfigProfile } from "../types";

const { t } = useI18n();
const configStore = useConfigStore();
const providerStore = useProviderStore();
const personaStore = usePersonaStore();
const skillStore = useSkillStore();
const platformStore = usePlatformStore();
const kbStore = useKnowledgeBaseStore();

const showForm = ref(false);
const editingConfig = ref<ConfigProfile | null>(null);
const saving = ref(false);
const deletingId = ref<string | null>(null);
const deleteConfirm = ref(false);

onMounted(async () => {
    await Promise.all([
        configStore.fetchConfigProfiles(),
        providerStore.fetchProviders(),
        personaStore.fetchPersonas(),
        kbStore.fetchKnowledgeBases(),
        skillStore.fetchSkills(),
        platformStore.fetchInstances(),
    ]);
});

function openCreate() {
    editingConfig.value = null;
    showForm.value = true;
}

function openEdit(config: ConfigProfile) {
    editingConfig.value = config;
    showForm.value = true;
}

async function handleSave(data: any) {
    saving.value = true;
    try {
        if (editingConfig.value) {
            await configStore.updateConfigProfile(editingConfig.value.id, data);
        } else {
            await configStore.createConfigProfile(data);
        }
        showForm.value = false;
        editingConfig.value = null;
    } catch (error) {
        console.error("Failed to save config:", error);
    } finally {
        saving.value = false;
    }
}

function handleCancel() {
    showForm.value = false;
    editingConfig.value = null;
}

function handleDelete(config: ConfigProfile) {
    deletingId.value = config.id;
    deleteConfirm.value = true;
}

async function confirmDelete() {
    if (deletingId.value) {
        try {
            await configStore.deleteConfigProfile(deletingId.value);
            deleteConfirm.value = false;
            deletingId.value = null;
        } catch (error) {
            console.error("Failed to delete config:", error);
        }
    }
}

async function handleActivate(config: ConfigProfile) {
    try {
        await configStore.activateConfigProfile(config.id);
    } catch (error) {
        console.error("Failed to activate config:", error);
    }
}

async function handleDeactivate(config: ConfigProfile) {
    try {
        await configStore.deactivateConfigProfile(config.id);
    } catch (error) {
        console.error("Failed to deactivate config:", error);
    }
}

async function handleToggleEnable(config: ConfigProfile) {
    try {
        await configStore.updateConfigProfile(config.id, {
            enable: !config.enable,
        });
    } catch (error) {
        console.error("Failed to toggle config enable:", error);
    }
}

function cancelDelete() {
    deleteConfirm.value = false;
    deletingId.value = null;
}

function getProviderName(providerId: string | null): string {
    if (!providerId) return t("config.none");
    const provider = providerStore.providers.find((p) => p.id === providerId);
    return provider ? provider.name : t("config.unknown");
}

function getPersonaName(personaId: string | null): string {
    if (!personaId) return t("config.none");
    const persona = personaStore.personas.find((p) => p.id === personaId);
    return persona ? persona.name : t("config.unknown");
}
</script>

<template>
    <div class="page">
        <!-- Page Header -->
        <div class="page-header">
            <div class="header-content">
                <div class="header-icon">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path
                            d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.1a2 2 0 0 1-1-1.72v-.51a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
                        ></path>
                        <circle cx="12" cy="12" r="3"></circle>
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">{{ t("config.title") }}</h1>
                    <p class="header-desc">{{ t("config.subtitle") }}</p>
                </div>
            </div>
            <button class="btn btn-accent" @click="openCreate">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <line x1="12" y1="5" x2="12" y2="19"></line>
                    <line x1="5" y1="12" x2="19" y2="12"></line>
                </svg>
                {{ t("config.addConfig") }}
            </button>
        </div>

        <!-- Error Banner -->
        <div v-if="configStore.error" class="error-banner">
            <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            >
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="15" y1="9" x2="9" y2="15"></line>
                <line x1="9" y1="9" x2="15" y2="15"></line>
            </svg>
            <span>{{ configStore.error }}</span>
        </div>

        <!-- Loading State -->
        <div v-if="configStore.loading" class="loading-state">
            <div class="loading-spinner"></div>
            <span class="loading-text">{{ t("common.loading") }}</span>
        </div>

        <!-- Empty State -->
        <div
            v-else-if="configStore.configProfiles.length === 0"
            class="empty-state"
        >
            <div class="empty-illustration">
                <div class="empty-icon-wrapper">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path
                            d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.1a2 2 0 0 1-1-1.72v-.51a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
                        ></path>
                        <circle cx="12" cy="12" r="3"></circle>
                    </svg>
                </div>
                <div class="empty-decoration">
                    <span class="deco-dot deco-dot-1"></span>
                    <span class="deco-dot deco-dot-2"></span>
                    <span class="deco-dot deco-dot-3"></span>
                </div>
            </div>
            <h3 class="empty-title">{{ t("config.noConfigs") }}</h3>
            <p class="empty-desc">{{ t("config.noConfigsDesc") }}</p>
            <button class="btn btn-accent" @click="openCreate">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <line x1="12" y1="5" x2="12" y2="19" />
                    <line x1="5" y1="12" x2="19" y2="12" />
                </svg>
                {{ t("config.addFirstConfig") }}
            </button>
        </div>

        <!-- Config List -->
        <div v-else class="card-list">
            <div
                v-for="(config, index) in configStore.configProfiles"
                :key="config.id"
                :class="[
                    'config-card',
                    { 'config-card--active': config.is_active },
                    { 'config-card--disabled': !config.enable },
                ]"
                :style="{ animationDelay: `${index * 0.05}s` }"
            >
                <div
                    v-if="config.is_active && config.enable"
                    class="card-glow card-glow--active"
                ></div>
                <div class="card-content">
                    <div class="card-info">
                        <div class="card-icon">
                            <span class="icon-wrapper">
                                <Icon
                                    :icon="
                                        !config.enable
                                            ? 'lucide:circle-off'
                                            : config.is_active
                                              ? 'lucide:check-circle'
                                              : 'lucide:settings'
                                    "
                                />
                            </span>
                            <span
                                :class="[
                                    'type-dot',
                                    !config.enable
                                        ? 'type-dot--disabled'
                                        : config.is_active
                                          ? 'type-dot--active'
                                          : 'type-dot--inactive',
                                ]"
                            ></span>
                        </div>
                        <div class="card-details">
                            <div class="card-title-row">
                                <h3 class="card-title">{{ config.name }}</h3>
                                <span
                                    :class="[
                                        'status-badge',
                                        !config.enable
                                            ? 'status-badge--disabled'
                                            : config.is_active
                                              ? 'status-badge--active'
                                              : 'status-badge--inactive',
                                    ]"
                                >
                                    <span class="status-dot"></span>
                                    {{
                                        !config.enable
                                            ? t("common.disabled")
                                            : config.is_active
                                              ? t("common.active")
                                              : t("common.inactive")
                                    }}
                                </span>
                            </div>
                            <div v-if="config.description" class="card-desc">
                                {{ config.description }}
                            </div>
                            <div class="card-meta">
                                <div class="card-info-row">
                                    <svg
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <path
                                            d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
                                        ></path>
                                        <polyline
                                            points="3.27 6.96 12 12.01 20.73 6.96"
                                        ></polyline>
                                        <line
                                            x1="12"
                                            y1="22.08"
                                            x2="12"
                                            y2="12"
                                        ></line>
                                    </svg>
                                    <span class="info-text">{{
                                        getProviderName(config.provider_id)
                                    }}</span>
                                </div>
                                <div class="card-info-row">
                                    <svg
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <path
                                            d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"
                                        ></path>
                                        <circle cx="12" cy="7" r="4"></circle>
                                    </svg>
                                    <span class="info-text">{{
                                        getPersonaName(config.persona_id)
                                    }}</span>
                                </div>
                                <div class="card-tags">
                                    <span
                                        v-if="!config.enable"
                                        class="tag tag--disabled"
                                    >
                                        {{ t("common.disabled") }}
                                    </span>
                                    <span
                                        v-if="config.proxy_config?.enabled"
                                        class="tag tag--proxy"
                                    >
                                        Proxy
                                    </span>
                                    <span
                                        v-if="config.web_search_enabled"
                                        class="tag"
                                        >Web</span
                                    >
                                    <span
                                        v-if="config.computer_use_enabled"
                                        class="tag"
                                        >Computer</span
                                    >

                                    <span
                                        v-if="
                                            config.command_prefix &&
                                            config.command_prefix !== '/'
                                        "
                                        class="tag"
                                    >
                                        {{ config.command_prefix }}
                                    </span>
                                    <span
                                        v-if="
                                            config.active_skill_names.length > 0
                                        "
                                        class="tag"
                                    >
                                        {{ config.active_skill_names.length }}
                                        skills
                                    </span>
                                    <span
                                        v-if="
                                            config.platform_ids &&
                                            config.platform_ids.length > 0
                                        "
                                        class="tag"
                                    >
                                        {{ config.platform_ids.length }}
                                        {{
                                            config.platform_ids.length === 1
                                                ? t("config.platform")
                                                : t("config.platforms")
                                        }}
                                    </span>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div class="card-actions">
                        <label
                            class="enable-switch"
                            :title="
                                config.enable
                                    ? t('common.disable')
                                    : t('common.enable')
                            "
                        >
                            <input
                                type="checkbox"
                                class="enable-switch-input"
                                :checked="config.enable"
                                @change="handleToggleEnable(config)"
                            />
                            <span class="enable-switch-slider"></span>
                        </label>
                        <button
                            v-if="!config.is_active"
                            class="btn btn-sm btn-success"
                            @click="handleActivate(config)"
                            :disabled="!config.enable"
                            :title="
                                !config.enable
                                    ? t('config.activateDisabledHint')
                                    : t('config.activate')
                            "
                        >
                            <svg
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <polyline
                                    points="22 11.08V12a10 10 0 1 1-5.93-9.14"
                                />
                                <polyline points="22 4 12 14.01 9 11.01" />
                            </svg>
                        </button>
                        <button
                            v-else
                            class="btn btn-sm btn-danger-ghost"
                            @click="handleDeactivate(config)"
                            :title="t('config.deactivate')"
                        >
                            <svg
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <circle cx="12" cy="12" r="10" />
                                <line x1="15" y1="9" x2="9" y2="15" />
                                <line x1="9" y1="9" x2="15" y2="15" />
                            </svg>
                        </button>
                        <button
                            class="btn btn-sm btn-ghost"
                            @click="openEdit(config)"
                            :title="t('common.edit')"
                        >
                            <svg
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <path
                                    d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"
                                />
                                <path
                                    d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"
                                />
                            </svg>
                        </button>
                        <button
                            class="btn btn-sm btn-danger-ghost"
                            @click="handleDelete(config)"
                            :title="t('common.delete')"
                        >
                            <svg
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <polyline points="3 6 5 6 21 6" />
                                <path
                                    d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                                />
                            </svg>
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Form Modal -->
        <Teleport to="body">
            <div v-if="showForm" class="modal-overlay">
                <div class="modal-content glass" @click.stop>
                    <ConfigForm
                        :config="editingConfig"
                        :saving="saving"
                        @save="handleSave"
                        @cancel="handleCancel"
                    />
                </div>
            </div>
        </Teleport>

        <!-- Delete Confirm Modal -->
        <Teleport to="body">
            <div v-if="deleteConfirm" class="modal-overlay">
                <div class="modal-content glass modal-sm" @click.stop>
                    <div class="modal-header">
                        <h2 class="modal-title">
                            {{ t("config.deleteConfirm") }}
                        </h2>
                    </div>
                    <div class="modal-body">
                        <p>{{ t("config.deleteConfirmDesc") }}</p>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-ghost" @click="cancelDelete">
                            {{ t("common.cancel") }}
                        </button>
                        <button class="btn btn-danger" @click="confirmDelete">
                            {{ t("common.confirm") }}
                        </button>
                    </div>
                </div>
            </div>
        </Teleport>
    </div>
</template>

<style scoped>
.page {
    padding: 1.5rem;
}

.page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
}

.header-content {
    display: flex;
    align-items: center;
    gap: 1rem;
}

.header-icon {
    width: 3rem;
    height: 3rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 0.75rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.2) 0%,
        hsl(var(--primary) / 0.1) 100%
    );
    border: 1px solid hsl(var(--primary) / 0.3);
}

.header-icon svg {
    width: 1.5rem;
    height: 1.5rem;
    color: hsl(var(--primary));
}

.header-text {
    display: flex;
    flex-direction: column;
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
    margin: 0;
    margin-top: 0.25rem;
}

.btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 1.25rem;
    font-size: 0.875rem;
    font-weight: 600;
    border-radius: 0.5rem;
    border: none;
    cursor: pointer;
    transition: all 0.2s;
}

.btn::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 0.5rem;
    opacity: 0;
    transition: opacity 0.2s;
}

.btn:hover::before {
    opacity: 1;
}

.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-sm {
    padding: 0.5rem;
}

.btn-accent {
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
}

.btn-accent:hover:not(:disabled) {
    background: hsl(var(--primary) / 0.9);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.3);
}

.btn-ghost {
    background: transparent;
    color: hsl(var(--foreground));
    border: 1px solid hsl(var(--border));
}

.btn-ghost:hover {
    background: hsl(var(--secondary));
    border-color: hsl(var(--border) / 0.8);
}

.btn-danger-ghost {
    background: transparent;
    color: hsl(var(--destructive));
    border: 1px solid transparent;
}

.btn-danger-ghost:hover {
    background: hsl(var(--destructive) / 0.1);
    border-color: hsl(var(--destructive) / 0.3);
}

.btn-danger {
    background: hsl(var(--destructive));
    color: hsl(var(--destructive-foreground));
}

.btn-success {
    background: transparent;
    color: hsl(var(--success));
    border: 1px solid transparent;
}

.btn-success:hover {
    background: hsl(var(--success) / 0.1);
    border-color: hsl(var(--success) / 0.3);
}

.btn svg {
    width: 1rem;
    height: 1rem;
}

.status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
}

.status-dot {
    width: 0.375rem;
    height: 0.375rem;
    border-radius: 9999px;
    animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

.status-badge--active {
    background: hsl(var(--success) / 0.15);
    color: hsl(var(--success));
}

.status-badge--active .status-dot {
    background: hsl(var(--success));
}

.status-badge--inactive {
    background: hsl(var(--muted) / 0.5);
    color: hsl(var(--muted-foreground));
}

.status-badge--inactive .status-dot {
    background: hsl(var(--muted-foreground));
    animation: none;
}

.status-badge--disabled {
    background: hsl(var(--destructive) / 0.15);
    color: hsl(var(--destructive));
}

.status-badge--disabled .status-dot {
    background: hsl(var(--destructive));
    animation: none;
}

.error-banner {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    margin-bottom: 1.5rem;
    background: hsl(var(--destructive) / 0.1);
    border: 1px solid hsl(var(--destructive) / 0.3);
    border-radius: 0.75rem;
    color: hsl(var(--destructive));
    font-size: 0.875rem;
}

.error-banner svg {
    width: 1.25rem;
    height: 1.25rem;
    flex-shrink: 0;
}

.loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    gap: 1rem;
}

.loading-spinner {
    width: 2rem;
    height: 2rem;
    border: 2px solid hsl(var(--muted));
    border-top-color: hsl(var(--primary));
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
}

.loading-text {
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    text-align: center;
}

.empty-illustration {
    position: relative;
    margin-bottom: 2rem;
}

.empty-icon-wrapper {
    width: 8rem;
    height: 8rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 1.5rem;
    background: hsl(var(--secondary) / 0.5);
    border: 2px dashed hsl(var(--border) / 0.5);
}

.empty-icon-wrapper svg {
    width: 4rem;
    height: 4rem;
    color: hsl(var(--muted-foreground));
}

.empty-decoration {
    position: absolute;
    inset: 0;
    pointer-events: none;
}

.deco-dot {
    position: absolute;
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: hsl(var(--primary));
}

.deco-dot-1 {
    top: 10%;
    right: 20%;
    animation: pulse 2s ease infinite 0s;
}

.deco-dot-2 {
    top: 60%;
    right: 5%;
    animation: pulse 2s ease infinite 0.5s;
}

.deco-dot-3 {
    bottom: 20%;
    left: 15%;
    animation: pulse 2s ease infinite 1s;
}

@keyframes pulse {
    0%,
    100% {
        opacity: 0.3;
        transform: scale(1);
    }
    50% {
        opacity: 1;
        transform: scale(1.2);
    }
}

.empty-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin-bottom: 0.5rem;
}

.empty-desc {
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
    margin-bottom: 1.5rem;
    max-width: 400px;
}

.card-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}

.config-card {
    position: relative;
    border-radius: 0.875rem;
    background: hsl(var(--background));
    border: 1px solid hsl(var(--border) / 0.5);
    overflow: hidden;
    transition: all 0.3s ease;
    animation: fadeInUp 0.4s ease both;
}

@keyframes fadeInUp {
    from {
        opacity: 0;
        transform: translateY(20px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

.card-glow {
    position: absolute;
    top: 2px;
    left: 2px;
    right: 2px;
    height: 2px;
    border-radius: 0.875rem 0.875rem 0 0;
    background: linear-gradient(
        90deg,
        transparent,
        hsl(var(--primary) / 0.5),
        transparent
    );
    animation: glowMove 3s ease-in-out infinite;
}

@keyframes glowMove {
    0%,
    100% {
        opacity: 0;
        transform: translateX(-100%);
    }
    50% {
        opacity: 1;
        transform: translateX(100%);
    }
}

.card-glow--active {
    background: linear-gradient(
        90deg,
        hsl(var(--success) / 0.3),
        hsl(var(--success) / 0.6),
        hsl(var(--success) / 0.3)
    );
    height: 3px;
    opacity: 1;
}

.config-card:hover {
    border-color: hsl(var(--primary) / 0.5);
    transform: translateY(-2px);
    box-shadow: 0 8px 24px hsl(var(--foreground) / 0.05);
}

.config-card:hover .card-glow {
    opacity: 1;
}

.config-card--active {
    border-color: hsl(var(--success) / 0.5);
    background: hsl(var(--success) / 0.03);
}

.config-card--active:hover {
    border-color: hsl(var(--success) / 0.6);
    box-shadow: 0 8px 24px hsl(var(--success) / 0.1);
}

.config-card--disabled {
    opacity: 0.65;
    border-color: hsl(var(--destructive) / 0.3);
}

.config-card--disabled:hover {
    opacity: 0.85;
    border-color: hsl(var(--destructive) / 0.4);
}

.config-card--disabled .icon-wrapper {
    color: hsl(var(--destructive));
}

.card-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.25rem;
    gap: 1rem;
}

.card-info {
    display: flex;
    gap: 1rem;
    align-items: center;
    flex: 1;
}

.card-icon {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 3rem;
    height: 3rem;
    border-radius: 0.625rem;
    background: hsl(var(--secondary));
    flex-shrink: 0;
}

.icon-wrapper {
    font-size: 1.5rem;
    color: hsl(var(--muted-foreground));
}

.config-card--active .icon-wrapper {
    color: hsl(var(--success));
}

.type-dot {
    position: absolute;
    top: -2px;
    right: -2px;
    width: 0.75rem;
    height: 0.75rem;
    border-radius: 50%;
    border: 2px solid hsl(var(--background));
}

.type-dot--active {
    background: hsl(var(--success));
}

.type-dot--inactive {
    background: hsl(var(--muted));
}

.type-dot--disabled {
    background: hsl(var(--destructive));
}

.card-details {
    flex: 1;
    min-width: 0;
}

.card-title-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.375rem;
}

.card-title {
    font-size: 1rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

.card-desc {
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
    margin-bottom: 0.625rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.card-meta {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
}

.card-info-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
}

.card-info-row svg {
    width: 0.875rem;
    height: 0.875rem;
    flex-shrink: 0;
}

.info-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.card-tags {
    display: flex;
    gap: 0.375rem;
    flex-wrap: wrap;
}

.tag {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.5rem;
    border-radius: 0.375rem;
    font-size: 0.6875rem;
    font-weight: 500;
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
    border: 1px solid hsl(var(--primary) / 0.2);
}

.tag--disabled {
    background: hsl(var(--destructive) / 0.1);
    color: hsl(var(--destructive));
    border-color: hsl(var(--destructive) / 0.2);
}

.tag--proxy {
    background: hsl(30 80% 50% / 0.1);
    color: hsl(30 80% 50%);
    border-color: hsl(30 80% 50% / 0.2);
}

.card-actions {
    display: flex;
    gap: 0.375rem;
    flex-shrink: 0;
    align-items: center;
}

/* Enable/Disable Toggle Switch */
.enable-switch {
    position: relative;
    display: inline-block;
    width: 2.25rem;
    height: 1.25rem;
    flex-shrink: 0;
    cursor: pointer;
}

.enable-switch-input {
    opacity: 0;
    width: 0;
    height: 0;
}

.enable-switch-slider {
    position: absolute;
    inset: 0;
    background-color: hsl(var(--muted));
    border-radius: 9999px;
    transition: all 0.3s ease;
}

.enable-switch-slider::before {
    content: "";
    position: absolute;
    height: 0.875rem;
    width: 0.875rem;
    left: 0.1875rem;
    bottom: 0.1875rem;
    background-color: hsl(var(--foreground));
    border-radius: 50%;
    transition: all 0.3s ease;
}

.enable-switch-input:checked + .enable-switch-slider {
    background-color: hsl(var(--success));
}

.enable-switch-input:checked + .enable-switch-slider::before {
    transform: translateX(1rem);
    background-color: white;
}

.enable-switch:hover .enable-switch-slider {
    box-shadow: 0 0 0 2px hsl(var(--primary) / 0.2);
}

/* Modal */
.modal-overlay {
    position: fixed;
    inset: 0;
    background: hsl(var(--background) / 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 1rem;
    animation: fadeIn 0.2s ease-out;
}

@keyframes fadeIn {
    from {
        opacity: 0;
    }
    to {
        opacity: 1;
    }
}

.modal-content {
    width: 100%;
    max-width: 700px;
    max-height: 90vh;
    border-radius: 1rem;
    background: hsl(var(--background));
    border: 1px solid hsl(var(--border));
    box-shadow: 0 25px 50px -12px hsl(var(--foreground) / 0.25);
    overflow: hidden;
    animation: modalSlideIn 0.25s ease-out;
}

@keyframes modalSlideIn {
    from {
        opacity: 0;
        transform: scale(0.95) translateY(10px);
    }
    to {
        opacity: 1;
        transform: scale(1) translateY(0);
    }
}

.modal-sm {
    max-width: 400px;
}

.glass {
    background: hsl(var(--background) / 0.9);
    backdrop-filter: blur(12px);
}

.modal-header {
    padding: 1rem 1.25rem;
    border-bottom: 1px solid hsl(var(--border) / 0.2);
}

.modal-title {
    font-size: 1.1rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

.modal-body {
    padding: 1.25rem;
}

.modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.625rem;
    padding: 1rem 1.25rem;
    border-top: 1px solid hsl(var(--border) / 0.2);
}

/* Responsive */
@media (max-width: 640px) {
    .page {
        padding: 1rem;
    }

    .page-header {
        flex-direction: column;
        align-items: flex-start;
        gap: 1rem;
    }

    .card-content {
        flex-direction: column;
        align-items: flex-start;
    }

    .card-actions {
        width: 100%;
        justify-content: flex-end;
    }

    .modal-content {
        max-width: 100%;
        max-height: 95vh;
    }
}
</style>

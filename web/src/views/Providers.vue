<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useProviderStore } from "../stores/provider";
import ProviderForm from "../components/ProviderForm.vue";
import type {
    Provider,
    ProviderType,
    ProviderConfig,
    CreateProviderRequest,
} from "../types";

const providerStore = useProviderStore();
const showForm = ref(false);
const editingProvider = ref<Provider | null>(null);

onMounted(() => {
    providerStore.fetchProviders();
});

function openCreate() {
    editingProvider.value = null;
    showForm.value = true;
}

function openEdit(provider: Provider) {
    editingProvider.value = provider;
    showForm.value = true;
}

async function handleSave(data: {
    name: string;
    provider_type: ProviderType;
    config: ProviderConfig;
}) {
    try {
        if (editingProvider.value) {
            await providerStore.updateProvider(editingProvider.value.id, data);
        } else {
            await providerStore.createProvider(data as CreateProviderRequest);
        }
        showForm.value = false;
        editingProvider.value = null;
    } catch {
        // error is in store
    }
}

async function handleDelete(id: string) {
    if (!confirm("确定要删除此供应商吗？")) return;
    try {
        await providerStore.deleteProvider(id);
    } catch {
        // error is in store
    }
}

async function handleActivate(id: string) {
    try {
        await providerStore.activateProvider(id);
    } catch {
        // error is in store
    }
}

const providerTypeIcon = (type: string) => {
    switch (type) {
        case "openai":
            return "🟢";
        case "anthropic":
            return "🟣";
        case "custom":
            return "⚙️";
        default:
            return "🔌";
    }
};

const providerTypeLabel = (type: string) => {
    switch (type) {
        case "openai":
            return "OpenAI";
        case "anthropic":
            return "Anthropic";
        case "custom":
            return "Custom";
        default:
            return type;
    }
};

function maskApiKey(key: string): string {
    if (!key) return "(未设置)";
    if (key.length <= 8) return "••••••••";
    return key.slice(0, 4) + "••••" + key.slice(-4);
}
</script>

<template>
    <div class="page">
        <!-- Header -->
        <div class="page-header">
            <div class="header-info">
                <h1 class="header-title">模型供应商</h1>
                <p class="header-desc">配置 AI 模型供应商</p>
            </div>
            <button class="btn btn-accent" @click="openCreate">
                <svg
                    width="14"
                    height="14"
                    viewBox="0 0 14 14"
                    fill="none"
                    xmlns="http://www.w3.org/2000/svg"
                >
                    <path
                        d="M7 1v12M1 7h12"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                    />
                </svg>
                添加供应商
            </button>
        </div>

        <!-- Error -->
        <div v-if="providerStore.error" class="error-banner">
            {{ providerStore.error }}
        </div>

        <!-- Loading -->
        <div
            v-if="providerStore.loading && providerStore.providers.length === 0"
            class="loading-state"
        >
            加载中...
        </div>

        <!-- Empty State -->
        <div
            v-else-if="providerStore.providers.length === 0"
            class="empty-state"
        >
            <div class="empty-icon">
                <svg
                    width="40"
                    height="40"
                    viewBox="0 0 24 24"
                    fill="none"
                    xmlns="http://www.w3.org/2000/svg"
                >
                    <path
                        d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2z"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    />
                    <path
                        d="M8 12h8M12 8v8"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                    />
                </svg>
            </div>
            <h3 class="empty-title">暂无供应商</h3>
            <p class="empty-desc">添加供应商以开始与 AI 模型对话</p>
            <button class="btn btn-accent" @click="openCreate">
                添加第一个供应商
            </button>
        </div>

        <!-- Provider Cards -->
        <div v-else class="card-list">
            <div
                v-for="provider in providerStore.providers"
                :key="provider.id"
                class="provider-card"
                :class="{ 'provider-card--active': provider.is_active }"
            >
                <div class="card-content">
                    <div class="card-info">
                        <div class="card-icon">
                            <span
                                class="type-dot"
                                :class="`type-dot--${provider.provider_type}`"
                            ></span>
                        </div>
                        <div class="card-details">
                            <div class="card-title-row">
                                <h3 class="card-title">{{ provider.name }}</h3>
                                <span
                                    v-if="provider.is_active"
                                    class="badge badge-accent"
                                    >活跃</span
                                >
                            </div>
                            <div class="card-meta">
                                {{ providerTypeLabel(provider.provider_type) }}
                                · {{ (provider.config as any).default_model }}
                            </div>
                            <div class="card-subtle">
                                {{ (provider.config as any).base_url }}
                            </div>
                            <div class="card-subtle">
                                密钥：{{
                                    maskApiKey(
                                        (provider.config as any).api_key || "",
                                    )
                                }}
                            </div>
                        </div>
                    </div>

                    <div class="card-actions">
                        <button
                            v-if="!provider.is_active"
                            class="btn btn-ghost btn-sm"
                            @click="handleActivate(provider.id)"
                        >
                            设为活跃
                        </button>
                        <button
                            class="btn btn-ghost btn-sm"
                            @click="openEdit(provider)"
                        >
                            编辑
                        </button>
                        <button
                            class="btn btn-ghost btn-sm btn-danger-ghost"
                            @click="handleDelete(provider.id)"
                        >
                            删除
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Form Modal -->
        <ProviderForm
            v-if="showForm"
            :provider="editingProvider"
            @save="handleSave"
            @cancel="
                showForm = false;
                editingProvider = null;
            "
        />
    </div>
</template>

<style scoped>
.page {
    padding: 1.5rem;
    max-width: 56rem;
    margin: 0 auto;
    animation: fadeIn var(--transition-normal) ease-out;
}

/* Header */
.page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.5rem;
}

.header-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-text);
    letter-spacing: -0.01em;
}

.header-desc {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    margin-top: 0.25rem;
}

/* Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    font-weight: 500;
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    background-color: var(--color-bg-mute);
    color: var(--color-text);
    cursor: pointer;
    transition: all var(--transition-fast);
    white-space: nowrap;
}

.btn:hover {
    background-color: var(--color-bg-hover);
    border-color: var(--color-border);
}

.btn-sm {
    padding: 0.25rem 0.625rem;
    font-size: 0.75rem;
}

.btn-accent {
    background-color: var(--color-accent);
    color: white;
    border-color: transparent;
}

.btn-accent:hover {
    background-color: var(--color-accent-hover);
    border-color: transparent;
}

.btn-ghost {
    background-color: transparent;
    border-color: transparent;
    color: var(--color-text-secondary);
}

.btn-ghost:hover {
    background-color: var(--color-bg-mute);
    border-color: var(--color-border);
    color: var(--color-text);
}

.btn-danger-ghost {
    color: var(--color-danger);
}

.btn-danger-ghost:hover {
    background-color: var(--color-danger-soft);
    border-color: transparent;
    color: var(--color-danger);
}

/* Badge */
.badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.125rem 0.5rem;
    font-size: 0.6875rem;
    font-weight: 500;
    border-radius: var(--radius-sm);
    background-color: var(--color-bg-mute);
    color: var(--color-text-secondary);
}

.badge-accent {
    background-color: var(--color-accent-soft);
    color: var(--color-accent-hover);
}

/* Error */
.error-banner {
    margin-bottom: 1rem;
    padding: 0.75rem 1rem;
    background-color: var(--color-danger-soft);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    color: var(--color-danger);
}

/* Loading */
.loading-state {
    text-align: center;
    padding: 3rem 0;
    color: var(--color-text-muted);
    font-size: 0.875rem;
}

/* Empty State */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 5rem 0;
}

.empty-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 4rem;
    height: 4rem;
    border-radius: var(--radius-lg);
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    color: var(--color-text-muted);
    margin-bottom: 1rem;
}

.empty-title {
    font-size: 1.125rem;
    font-weight: 500;
    color: var(--color-text-secondary);
    margin-bottom: 0.375rem;
}

.empty-desc {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    margin-bottom: 1.5rem;
}

/* Card List */
.card-list {
    display: grid;
    gap: 0.75rem;
}

/* Provider Card */
.provider-card {
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
    transition:
        border-color var(--transition-fast),
        background-color var(--transition-fast);
}

.provider-card:hover {
    border-color: var(--color-border-hover);
}

.provider-card--active {
    background-color: var(--color-accent-soft);
    border-color: rgba(134, 59, 255, 0.25);
    border-left: 3px solid var(--color-accent);
}

.provider-card--active:hover {
    border-color: rgba(134, 59, 255, 0.4);
    border-left: 3px solid var(--color-accent);
}

.card-content {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
}

.card-info {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    min-width: 0;
}

/* Type dot indicator */
.card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border-radius: var(--radius-sm);
    background-color: var(--color-bg-mute);
    margin-top: 0.125rem;
    flex-shrink: 0;
}

.type-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: block;
}

.type-dot--openai {
    background-color: var(--color-success);
}

.type-dot--anthropic {
    background-color: var(--color-accent);
}

.type-dot--custom {
    background-color: var(--color-text-muted);
}

/* Card details */
.card-details {
    min-width: 0;
}

.card-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.card-title {
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--color-text);
}

.card-meta {
    font-size: 0.8125rem;
    color: var(--color-text-secondary);
    margin-top: 0.125rem;
}

.card-subtle {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    margin-top: 0.25rem;
}

/* Card actions */
.card-actions {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    flex-shrink: 0;
}

/* Responsive */
@media (max-width: 640px) {
    .page {
        padding: 1rem;
    }

    .card-content {
        flex-direction: column;
    }

    .card-actions {
        align-self: flex-end;
    }
}
</style>

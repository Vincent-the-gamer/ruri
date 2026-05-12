<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useProviderStore } from "../stores/provider";
import ProviderForm from "../components/ProviderForm.vue";
import type {
    Provider,
    ProviderType,
    ProviderConfig,
    CreateProviderRequest,
} from "../types";

const { t } = useI18n();
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
    if (!confirm(t("providers.deleteConfirm"))) return;
    try {
        await providerStore.deleteProvider(id);
    } catch {
        // error is in store
    }
}

const providerTypeLabel = (type: string) => {
    switch (type) {
        case "openai":
            return t("providers.type.openai");
        case "anthropic":
            return t("providers.type.anthropic");
        case "gemini":
            return t("providers.type.gemini");
        case "custom":
            return t("providers.type.custom");
        default:
            return type;
    }
};

function maskApiKey(key: string): string {
    if (!key) return t("providers.apiKeyNotSet");
    if (key.length <= 8) return "••••••••";
    return key.slice(0, 4) + "••••" + key.slice(-4);
}
</script>

<template>
    <div class="page">
        <!-- Header -->
        <div class="page-header">
            <div class="header-content">
                <div class="header-icon">
                    <svg
                        width="24"
                        height="24"
                        viewBox="0 0 24 24"
                        fill="none"
                        xmlns="http://www.w3.org/2000/svg"
                    >
                        <path
                            d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2z"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <path
                            d="M8 12h8M12 8v8"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                        />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">{{ t("providers.title") }}</h1>
                    <p class="header-desc">{{ t("providers.subtitle") }}</p>
                </div>
            </div>
            <button class="btn btn-accent" @click="openCreate">
                <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    xmlns="http://www.w3.org/2000/svg"
                >
                    <path
                        d="M12 5v14M5 12h14"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                    />
                </svg>
                {{ t("providers.addProvider") }}
            </button>
        </div>

        <!-- Error -->
        <div v-if="providerStore.error" class="error-banner">
            <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
            >
                <circle cx="12" cy="12" r="10" />
                <line x1="15" y1="9" x2="9" y2="15" />
                <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
            {{ providerStore.error }}
        </div>

        <!-- Loading -->
        <div
            v-if="providerStore.loading && providerStore.providers.length === 0"
            class="loading-state"
        >
            <div class="loading-spinner"></div>
            <span class="loading-text">{{ t("common.loading") }}</span>
        </div>

        <!-- Empty State -->
        <div
            v-else-if="providerStore.providers.length === 0"
            class="empty-state"
        >
            <div class="empty-illustration">
                <div class="empty-icon-wrapper">
                    <svg
                        width="48"
                        height="48"
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
                <div class="empty-decoration">
                    <span class="deco-dot deco-dot-1"></span>
                    <span class="deco-dot deco-dot-2"></span>
                    <span class="deco-dot deco-dot-3"></span>
                </div>
            </div>
            <h3 class="empty-title">{{ t("providers.noProviders") }}</h3>
            <p class="empty-desc">{{ t("providers.noProvidersDesc") }}</p>
            <button class="btn btn-accent" @click="openCreate">
                <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                >
                    <path d="M12 5v14M5 12h14" />
                </svg>
                {{ t("providers.addFirstProvider") }}
            </button>
        </div>

        <!-- Provider Cards -->
        <div v-else class="card-list">
            <div
                v-for="(provider, index) in providerStore.providers"
                :key="provider.id"
                class="provider-card"
                :style="{ animationDelay: `${index * 50}ms` }"
            >
                <div class="card-glow"></div>
                <div class="card-content">
                    <div class="card-info">
                        <div
                            class="card-icon"
                            :class="`card-icon--${provider.provider_type}`"
                        >
                            <span
                                class="type-dot"
                                :class="`type-dot--${provider.provider_type}`"
                            ></span>
                        </div>
                        <div class="card-details">
                            <div class="card-title-row">
                                <h3 class="card-title">{{ provider.name }}</h3>
                            </div>
                            <div class="card-meta">
                                {{ providerTypeLabel(provider.provider_type) }}
                                · {{ (provider.config as any).default_model }}
                            </div>
                            <div class="card-info-row">
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
                                        d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"
                                    />
                                    <path
                                        d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"
                                    />
                                </svg>
                                <span class="info-text">
                                    {{ (provider.config as any).base_url }}
                                </span>
                            </div>
                            <div class="card-info-row">
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
                                    <rect
                                        x="3"
                                        y="11"
                                        width="18"
                                        height="11"
                                        rx="2"
                                        ry="2"
                                    />
                                    <path d="M7 11V7a5 5 0 0 1 10 0v4" />
                                </svg>
                                <span class="info-text">
                                    {{ t("providers.apiKey")
                                    }}{{
                                        maskApiKey(
                                            (provider.config as any).api_key ||
                                                "",
                                        )
                                    }}
                                </span>
                            </div>
                        </div>
                    </div>

                    <div class="card-actions">
                        <button
                            class="btn btn-ghost btn-sm"
                            @click="openEdit(provider)"
                            :title="t('providers.editProvider')"
                        >
                            <svg
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
                                    d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"
                                />
                                <path
                                    d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"
                                />
                            </svg>
                            {{ t("providers.edit") }}
                        </button>
                        <button
                            class="btn btn-ghost btn-sm btn-danger-ghost"
                            @click="handleDelete(provider.id)"
                            :title="t('providers.deleteProvider')"
                        >
                            <svg
                                width="14"
                                height="14"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
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
/* ═══════════════════════════════════════════════════════════════
 * Providers Page - Enhanced frosted glass design with animations
 * ═══════════════════════════════════════════════════════════════ */
.page {
    padding: 1.5rem;
    max-width: 56rem;
    margin: 0 auto;
    animation: fadeIn var(--transition-normal) cubic-bezier(0.25, 0.1, 0.25, 1);
}

/* Header - Enhanced with icon and glass effect */
.page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.5rem;
    padding: 1.25rem 1.5rem;
    background: hsl(var(--card));
    border-radius: var(--radius-xl);
    border: 1px solid hsl(var(--border));
    box-shadow: var(--shadow-sm);
}

.header-content {
    display: flex;
    align-items: center;
    gap: 1rem;
}

.header-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 3rem;
    height: 3rem;
    border-radius: var(--radius);
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.2);
    flex-shrink: 0;
}

.header-title {
    font-size: 1.5rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    background: linear-gradient(
        135deg,
        var(--color-text) 0%,
        var(--color-accent) 100%
    );
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.header-desc {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    margin-top: 0.25rem;
}

/* Buttons - Clear and vibrant */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.625rem 1.25rem;
    font-size: 0.875rem;
    font-weight: 600;
    border-radius: 0.5rem;
    border: 2px solid hsl(var(--border));
    background-color: hsl(var(--secondary));
    color: hsl(var(--secondary-foreground));
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    white-space: nowrap;
    position: relative;
    overflow: hidden;
}

.btn::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.2), transparent);
    opacity: 0;
    transition: opacity 0.2s ease;
}

.btn:hover::before {
    opacity: 1;
}

.btn:hover {
    border-color: hsl(var(--primary) / 0.5);
    background-color: hsl(var(--accent));
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.btn-sm {
    padding: 0.375rem 0.875rem;
    font-size: 0.8125rem;
}

.btn-accent {
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(207 70% 55%));
    color: white;
    border: none;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.3);
}

.btn-accent:hover {
    background: linear-gradient(135deg, hsl(207 70% 55%), hsl(var(--primary)));
    box-shadow: 0 4px 16px hsl(var(--primary) / 0.4);
    transform: translateY(-2px);
}

.btn-ghost {
    background-color: transparent;
    color: hsl(var(--muted-foreground));
    border: 1px solid transparent;
}

.btn-ghost:hover {
    background-color: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
    border-color: hsl(var(--primary) / 0.3);
}

.btn-danger-ghost {
    color: hsl(var(--destructive));
    border: 1px solid transparent;
}

.btn-danger-ghost:hover {
    background-color: hsl(var(--destructive) / 0.1);
    border-color: hsl(var(--destructive) / 0.5);
    color: hsl(var(--destructive));
}

/* Status Badge with animated dot */
.status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.625rem;
    font-size: 0.6875rem;
    font-weight: 500;
    border-radius: 9999px;
    transition: all var(--transition-fast);
}

.status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    animation: pulse-dot 2s ease-in-out infinite;
}

.status-badge--active {
    background: linear-gradient(
        135deg,
        rgba(236, 72, 153, 0.1) 0%,
        rgba(139, 92, 246, 0.08) 100%
    );
    color: var(--color-accent);
    border: 1px solid rgba(236, 72, 153, 0.2);
}

.status-badge--active .status-dot {
    background: var(--color-accent);
    box-shadow: 0 0 8px rgba(236, 72, 153, 0.6);
}

/* Error Banner with icon */
.error-banner {
    margin-bottom: 1rem;
    padding: 0.875rem 1.25rem;
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    background: linear-gradient(
        135deg,
        rgba(239, 68, 68, 0.08) 0%,
        rgba(239, 68, 68, 0.05) 100%
    );
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    color: var(--color-danger);
    box-shadow: 0 2px 8px rgba(239, 68, 68, 0.08);
}

.error-banner svg {
    flex-shrink: 0;
    margin-top: 0.125rem;
}

/* Loading State with spinner */
.loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 4rem 0;
    color: var(--color-text-muted);
    font-size: 0.875rem;
}

.loading-spinner {
    width: 2rem;
    height: 2rem;
    border: 3px solid rgba(139, 92, 246, 0.15);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
}

.loading-text {
    color: var(--color-text-muted);
}

/* Empty State with decorations */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    background: hsl(var(--card));
    border-radius: var(--radius-xl);
    border: 1px dashed hsl(var(--primary) / 0.3);
    position: relative;
    overflow: hidden;
}

.empty-illustration {
    position: relative;
    margin-bottom: 1.5rem;
}

.empty-icon-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 5rem;
    height: 5rem;
    border-radius: var(--radius-lg);
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
    box-shadow: var(--shadow-md);
    color: hsl(var(--primary));
}

.empty-decoration {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
}

.deco-dot {
    position: absolute;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    animation: float-dot 3s ease-in-out infinite;
}

.deco-dot-1 {
    top: 0.5rem;
    right: 0.5rem;
    background: var(--color-accent);
    opacity: 0.4;
    animation-delay: 0s;
}

.deco-dot-2 {
    bottom: 0.5rem;
    left: 0.5rem;
    background: var(--color-primary);
    opacity: 0.3;
    animation-delay: 1s;
}

.deco-dot-3 {
    top: 50%;
    right: 0;
    background: var(--color-info);
    opacity: 0.35;
    animation-delay: 2s;
}

.empty-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--color-text-secondary);
    margin-bottom: 0.5rem;
}

.empty-desc {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    margin-bottom: 1.75rem;
    text-align: center;
    max-width: 28rem;
    line-height: 1.5;
}

/* Card List - Staggered animation */
.card-list {
    display: grid;
    gap: 1rem;
}

/* Provider Card - Enhanced with glow effect */
.provider-card {
    background: linear-gradient(
        135deg,
        hsl(var(--card) / 0.95) 0%,
        hsl(var(--card) / 0.9) 100%
    );
    backdrop-filter: blur(12px) saturate(150%);
    -webkit-backdrop-filter: blur(12px) saturate(150%);
    border: 1px solid hsl(var(--border) / 0.5);
    border-radius: var(--radius-xl);
    padding: 1.25rem;
    transition: all var(--transition-fast);
    box-shadow: var(--shadow-sm);
    position: relative;
    overflow: hidden;
    animation: slideUp 0.4s cubic-bezier(0.25, 0.1, 0.25, 1) both;
}

/* Card glow effect */
.card-glow {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    border-radius: var(--radius-xl);
    opacity: 0;
    transition: opacity var(--transition-fast);
    pointer-events: none;
    background:
        radial-gradient(
            ellipse at top left,
            rgba(139, 92, 246, 0.08) 0%,
            transparent 50%
        ),
        radial-gradient(
            ellipse at bottom right,
            rgba(236, 72, 153, 0.06) 0%,
            transparent 50%
        );
}

.card-glow--active {
    opacity: 1;
    background:
        radial-gradient(
            ellipse at top left,
            rgba(139, 92, 246, 0.12) 0%,
            transparent 50%
        ),
        radial-gradient(
            ellipse at bottom right,
            rgba(236, 72, 153, 0.1) 0%,
            transparent 50%
        );
}

.provider-card:hover {
    border-color: rgba(192, 132, 252, 0.4);
    transform: translateY(-3px);
    box-shadow:
        0 8px 24px rgba(139, 92, 246, 0.12),
        0 4px 12px rgba(236, 72, 153, 0.08);
}

.provider-card:hover .card-glow {
    opacity: 0.6;
}

.provider-card--active {
    background: hsl(var(--primary) / 0.05);
    border: 1px solid hsl(var(--primary) / 0.3);
    box-shadow:
        0 4px 16px hsl(var(--primary) / 0.1),
        var(--shadow-sm);
}

.provider-card--active:hover {
    border-color: rgba(236, 72, 153, 0.4);
    transform: translateY(-4px);
    box-shadow:
        0 12px 32px rgba(236, 72, 153, 0.15),
        0 6px 16px rgba(139, 92, 246, 0.12);
}

.card-content {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    position: relative;
    z-index: 1;
}

.card-info {
    display: flex;
    align-items: flex-start;
    gap: 0.875rem;
    min-width: 0;
    flex: 1;
}

/* Card icon container - with type-specific colors */
.card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: var(--radius-md);
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
    flex-shrink: 0;
    transition: all var(--transition-fast);
}

.card-icon--openai {
    background: linear-gradient(
        135deg,
        rgba(34, 197, 94, 0.1) 0%,
        rgba(22, 163, 74, 0.08) 100%
    );
    border-color: rgba(34, 197, 94, 0.25);
}

.card-icon--anthropic {
    background: linear-gradient(
        135deg,
        rgba(168, 85, 247, 0.1) 0%,
        rgba(139, 92, 246, 0.08) 100%
    );
    border-color: rgba(168, 85, 247, 0.25);
}

.card-icon--gemini {
    background: linear-gradient(135deg, #4285f4, #ea4335);
}

.card-icon--custom {
    background: linear-gradient(
        135deg,
        rgba(59, 130, 246, 0.1) 0%,
        rgba(99, 102, 241, 0.08) 100%
    );
    border-color: rgba(59, 130, 246, 0.25);
}

.type-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    display: block;
}

.type-dot--openai {
    background-color: var(--color-success);
    box-shadow: 0 0 8px rgba(34, 197, 94, 0.4);
}

.type-dot--anthropic {
    background-color: var(--color-accent);
    box-shadow: 0 0 8px rgba(168, 85, 247, 0.4);
}

.type-dot--gemini {
    background: #4285f4;
}

.type-dot--custom {
    background-color: var(--color-info);
    box-shadow: 0 0 8px rgba(59, 130, 246, 0.4);
}

/* Card details */
.card-details {
    min-width: 0;
    flex: 1;
}

.card-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
}

.card-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text);
    letter-spacing: -0.01em;
}

.card-meta {
    font-size: 0.8125rem;
    color: var(--color-text-secondary);
    margin-top: 0.25rem;
}

.card-info-row {
    display: flex;
    align-items: flex-start;
    gap: 0.375rem;
    margin-top: 0.375rem;
    font-size: 0.75rem;
    color: var(--color-text-muted);
}

.card-info-row svg {
    flex-shrink: 0;
    margin-top: 0.125rem;
    color: var(--color-text-secondary);
}

.info-text {
    color: var(--color-text-muted);
    word-break: break-all;
}

/* Card actions */
.card-actions {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    flex-shrink: 0;
}

/* Animations */
@keyframes slideUp {
    from {
        opacity: 0;
        transform: translateY(12px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

@keyframes pulse-dot {
    0%,
    100% {
        opacity: 1;
    }
    50% {
        opacity: 0.5;
    }
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

@keyframes float-dot {
    0%,
    100% {
        transform: translateY(0) scale(1);
    }
    50% {
        transform: translateY(-8px) scale(1.2);
    }
}

/* Responsive */
@media (max-width: 640px) {
    .page {
        padding: 1rem;
    }

    .page-header {
        flex-direction: column;
        gap: 1rem;
        align-items: stretch;
    }

    .header-content {
        flex-direction: row;
    }

    .card-content {
        flex-direction: column;
    }

    .card-actions {
        align-self: flex-end;
        width: 100%;
        justify-content: flex-end;
    }
}
</style>

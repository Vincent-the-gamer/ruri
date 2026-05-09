<script setup lang="ts">
import { onMounted, ref, reactive, computed } from "vue";
import { useI18n } from "vue-i18n";
import { usePlatformStore } from "../stores/platform";
import type {
    PlatformInstance,
    PlatformType,
    PlatformStatus,
    CreatePlatformRequest,
    UpdatePlatformRequest,
} from "../types";

const { t } = useI18n();
const platformStore = usePlatformStore();

const showForm = ref(false);
const editingInstance = ref<PlatformInstance | null>(null);

const formData = reactive({
    id: "",
    platform_type: "dingtalk" as PlatformType,
    client_id: "",
    client_secret: "",
    // Discord fields
    token: "",
    pre_response_reactions: false,
    reaction_emojis: "",
});

onMounted(() => {
    platformStore.fetchInstances();
});

function resetForm() {
    formData.id = "";
    formData.platform_type = "dingtalk";
    formData.client_id = "";
    formData.client_secret = "";
    formData.token = "";
    formData.pre_response_reactions = false;
    formData.reaction_emojis = "";
}

function openCreate() {
    editingInstance.value = null;
    resetForm();
    showForm.value = true;
}

function openEdit(instance: PlatformInstance) {
    editingInstance.value = instance;
    formData.id = instance.id;
    formData.platform_type = instance.platform_type;

    // Parse platform-specific config
    const config = instance.config as Record<string, unknown>;
    if (instance.platform_type === "dingtalk") {
        formData.client_id = (config.client_id as string) || "";
        formData.client_secret = (config.client_secret as string) || "";
    } else if (instance.platform_type === "discord") {
        formData.token = (config.token as string) || "";
        formData.pre_response_reactions =
            (config.pre_response_reactions as boolean) || false;
        const emojis = config.reaction_emojis as string[] | undefined;
        formData.reaction_emojis = emojis ? emojis.join(",") : "";
    }
    showForm.value = true;
}

function buildPlatformConfig(): Record<string, unknown> {
    if (formData.platform_type === "dingtalk") {
        return {
            client_id: formData.client_id,
            client_secret: formData.client_secret,
        };
    } else if (formData.platform_type === "discord") {
        const config: Record<string, unknown> = {
            token: formData.token,
        };
        if (formData.pre_response_reactions) {
            config.pre_response_reactions = true;
        }
        if (formData.reaction_emojis.trim()) {
            config.reaction_emojis = formData.reaction_emojis
                .split(",")
                .map((e: string) => e.trim())
                .filter(Boolean);
        }
        return config;
    }
    return {};
}

async function handleSave() {
    try {
        const platformConfig = buildPlatformConfig();
        if (editingInstance.value) {
            await platformStore.updateInstance(editingInstance.value.id, {
                type: formData.platform_type,
                ...platformConfig,
            } as UpdatePlatformRequest);
        } else {
            await platformStore.createInstance({
                id: formData.id,
                type: formData.platform_type,
                ...platformConfig,
            } as CreatePlatformRequest);
        }
        showForm.value = false;
        editingInstance.value = null;
    } catch {
        // error is in store
    }
}

function handleCancel() {
    showForm.value = false;
    editingInstance.value = null;
}

async function handleDelete(id: string) {
    if (!confirm(t("platformConfig.deleteConfirm"))) return;
    try {
        await platformStore.deleteInstance(id);
        await platformStore.fetchInstances();
    } catch {
        // error is in store
    }
}

function getPlatformLabel(type: PlatformType): string {
    switch (type) {
        case "dingtalk":
            return t("platformConfig.types.dingtalk");
        case "discord":
            return t("platformConfig.types.discord");
        default:
            return type;
    }
}

function getPlatformIcon(type: PlatformType): string {
    switch (type) {
        case "dingtalk":
            return "💬";
        case "discord":
            return "🎮";
        default:
            return "🔗";
    }
}

function getStatusClass(status: PlatformStatus): string {
    switch (status) {
        case "running":
            return "status-badge--running";
        case "stopped":
            return "status-badge--stopped";
        case "error":
            return "status-badge--error";
        case "pending":
            return "status-badge--pending";
        default:
            return "status-badge--stopped";
    }
}

function getStatusLabel(status: PlatformStatus): string {
    switch (status) {
        case "running":
            return "● Running";
        case "stopped":
            return "● Stopped";
        case "error":
            return "● Error";
        case "pending":
            return "● Pending";
        default:
            return status;
    }
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
                            d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <path
                            d="M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.4"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <circle
                            cx="12"
                            cy="12"
                            r="2"
                            stroke="currentColor"
                            stroke-width="2"
                        />
                        <path
                            d="M16.2 7.8c2.3 2.3 2.3 6.1 0 8.4"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <path
                            d="M19.1 4.9C23 8.8 23 15.2 19.1 19.1"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">
                        {{ t("platformConfig.title") }}
                    </h1>
                    <p class="header-desc">
                        {{ t("platformConfig.subtitle") }}
                    </p>
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
                {{ t("platformConfig.addPlatform") }}
            </button>
        </div>

        <!-- Info Banner -->
        <div class="info-banner">
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
                <line x1="12" y1="16" x2="12" y2="12" />
                <line x1="12" y1="8" x2="12.01" y2="8" />
            </svg>
            <span>{{ t("platformConfig.infoBanner") }}</span>
        </div>

        <!-- Error -->
        <div v-if="platformStore.error" class="error-banner">
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
            {{ platformStore.error }}
        </div>

        <!-- Loading -->
        <div
            v-if="platformStore.loading && platformStore.instances.length === 0"
            class="loading-state"
        >
            <div class="loading-spinner"></div>
            <span class="loading-text">{{ t("common.loading") }}</span>
        </div>

        <!-- Empty State -->
        <div
            v-else-if="platformStore.instances.length === 0"
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
                            d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <path
                            d="M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.4"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <circle
                            cx="12"
                            cy="12"
                            r="2"
                            stroke="currentColor"
                            stroke-width="1.5"
                        />
                        <path
                            d="M16.2 7.8c2.3 2.3 2.3 6.1 0 8.4"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <path
                            d="M19.1 4.9C23 8.8 23 15.2 19.1 19.1"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                    </svg>
                </div>
                <div class="empty-decoration">
                    <span class="deco-dot deco-dot-1"></span>
                    <span class="deco-dot deco-dot-2"></span>
                    <span class="deco-dot deco-dot-3"></span>
                </div>
            </div>
            <h3 class="empty-title">{{ t("platformConfig.noPlatforms") }}</h3>
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
                {{ t("platformConfig.addFirstPlatform") }}
            </button>
        </div>

        <!-- Platform Cards -->
        <div v-else class="card-list">
            <div
                v-for="(instance, index) in platformStore.instances"
                :key="instance.id"
                class="platform-card"
                :style="{ animationDelay: `${index * 50}ms` }"
            >
                <div class="card-glow"></div>
                <div class="card-content">
                    <div class="card-info">
                        <div class="card-icon">
                            <span class="icon-emoji">{{
                                getPlatformIcon(instance.platform_type)
                            }}</span>
                        </div>
                        <div class="card-details">
                            <div class="card-title-row">
                                <h3 class="card-title">{{ instance.id }}</h3>
                                <span class="transport-badge">{{
                                    getPlatformLabel(instance.platform_type)
                                }}</span>
                                <span
                                    :class="[
                                        'status-badge',
                                        getStatusClass(instance.status),
                                    ]"
                                >
                                    <span class="status-dot"></span>
                                    {{ instance.status }}
                                </span>
                            </div>
                            <div class="card-summary">
                                <span class="summary-label">
                                    {{
                                        instance.platform_type === "discord"
                                            ? "🤖"
                                            : "🔑"
                                    }}
                                </span>
                                <span class="summary-text">
                                    <template
                                        v-if="
                                            instance.platform_type ===
                                            'dingtalk'
                                        "
                                    >
                                        Client ID:
                                        {{
                                            (instance.config as any).client_id
                                                ? (
                                                      instance.config as any
                                                  ).client_id
                                                      .slice(0, 8)
                                                      .padEnd(
                                                          (
                                                              instance.config as any
                                                          ).client_id.length,
                                                          "•",
                                                      )
                                                : "—"
                                        }}
                                    </template>
                                    <template
                                        v-else-if="
                                            instance.platform_type === 'discord'
                                        "
                                    >
                                        Token:
                                        {{
                                            (instance.config as any).token
                                                ? (instance.config as any).token
                                                      .slice(0, 8)
                                                      .padEnd(
                                                          (
                                                              instance.config as any
                                                          ).token.length,
                                                          "•",
                                                      )
                                                : "—"
                                        }}
                                    </template>
                                    <template v-else>—</template>
                                </span>
                            </div>
                        </div>
                    </div>

                    <div class="card-actions">
                        <button
                            class="btn btn-ghost btn-sm"
                            @click="openEdit(instance)"
                            :title="t('platformConfig.edit')"
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
                            {{ t("platformConfig.edit") }}
                        </button>
                        <button
                            class="btn btn-ghost btn-sm btn-danger-ghost"
                            @click="handleDelete(instance.id)"
                            :title="t('platformConfig.delete')"
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
        <Teleport to="body">
            <div v-if="showForm" class="persona-modal-overlay">
                <div class="persona-modal-content glass" @click.stop>
                    <div class="modal-header">
                        <h2 class="modal-title">
                            {{
                                editingInstance
                                    ? t("platformConfig.editPlatform")
                                    : t("platformConfig.createPlatform")
                            }}
                        </h2>
                        <button class="modal-close" @click="handleCancel">
                            <svg
                                width="18"
                                height="18"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                            >
                                <path d="M18 6L6 18M6 6l12 12" />
                            </svg>
                        </button>
                    </div>
                    <div class="modal-body">
                        <!-- Instance ID -->
                        <div class="form-group">
                            <label class="form-label">{{
                                t("platformConfig.instanceId")
                            }}</label>
                            <input
                                v-model="formData.id"
                                type="text"
                                class="form-input"
                                :placeholder="
                                    t('platformConfig.instanceIdPlaceholder')
                                "
                                :disabled="!!editingInstance"
                            />
                            <span class="form-hint">{{
                                t("platformConfig.instanceIdHint")
                            }}</span>
                        </div>

                        <!-- Platform Type -->
                        <div class="form-group">
                            <label class="form-label">{{
                                t("platformConfig.platformType")
                            }}</label>
                            <select
                                v-model="formData.platform_type"
                                class="form-input"
                            >
                                <option value="dingtalk">
                                    {{ t("platformConfig.types.dingtalk") }}
                                </option>
                                <option value="discord">
                                    {{ t("platformConfig.types.discord") }}
                                </option>
                            </select>
                        </div>

                        <!-- DingTalk Config -->
                        <template v-if="formData.platform_type === 'dingtalk'">
                            <div class="form-section-title">
                                {{ t("platformConfig.dingtalkConfig") }}
                            </div>
                            <div class="form-group">
                                <label class="form-label">{{
                                    t("platformConfig.clientId")
                                }}</label>
                                <input
                                    v-model="formData.client_id"
                                    type="text"
                                    class="form-input"
                                    :placeholder="
                                        t('platformConfig.clientIdPlaceholder')
                                    "
                                />
                            </div>
                            <div class="form-group">
                                <label class="form-label">{{
                                    t("platformConfig.clientSecret")
                                }}</label>
                                <input
                                    v-model="formData.client_secret"
                                    type="password"
                                    class="form-input"
                                    :placeholder="
                                        t(
                                            'platformConfig.clientSecretPlaceholder',
                                        )
                                    "
                                />
                            </div>
                        </template>

                        <!-- Discord Config -->
                        <template v-if="formData.platform_type === 'discord'">
                            <div class="form-section-title">
                                {{ t("platformConfig.discordConfig") }}
                            </div>
                            <div class="form-group">
                                <label class="form-label">{{
                                    t("platformConfig.token")
                                }}</label>
                                <input
                                    v-model="formData.token"
                                    type="password"
                                    class="form-input"
                                    :placeholder="
                                        t('platformConfig.tokenPlaceholder')
                                    "
                                />
                                <span class="form-hint">{{
                                    t("platformConfig.tokenHint")
                                }}</span>
                            </div>

                            <div class="form-group">
                                <label
                                    class="form-label flex items-center gap-2"
                                >
                                    <input
                                        v-model="
                                            formData.pre_response_reactions
                                        "
                                        type="checkbox"
                                        class="form-checkbox"
                                    />
                                    {{
                                        t("platformConfig.preResponseReactions")
                                    }}
                                </label>
                                <span class="form-hint">{{
                                    t("platformConfig.preResponseReactionsHint")
                                }}</span>
                            </div>
                            <div
                                v-if="formData.pre_response_reactions"
                                class="form-group"
                            >
                                <label class="form-label">{{
                                    t("platformConfig.reactionEmojis")
                                }}</label>
                                <input
                                    v-model="formData.reaction_emojis"
                                    type="text"
                                    class="form-input"
                                    :placeholder="
                                        t(
                                            'platformConfig.reactionEmojisPlaceholder',
                                        )
                                    "
                                />
                                <span class="form-hint">{{
                                    t("platformConfig.reactionEmojisHint")
                                }}</span>
                            </div>
                        </template>

                        <!-- Enable toggle -->
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-ghost" @click="handleCancel">
                            {{ t("platformConfig.cancel") }}
                        </button>
                        <button
                            class="btn btn-accent"
                            @click="handleSave"
                            :disabled="
                                !formData.id.trim() ||
                                (formData.platform_type === 'dingtalk' &&
                                    (!formData.client_id.trim() ||
                                        !formData.client_secret.trim())) ||
                                (formData.platform_type === 'discord' &&
                                    !formData.token.trim())
                            "
                        >
                            {{ t("platformConfig.save") }}
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
    max-width: 960px;
    margin: 0 auto;
    animation: fadeIn 0.4s ease-out;
}

@keyframes fadeIn {
    from {
        opacity: 0;
    }
    to {
        opacity: 1;
    }
}

/* Page Header */
.page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
    gap: 1rem;
}

.header-content {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.header-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 0.75rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.2) 0%,
        hsl(var(--primary) / 0.1) 100%
    );
    color: hsl(var(--primary));
    border: 1px solid hsl(var(--primary) / 0.2);
}

.header-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: hsl(var(--foreground));
    margin: 0;
    line-height: 1.2;
}

.header-desc {
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
    margin: 0;
}

/* Info Banner */
.info-banner {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: hsl(var(--primary) / 0.06);
    border: 1px solid hsl(var(--primary) / 0.15);
    border-radius: 0.5rem;
    color: hsl(var(--muted-foreground));
    font-size: 0.8rem;
    margin-bottom: 1rem;
    line-height: 1.5;
}

.info-banner svg {
    flex-shrink: 0;
    margin-top: 1px;
    color: hsl(var(--primary));
}

/* Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: 0.5rem;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    border: 1px solid transparent;
}

.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-accent {
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(var(--primary) / 0.9) 100%
    );
    color: hsl(var(--primary-foreground));
    border-color: hsl(var(--primary) / 0.3);
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.2);
}

.btn-accent:hover:not(:disabled) {
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.3);
    transform: translateY(-1px);
}

.btn-ghost {
    background: transparent;
    color: hsl(var(--muted-foreground));
    border-color: transparent;
}

.btn-ghost:hover {
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--foreground));
}

.btn-danger-ghost:hover {
    background: hsl(var(--destructive) / 0.1);
    color: hsl(var(--destructive));
}

.btn-sm {
    padding: 0.35rem 0.65rem;
    font-size: 0.8rem;
}

/* Transport Badge */
.transport-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.15rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--muted-foreground));
    border: 1px solid hsl(var(--border) / 0.3);
}

/* Status Badge */
.status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.6rem;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 600;
}

.status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
}

.status-badge--active {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
    border: 1px solid hsl(var(--primary) / 0.2);
}

.status-badge--active .status-dot {
    background: hsl(var(--primary));
    box-shadow: 0 0 6px hsl(var(--primary) / 0.5);
}

.status-badge--inactive {
    background: hsl(var(--muted) / 0.3);
    color: hsl(var(--muted-foreground));
    border: 1px solid hsl(var(--border) / 0.3);
}

.status-badge--inactive .status-dot {
    background: hsl(var(--muted-foreground));
}

.status-badge--running {
    background: hsl(142 / 0.1);
    color: hsl(142 / 0.8);
    border: 1px solid hsl(142 / 0.2);
}

.status-badge--running .status-dot {
    background: hsl(142 / 0.8);
    box-shadow: 0 0 6px hsl(142 / 0.5);
}

.status-badge--stopped {
    background: hsl(var(--muted) / 0.3);
    color: hsl(var(--muted-foreground));
    border: 1px solid hsl(var(--border) / 0.3);
}

.status-badge--stopped .status-dot {
    background: hsl(var(--muted-foreground));
}

.status-badge--error {
    background: hsl(var(--destructive) / 0.1);
    color: hsl(var(--destructive));
    border: 1px solid hsl(var(--destructive) / 0.2);
}

.status-badge--error .status-dot {
    background: hsl(var(--destructive));
    box-shadow: 0 0 6px hsl(var(--destructive) / 0.5);
}

.status-badge--pending {
    background: hsl(45 / 0.1);
    color: hsl(45 / 0.8);
    border: 1px solid hsl(45 / 0.2);
}

.status-badge--pending .status-dot {
    background: hsl(45 / 0.8);
    box-shadow: 0 0 6px hsl(45 / 0.5);
}

/* Error Banner */
.error-banner {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: hsl(var(--destructive) / 0.1);
    border: 1px solid hsl(var(--destructive) / 0.2);
    border-radius: 0.5rem;
    color: hsl(var(--destructive));
    font-size: 0.875rem;
    margin-bottom: 1rem;
}

/* Loading State */
.loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 3rem;
    color: hsl(var(--muted-foreground));
}

.loading-spinner {
    width: 2rem;
    height: 2rem;
    border: 3px solid hsl(var(--primary) / 0.2);
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
    font-size: 0.875rem;
}

/* Empty State */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 3rem 1rem;
    text-align: center;
}

.empty-illustration {
    position: relative;
    margin-bottom: 0.5rem;
}

.empty-icon-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 5rem;
    height: 5rem;
    border-radius: 1rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.15) 0%,
        hsl(var(--primary) / 0.05) 100%
    );
    color: hsl(var(--primary));
    border: 1px solid hsl(var(--primary) / 0.15);
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
    background: hsl(var(--primary) / 0.4);
    animation: float 3s ease-in-out infinite;
}

.deco-dot-1 {
    top: 10%;
    right: 15%;
    animation-delay: 0s;
}

.deco-dot-2 {
    bottom: 15%;
    left: 10%;
    animation-delay: 1s;
}

.deco-dot-3 {
    top: 50%;
    right: 5%;
    animation-delay: 2s;
}

@keyframes float {
    0%,
    100% {
        transform: translateY(0);
        opacity: 0.6;
    }
    50% {
        transform: translateY(-8px);
        opacity: 1;
    }
}

.empty-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

/* Card List */
.card-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}

.platform-card {
    position: relative;
    border-radius: 0.75rem;
    overflow: hidden;
    animation: slideUp 0.4s ease-out both;
}

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

.card-glow {
    position: absolute;
    inset: 0;
    border-radius: 0.75rem;
    padding: 1px;
    background: linear-gradient(
        135deg,
        hsl(var(--border) / 0.3) 0%,
        transparent 50%,
        hsl(var(--border) / 0.2) 100%
    );
    -webkit-mask:
        linear-gradient(#fff 0 0) content-box,
        linear-gradient(#fff 0 0);
    mask:
        linear-gradient(#fff 0 0) content-box,
        linear-gradient(#fff 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
}

.card-glow--active {
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.4) 0%,
        transparent 50%,
        hsl(var(--primary) / 0.3) 100%
    );
}

.platform-card:hover {
    transform: translateY(-2px);
    transition: transform 0.2s ease;
}

.platform-card--enabled {
    box-shadow: 0 4px 16px hsl(var(--primary) / 0.1);
}

.card-content {
    position: relative;
    background: linear-gradient(
        180deg,
        hsl(var(--card) / 0.95) 0%,
        hsl(var(--card) / 0.85) 100%
    );
    backdrop-filter: blur(12px);
    padding: 1rem;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
}

.card-info {
    display: flex;
    gap: 0.75rem;
    flex: 1;
    min-width: 0;
}

.card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 0.5rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.15) 0%,
        hsl(var(--primary) / 0.08) 100%
    );
    flex-shrink: 0;
}

.icon-emoji {
    font-size: 1.25rem;
}

.card-details {
    flex: 1;
    min-width: 0;
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
    color: hsl(var(--foreground));
    margin: 0;
}

.card-summary {
    display: flex;
    align-items: flex-start;
    gap: 0.35rem;
    margin-top: 0.5rem;
    padding: 0.4rem 0.6rem;
    background: hsl(var(--muted) / 0.2);
    border-radius: 0.35rem;
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
}

.summary-label {
    flex-shrink: 0;
}

.summary-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* Card Actions */
.card-actions {
    display: flex;
    gap: 0.35rem;
    flex-shrink: 0;
}

/* Modal */
.persona-modal-overlay {
    position: fixed;
    inset: 0;
    background: hsl(var(--background) / 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    padding: 1rem;
    animation: fadeIn 0.2s ease-out;
}

.persona-modal-overlay .persona-modal-content {
    width: 100%;
    max-width: 520px;
    max-height: 90vh;
    border-radius: 1rem;
    background: hsl(var(--background));
    border: 1px solid hsl(var(--border));
    box-shadow: 0 25px 50px -12px hsl(var(--foreground) / 0.25);
    overflow: hidden;
    display: flex;
    flex-direction: column;
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

.modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid hsl(var(--border) / 0.2);
}

.modal-title {
    font-size: 1.1rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

.modal-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.75rem;
    height: 1.75rem;
    border-radius: 0.375rem;
    background: transparent;
    border: none;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s ease;
}

.modal-close:hover {
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--foreground));
}

.modal-body {
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    overflow-y: auto;
}

.form-group {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
}

.form-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
}

.form-hint {
    font-size: 0.7rem;
    color: hsl(var(--muted-foreground) / 0.7);
}

.form-input,
.form-textarea {
    padding: 0.6rem 0.75rem;
    border-radius: 0.5rem;
    border: 1px solid hsl(var(--border) / 0.4);
    background: hsl(var(--background) / 0.5);
    color: hsl(var(--foreground));
    font-size: 0.875rem;
    outline: none;
    transition: all 0.2s ease;
}

.form-input:focus,
.form-textarea:focus {
    border-color: hsl(var(--primary) / 0.5);
    box-shadow: 0 0 0 2px hsl(var(--primary) / 0.1);
}

.form-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

select.form-input {
    appearance: none;
    background-image: url("data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 20 20'%3e%3cpath stroke='%236b7280' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.5' d='M6 8l4 4 4-4'/%3e%3c/svg%3e");
    background-position: right 0.5rem center;
    background-repeat: no-repeat;
    background-size: 1.5em 1.5em;
    padding-right: 2.5rem;
}

.form-checkbox {
    width: 1rem;
    height: 1rem;
    accent-color: hsl(var(--primary));
    cursor: pointer;
}

.form-section-title {
    font-size: 0.85rem;
    font-weight: 700;
    color: hsl(var(--foreground));
    padding-bottom: 0.35rem;
    border-bottom: 1px solid hsl(var(--border) / 0.2);
}

.modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
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
    }

    .card-content {
        flex-direction: column;
    }

    .card-actions {
        width: 100%;
        justify-content: flex-end;
    }
}
</style>

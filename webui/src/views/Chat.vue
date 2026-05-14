<script setup lang="ts">
defineOptions({ name: "Chat" });
import { onMounted, onActivated, ref, nextTick, computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useChatStore } from "../stores/chat";
import { useProviderStore } from "../stores/provider";
import { useConfigStore } from "../stores/config";
import { useAuthStore } from "../stores/auth";
import { useDebugSessionStore } from "../stores/debugSession";
import { useKnowledgeBaseStore } from "../stores/knowledgeBase";
import type { AttachedFile } from "../types";
import ChatMessageComp from "../components/ChatMessage.vue";
import ChatInput from "../components/ChatInput.vue";
import ruriAvatar from "../../assets/ruri-avatar.png";
import ChatConfigModal from "../components/ChatConfigModal.vue";

const { t } = useI18n();
const chatStore = useChatStore();
const providerStore = useProviderStore();
const configStore = useConfigStore();
const authStore = useAuthStore();
const debugSessionStore = useDebugSessionStore();
const kbStore = useKnowledgeBaseStore();

const messagesContainer = ref<HTMLElement | null>(null);
const showConfigModal = ref(false);
const chatConfigModal = ref<InstanceType<typeof ChatConfigModal> | null>(null);
const temperature = ref(0.7);
const maxTokens = ref(4096);

const effectivePersona = computed(() => {
    // Use the persona form if set, then debug session's embedded persona, then active config profile's
    return (
        chatConfigModal.value?.personaForm ??
        debugSessionStore.embeddedPersona ??
        configStore.activeEmbeddedPersona
    );
});

const effectiveProvider = computed(() => {
    // Only use explicit selection from debug session — no automatic fallback
    const id =
        chatConfigModal.value?.selectedProviderId ??
        debugSessionStore.providerId;
    if (id) {
        return providerStore.providers.find((p) => p.id === id) || null;
    }
    return null;
});

const hasAnyProvider = computed(() => providerStore.providers.length > 0);

const isConfigEnabled = computed(
    () => configStore.activeConfigProfile?.enable ?? false,
);

onMounted(async () => {
    // fetchHistory is cache-first, so it restores from localStorage instantly
    // then syncs with DB in the background — no need to await it
    chatStore.fetchHistory();
    await Promise.all([
        debugSessionStore.fetchDebugSession(),
        providerStore.fetchProviders(),
        configStore.fetchConfigProfiles(),
        kbStore.fetchKnowledgeBases(),
    ]);
    scrollToBottom();
});

// When activated from keep-alive cache, gently sync with database
// (does NOT show loading state, does NOT replace if streaming)
onActivated(async () => {
    chatStore.syncWithDatabase();
    await kbStore.fetchKnowledgeBases();
    scrollToBottom();
});

const messages = computed(() =>
    chatStore.messages.filter((m) => m.role !== "system"),
);

function scrollToBottom() {
    nextTick(() => {
        if (messagesContainer.value) {
            messagesContainer.value.scrollTop =
                messagesContainer.value.scrollHeight;
        }
    });
}

// Auto-scroll when streaming content changes
watch(
    () => chatStore.streamingContent,
    () => {
        scrollToBottom();
    },
);

async function handleSend(
    message: string,
    images: string[] = [],
    files: AttachedFile[] = [],
) {
    const effectiveTemp =
        chatConfigModal.value?.temperature ?? temperature.value;
    const effectiveMaxTokens =
        chatConfigModal.value?.maxTokens ?? maxTokens.value;
    // Resolve provider_id: only from explicit user selection or debug session
    // No automatic fallback to profile or global active provider — user must explicitly choose
    const effectiveProviderId =
        chatConfigModal.value?.selectedProviderId ??
        debugSessionStore.providerId ??
        undefined;

    if (!effectiveProviderId) {
        // No provider selected — show error message in chat
        messages.value.push({
            role: "user",
            content: message,
        });
        messages.value.push({
            role: "assistant",
            content: t(
                "chat.noProviderSelected",
                "⚠️ No provider selected. Please select a model provider in chat config (⚙️ icon).",
            ),
        });
        scrollToBottom();
        return;
    }

    try {
        await chatStore.sendMessage({
            message,
            images: images.length > 0 ? images : undefined,
            files: files.length > 0 ? files : undefined,
            provider_id: effectiveProviderId,
            temperature: effectiveTemp,
            max_tokens: effectiveMaxTokens,
            knowledge_base_ids: chatConfigModal.value?.selectedKbIds?.length
                ? chatConfigModal.value.selectedKbIds
                : debugSessionStore.knowledgeBaseIds.length
                  ? debugSessionStore.knowledgeBaseIds
                  : undefined,
            custom_error_message:
                chatConfigModal.value?.customErrorMessage ||
                debugSessionStore.customErrorMessage ||
                undefined,
            user_id: authStore.user?.id || undefined,
        });
    } catch {
        // Error already added to chat messages by the store; no need to re-throw
    }
    scrollToBottom();
}

async function handleClear() {
    if (!confirm(t("chat.confirmClear"))) return;
    await chatStore.clearHistory();
}

function handleStop() {
    chatStore.stopGeneration();
}
</script>

<template>
    <div class="chat-view" :class="{ 'no-animation': !isConfigEnabled }">
        <!-- Header - 可爱风格 -->
        <header class="chat-header glass">
            <div class="header-left">
                <div class="header-icon" :class="{ bounce: isConfigEnabled }">
                    <img :src="ruriAvatar" alt="Ruri" class="header-icon-img" />
                </div>
                <div>
                    <h1 class="header-title font-cute">
                        <span>💎</span>
                        <span>{{ t("chat.title") }}</span>
                        <span>✨</span>
                    </h1>
                    <span
                        v-if="chatStore.isThinking"
                        class="thinking-indicator"
                    >
                        <svg
                            class="spinner-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                        >
                            <circle
                                class="spinner-track"
                                cx="12"
                                cy="12"
                                r="10"
                                stroke="currentColor"
                                stroke-width="2.5"
                                stroke-dasharray="31.4 31.4"
                                stroke-linecap="round"
                            />
                        </svg>
                        <span>{{ t("chat.thinking") }}</span>
                    </span>
                    <span v-else class="thinking-indicator ready">
                        <span>✨ {{ t("chat.ready") }}</span>
                    </span>
                </div>
            </div>

            <div class="header-right">
                <div v-if="effectivePersona" class="model-badge persona-badge">
                    <span class="badge-icon">🎭</span>
                    <span>{{ effectivePersona.name }}</span>
                </div>
                <div v-if="effectiveProvider" class="model-badge">
                    <span class="badge-icon">🤖</span>
                    <span>{{ effectiveProvider.name }}</span>
                    <span class="badge-divider">·</span>
                    <span>{{
                        (effectiveProvider.config as any)?.default_model
                    }}</span>
                </div>
                <div
                    v-if="debugSessionStore.knowledgeBaseIds.length"
                    class="model-badge kb-badge"
                >
                    <span class="badge-icon">📚</span>
                    <span>{{
                        t("chat.kbBadge", {
                            count: debugSessionStore.knowledgeBaseIds.length,
                        })
                    }}</span>
                </div>
                <div
                    class="header-actions"
                    :class="{ 'has-badge': !!effectiveProvider }"
                >
                    <button
                        class="icon-btn"
                        :title="t('chat.settings') + ' ⚙️'"
                        @click="showConfigModal = true"
                    >
                        <span class="btn-icon">⚙️</span>
                    </button>
                    <button
                        class="icon-btn danger"
                        @click="handleClear"
                        :title="t('chat.clearHistory') + ' 🗑️'"
                    >
                        <span class="btn-icon">🗑️</span>
                    </button>
                </div>
            </div>
        </header>

        <!-- No Provider Warning - 友好提示 -->
        <div v-if="!hasAnyProvider && !chatStore.loading" class="warning-bar">
            <span class="warning-emoji">💡</span>
            <span>{{ t("chat.noProvider") }}</span>
            <router-link to="/providers" class="warning-link">
                <span>{{ t("chat.goToConfig") }} 💖</span>
            </router-link>
        </div>

        <!-- Messages Area -->
        <div ref="messagesContainer" class="messages-area">
            <div class="messages-inner">
                <!-- Empty State - 可爱的空状态 -->
                <div v-if="messages.length === 0" class="empty-state">
                    <div class="empty-icon-wrapper float">
                        <span class="empty-icon">💌</span>
                        <span class="decoration-1">✨</span>
                        <span class="decoration-2">💫</span>
                    </div>
                    <h2 class="empty-title font-cute">
                        <span>💎</span>
                        <span>{{ t("chat.emptyTitle") }}</span>
                        <span>✨</span>
                    </h2>
                    <p class="empty-desc">
                        {{ t("chat.emptyDesc") }}
                    </p>
                    <router-link
                        v-if="!hasAnyProvider"
                        to="/providers"
                        class="cta-button"
                    >
                        <span>💖</span>
                        <span>{{ t("chat.configureProvider") }}</span>
                        <span>🚀</span>
                    </router-link>
                </div>

                <!-- Chat Messages -->
                <ChatMessageComp
                    v-for="(msg, i) in messages"
                    :key="i"
                    :message="msg"
                />

                <!-- Thinking Indicator Message -->
                <Transition name="thinking-fade">
                    <div
                        v-if="chatStore.isThinking && !chatStore.isStreaming"
                        class="thinking-message"
                    >
                        <div class="thinking-avatar">
                            <div class="thinking-ring"></div>
                            <img
                                :src="ruriAvatar"
                                alt="琉璃"
                                class="thinking-avatar-img"
                            />
                        </div>
                        <div class="thinking-content-wrapper">
                            <div class="thinking-label">
                                <svg
                                    class="thinking-icon"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                >
                                    <circle
                                        cx="12"
                                        cy="12"
                                        r="10"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-dasharray="31.4 31.4"
                                        stroke-linecap="round"
                                    />
                                </svg>
                                <span>琉璃</span>
                                <span class="thinking-status">思考中...</span>
                            </div>
                            <div class="thinking-content">
                                <div class="thinking-animation">
                                    <span class="wave-dot wave-dot-1"></span>
                                    <span class="wave-dot wave-dot-2"></span>
                                    <span class="wave-dot wave-dot-3"></span>
                                </div>
                            </div>
                        </div>
                    </div>
                </Transition>
            </div>
        </div>

        <!-- Input Area -->
        <div class="input-area">
            <ChatInput
                @send="handleSend"
                @stop="handleStop"
                :disabled="chatStore.isThinking"
                :sending="chatStore.isStreaming"
            />
        </div>

        <!-- Chat Config Modal -->
        <Teleport to="body">
            <ChatConfigModal v-model="showConfigModal" ref="chatConfigModal" />
        </Teleport>
    </div>
</template>

<style scoped>
.chat-view {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: transparent;
    position: relative;
}

/* ── Header ─────────────────────────────────────── */

.chat-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    min-height: 64px;
    border-bottom: 2px solid rgba(249, 168, 212, 0.3);
    position: relative;
    z-index: 10;
}

.chat-header::after {
    content: "";
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--color-accent), #a855f7, #818cf8);
    opacity: 0.3;
}

.header-left {
    display: flex;
    align-items: center;
    gap: 1rem;
}

.header-icon {
    font-size: 1.75rem;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    overflow: hidden;
    border: 2px solid hsl(var(--primary) / 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
}

.header-icon-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.header-title {
    font-size: 1.125rem;
    font-weight: 700;
    color: var(--color-text);
    display: flex;
    align-items: center;
    gap: 0.375rem;
}

.header-title span {
    display: inline-flex;
    align-items: center;
}

.thinking-indicator {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--color-accent);
    font-weight: 600;
    margin-top: 0.25rem;
    transition:
        color 0.3s ease,
        opacity 0.3s ease;
}

.thinking-indicator.ready {
    color: var(--color-success);
}

.spinner-icon {
    width: 14px;
    height: 14px;
    animation: ruriSpin 1s linear infinite;
    transform-origin: center;
}

.spinner-track {
    stroke: var(--color-accent);
}

@keyframes ruriSpin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

.header-right {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.model-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.6875rem;
    color: hsl(var(--muted-foreground));
    padding: 0.375rem 0.75rem;
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
    border-radius: var(--radius-full);
    font-weight: 600;
    white-space: nowrap;
}

.badge-icon {
    font-size: 1rem;
}

.badge-divider {
    opacity: 0.5;
}

.header-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border: 2px solid transparent;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    font-size: 1.125rem;
}

.icon-btn:hover {
    background: hsl(var(--secondary));
    border-color: hsl(var(--border));
    color: hsl(var(--foreground));
    transform: scale(1.1);
}

.icon-btn.active {
    background: linear-gradient(135deg, var(--color-accent) 0%, #a855f7 100%);
    border-color: var(--color-accent);
    color: white;
    box-shadow: 0 4px 12px rgba(236, 72, 153, 0.3);
}

.icon-btn.danger:hover {
    background: linear-gradient(
        135deg,
        rgba(252, 165, 165, 0.2) 0%,
        rgba(248, 113, 113, 0.2) 100%
    );
    border-color: var(--color-danger);
    color: var(--color-danger);
}

/* ── Warning Bar ─────────────────────────────────── */

.warning-bar {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.75rem 1.25rem;
    background: linear-gradient(
        135deg,
        rgba(252, 211, 77, 0.15) 0%,
        rgba(251, 191, 36, 0.15) 100%
    );
    border-top: 2px solid rgba(251, 191, 36, 0.3);
    border-bottom: 2px solid rgba(251, 191, 36, 0.3);
    font-size: 0.8125rem;
    color: #92400e;
    font-weight: 600;
}

.warning-emoji {
    font-size: 1.125rem;
}

.warning-link {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    color: #d97706;
    text-decoration: none;
    padding: 0.25rem 0.5rem;
    border-radius: var(--radius-sm);
    background: rgba(251, 191, 36, 0.2);
    transition: all 0.3s ease;
}

.warning-link:hover {
    background: rgba(251, 191, 36, 0.3);
    transform: translateY(-1px);
    box-shadow: 0 2px 6px rgba(251, 191, 36, 0.2);
}

/* ── Messages Area ───────────────────────────────── */

.messages-area {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem 1.25rem;
    position: relative;
}

.messages-inner {
    max-width: 56rem;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
}

/* ── Empty State ─────────────────────────────────── */

.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 60vh;
    text-align: center;
    padding: 2rem;
}

.empty-icon-wrapper {
    position: relative;
    margin-bottom: 1.5rem;
}

.empty-icon {
    font-size: 4rem;
    display: inline-block;
    filter: drop-shadow(0 4px 8px rgba(168, 85, 247, 0.2));
}

.decoration-1,
.decoration-2 {
    position: absolute;
    font-size: 1.5rem;
    animation: float 3s ease-in-out infinite;
}

.decoration-1 {
    top: 0;
    right: -1rem;
    animation-delay: 0s;
}

.decoration-2 {
    bottom: 0;
    left: -1rem;
    animation-delay: 1s;
}

.empty-title {
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--color-text);
    margin-bottom: 0.75rem;
    letter-spacing: 0.02em;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
}

.empty-title span {
    display: inline-flex;
    align-items: center;
}

.empty-desc {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    max-width: 24rem;
    line-height: 1.75;
    margin-bottom: 1.5rem;
}

.cta-button {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.5rem;
    background: linear-gradient(135deg, var(--color-accent) 0%, #a855f7 100%);
    color: white;
    text-decoration: none;
    border-radius: var(--radius-full);
    font-weight: 700;
    font-size: 0.875rem;
    box-shadow: 0 4px 12px rgba(236, 72, 153, 0.3);
    transition: all 0.3s ease;
}

.cta-button:hover {
    transform: translateY(-2px) scale(1.05);
    box-shadow: 0 8px 20px rgba(236, 72, 153, 0.4);
}

.cta-button span {
    display: inline-flex;
    align-items: center;
}

/* ── Input Area ──────────────────────────────────── */

.input-area {
    padding: 0;
    background: transparent;
    position: relative;
    z-index: 5;
}

/* ── Thinking Message ─────────────────────────────── */

.thinking-message {
    display: flex;
    gap: 0.625rem;
    max-width: 85%;
    margin-bottom: 1rem;
    animation: slideIn 0.4s ease-out;
}

@keyframes slideIn {
    from {
        opacity: 0;
        transform: translateY(12px) scale(0.98);
    }
    to {
        opacity: 1;
        transform: translateY(0) scale(1);
    }
}

.thinking-avatar {
    position: relative;
    flex-shrink: 0;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: hsl(var(--secondary));
    border: none;
    overflow: visible;
}

.thinking-ring {
    position: absolute;
    top: -3px;
    left: -3px;
    right: -3px;
    bottom: -3px;
    border-radius: 50%;
    border: 2.5px solid transparent;
    border-top-color: hsl(var(--primary));
    border-right-color: hsl(var(--primary) / 0.4);
    animation: ringSpin 1s linear infinite;
}

.thinking-avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 50%;
    border: 2px solid hsl(var(--primary) / 0.3);
}

@keyframes ringSpin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

.thinking-content-wrapper {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    flex: 1;
}

.thinking-label {
    font-size: 0.625rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: hsl(var(--primary));
}

.thinking-icon {
    width: 14px;
    height: 14px;
    animation: ruriSpin 1s linear infinite;
    transform-origin: center;
    color: hsl(var(--primary));
}

.thinking-status {
    background: linear-gradient(
        90deg,
        hsl(var(--primary) / 0.2) 0%,
        hsl(280 70% 60% / 0.2) 100%
    );
    padding: 0.125rem 0.5rem;
    border-radius: 0.375rem;
    animation: statusPulse 2s ease-in-out infinite;
}

@keyframes statusPulse {
    0%,
    100% {
        opacity: 0.7;
    }
    50% {
        opacity: 1;
    }
}

.thinking-content {
    padding: 1rem 1.25rem;
    background: hsl(var(--card));
    border: 2px solid hsl(var(--primary) / 0.3);
    border-radius: 1rem 1rem 1rem 0.25rem;
    color: hsl(var(--foreground));
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.15);
    position: relative;
    overflow: hidden;
    animation: borderGlow 2s ease-in-out infinite;
}

@keyframes borderGlow {
    0%,
    100% {
        border-color: hsl(var(--primary) / 0.2);
        box-shadow: 0 2px 8px hsl(var(--primary) / 0.1);
    }
    50% {
        border-color: hsl(var(--primary) / 0.5);
        box-shadow: 0 2px 16px hsl(var(--primary) / 0.25);
    }
}

.thinking-content::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(
        45deg,
        transparent 30%,
        hsl(var(--primary) / 0.05) 50%,
        transparent 70%
    );
    animation: shimmer 2s ease-in-out infinite;
}

@keyframes shimmer {
    0% {
        transform: translateX(-100%);
    }
    100% {
        transform: translateX(100%);
    }
}

.thinking-animation {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.5rem 0;
}

.wave-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: hsl(var(--primary));
    display: inline-block;
    animation: waveBounce 1.4s ease-in-out infinite;
}

.wave-dot-1 {
    animation-delay: 0s;
}

.wave-dot-2 {
    animation-delay: 0.2s;
}

.wave-dot-3 {
    animation-delay: 0.4s;
}

@keyframes waveBounce {
    0%,
    80%,
    100% {
        transform: scale(0.6);
        opacity: 0.4;
    }
    40% {
        transform: scale(1.2);
        opacity: 1;
    }
}

/* ── Transitions ─────────────────────────────────── */

.thinking-fade-enter-active {
    transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}

.thinking-fade-leave-active {
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.thinking-fade-enter-from {
    opacity: 0;
    transform: translateY(12px) scale(0.98);
}

.thinking-fade-leave-to {
    opacity: 0;
    transform: translateY(-8px) scale(0.95);
}

/* 响应式 */
@media (max-width: 768px) {
    .header-title {
        font-size: 0.95rem;
    }

    .header-icon {
        font-size: 1.5rem;
    }

    .model-badge {
        display: none;
    }

    .empty-icon {
        font-size: 3rem;
    }

    .empty-title {
        font-size: 1.125rem;
    }

    .empty-desc {
        font-size: 0.8125rem;
        max-width: 18rem;
    }
}

/* Disable all animations when config is disabled, but exclude thinking-related elements */
.chat-view.no-animation
    *:not(.thinking-message):not(.thinking-message *):not(
        .thinking-indicator
    ):not(.thinking-indicator *):not(.spinner-icon):not(.thinking-icon):not(
        .thinking-ring
    ):not(.thinking-animation):not(.thinking-status):not(.thinking-content):not(
        .wave-dot
    ),
.chat-view.no-animation
    *:not(.thinking-message):not(.thinking-message *):not(
        .thinking-indicator
    ):not(.thinking-indicator *)::before,
.chat-view.no-animation
    *:not(.thinking-message):not(.thinking-message *):not(
        .thinking-indicator
    ):not(.thinking-indicator *)::after {
    animation-duration: 0s !important;
    animation-delay: 0s !important;
    transition-duration: 0s !important;
}

.chat-view.no-animation .float {
    animation: none !important;
}

.chat-view.no-animation .bounce {
    animation: none !important;
}

.chat-view.no-animation .pulse-dot {
    animation: none !important;
}
</style>

<style>
/* Global (non-scoped) keyframes to avoid Vue 3 scoped style issues with animation-name references */
@keyframes ruriSpin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

@keyframes ringSpin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

@keyframes waveBounce {
    0%,
    80%,
    100% {
        transform: scale(0.6);
        opacity: 0.4;
    }
    40% {
        transform: scale(1.2);
        opacity: 1;
    }
}

@keyframes statusPulse {
    0%,
    100% {
        opacity: 0.7;
    }
    50% {
        opacity: 1;
    }
}

@keyframes borderGlow {
    0%,
    100% {
        border-color: hsl(var(--primary) / 0.2);
        box-shadow: 0 2px 8px hsl(var(--primary) / 0.1);
    }
    50% {
        border-color: hsl(var(--primary) / 0.5);
        box-shadow: 0 2px 16px hsl(var(--primary) / 0.25);
    }
}

@keyframes shimmer {
    0% {
        transform: translateX(-100%);
    }
    100% {
        transform: translateX(100%);
    }
}

@keyframes slideIn {
    from {
        opacity: 0;
        transform: translateY(12px) scale(0.98);
    }
    to {
        opacity: 1;
        transform: translateY(0) scale(1);
    }
}
</style>

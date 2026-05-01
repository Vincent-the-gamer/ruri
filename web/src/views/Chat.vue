<script setup lang="ts">
import { onMounted, ref, nextTick, computed } from "vue";
import { useChatStore } from "../stores/chat";
import { useProviderStore } from "../stores/provider";
import ChatMessageComp from "../components/ChatMessage.vue";
import ChatInput from "../components/ChatInput.vue";

const chatStore = useChatStore();
const providerStore = useProviderStore();

const messagesContainer = ref<HTMLElement | null>(null);
const temperature = ref(0.7);
const maxTokens = ref(4096);
const showSettings = ref(false);

onMounted(async () => {
    await Promise.all([
        chatStore.fetchHistory(),
        providerStore.fetchProviders(),
    ]);
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

async function handleSend(message: string) {
    await chatStore.sendMessage({
        message,
        temperature: temperature.value,
        max_tokens: maxTokens.value,
    });
    scrollToBottom();
}

async function handleClear() {
    if (!confirm("确定清空所有聊天记录？")) return;
    await chatStore.clearHistory();
}

function toggleSettings() {
    showSettings.value = !showSettings.value;
}
</script>

<template>
    <div class="chat-view">
        <!-- Header -->
        <header class="chat-header">
            <div class="header-left">
                <h1 class="header-title">对话</h1>
                <span v-if="chatStore.loading" class="thinking-indicator">
                    <svg class="spinner-icon" viewBox="0 0 24 24" fill="none">
                        <circle
                            class="spinner-track"
                            cx="12"
                            cy="12"
                            r="10"
                            stroke="currentColor"
                            stroke-width="2"
                        />
                        <path
                            class="spinner-head"
                            fill="currentColor"
                            d="M12 2a10 10 0 0 1 10 10h-2a8 8 0 0 0-8-8V2z"
                        />
                    </svg>
                    思考中
                </span>
                <span v-if="providerStore.activeProvider" class="model-badge">
                    {{ providerStore.activeProvider.name }} ·
                    {{
                        (providerStore.activeProvider.config as any)
                            ?.default_model
                    }}
                </span>
            </div>
            <div class="header-actions">
                <button
                    class="icon-btn"
                    :class="{ active: showSettings }"
                    @click="toggleSettings"
                    title="设置"
                >
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
                        <path
                            d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
                        />
                    </svg>
                </button>
                <button
                    class="icon-btn danger"
                    @click="handleClear"
                    title="清空记录"
                >
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <polyline points="3 6 5 6 21 6" />
                        <path
                            d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"
                        />
                        <path d="M10 11v6" />
                        <path d="M14 11v6" />
                        <path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
                    </svg>
                </button>
            </div>
        </header>

        <!-- Settings Panel -->
        <Transition name="slide-down">
            <div v-if="showSettings" class="settings-panel">
                <div class="settings-inner">
                    <div class="setting-item">
                        <label class="setting-label">温度</label>
                        <div class="setting-control">
                            <input
                                v-model.number="temperature"
                                type="range"
                                min="0"
                                max="2"
                                step="0.1"
                                class="range-slider"
                            />
                            <span class="setting-value">{{ temperature }}</span>
                        </div>
                    </div>
                    <div class="setting-item">
                        <label class="setting-label">最大 Token 数</label>
                        <div class="setting-control">
                            <input
                                v-model.number="maxTokens"
                                type="number"
                                min="1"
                                max="128000"
                                step="1"
                                class="number-input"
                            />
                        </div>
                    </div>
                </div>
            </div>
        </Transition>

        <!-- No Provider Warning -->
        <div
            v-if="!providerStore.activeProvider && !chatStore.loading"
            class="warning-bar"
        >
            <svg
                class="warning-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
            >
                <path
                    d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"
                />
                <line x1="12" y1="9" x2="12" y2="13" />
                <line x1="12" y1="17" x2="12.01" y2="17" />
            </svg>
            <span
                >暂无活跃的模型供应商。<router-link
                    to="/providers"
                    class="warning-link"
                    >配置供应商</router-link
                >
                开始对话。</span
            >
        </div>

        <!-- Messages Area -->
        <div ref="messagesContainer" class="messages-area">
            <div class="messages-inner">
                <!-- Empty State -->
                <div v-if="messages.length === 0" class="empty-state">
                    <svg
                        class="empty-icon"
                        viewBox="0 0 48 48"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <rect x="6" y="10" width="36" height="24" rx="4" />
                        <path d="M6 18h36" />
                        <circle
                            cx="12"
                            cy="14"
                            r="1.5"
                            fill="currentColor"
                            stroke="none"
                        />
                        <circle
                            cx="17"
                            cy="14"
                            r="1.5"
                            fill="currentColor"
                            stroke="none"
                        />
                        <circle
                            cx="22"
                            cy="14"
                            r="1.5"
                            fill="currentColor"
                            stroke="none"
                        />
                        <path d="M15 28l4-4 3 3 5-5" />
                        <path d="M24 22h3v3" />
                    </svg>
                    <h2 class="empty-title">Ruri 对话</h2>
                    <p class="empty-desc">
                        开始与 AI 智能体对话。请确保已配置并激活了模型供应商。
                    </p>
                </div>

                <!-- Chat Messages -->
                <ChatMessageComp
                    v-for="(msg, i) in messages"
                    :key="i"
                    :message="msg"
                />
            </div>
        </div>

        <!-- Input Area -->
        <div class="input-area">
            <ChatInput @send="handleSend" />
        </div>
    </div>
</template>

<style scoped>
.chat-view {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background-color: var(--color-bg-soft);
}

/* ── Header ─────────────────────────────────────── */

.chat-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.25rem;
    height: 52px;
    min-height: 52px;
    border-bottom: 1px solid var(--color-border);
    background-color: var(--color-bg);
}

.header-left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.header-title {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--color-text);
    letter-spacing: -0.01em;
}

.thinking-indicator {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.75rem;
    color: var(--color-accent);
}

.spinner-icon {
    width: 14px;
    height: 14px;
    animation: spin 1s linear infinite;
}

.spinner-track {
    opacity: 0.2;
}

.spinner-head {
    opacity: 0.8;
}

@keyframes spin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

.model-badge {
    font-size: 0.6875rem;
    color: var(--color-text-muted);
    padding: 0.125rem 0.5rem;
    background-color: var(--color-bg-mute);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
}

.header-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
}

.icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
}

.icon-btn svg {
    width: 16px;
    height: 16px;
}

.icon-btn:hover {
    background-color: var(--color-bg-mute);
    color: var(--color-text);
}

.icon-btn.active {
    background-color: var(--color-accent-soft);
    color: var(--color-accent);
}

.icon-btn.danger:hover {
    background-color: var(--color-danger-soft);
    color: var(--color-danger);
}

/* ── Settings Panel ─────────────────────────────── */

.settings-panel {
    border-bottom: 1px solid var(--color-border);
    background-color: var(--color-bg);
}

.settings-inner {
    display: flex;
    align-items: center;
    gap: 2rem;
    padding: 0.875rem 1.25rem;
    max-width: 48rem;
}

.setting-item {
    display: flex;
    align-items: center;
    gap: 0.625rem;
}

.setting-label {
    font-size: 0.8125rem;
    color: var(--color-text-secondary);
    white-space: nowrap;
}

.setting-control {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.setting-value {
    font-size: 0.75rem;
    color: var(--color-text);
    min-width: 2rem;
    text-align: right;
    font-variant-numeric: tabular-nums;
}

/* Range slider */
.range-slider {
    -webkit-appearance: none;
    appearance: none;
    width: 120px;
    height: 4px;
    border-radius: 2px;
    background: var(--color-border);
    outline: none;
    cursor: pointer;
}

.range-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--color-accent);
    border: 2px solid var(--color-bg);
    box-shadow: 0 0 0 1px var(--color-accent);
    cursor: pointer;
    transition: transform var(--transition-fast);
}

.range-slider::-webkit-slider-thumb:hover {
    transform: scale(1.15);
}

.range-slider::-moz-range-thumb {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--color-accent);
    border: 2px solid var(--color-bg);
    box-shadow: 0 0 0 1px var(--color-accent);
    cursor: pointer;
}

/* Number input */
.number-input {
    width: 5.5rem;
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    color: var(--color-text);
    background-color: var(--color-bg-mute);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    outline: none;
    font-variant-numeric: tabular-nums;
    transition: border-color var(--transition-fast);
}

.number-input:focus {
    border-color: var(--color-accent);
}

/* ── Warning Bar ─────────────────────────────────── */

.warning-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1.25rem;
    background-color: var(--color-warning-soft);
    border-bottom: 1px solid rgba(245, 158, 11, 0.2);
    font-size: 0.8125rem;
    color: var(--color-warning);
}

.warning-icon {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
}

.warning-link {
    color: var(--color-warning);
    text-decoration: underline;
    text-underline-offset: 2px;
    transition: opacity var(--transition-fast);
}

.warning-link:hover {
    opacity: 0.8;
}

/* ── Messages Area ───────────────────────────────── */

.messages-area {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem 1.25rem;
}

.messages-inner {
    max-width: 48rem;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}

/* ── Empty State ─────────────────────────────────── */

.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 50vh;
    text-align: center;
}

.empty-icon {
    width: 48px;
    height: 48px;
    color: var(--color-text-dim);
    margin-bottom: 1rem;
}

.empty-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--color-text);
    margin-bottom: 0.375rem;
    letter-spacing: -0.01em;
}

.empty-desc {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    max-width: 20rem;
    line-height: 1.5;
}

/* ── Input Area ──────────────────────────────────── */

.input-area {
    padding: 0.75rem 1.25rem 1rem;
    border-top: 1px solid var(--color-border);
    background-color: var(--color-bg);
}

/* ── Transitions ─────────────────────────────────── */

.slide-down-enter-active,
.slide-down-leave-active {
    transition: all var(--transition-normal);
}

.slide-down-enter-from,
.slide-down-leave-to {
    opacity: 0;
    transform: translateY(-4px);
}
</style>

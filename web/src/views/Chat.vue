<script setup lang="ts">
import { onMounted, onActivated, ref, nextTick, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useChatStore } from "../stores/chat";
import { useProviderStore } from "../stores/provider";
import { usePersonaStore } from "../stores/persona";
import ChatMessageComp from "../components/ChatMessage.vue";
import ChatInput from "../components/ChatInput.vue";

const { t } = useI18n();
const chatStore = useChatStore();
const providerStore = useProviderStore();
const personaStore = usePersonaStore();

const messagesContainer = ref<HTMLElement | null>(null);
const temperature = ref(0.7);
const maxTokens = ref(4096);
const showSettings = ref(false);

onMounted(async () => {
    await Promise.all([
        chatStore.fetchHistory(),
        providerStore.fetchProviders(),
        personaStore.fetchPersonas(),
    ]);
    scrollToBottom();
});

// When activated from keep-alive cache, just scroll to bottom
// No need to refetch history as component state is preserved
onActivated(() => {
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
        persona_id: personaStore.activePersona?.id,
        temperature: temperature.value,
        max_tokens: maxTokens.value,
    });
    scrollToBottom();
}

async function handleClear() {
    if (!confirm(t("chat.confirmClear"))) return;
    await chatStore.clearHistory();
}

function toggleSettings() {
    showSettings.value = !showSettings.value;
}
</script>

<template>
    <div class="chat-view">
        <!-- Header - 可爱风格 -->
        <header class="chat-header glass">
            <div class="header-left">
                <div class="header-icon bounce">
                    <span>💬</span>
                </div>
                <div>
                    <h1 class="header-title font-cute">
                        <span>💎</span>
                        <span>{{ t("chat.title") }}</span>
                        <span>✨</span>
                    </h1>
                    <span v-if="chatStore.loading" class="thinking-indicator">
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
                                stroke-width="2"
                            />
                            <path
                                class="spinner-head"
                                fill="currentColor"
                                d="M12 2a10 10 0 0 1 10 10h-2a8 8 0 0 0-8-8V2z"
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
                <div
                    v-if="personaStore.activePersona"
                    class="model-badge persona-badge"
                >
                    <span class="badge-icon">🎭</span>
                    <span>{{ personaStore.activePersona.name }}</span>
                </div>
                <div v-if="providerStore.activeProvider" class="model-badge">
                    <span class="badge-icon">🤖</span>
                    <span>{{ providerStore.activeProvider.name }}</span>
                    <span class="badge-divider">·</span>
                    <span>{{
                        (providerStore.activeProvider.config as any)
                            ?.default_model
                    }}</span>
                </div>
                <div
                    class="header-actions"
                    :class="{ 'has-badge': !!providerStore.activeProvider }"
                >
                    <button
                        class="icon-btn"
                        :class="{ active: showSettings }"
                        @click="toggleSettings"
                        :title="t('chat.settings') + ' ⚙️'"
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

        <!-- Settings Panel - 可爱风格 -->
        <Transition name="slide-down">
            <div v-if="showSettings" class="settings-panel glass-subtle">
                <div class="settings-inner">
                    <div class="setting-item">
                        <label class="setting-label font-cute">
                            <span>🌡️</span>
                            <span>{{ t("chat.temperature") }}</span>
                        </label>
                        <div class="setting-control">
                            <input
                                v-model.number="temperature"
                                type="range"
                                min="0"
                                max="2"
                                step="0.1"
                                class="range-slider"
                            />
                            <div class="setting-value-badge">
                                {{ temperature }}
                            </div>
                        </div>
                    </div>
                    <div class="setting-item">
                        <label class="setting-label font-cute">
                            <span>📊</span>
                            <span>{{ t("chat.maxTokens") }}</span>
                        </label>
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

        <!-- No Provider Warning - 友好提示 -->
        <div
            v-if="!providerStore.activeProvider && !chatStore.loading"
            class="warning-bar"
        >
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
                        v-if="!providerStore.activeProvider"
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
                <div v-if="chatStore.loading" class="thinking-message">
                    <div class="thinking-avatar">
                        <svg
                            class="avatar-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                        >
                            <defs>
                                <linearGradient
                                    id="thinking-gradient"
                                    x1="0%"
                                    y1="0%"
                                    x2="100%"
                                    y2="100%"
                                >
                                    <stop
                                        offset="0%"
                                        stop-color="hsl(var(--primary))"
                                    />
                                    <stop
                                        offset="100%"
                                        stop-color="hsl(280 70% 60%)"
                                    />
                                </linearGradient>
                            </defs>
                            <path
                                d="M12 2 L20 10 L12 22 L4 10 Z"
                                fill="url(#thinking-gradient)"
                            />
                            <path
                                d="M12 2 L16 8 L12 6 L8 8 Z"
                                fill="rgba(255,255,255,0.4)"
                            />
                        </svg>
                        <div class="thinking-dots">
                            <span class="dot dot-1"></span>
                            <span class="dot dot-2"></span>
                            <span class="dot dot-3"></span>
                        </div>
                    </div>
                    <div class="thinking-content-wrapper">
                        <div class="thinking-label">
                            <span>琉璃</span>
                            <span class="thinking-status">💭 思考中...</span>
                        </div>
                        <div class="thinking-content">
                            <div class="thinking-animation">
                                <span class="spark sparkle-1">✨</span>
                                <span class="spark sparkle-2">💫</span>
                                <span class="spark sparkle-3">⭐</span>
                            </div>
                        </div>
                    </div>
                </div>
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
}

.thinking-indicator.ready {
    color: var(--color-success);
}

.spinner-icon {
    width: 14px;
    height: 14px;
    animation: spin 1s linear infinite;
}

.spinner-track {
    opacity: 0.25;
    stroke: var(--color-accent);
}

.spinner-head {
    opacity: 0.8;
    fill: var(--color-accent);
    stroke: var(--color-accent);
}

@keyframes spin {
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

/* ── Settings Panel ─────────────────────────────── */

.settings-panel {
    border-bottom: 2px solid rgba(249, 168, 212, 0.3);
    position: relative;
    z-index: 5;
}

.settings-inner {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2rem;
    padding: 1rem 1.25rem;
    max-width: 56rem;
}

.setting-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.setting-label {
    font-size: 0.875rem;
    color: var(--color-text-secondary);
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-weight: 600;
}

.setting-control {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.setting-value-badge {
    font-size: 0.8125rem;
    font-weight: 700;
    color: var(--color-accent);
    background: linear-gradient(
        135deg,
        rgba(236, 72, 153, 0.15) 0%,
        rgba(192, 132, 252, 0.15) 100%
    );
    padding: 0.25rem 0.625rem;
    border-radius: var(--radius-sm);
    border: 2px solid var(--color-accent);
    min-width: 2.5rem;
    text-align: center;
    box-shadow: 0 2px 6px rgba(236, 72, 153, 0.1);
}

/* Range slider */
.range-slider {
    -webkit-appearance: none;
    appearance: none;
    width: 140px;
    height: 6px;
    border-radius: 3px;
    background: linear-gradient(
        90deg,
        rgba(236, 72, 153, 0.2) 0%,
        rgba(192, 132, 252, 0.2) 100%
    );
    outline: none;
    cursor: pointer;
}

.range-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(280 70% 60%) 100%
    );
    border: 2px solid hsl(var(--background));
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.4);
    cursor: pointer;
    transition:
        transform 0.2s ease,
        box-shadow 0.2s ease;
}

.range-slider::-webkit-slider-thumb:hover {
    transform: scale(1.2);
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.5);
}

.range-slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(280 70% 60%) 100%
    );
    border: 2px solid hsl(var(--background));
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.4);
    cursor: pointer;
}

/* Number input */
.number-input {
    width: 6rem;
    padding: 0.375rem 0.625rem;
    font-size: 0.8125rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: var(--radius-md);
    outline: none;
    font-variant-numeric: tabular-nums;
    transition: all 0.2s ease;
    box-shadow: var(--shadow-sm);
}

.number-input:focus {
    border-color: hsl(var(--primary));
    box-shadow: 0 0 0 3px hsl(var(--primary) / 0.15);
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
    width: 32px;
    height: 32px;
    border-radius: 0.625rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: hsl(var(--secondary));
    border: 2px solid hsl(var(--primary));
}

.thinking-dots {
    position: absolute;
    bottom: -8px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    gap: 3px;
}

.dot {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: hsl(var(--primary));
    animation: bounceDot 1.4s ease-in-out infinite;
}

.dot-1 {
    animation-delay: 0s;
}

.dot-2 {
    animation-delay: 0.2s;
}

.dot-3 {
    animation-delay: 0.4s;
}

@keyframes bounceDot {
    0%,
    80%,
    100% {
        transform: scale(1);
        opacity: 0.6;
    }
    40% {
        transform: scale(1.5);
        opacity: 1;
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

.thinking-status {
    background: linear-gradient(
        90deg,
        hsl(var(--primary) / 0.2) 0%,
        hsl(280 70% 60% / 0.2) 100%
    );
    padding: 0.125rem 0.5rem;
    border-radius: 0.375rem;
    animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
    0%,
    100% {
        opacity: 0.8;
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
    gap: 0.75rem;
    padding: 0.5rem 0;
}

.spark {
    font-size: 1.5rem;
    display: inline-flex;
    animation: float 2s ease-in-out infinite;
}

.sparkle-1 {
    animation-delay: 0s;
}

.sparkle-2 {
    animation-delay: 0.3s;
}

.sparkle-3 {
    animation-delay: 0.6s;
}

@keyframes float {
    0%,
    100% {
        transform: translateY(0) rotate(0deg);
        opacity: 0.7;
    }
    50% {
        transform: translateY(-10px) rotate(10deg);
        opacity: 1;
    }
}

/* ── Transitions ─────────────────────────────────── */

.slide-down-enter-active,
.slide-down-leave-active {
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-down-enter-from,
.slide-down-leave-to {
    opacity: 0;
    transform: translateY(-12px) scale(0.95);
}

/* 动画 */
@keyframes float {
    0%,
    100% {
        transform: translateY(0) rotate(0deg);
    }
    50% {
        transform: translateY(-6px) rotate(3deg);
    }
}

/* 响应式 */
@media (max-width: 768px) {
    .header-title {
        font-size: 0.95rem;
    }

    .header-icon {
        font-size: 1.5rem;
    }

    .settings-inner {
        flex-direction: column;
        align-items: flex-start;
        gap: 1rem;
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
</style>

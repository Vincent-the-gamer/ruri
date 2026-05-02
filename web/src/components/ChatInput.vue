<script setup lang="ts">
import { ref } from "vue";

const emit = defineEmits<{
    send: [message: string];
}>();

const inputText = ref("");
const isComposing = ref(false);

function handleSend() {
    const text = inputText.value.trim();
    if (!text) return;
    emit("send", text);
    inputText.value = "";
}

function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !isComposing.value) {
        e.preventDefault();
        handleSend();
    }
}
</script>

<template>
    <div class="chat-input-wrapper glass-subtle">
        <!-- 装饰元素 -->
        <div class="decoration-stars">
            <span class="star star-1">⭐</span>
            <span class="star star-2">✨</span>
            <span class="star star-3">💫</span>
        </div>

        <div class="chat-input-container">
            <div class="input-field">
                <div class="input-wrapper">
                    <div class="input-icon">
                        <span>💬</span>
                    </div>
                    <textarea
                        v-model="inputText"
                        @keydown="handleKeydown"
                        @compositionstart="isComposing = true"
                        @compositionend="isComposing = false"
                        placeholder="和琉璃说点什么吧... (Enter 发送, Shift+Enter 换行)"
                        rows="3"
                        class="input-textarea font-cute"
                        @input="
                            (
                                $event.target as HTMLTextAreaElement
                            ).style.height = 'auto';
                            (
                                $event.target as HTMLTextAreaElement
                            ).style.height =
                                Math.min(
                                    ($event.target as HTMLTextAreaElement)
                                        .scrollHeight,
                                    160,
                                ) + 'px';
                        "
                    ></textarea>
                    <!-- 装饰波浪线 -->
                    <div class="input-decoration"></div>
                </div>
            </div>
            <button
                @click="handleSend"
                :disabled="!inputText.trim()"
                class="send-button"
                :class="{ disabled: !inputText.trim() }"
                :title="inputText.trim() ? '发送消息 💕' : '请输入消息后再发送'"
            >
                <span class="send-emoji">💌</span>
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="send-icon"
                >
                    <line x1="22" y1="2" x2="11" y2="13" />
                    <polygon points="22 2 15 22 11 13 2 9 22 2" />
                </svg>
            </button>
        </div>

        <!-- 底部提示 -->
        <div class="input-hint">
            <span class="hint-text">💡 琉璃会用粉色的心️回答你哦~</span>
        </div>
    </div>
</template>

<style scoped>
.chat-input-wrapper {
    border-top: 2px solid var(--color-border);
    padding: 1.5rem 1.25rem;
    position: relative;
    overflow: hidden;
    background: linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.95) 0%,
        rgba(250, 245, 255, 0.95) 100%
    );
}

.chat-input-wrapper::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: linear-gradient(90deg, var(--color-accent), #a855f7, #818cf8);
    opacity: 0.5;
}

/* 装饰星星 */
.decoration-stars {
    position: absolute;
    width: 100%;
    height: 100%;
    pointer-events: none;
    overflow: hidden;
}

.star {
    position: absolute;
    font-size: 1rem;
    opacity: 0.5;
    animation: float 3s ease-in-out infinite;
}

.star-1 {
    top: 20%;
    left: 5%;
    animation-delay: 0s;
}

.star-2 {
    top: 30%;
    right: 10%;
    animation-delay: 1s;
}

.star-3 {
    bottom: 25%;
    left: 15%;
    animation-delay: 2s;
}

@keyframes float {
    0%,
    100% {
        transform: translateY(0) rotate(0deg);
    }
    50% {
        transform: translateY(-8px) rotate(5deg);
    }
}

.chat-input-container {
    max-width: 52rem;
    margin: 0 auto 0.75rem;
    display: flex;
    align-items: flex-end;
    gap: 1rem;
    position: relative;
    z-index: 1;
}

.input-field {
    flex: 1;
    position: relative;
}

.input-wrapper {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
}

.input-icon {
    flex-shrink: 0;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: var(--radius-full);
    background: linear-gradient(135deg, var(--color-accent) 0%, #a855f7 100%);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.125rem;
    box-shadow: 0 4px 12px rgba(236, 72, 153, 0.2);
    animation: bounce 2s ease-in-out infinite;
}

@keyframes bounce {
    0%,
    100% {
        transform: translateY(0);
    }
    50% {
        transform: translateY(-3px);
    }
}

.input-textarea {
    flex: 1;
    background: linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.9) 0%,
        rgba(253, 242, 248, 0.9) 100%
    );
    border: 2px solid var(--color-border);
    border-radius: var(--radius-xl);
    padding: 0.875rem 1rem;
    font-size: 0.95rem;
    line-height: 1.6;
    color: var(--color-text);
    resize: none;
    min-height: 3.5rem;
    max-height: 160px;
    transition: all 0.3s ease;
    font-family: inherit;
    box-shadow: 0 2px 8px rgba(168, 85, 247, 0.05);
}

.input-textarea::placeholder {
    color: var(--color-text-muted);
    font-style: italic;
}

.input-textarea:focus {
    outline: none;
    border-color: var(--color-accent);
    background: linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.98) 0%,
        rgba(253, 242, 248, 0.98) 100%
    );
    box-shadow:
        0 0 0 3px var(--color-accent-soft),
        0 4px 16px rgba(168, 85, 247, 0.15);
    transform: translateY(-1px);
}

.input-textarea::-webkit-scrollbar {
    width: 8px;
}

.input-textarea::-webkit-scrollbar-track {
    background: rgba(249, 168, 212, 0.2);
    border-radius: 4px;
}

.input-textarea::-webkit-scrollbar-thumb {
    background: linear-gradient(180deg, var(--color-accent) 0%, #a855f7 100%);
    border-radius: 4px;
    border: 2px solid rgba(249, 168, 212, 0.2);
}

.input-textarea::-webkit-scrollbar-thumb:hover {
    background: linear-gradient(
        180deg,
        var(--color-accent-hover) 0%,
        #c084fc 100%
    );
}

.input-decoration {
    position: absolute;
    bottom: -1px;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--color-accent), #a855f7, #818cf8);
    opacity: 0;
    transition: opacity 0.3s ease;
    border-radius: 0 0 var(--radius-xl) var(--radius-xl);
}

.input-wrapper:focus-within .input-decoration {
    opacity: 0.6;
}

.send-button {
    flex-shrink: 0;
    width: 3rem;
    height: 3rem;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    background: linear-gradient(135deg, var(--color-accent) 0%, #a855f7 100%);
    border: 2px solid transparent;
    border-radius: var(--radius-full);
    cursor: pointer;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 4px 12px rgba(236, 72, 153, 0.3);
    overflow: hidden;
}

.send-button::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.3) 0%,
        rgba(255, 255, 255, 0.1) 100%
    );
    opacity: 0;
    transition: opacity 0.3s ease;
}

.send-button:hover:not(.disabled)::before {
    opacity: 1;
}

.send-button:hover:not(.disabled) {
    transform: translateY(-2px) scale(1.05);
    box-shadow:
        0 8px 20px rgba(236, 72, 153, 0.4),
        0 0 20px rgba(236, 72, 153, 0.2);
}

.send-button:active:not(.disabled) {
    transform: translateY(0) scale(0.98);
}

.send-icon {
    width: 1.25rem;
    height: 1.25rem;
    color: white;
    transition: transform 0.2s ease;
    position: relative;
    z-index: 1;
}

.send-button:hover:not(.disabled) .send-icon {
    transform: translateX(2px) translateY(-2px);
}

.send-emoji {
    position: absolute;
    font-size: 1.125rem;
    opacity: 0;
    transform: scale(0.5);
    transition: all 0.3s ease;
}

.send-button:hover:not(.disabled) .send-emoji {
    opacity: 1;
    transform: scale(1);
}

.send-button.disabled {
    background: linear-gradient(135deg, #f1f5f9 0%, #e2e8f0 100%);
    border-color: #cbd5e1;
    cursor: not-allowed;
    opacity: 0.7;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.send-button.disabled .send-icon {
    color: #94a3b8;
}

/* 底部提示 */
.input-hint {
    text-align: center;
    padding: 0.5rem;
    animation: fadeIn 0.5s ease-out;
}

@keyframes fadeIn {
    from {
        opacity: 0;
        transform: translateY(5px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

.hint-text {
    font-size: 0.8rem;
    color: var(--color-text-muted);
    font-style: italic;
}

/* 响应式 */
@media (max-width: 640px) {
    .chat-input-container {
        flex-direction: column;
        gap: 0.75rem;
    }

    .input-wrapper {
        gap: 0.5rem;
    }

    .input-icon {
        width: 2.25rem;
        height: 2.25rem;
        font-size: 1rem;
    }

    .send-button {
        width: 100%;
        height: 2.5rem;
        flex-direction: row;
        gap: 0.5rem;
    }

    .send-button .send-icon {
        width: 1rem;
        height: 1rem;
    }

    .star {
        display: none;
    }
}
</style>

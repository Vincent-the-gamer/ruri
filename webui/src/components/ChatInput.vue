<script setup lang="ts">
import { ref } from "vue";

const props = defineProps<{
    disabled?: boolean;
}>();

const emit = defineEmits<{
    send: [message: string];
}>();

const inputText = ref("");
const isComposing = ref(false);

function handleSend() {
    const text = inputText.value.trim();
    if (!text || props.disabled) return;
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
    <div class="chat-input-wrapper">
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
                        class="input-textarea"
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
    border-top: 1px solid hsl(var(--border));
    padding: 1.25rem 1.5rem;
    position: relative;
    overflow: hidden;
    background: hsl(var(--background));
    border-radius: 1rem 1rem 0 0;
}

.chat-input-wrapper::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(
        90deg,
        hsl(var(--primary)),
        hsl(280 70% 60%),
        hsl(var(--primary))
    );
    opacity: 0.6;
}

/* 装饰星星 */
.decoration-stars {
    position: absolute;
    width: 100%;
    height: 100%;
    pointer-events: none;
    overflow: hidden;
    opacity: 0.4;
}

.dark .decoration-stars {
    opacity: 0.2;
}

.star {
    position: absolute;
    font-size: 0.875rem;
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
    margin: 0 auto 0.5rem;
    display: flex;
    align-items: flex-end;
    gap: 0.75rem;
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
    gap: 0.625rem;
}

.input-icon {
    flex-shrink: 0;
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 0.75rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(280 70% 60%) 100%
    );
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1rem;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.25);
    animation: pulse-glow 2s ease-in-out infinite;
}

@keyframes pulse-glow {
    0%,
    100% {
        box-shadow: 0 2px 8px hsl(var(--primary) / 0.25);
    }
    50% {
        box-shadow: 0 4px 16px hsl(var(--primary) / 0.35);
    }
}

.input-textarea {
    flex: 1;
    background: hsl(var(--card));
    border: 1.5px solid hsl(var(--border));
    border-radius: 0.875rem;
    padding: 0.75rem 1rem;
    font-size: 0.9375rem;
    line-height: 1.5;
    color: hsl(var(--foreground));
    resize: none;
    min-height: 2.75rem;
    max-height: 160px;
    transition: all 0.2s ease;
    font-family: inherit;
}

.input-textarea::placeholder {
    color: hsl(var(--muted-foreground));
}

.input-textarea:focus {
    outline: none;
    border-color: hsl(var(--primary));
    box-shadow: 0 0 0 3px hsl(var(--primary) / 0.15);
    background: hsl(var(--card));
}

.input-textarea:focus {
    border-color: hsl(var(--primary));
}

.input-textarea::-webkit-scrollbar {
    width: 6px;
}

.input-textarea::-webkit-scrollbar-track {
    background: transparent;
}

.input-textarea::-webkit-scrollbar-thumb {
    background: hsl(var(--muted));
    border-radius: 3px;
}

.input-textarea::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--muted-foreground) / 0.5);
}

/* 发送按钮 */
.send-button {
    flex-shrink: 0;
    width: 2.75rem;
    height: 2.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(280 70% 60%) 100%
    );
    border: none;
    border-radius: 0.75rem;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.3);
    overflow: hidden;
}

.send-button::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.2) 0%,
        rgba(255, 255, 255, 0.05) 100%
    );
    opacity: 0;
    transition: opacity 0.2s ease;
}

.send-button:hover:not(.disabled)::before {
    opacity: 1;
}

.send-button:hover:not(.disabled) {
    transform: translateY(-2px);
    box-shadow: 0 4px 16px hsl(var(--primary) / 0.4);
}

.send-button:active:not(.disabled) {
    transform: translateY(0) scale(0.98);
}

.send-icon {
    width: 1.125rem;
    height: 1.125rem;
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
    font-size: 1rem;
    opacity: 0;
    transform: scale(0.5);
    transition: all 0.2s ease;
}

.send-button:hover:not(.disabled) .send-emoji {
    opacity: 1;
    transform: scale(1);
}

.send-button.disabled {
    background: hsl(var(--muted));
    cursor: not-allowed;
    opacity: 0.5;
    box-shadow: none;
}

.send-button.disabled .send-icon {
    color: hsl(var(--muted-foreground));
}

/* 底部提示 */
.input-hint {
    text-align: center;
    padding-top: 0.25rem;
}

.hint-text {
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
}

/* 响应式 */
@media (max-width: 640px) {
    .chat-input-wrapper {
        padding: 1rem;
    }

    .chat-input-container {
        flex-direction: column;
        gap: 0.625rem;
    }

    .input-wrapper {
        gap: 0.5rem;
    }

    .input-icon {
        width: 2rem;
        height: 2rem;
        font-size: 0.875rem;
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

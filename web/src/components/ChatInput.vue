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
    <div class="chat-input-wrapper">
        <div class="chat-input-container">
            <div class="input-field">
                <textarea
                    v-model="inputText"
                    @keydown="handleKeydown"
                    @compositionstart="isComposing = true"
                    @compositionend="isComposing = false"
                    placeholder="Type a message... (Enter to send, Shift+Enter for new line)"
                    rows="1"
                    class="input-textarea"
                    @input="
                        ($event.target as HTMLTextAreaElement).style.height =
                            'auto';
                        ($event.target as HTMLTextAreaElement).style.height =
                            ($event.target as HTMLTextAreaElement)
                                .scrollHeight + 'px';
                    "
                ></textarea>
            </div>
            <button
                @click="handleSend"
                :disabled="!inputText.trim()"
                class="send-button"
                :class="{ disabled: !inputText.trim() }"
            >
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <line x1="22" y1="2" x2="11" y2="13" />
                    <polygon points="22 2 15 22 11 13 2 9 22 2" />
                </svg>
            </button>
        </div>
    </div>
</template>

<style scoped>
.chat-input-wrapper {
    border-top: 1px solid var(--color-border);
    background: var(--color-bg-soft);
    padding: 16px;
}

.chat-input-container {
    max-width: 48rem;
    margin: 0 auto;
    display: flex;
    align-items: flex-end;
    gap: 12px;
}

.input-field {
    flex: 1;
    position: relative;
}

.input-textarea {
    width: 100%;
    background: var(--color-bg, #0f172a);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 12px 16px;
    font-size: 0.9375rem;
    line-height: 1.5;
    color: var(--color-text);
    placeholder-color: var(--color-text-muted);
    resize: none;
    max-height: 160px;
    transition:
        border-color 0.2s ease,
        box-shadow 0.2s ease;
    font-family: inherit;
}

.input-textarea::placeholder {
    color: var(--color-text-muted);
}

.input-textarea:focus {
    outline: none;
    border-color: var(--color-accent);
    box-shadow: 0 0 0 2px var(--color-accent-soft);
}

.input-textarea::-webkit-scrollbar {
    width: 6px;
}

.input-textarea::-webkit-scrollbar-track {
    background: transparent;
}

.input-textarea::-webkit-scrollbar-thumb {
    background: var(--color-border);
    border-radius: 3px;
}

.input-textarea::-webkit-scrollbar-thumb:hover {
    background: var(--color-text-muted);
}

.send-button {
    flex-shrink: 0;
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-accent);
    border: none;
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition:
        background-color 0.2s ease,
        opacity 0.2s ease,
        transform 0.1s ease;
}

.send-button svg {
    width: 20px;
    height: 20px;
    color: white;
    transition: transform 0.15s ease;
}

.send-button:hover:not(.disabled) {
    filter: brightness(1.1);
}

.send-button:hover:not(.disabled) svg {
    transform: translateX(1px) translateY(-1px);
}

.send-button:active:not(.disabled) {
    transform: scale(0.97);
}

.send-button.disabled {
    background: var(--color-bg-mute);
    cursor: not-allowed;
    opacity: 0.6;
}

.send-button.disabled svg {
    color: var(--color-text-muted);
}
</style>

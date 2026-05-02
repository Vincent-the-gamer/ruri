<script setup lang="ts">
import { ref } from "vue";
import type { ChatMessage as ChatMessageType } from "../types";

const props = defineProps<{
    message: ChatMessageType;
}>();

const showToolCalls = ref(false);

const isUser = props.message.role === "user";
const isAssistant = props.message.role === "assistant";
const isTool = props.message.role === "tool";
const isSystem = props.message.role === "system";

const hasToolCalls =
    props.message.tool_calls && props.message.tool_calls.length > 0;

function formatArgs(args: string): string {
    try {
        return JSON.stringify(JSON.parse(args), null, 2);
    } catch {
        return args;
    }
}
</script>

<template>
    <div class="message-wrapper fade-in">
        <!-- User Message -->
        <div v-if="isUser" class="message message-user">
            <div class="message-avatar">
                <svg class="avatar-icon" viewBox="0 0 24 24" fill="none">
                    <circle cx="12" cy="8" r="4" fill="var(--color-accent)" />
                    <path
                        d="M4 20c0-4.418 3.582-8 8-8s8 3.582 8 8"
                        stroke="var(--color-accent)"
                        stroke-width="2"
                        stroke-linecap="round"
                    />
                </svg>
            </div>
            <div class="message-content-wrapper">
                <div class="message-label">你</div>
                <div class="message-content user-content">
                    {{ message.content }}
                </div>
            </div>
        </div>

        <!-- Assistant Message -->
        <div v-else-if="isAssistant" class="message message-assistant">
            <div class="message-avatar assistant-avatar">
                <svg class="avatar-icon bounce" viewBox="0 0 24 24" fill="none">
                    <defs>
                        <linearGradient
                            id="crystal-gradient"
                            x1="0%"
                            y1="0%"
                            x2="100%"
                            y2="100%"
                        >
                            <stop offset="0%" stop-color="#ec4899" />
                            <stop offset="100%" stop-color="#8b5cf6" />
                        </linearGradient>
                    </defs>
                    <path
                        d="M12 2 L20 10 L12 22 L4 10 Z"
                        fill="url(#crystal-gradient)"
                    />
                    <path
                        d="M12 2 L16 8 L12 6 L8 8 Z"
                        fill="rgba(255,255,255,0.4)"
                    />
                    <circle
                        cx="13"
                        cy="9"
                        r="1.5"
                        fill="white"
                        class="sparkle sparkle-1"
                    />
                    <circle
                        cx="10"
                        cy="13"
                        r="1"
                        fill="white"
                        class="sparkle sparkle-2"
                    />
                </svg>
            </div>
            <div class="message-content-wrapper">
                <div class="message-label assistant-label">
                    <span>琉璃</span>
                    <span class="label-dot"></span>
                </div>
                <div class="message-content assistant-content">
                    {{ message.content }}
                </div>
                <!-- Tool calls -->
                <div v-if="hasToolCalls" class="tool-calls">
                    <div
                        @click="showToolCalls = !showToolCalls"
                        class="tool-toggle"
                        :class="{ expanded: showToolCalls }"
                    >
                        <svg
                            class="toggle-icon"
                            :class="{ expanded: showToolCalls }"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <path d="M6 9l6 6 6-6" />
                        </svg>
                        <span
                            >Tool calls ({{ message.tool_calls!.length }})</span
                        >
                    </div>
                    <div v-if="showToolCalls" class="tool-list">
                        <div
                            v-for="tc in message.tool_calls"
                            :key="tc.id"
                            class="tool-item"
                        >
                            <div class="tool-header">
                                <span class="tool-badge">{{
                                    tc.function.name
                                }}</span>
                            </div>
                            <pre class="tool-args">{{
                                formatArgs(tc.function.arguments)
                            }}</pre>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <!-- Tool Message -->
        <div v-else-if="isTool" class="message message-tool">
            <div class="message-avatar tool-avatar">
                <svg class="avatar-icon" viewBox="0 0 24 24" fill="none">
                    <path
                        d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"
                        stroke="var(--color-warning)"
                        stroke-width="1.5"
                    />
                </svg>
            </div>
            <div class="message-content-wrapper">
                <div class="message-label">
                    <span>Tool</span>
                    <span v-if="message.tool_call_id" class="tool-id">{{
                        message.tool_call_id
                    }}</span>
                </div>
                <div class="message-content tool-content">
                    {{ message.content }}
                </div>
            </div>
        </div>

        <!-- System Message -->
        <div v-else-if="isSystem" class="message message-system">
            <div class="message-content">
                {{ message.content }}
            </div>
        </div>
    </div>
</template>

<style scoped>
.message-wrapper {
    display: flex;
    width: 100%;
    margin-bottom: 1.25rem;
    animation: fadeIn 0.3s ease-out;
}

@keyframes fadeIn {
    from {
        opacity: 0;
        transform: translateY(8px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

.message {
    display: flex;
    gap: 0.75rem;
    max-width: 85%;
}

/* Avatar Styles */
.message-avatar {
    flex-shrink: 0;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-bg-mute);
    border: 1px solid var(--color-border);
}

.message-user .message-avatar {
    order: 2;
}

.message-assistant .message-avatar {
    order: 1;
}

.message-tool .message-avatar {
    order: 1;
    background: var(--color-warning-soft);
    border-color: var(--color-warning);
}

.avatar-icon {
    width: 20px;
    height: 20px;
}

.assistant-avatar {
    border: 2px solid var(--color-accent);
}

.tool-avatar .avatar-icon {
    width: 18px;
    height: 18px;
}

.avatar-icon .sparkle {
    animation: sparkle 2s ease-in-out infinite;
}

.sparkle-1 {
    animation-delay: 0s;
}

.sparkle-2 {
    animation-delay: 0.5s;
}

@keyframes sparkle {
    0%,
    100% {
        opacity: 0.6;
    }
    50% {
        opacity: 1;
    }
}

/* Message Content Wrapper */
.message-content-wrapper {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
    flex: 1;
    order: 2;
}

.message-user .message-content-wrapper {
    align-items: flex-end;
}

/* Message Label */
.message-label {
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    display: flex;
    align-items: center;
    gap: 0.375rem;
}

.assistant-label {
    color: var(--color-accent);
}

.label-dot {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--color-accent);
    animation: pulse-dot 1.5s ease-in-out infinite;
}

@keyframes pulse-dot {
    0%,
    100% {
        opacity: 0.6;
        transform: scale(1);
    }
    50% {
        opacity: 1;
        transform: scale(1.2);
    }
}

/* Message Content */
.message-content {
    padding: 0.875rem 1rem;
    font-size: 0.875rem;
    line-height: 1.6;
    word-break: break-word;
    white-space: pre-wrap;
}

.user-content {
    background: linear-gradient(
        135deg,
        var(--color-accent) 0%,
        var(--color-primary) 100%
    );
    color: white;
    border-radius: var(--radius-md) var(--radius-md) var(--radius-sm)
        var(--radius-md);
    box-shadow: 0 2px 8px rgba(236, 72, 153, 0.2);
}

.assistant-content {
    background: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md) var(--radius-md) var(--radius-md)
        var(--radius-sm);
    color: var(--color-text);
    box-shadow: 0 1px 4px rgba(139, 92, 246, 0.04);
}

.tool-content {
    background: var(--color-warning-soft);
    border: 1px solid var(--color-warning);
    border-radius: var(--radius-md) var(--radius-md) var(--radius-md)
        var(--radius-sm);
    color: var(--color-warning-text);
    font-size: 0.8125rem;
}

.tool-id {
    font-family: monospace;
    font-size: 0.625rem;
    background: rgba(245, 158, 11, 0.2);
    padding: 0.125rem 0.4375rem;
    border-radius: var(--radius-sm);
    color: var(--color-warning-text);
}

/* System Message */
.message-system {
    justify-content: center;
    max-width: 100%;
}

.message-system .message-content {
    background: var(--color-info-soft);
    border: 1px dashed var(--color-info);
    border-radius: var(--radius-md);
    color: var(--color-info-text);
    font-size: 0.75rem;
    padding: 0.5rem 0.875rem;
    text-align: center;
    max-width: 400px;
    opacity: 0.9;
}

/* Tool Calls */
.tool-calls {
    margin-top: 0.5rem;
    border-top: 1px solid var(--color-border);
    padding-top: 0.5rem;
}

.tool-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--color-bg-mute);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0.5rem 0.625rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-text-secondary);
    cursor: pointer;
    transition: all var(--transition-fast);
}

.tool-toggle:hover {
    background: var(--color-bg-hover);
    border-color: var(--color-border-hover);
}

.tool-toggle .toggle-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    transition: transform var(--transition-fast);
}

.tool-toggle.expanded .toggle-icon {
    transform: rotate(180deg);
}

.tool-list {
    margin-top: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.tool-item {
    background: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0.5rem 0.625rem;
}

.tool-header {
    margin-bottom: 0.375rem;
}

.tool-badge {
    font-family: monospace;
    font-size: 0.71875rem;
    font-weight: 600;
    color: var(--color-accent);
    background: var(--color-accent-soft);
    padding: 0.125rem 0.4375rem;
    border-radius: var(--radius-sm);
}

.tool-args {
    font-family: monospace;
    font-size: 0.6875rem;
    color: var(--color-text-muted);
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 150px;
    overflow-y: auto;
    padding: 0.375rem;
    background: var(--color-bg-mute);
    border-radius: var(--radius-sm);
}
</style>

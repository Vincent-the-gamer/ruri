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
    <div class="message-wrapper">
        <!-- User Message -->
        <div v-if="isUser" class="message message-user">
            <div class="message-content">
                {{ message.content }}
            </div>
        </div>

        <!-- Assistant Message -->
        <div v-else-if="isAssistant" class="message message-assistant">
            <div class="message-header">
                <div class="role-icon">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path d="M12 8V4H8" />
                        <rect width="16" height="12" x="4" y="8" rx="2" />
                        <path d="M2 14h2" />
                        <path d="M20 14h2" />
                        <path d="M15 13v2" />
                        <path d="M9 13v2" />
                    </svg>
                </div>
                <span class="role-label">Assistant</span>
            </div>
            <div class="message-content">
                {{ message.content }}
            </div>
            <!-- Tool calls -->
            <div v-if="hasToolCalls" class="tool-calls">
                <button
                    @click="showToolCalls = !showToolCalls"
                    class="tool-calls-toggle"
                >
                    <svg
                        class="chevron"
                        :class="{ expanded: showToolCalls }"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path d="M9 18l6-6-6-6" />
                    </svg>
                    <span>Tool calls ({{ message.tool_calls!.length }})</span>
                </button>
                <div v-if="showToolCalls" class="tool-calls-list">
                    <div
                        v-for="tc in message.tool_calls"
                        :key="tc.id"
                        class="tool-call-item"
                    >
                        <div class="tool-call-name">{{ tc.function.name }}</div>
                        <pre class="tool-call-args">{{
                            formatArgs(tc.function.arguments)
                        }}</pre>
                    </div>
                </div>
            </div>
        </div>

        <!-- Tool Message -->
        <div v-else-if="isTool" class="message message-tool">
            <div class="message-header">
                <div class="role-icon">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path
                            d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"
                        />
                    </svg>
                </div>
                <span class="role-label">Tool</span>
                <span v-if="message.tool_call_id" class="tool-id">{{
                    message.tool_call_id
                }}</span>
            </div>
            <div class="message-content">
                {{ message.content }}
            </div>
        </div>

        <!-- System Message -->
        <div v-else-if="isSystem" class="message message-system">
            <div class="message-header">
                <div class="role-icon">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path
                            d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
                        />
                        <circle cx="12" cy="12" r="3" />
                    </svg>
                </div>
                <span class="role-label">System</span>
            </div>
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
    animation: fade-in 0.2s ease-out;
}

@keyframes fade-in {
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
    max-width: 85%;
    border-radius: var(--radius-lg);
}

/* User Message */
.message-user {
    margin-left: auto;
    margin-right: 0;
    background: var(--color-bg-soft);
    border-left: 3px solid var(--color-accent);
    padding: 12px 16px;
}

.message-user .message-content {
    color: var(--color-text);
}

/* Assistant Message */
.message-assistant {
    background: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    padding: 16px;
    margin-right: auto;
}

/* Tool Message */
.message-tool {
    background: var(--color-bg-mute);
    border: 1px solid var(--color-border);
    padding: 12px 16px;
    margin-right: auto;
    font-size: 0.875rem;
}

/* System Message */
.message-system {
    background: transparent;
    border: 1px dashed var(--color-border);
    padding: 8px 12px;
    margin-right: auto;
    font-size: 0.8125rem;
    opacity: 0.8;
}

/* Message Header */
.message-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
}

.role-icon {
    width: 16px;
    height: 16px;
    color: var(--color-text-secondary);
    flex-shrink: 0;
}

.role-icon svg {
    width: 100%;
    height: 100%;
}

.message-assistant .role-icon {
    color: var(--color-accent);
}

.message-tool .role-icon {
    color: var(--color-warning, #eab308);
}

.message-system .role-icon {
    color: var(--color-text-muted);
}

.role-label {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--color-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.message-tool .role-label {
    color: var(--color-warning, #eab308);
}

.message-system .role-label {
    color: var(--color-text-muted);
}

.tool-id {
    font-family: monospace;
    font-size: 0.6875rem;
    color: var(--color-text-muted);
    background: var(--color-bg-mute);
    padding: 2px 6px;
    border-radius: var(--radius-md);
    margin-left: auto;
}

/* Message Content */
.message-content {
    color: var(--color-text);
    white-space: pre-wrap;
    word-break: break-word;
    line-height: 1.6;
}

.message-tool .message-content {
    color: var(--color-text-secondary);
}

.message-system .message-content {
    color: var(--color-text-muted);
}

/* Tool Calls */
.tool-calls {
    margin-top: 12px;
    border-top: 1px solid var(--color-border);
    padding-top: 12px;
}

.tool-calls-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.75rem;
    color: var(--color-text-secondary);
    padding: 4px 0;
    transition: color 0.15s ease;
}

.tool-calls-toggle:hover {
    color: var(--color-accent);
}

.chevron {
    width: 14px;
    height: 14px;
    transition: transform 0.2s ease;
}

.chevron.expanded {
    transform: rotate(90deg);
}

.tool-calls-list {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.tool-call-item {
    background: var(--color-bg-mute);
    border-radius: var(--radius-md);
    padding: 10px 12px;
    border: 1px solid var(--color-border);
}

.tool-call-name {
    font-family: monospace;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-accent);
    margin-bottom: 6px;
}

.tool-call-args {
    font-family: monospace;
    font-size: 0.75rem;
    color: var(--color-text-muted);
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    overflow-x: auto;
}
</style>

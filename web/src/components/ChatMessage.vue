<script setup lang="ts">
import { ref, computed } from "vue";
import { marked } from "marked";
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

function renderMarkdown(content: string): string {
    try {
        return marked.parse(content, { async: false }) as string;
    } catch {
        return content;
    }
}
</script>

<template>
    <div class="message-wrapper">
        <!-- User Message -->
        <div v-if="isUser" class="message message-user">
            <div class="message-avatar">
                <svg class="avatar-icon" viewBox="0 0 24 24" fill="none">
                    <circle cx="12" cy="8" r="4" fill="hsl(var(--primary))" />
                    <path
                        d="M4 20c0-4.418 3.582-8 8-8s8 3.582 8 8"
                        stroke="hsl(var(--primary))"
                        stroke-width="2"
                        stroke-linecap="round"
                    />
                </svg>
            </div>
            <div class="message-content-wrapper">
                <div class="message-label">你</div>
                <div
                    class="message-content user-content"
                    v-html="renderMarkdown(message.content)"
                ></div>
            </div>
        </div>

        <!-- Assistant Message -->
        <div v-else-if="isAssistant" class="message message-assistant">
            <div class="message-avatar assistant-avatar">
                <svg class="avatar-icon" viewBox="0 0 24 24" fill="none">
                    <defs>
                        <linearGradient
                            id="crystal-gradient"
                            x1="0%"
                            y1="0%"
                            x2="100%"
                            y2="100%"
                        >
                            <stop
                                offset="0%"
                                stop-color="hsl(var(--primary))"
                            />
                            <stop offset="100%" stop-color="hsl(280 70% 60%)" />
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
                <div
                    class="message-content assistant-content"
                    v-html="renderMarkdown(message.content)"
                ></div>
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
                        stroke="hsl(38 92% 50%)"
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
                <div
                    class="message-content tool-content"
                    v-html="renderMarkdown(message.content)"
                ></div>
            </div>
        </div>

        <!-- System Message -->
        <div v-else-if="isSystem" class="message message-system">
            <div class="message-content">
                <div v-html="renderMarkdown(message.content)"></div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.message-wrapper {
    display: flex;
    width: 100%;
    margin-bottom: 1rem;
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
    gap: 0.625rem;
    max-width: 85%;
}

/* Avatar Styles */
.message-avatar {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    border-radius: 0.625rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
}

.message-user .message-avatar {
    order: 2;
}

.message-assistant .message-avatar {
    order: 1;
}

.message-tool .message-avatar {
    order: 1;
    background: hsl(38 92% 50% / 0.1);
    border-color: hsl(38 92% 50% / 0.3);
}

.avatar-icon {
    width: 18px;
    height: 18px;
}

.assistant-avatar {
    border: 2px solid hsl(var(--primary));
}

.tool-avatar .avatar-icon {
    width: 16px;
    height: 16px;
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
    gap: 0.25rem;
    flex: 1;
    order: 2;
}

.message-user .message-content-wrapper {
    align-items: flex-end;
}

/* Message Label */
.message-label {
    font-size: 0.625rem;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
    text-transform: uppercase;
    letter-spacing: 0.05em;
    display: flex;
    align-items: center;
    gap: 0.25rem;
}

.assistant-label {
    color: hsl(var(--primary));
}

.label-dot {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: hsl(var(--primary));
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
    padding: 0.75rem 1rem;
    font-size: 0.875rem;
    line-height: 1.6;
    word-break: break-word;
}

/* Markdown styles */
.message-content :deep(p) {
    margin: 0.25rem 0;
}

.message-content :deep(p:first-child) {
    margin-top: 0;
}

.message-content :deep(p:last-child) {
    margin-bottom: 0;
}

.message-content :deep(pre) {
    background: hsl(var(--muted) / 0.5);
    border: 1px solid hsl(var(--border));
    border-radius: 0.5rem;
    padding: 0.75rem;
    overflow-x: auto;
    margin: 0.5rem 0;
}

.message-content :deep(code) {
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;
    font-size: 0.8125rem;
}

.message-content :deep(:not(pre) > code) {
    background: hsl(var(--muted) / 0.5);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
}

.message-content :deep(pre code) {
    background: none;
    padding: 0;
}

.message-content :deep(ul),
.message-content :deep(ol) {
    margin: 0.25rem 0;
    padding-left: 1.5rem;
}

.message-content :deep(li) {
    margin: 0.125rem 0;
}

.message-content :deep(blockquote) {
    border-left: 3px solid hsl(var(--primary) / 0.5);
    padding-left: 0.75rem;
    margin: 0.5rem 0;
    color: hsl(var(--muted-foreground));
}

.message-content :deep(a) {
    color: hsl(var(--primary));
    text-decoration: underline;
    text-underline-offset: 2px;
}

.message-content :deep(a:hover) {
    text-decoration-thickness: 2px;
}

.message-content :deep(hr) {
    border: none;
    border-top: 1px solid hsl(var(--border));
    margin: 0.75rem 0;
}

.message-content :deep(table) {
    border-collapse: collapse;
    width: 100%;
    margin: 0.5rem 0;
}

.message-content :deep(th),
.message-content :deep(td) {
    border: 1px solid hsl(var(--border));
    padding: 0.375rem 0.625rem;
    text-align: left;
}

.message-content :deep(th) {
    background: hsl(var(--muted) / 0.3);
    font-weight: 600;
}

.message-content :deep(img) {
    max-width: 100%;
    border-radius: 0.5rem;
    margin: 0.25rem 0;
}

.message-content :deep(h1),
.message-content :deep(h2),
.message-content :deep(h3),
.message-content :deep(h4),
.message-content :deep(h5),
.message-content :deep(h6) {
    margin: 0.5rem 0 0.25rem;
    font-weight: 600;
    line-height: 1.3;
}

.message-content :deep(h1) {
    font-size: 1.25rem;
}

.message-content :deep(h2) {
    font-size: 1.125rem;
}

.message-content :deep(h3) {
    font-size: 1rem;
}

.user-content {
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(280 70% 60%) 100%
    );
    color: white;
    border-radius: 1rem 1rem 0.25rem 1rem;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.2);
}

.assistant-content {
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 1rem 1rem 1rem 0.25rem;
    color: hsl(var(--foreground));
    box-shadow: 0 1px 3px hsl(var(--primary) / 0.05);
}

.tool-content {
    background: hsl(38 92% 50% / 0.1);
    border: 1px solid hsl(38 92% 50% / 0.3);
    border-radius: 1rem 1rem 1rem 0.25rem;
    color: hsl(38 92% 30%);
    font-size: 0.8125rem;
}

.dark .tool-content {
    color: hsl(38 92% 70%);
}

.tool-id {
    font-family: monospace;
    font-size: 0.5625rem;
    background: hsl(38 92% 50% / 0.2);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    color: hsl(38 92% 30%);
}

.dark .tool-id {
    color: hsl(38 92% 70%);
}

/* System Message */
.message-system {
    justify-content: center;
    max-width: 100%;
}

.message-system .message-content {
    background: hsl(200 80% 50% / 0.1);
    border: 1px dashed hsl(200 80% 50% / 0.4);
    border-radius: 0.625rem;
    color: hsl(200 80% 40%);
    font-size: 0.75rem;
    padding: 0.5rem 0.75rem;
    text-align: center;
    max-width: 400px;
    opacity: 0.9;
}

.dark .message-system .message-content {
    color: hsl(200 80% 70%);
    border-color: hsl(200 80% 50% / 0.5);
}

/* Tool Calls */
.tool-calls {
    margin-top: 0.375rem;
    border-top: 1px solid hsl(var(--border));
    padding-top: 0.375rem;
}

.tool-toggle {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
    border-radius: 0.375rem;
    padding: 0.375rem 0.5rem;
    font-size: 0.6875rem;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s ease;
}

.tool-toggle:hover {
    background: hsl(var(--accent));
    border-color: hsl(var(--primary) / 0.3);
    color: hsl(var(--foreground));
}

.tool-toggle .toggle-icon {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
    transition: transform 0.2s ease;
}

.tool-toggle.expanded .toggle-icon {
    transform: rotate(180deg);
}

.tool-list {
    margin-top: 0.375rem;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
}

.tool-item {
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
    border-radius: 0.375rem;
    padding: 0.375rem 0.5rem;
}

.tool-header {
    margin-bottom: 0.25rem;
}

.tool-badge {
    font-family: monospace;
    font-size: 0.625rem;
    font-weight: 600;
    color: hsl(var(--primary));
    background: hsl(var(--primary) / 0.1);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
}

.tool-args {
    font-family: monospace;
    font-size: 0.625rem;
    color: hsl(var(--muted-foreground));
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 120px;
    overflow-y: auto;
    padding: 0.25rem;
    background: hsl(var(--background));
    border-radius: 0.25rem;
    border: 1px solid hsl(var(--border));
}
</style>

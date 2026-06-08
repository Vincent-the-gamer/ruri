<script setup lang="ts">
import { ref, computed } from "vue";
import { marked } from "marked";
import type { ChatMessage as ChatMessageType, ContentPart } from "../types";
import { useChatStore } from "../stores/chat";
import ruriAvatar from "../../assets/ruri-avatar.png";

const props = defineProps<{
    message: ChatMessageType;
}>();

const chatStore = useChatStore();
const showToolCalls = ref(false);
const copied = ref(false);

function getMessageText(content: string | ContentPart[]): string {
    if (typeof content === "string") return content;
    return content
        .filter((p) => p.type === "text" && p.text)
        .map((p) => p.text!)
        .join("\n");
}

async function copyToClipboard() {
    const text = getMessageText(props.message.content);
    if (!text) return;
    try {
        await navigator.clipboard.writeText(text);
        copied.value = true;
        setTimeout(() => {
            copied.value = false;
        }, 2000);
    } catch {
        // Fallback for older browsers
        const textarea = document.createElement("textarea");
        textarea.value = text;
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        document.body.removeChild(textarea);
        copied.value = true;
        setTimeout(() => {
            copied.value = false;
        }, 2000);
    }
}

const isUser = computed(() => props.message.role === "user");
const isAssistant = computed(() => props.message.role === "assistant");
const isTool = computed(() => props.message.role === "tool");
const isSystem = computed(() => props.message.role === "system");

const hasToolCalls =
    props.message.tool_calls && props.message.tool_calls.length > 0;

/** Whether this assistant message is currently being streamed */
const isCurrentlyStreaming = computed(
    () =>
        isAssistant.value &&
        chatStore.isStreaming &&
        chatStore.streamingContent !== "" &&
        chatStore.messages[chatStore.messages.length - 1] === props.message,
);

/** Whether this message has no visible content and should be hidden */
const hasContent = computed(() => {
    const c = props.message.content;
    if (typeof c === "string") return c.trim().length > 0;
    if (Array.isArray(c)) return c.length > 0;
    return false;
});

function formatArgs(args: string): string {
    try {
        return JSON.stringify(JSON.parse(args), null, 2);
    } catch {
        return args;
    }
}

function renderMarkdown(
    content: string | import("../types").ContentPart[],
): string {
    const text =
        typeof content === "string"
            ? content
            : content
                  .filter((p) => p.type === "text" && p.text)
                  .map((p) => p.text!)
                  .join("\n");
    try {
        return marked.parse(text, { async: false }) as string;
    } catch {
        return text;
    }
}
</script>

<template>
    <div
        v-if="hasContent || hasToolCalls || isCurrentlyStreaming"
        class="message-wrapper"
        :class="{
            'message-wrapper-user': isUser,
            'message-wrapper-assistant': isAssistant,
        }"
    >
        <!-- User Message -->
        <div v-if="isUser" class="message message-user">
            <div class="message-content-wrapper user-content-wrapper">
                <div class="message-label">
                    你
                    <button
                        class="copy-btn"
                        @click.stop="copyToClipboard"
                        :title="copied ? '已复制' : '复制文本'"
                    >
                        <svg
                            v-if="!copied"
                            class="copy-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <rect
                                x="9"
                                y="9"
                                width="13"
                                height="13"
                                rx="2"
                                ry="2"
                            />
                            <path
                                d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
                            />
                        </svg>
                        <svg
                            v-else
                            class="copy-icon copied"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <polyline points="20 6 9 17 4 12" />
                        </svg>
                    </button>
                </div>
                <div class="message-content user-content">
                    <template v-if="Array.isArray(message.content)">
                        <template
                            v-for="(part, idx) in message.content"
                            :key="idx"
                        >
                            <img
                                v-if="
                                    part.type === 'image_url' && part.image_url
                                "
                                :src="part.image_url.url"
                                class="chat-image"
                                alt="Attached image"
                            />
                            <div
                                v-else-if="part.type === 'text' && part.text"
                                :class="{
                                    'file-content-block':
                                        part.text.startsWith('--- File:'),
                                    'file-badge': part.text.startsWith('📎'),
                                }"
                            >
                                <template
                                    v-if="part.text.startsWith('--- File:')"
                                >
                                    <div class="file-content-header">
                                        <svg
                                            class="file-icon-inline"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        >
                                            <path
                                                d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                                            />
                                            <polyline points="14 2 14 8 20 8" />
                                        </svg>
                                        <span class="file-name-inline">{{
                                            part.text
                                                .split("\n")[0]
                                                .replace("--- File: ", "")
                                                .replace(" ---", "")
                                        }}</span>
                                    </div>
                                    <pre class="file-content-pre">{{
                                        part.text
                                            .split("\n")
                                            .slice(1)
                                            .join("\n")
                                    }}</pre>
                                </template>
                                <template
                                    v-else-if="part.text.startsWith('🎵')"
                                >
                                    <div class="audio-badge">
                                        <svg
                                            class="file-icon-inline"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        >
                                            <path d="M9 18V5l12-2v13" />
                                            <circle cx="6" cy="18" r="3" />
                                            <circle cx="18" cy="16" r="3" />
                                        </svg>
                                        {{ part.text.replace("🎵 ", "") }}
                                    </div>
                                </template>
                                <template
                                    v-else-if="part.text.startsWith('📎')"
                                >
                                    <svg
                                        class="file-icon-inline"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <path
                                            d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                                        />
                                        <polyline points="14 2 14 8 20 8" />
                                    </svg>
                                    {{ part.text.replace("📎 ", "") }}
                                </template>
                                <template v-else>
                                    <div
                                        v-html="renderMarkdown(part.text)"
                                    ></div>
                                </template>
                            </div>
                        </template>
                    </template>
                    <div v-else v-html="renderMarkdown(message.content)"></div>
                </div>
            </div>
            <div class="message-avatar user-avatar">
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
        </div>

        <!-- Assistant Message -->
        <div v-else-if="isAssistant" class="message message-assistant">
            <div class="message-avatar assistant-avatar">
                <img :src="ruriAvatar" alt="琉璃" class="avatar-img" />
            </div>
            <div class="message-content-wrapper">
                <div class="message-label assistant-label">
                    <span>琉璃</span>
                    <span class="label-dot"></span>
                    <button
                        class="copy-btn"
                        @click.stop="copyToClipboard"
                        :title="copied ? '已复制' : '复制文本'"
                    >
                        <svg
                            v-if="!copied"
                            class="copy-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <rect
                                x="9"
                                y="9"
                                width="13"
                                height="13"
                                rx="2"
                                ry="2"
                            />
                            <path
                                d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
                            />
                        </svg>
                        <svg
                            v-else
                            class="copy-icon copied"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <polyline points="20 6 9 17 4 12" />
                        </svg>
                    </button>
                </div>
                <!-- Tool calls (process) — shown before the response text (result) -->
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
                <!-- Response text (result) — shown after the tool calls (process) -->
                <div class="message-content assistant-content">
                    <template v-if="Array.isArray(message.content)">
                        <template
                            v-for="(part, idx) in message.content"
                            :key="idx"
                        >
                            <img
                                v-if="
                                    part.type === 'image_url' && part.image_url
                                "
                                :src="part.image_url.url"
                                class="chat-image"
                                alt="Image"
                            />
                            <div
                                v-else-if="part.type === 'text' && part.text"
                                v-html="renderMarkdown(part.text)"
                            ></div>
                        </template>
                    </template>
                    <div v-else v-html="renderMarkdown(message.content)"></div>
                    <span
                        v-if="isCurrentlyStreaming"
                        class="streaming-cursor"
                    ></span>

                    <!-- Tool result footnotes: shown inline below the message -->
                    <div
                        v-if="
                            (message as any)._tool_results &&
                            (message as any)._tool_results.length > 0
                        "
                        class="tool-results-footnote"
                    >
                        <details>
                            <summary class="tool-results-summary">
                                <span class="footnote-icon">📎</span>
                                <span
                                    >工具调用结果（{{
                                        (message as any)._tool_results.length
                                    }}）</span
                                >
                            </summary>
                            <div
                                v-for="(tr, trIdx) in (message as any)
                                    ._tool_results"
                                :key="trIdx"
                                class="tool-result-item"
                            >
                                <div class="tool-result-header">
                                    <span class="tool-badge-inline">{{
                                        tr.tool_name
                                    }}</span>
                                </div>
                                <pre class="tool-result-content">{{
                                    tr.content
                                }}</pre>
                            </div>
                        </details>
                    </div>
                </div>
            </div>
        </div>

        <!-- Tool Message -->
        <div
            v-else-if="isTool"
            class="message message-tool"
            :data-inline="(message as any)._inline ? 'true' : undefined"
        >
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
                    <span v-if="message.tool_name" class="tool-result-name"
                        >🔧 {{ message.tool_name }}</span
                    >
                    <span v-else>Tool</span>
                    <span v-if="message.tool_call_id" class="tool-id">{{
                        message.tool_call_id
                    }}</span>
                    <button
                        class="copy-btn"
                        @click.stop="copyToClipboard"
                        :title="copied ? 'Copied' : 'Copy'"
                    >
                        <svg
                            v-if="!copied"
                            class="copy-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <rect
                                x="9"
                                y="9"
                                width="13"
                                height="13"
                                rx="2"
                                ry="2"
                            />
                            <path
                                d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
                            />
                        </svg>
                        <svg
                            v-else
                            class="copy-icon copied"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <polyline points="20 6 9 17 4 12" />
                        </svg>
                    </button>
                </div>
                <div class="message-content tool-content">
                    <div v-html="renderMarkdown(message.content)"></div>
                </div>
            </div>
        </div>

        <!-- System Message -->
        <div v-else-if="isSystem" class="message message-system">
            <div class="message-content">
                <div v-html="renderMarkdown(message.content)"></div>
                <button
                    class="copy-btn"
                    @click.stop="copyToClipboard"
                    :title="copied ? 'Copied' : 'Copy'"
                >
                    <svg
                        v-if="!copied"
                        class="copy-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <rect
                            x="9"
                            y="9"
                            width="13"
                            height="13"
                            rx="2"
                            ry="2"
                        />
                        <path
                            d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
                        />
                    </svg>
                    <svg
                        v-else
                        class="copy-icon copied"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <polyline points="20 6 9 17 4 12" />
                    </svg>
                </button>
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

.message-wrapper-user {
    justify-content: flex-end;
}

.message-wrapper-assistant {
    justify-content: flex-start;
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
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
    overflow: hidden;
}

.avatar-icon {
    width: 18px;
    height: 18px;
}

.avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.user-avatar {
    /* No order override — avatar naturally appears after content (right side) */
}

.assistant-avatar {
    /* No order override — avatar naturally appears before content (left side) */
    border: 2px solid hsl(var(--primary));
}

.tool-avatar {
    /* No order override — avatar naturally appears before content (left side) */
    background: hsl(38 92% 50% / 0.1);
    border-color: hsl(38 92% 50% / 0.3);
    border-radius: 0.625rem;
}

.tool-avatar .avatar-icon {
    width: 16px;
    height: 16px;
}

/* Message Content Wrapper */
.message-content-wrapper {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    flex: 1;
}

.user-content-wrapper {
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
    border-radius: 1rem 0.25rem 1rem 1rem;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.2);
}

.assistant-content {
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 0.25rem 1rem 1rem 1rem;
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
.tool-executing {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    opacity: 0.8;
    animation: pulse 1.5s ease-in-out infinite;
}
.tool-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid hsl(38 92% 50% / 0.3);
    border-top-color: hsl(38 92% 50%);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    flex-shrink: 0;
}
@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}
@keyframes pulse {
    0%,
    100% {
        opacity: 0.8;
    }
    50% {
        opacity: 0.5;
    }
}

.chat-image {
    max-width: 280px;
    max-height: 280px;
    border-radius: 0.5rem;
    margin: 0.25rem 0;
    object-fit: contain;
    cursor: pointer;
    transition: transform 0.2s ease;
}

.chat-image:hover {
    transform: scale(1.02);
}

/* File content block (text files shown inline) */
.file-content-block {
    background: hsl(var(--foreground) / 0.08);
    border-radius: 0.5rem;
    margin: 0.375rem 0;
    overflow: hidden;
    max-width: 100%;
}

.dark .file-content-block {
    background: hsl(var(--foreground) / 0.06);
}

.file-content-header {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 0.75rem;
    background: hsl(var(--foreground) / 0.06);
    border-bottom: 1px solid hsl(var(--foreground) / 0.08);
    font-size: 0.75rem;
    color: hsl(var(--foreground) / 0.7);
}

.dark .file-content-header {
    background: hsl(var(--foreground) / 0.08);
    border-bottom-color: hsl(var(--foreground) / 0.06);
}

.file-icon-inline {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: hsl(var(--primary));
}

.file-name-inline {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.file-content-pre {
    margin: 0;
    padding: 0.5rem 0.75rem;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;
    font-size: 0.75rem;
    line-height: 1.5;
    color: hsl(var(--foreground) / 0.85);
    overflow-x: auto;
    max-height: 300px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
}

/* File badge (binary file attachment indicator) */
.file-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.75rem;
    background: hsl(var(--foreground) / 0.08);
    border-radius: 0.5rem;
    font-size: 0.8125rem;
    color: hsl(var(--foreground) / 0.8);
    margin: 0.25rem 0;
}

.audio-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.75rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.1) 0%,
        hsl(280 70% 60% / 0.1) 100%
    );
    border: 1px solid hsl(var(--primary) / 0.2);
    border-radius: 0.5rem;
    font-size: 0.8125rem;
    color: hsl(var(--primary));
    margin: 0.25rem 0;
}

.dark .file-badge {
    background: hsl(var(--foreground) / 0.06);
}

.file-badge .file-icon-inline {
    color: hsl(var(--muted-foreground));
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

.tool-result-name {
    font-weight: 600;
    color: hsl(38 92% 40%);
}

.dark .tool-result-name {
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

/* Streaming cursor */
.streaming-cursor {
    display: inline-block;
    width: 2px;
    height: 1em;
    background: hsl(var(--primary));
    margin-left: 2px;
    vertical-align: text-bottom;
    animation: blink-cursor 1s step-end infinite;
}

@keyframes blink-cursor {
    0%,
    100% {
        opacity: 1;
    }
    50% {
        opacity: 0;
    }
}

/* Copy button */
.copy-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: none;
    border-radius: 0.25rem;
    background: transparent;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    opacity: 0;
    transition:
        opacity 0.2s ease,
        background 0.2s ease;
    padding: 0;
    flex-shrink: 0;
    vertical-align: middle;
}

.message-wrapper:hover .copy-btn {
    opacity: 1;
}

.copy-btn:hover {
    background: hsl(var(--foreground) / 0.1);
    color: hsl(var(--foreground));
}

.copy-btn:active {
    transform: scale(0.9);
}

.copy-icon {
    width: 12px;
    height: 12px;
}

.copy-icon.copied {
    color: hsl(142 70% 45%);
}

/* ── Tool result footnotes (inline in assistant messages) ────── */
.tool-results-footnote {
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px dashed hsl(var(--border) / 0.6);
}

.tool-results-summary {
    cursor: pointer;
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0;
    user-select: none;
    list-style: none;
}

.tool-results-summary::-webkit-details-marker {
    display: none;
}

.tool-results-summary:hover {
    color: hsl(var(--foreground) / 0.8);
}

.footnote-icon {
    font-size: 0.75rem;
}

.tool-result-item {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background: hsl(var(--background) / 0.5);
    border-radius: 0.375rem;
    border: 1px solid hsl(var(--border) / 0.5);
}

.tool-result-item + .tool-result-item {
    margin-top: 0.375rem;
}

.tool-result-header {
    margin-bottom: 0.25rem;
}

.tool-badge-inline {
    font-family: monospace;
    font-size: 0.625rem;
    font-weight: 600;
    color: hsl(var(--primary));
    background: hsl(var(--primary) / 0.1);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
}

.tool-result-content {
    font-family: monospace;
    font-size: 0.7rem;
    color: hsl(var(--muted-foreground));
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 300px;
    overflow-y: auto;
    padding: 0.375rem;
    background: hsl(var(--background));
    border-radius: 0.25rem;
    border: 1px solid hsl(var(--border));
}

/* Hide inline tool status markers */
.message-tool[data-inline="true"],
.message-wrapper:has(.message-tool[data-inline="true"]) {
    display: none;
}
</style>

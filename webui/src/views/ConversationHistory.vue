<script setup lang="ts">
import { ref, onMounted, computed, nextTick } from "vue";
import { useI18n } from "vue-i18n";
import {
    listConversations,
    deleteConversation,
    getConversationMessages,
    type Conversation,
    type ConversationFilter,
    type Message,
} from "../api/conversations";
import { Icon } from "@iconify/vue";
import { marked } from "marked";

const { t } = useI18n();

// ── localStorage cache helpers for conversation messages ──
const CONV_MESSAGES_CACHE_KEY = "ruri_conv_messages_cache";
const CONV_MESSAGES_CACHE_TTL = 5 * 60 * 1000; // 5 minutes

interface CachedMessages {
    messages: Message[];
    timestamp: number;
}

function saveConvMessagesToCache(conversationId: string, messages: Message[]) {
    try {
        const cache = loadAllConvMessagesCache();
        cache[conversationId] = { messages, timestamp: Date.now() };
        // Prune expired entries
        const now = Date.now();
        for (const [key, val] of Object.entries(cache)) {
            if (now - val.timestamp > CONV_MESSAGES_CACHE_TTL) {
                delete cache[key];
            }
        }
        localStorage.setItem(CONV_MESSAGES_CACHE_KEY, JSON.stringify(cache));
    } catch {
        // localStorage might be full
    }
}

function loadAllConvMessagesCache(): Record<string, CachedMessages> {
    try {
        const raw = localStorage.getItem(CONV_MESSAGES_CACHE_KEY);
        if (raw) return JSON.parse(raw);
    } catch {
        // ignore
    }
    return {};
}

function loadConvMessagesFromCache(conversationId: string): Message[] | null {
    const cache = loadAllConvMessagesCache();
    const entry = cache[conversationId];
    if (entry && Date.now() - entry.timestamp < CONV_MESSAGES_CACHE_TTL) {
        return entry.messages;
    }
    return null;
}

// 对话列表
const conversations = ref<Conversation[]>([]);
// 加载状态
const loading = ref(false);
// 错误信息
const error = ref<string | null>(null);

// 对话预览消息 (keyed by conversation id)
const previewMessages = ref<Record<string, Message[]>>({});

// 筛选条件
const filter = ref<ConversationFilter>({
    bot_name: "",
    chat_type: undefined,
    keyword: "",
});

// 对话类型选项
const chatTypeOptions = [
    { value: "", label: t("conversationHistory.all") },
    { value: "group", label: t("conversationHistory.chatTypeGroup") },
    { value: "private", label: t("conversationHistory.chatTypePrivate") },
];

// ======== 详情面板状态 ========
const detailOpen = ref(false);
const detailConversation = ref<Conversation | null>(null);
const detailMessages = ref<Message[]>([]);
const detailLoading = ref(false);
const detailError = ref<string | null>(null);
const messagesContainer = ref<HTMLElement | null>(null);

// 加载对话列表
async function loadConversations() {
    loading.value = true;
    error.value = null;

    try {
        conversations.value = await listConversations(filter.value);
        // Load preview messages for all conversations (cache-first)
        await loadPreviewMessages();
    } catch (err: any) {
        console.error("Failed to load conversations:", err);
        error.value =
            err.response?.data?.error || err.message || t("common.error");
    } finally {
        loading.value = false;
    }
}

// Load preview messages for all conversations (cache-first strategy)
async function loadPreviewMessages() {
    const BATCH_SIZE = 5; // Fetch max 5 conversations' messages concurrently
    const ids = conversations.value.map((c) => c.id);

    for (let i = 0; i < ids.length; i += BATCH_SIZE) {
        const batch = ids.slice(i, i + BATCH_SIZE);
        const promises = batch.map(async (convId) => {
            // 1. Try cache first
            const cached = loadConvMessagesFromCache(convId);
            if (cached) {
                previewMessages.value[convId] = cached;
                return;
            }
            // 2. Fetch from server
            try {
                const msgs = await getConversationMessages(convId);
                previewMessages.value[convId] = msgs;
                saveConvMessagesToCache(convId, msgs);
            } catch {
                previewMessages.value[convId] = [];
            }
        });
        await Promise.all(promises);
    }
}

// Get preview text for a conversation (first few messages)
function getConversationPreview(conversationId: string): string {
    const msgs = previewMessages.value[conversationId];
    if (!msgs || msgs.length === 0) return "";

    // Show first 3 non-system messages as a preview
    const displayMsgs = msgs.filter((m) => m.role !== "system").slice(0, 3);

    return displayMsgs
        .map((m) => {
            const prefix =
                m.role === "user" ? "👤" : m.role === "assistant" ? "🤖" : "🔧";
            const text = typeof m.content === "string" ? m.content : "";
            // Truncate long content
            const truncated =
                text.length > 100 ? text.slice(0, 100) + "..." : text;
            return `${prefix} ${truncated}`;
        })
        .join("\n");
}

// Get message count for a conversation
function getMessageCount(conversationId: string): number {
    return previewMessages.value[conversationId]?.length ?? 0;
}

// 应用筛选
function applyFilter() {
    loadConversations();
}

// 重置筛选
function resetFilter() {
    filter.value = {
        bot_name: "",
        chat_type: undefined,
        keyword: "",
    };
    loadConversations();
}

// 删除对话
async function handleDeleteConversation(id: string) {
    if (!confirm(t("conversationHistory.confirmDelete"))) {
        return;
    }

    try {
        await deleteConversation(id);
        // 如果删除的是当前正在查看的对话，关闭面板
        if (detailConversation.value?.id === id) {
            closeDetail();
        }
        await loadConversations();
    } catch (err: any) {
        console.error("Failed to delete conversation:", err);
        error.value =
            err.response?.data?.error || err.message || t("common.error");
    }
}

// 打开对话详情面板
async function openConversationDetail(conversation: Conversation) {
    detailConversation.value = conversation;
    detailMessages.value = [];
    detailError.value = null;
    detailOpen.value = true;
    detailLoading.value = true;

    // 1. Try to show from preview cache first for instant display
    const cached = previewMessages.value[conversation.id];
    if (cached && cached.length > 0) {
        detailMessages.value = cached;
        detailLoading.value = false;
        await nextTick();
        scrollToBottom();
        return;
    }

    // 2. Fallback: fetch from server
    try {
        const messages = await getConversationMessages(conversation.id);
        detailMessages.value = messages;
        // Update cache
        previewMessages.value[conversation.id] = messages;
        saveConvMessagesToCache(conversation.id, messages);
        // 等待 DOM 更新后滚动到底部
        await nextTick();
        scrollToBottom();
    } catch (err: any) {
        console.error("Failed to load messages:", err);
        detailError.value =
            err.response?.data?.error || err.message || t("common.error");
    } finally {
        detailLoading.value = false;
    }
}

// 关闭详情面板
function closeDetail() {
    detailOpen.value = false;
    // 延迟清除数据，等动画结束
    setTimeout(() => {
        detailConversation.value = null;
        detailMessages.value = [];
        detailError.value = null;
    }, 300);
}

// 滚动到底部
function scrollToBottom() {
    if (messagesContainer.value) {
        messagesContainer.value.scrollTop =
            messagesContainer.value.scrollHeight;
    }
}

// 格式化日期时间
function formatDateTime(dateString: string) {
    return new Date(dateString).toLocaleString();
}

// 格式化时间（短格式，用于消息时间戳）
function formatTime(dateString: string) {
    return new Date(dateString).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
    });
}

// 聊天类型显示文本
const chatTypeLabel = computed(() => (type: string) => {
    if (type === "group") {
        return t("conversationHistory.chatTypeGroup");
    } else if (type === "private") {
        return t("conversationHistory.chatTypePrivate");
    }
    return type;
});

// 角色标签
function roleLabel(role: string): string {
    switch (role) {
        case "user":
            return t("conversationHistory.roleUser");
        case "assistant":
            return t("conversationHistory.roleAssistant");
        case "system":
            return t("conversationHistory.roleSystem");
        case "tool":
            return t("conversationHistory.roleTool");
        default:
            return role;
    }
}

// 渲染 Markdown
function renderMarkdown(content: string): string {
    try {
        return marked.parse(content, { async: false }) as string;
    } catch {
        return content;
    }
}

// 页面加载时获取对话列表
onMounted(() => {
    loadConversations();
});
</script>

<template>
    <div class="conversation-history-view">
        <!-- Header -->
        <div class="page-header">
            <div class="header-content">
                <div class="header-icon">
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="24"
                        height="24"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path d="M12 8v4l3 3" />
                        <circle cx="12" cy="12" r="10" />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">
                        {{ t("conversationHistory.title") }}
                    </h1>
                    <p class="header-desc">
                        {{ t("conversationHistory.subtitle") }}
                    </p>
                </div>
            </div>
        </div>

        <!-- 筛选工具栏 -->
        <div class="filter-toolbar">
            <div class="filter-item">
                <label for="bot-name">{{
                    t("conversationHistory.botName")
                }}</label>
                <input
                    id="bot-name"
                    v-model="filter.bot_name"
                    type="text"
                    :placeholder="t('conversationHistory.botNamePlaceholder')"
                    @keyup.enter="applyFilter"
                />
            </div>

            <div class="filter-item">
                <label for="chat-type">{{
                    t("conversationHistory.chatType")
                }}</label>
                <select
                    id="chat-type"
                    v-model="filter.chat_type"
                    @change="applyFilter"
                >
                    <option
                        v-for="option in chatTypeOptions"
                        :key="option.value"
                        :value="option.value || undefined"
                    >
                        {{ option.label }}
                    </option>
                </select>
            </div>

            <div class="filter-item">
                <label for="keyword">{{
                    t("conversationHistory.keyword")
                }}</label>
                <input
                    id="keyword"
                    v-model="filter.keyword"
                    type="text"
                    :placeholder="t('conversationHistory.keywordPlaceholder')"
                    @keyup.enter="applyFilter"
                />
            </div>

            <div class="filter-actions">
                <button class="btn btn-primary" @click="applyFilter">
                    <Icon icon="lucide:search" class="icon" />
                    {{ t("conversationHistory.search") }}
                </button>
                <button class="btn btn-secondary" @click="resetFilter">
                    <Icon icon="lucide:rotate-ccw" class="icon" />
                    {{ t("conversationHistory.reset") }}
                </button>
            </div>
        </div>

        <!-- 错误提示 -->
        <div v-if="error" class="error-banner">
            <Icon icon="lucide:alert-circle" class="icon" />
            <span>{{ error }}</span>
        </div>

        <!-- 加载状态 -->
        <div v-if="loading && conversations.length === 0" class="loading-state">
            <Icon icon="lucide:loader-2" class="icon spin" />
            <span>{{ t("common.loading") }}</span>
        </div>

        <!-- 对话卡片列表 -->
        <div v-else-if="conversations.length > 0" class="conversation-cards">
            <div
                v-for="conversation in conversations"
                :key="conversation.id"
                class="conversation-card"
                @click="openConversationDetail(conversation)"
            >
                <!-- 卡片头部：标题 + 元信息 -->
                <div class="card-header">
                    <div class="card-title-row">
                        <h3 class="card-title">
                            {{
                                conversation.title ||
                                t("conversationHistory.noTitle")
                            }}
                        </h3>
                        <div class="card-actions" @click.stop>
                            <button
                                class="btn-icon btn-view"
                                :title="t('conversationHistory.view')"
                                @click="openConversationDetail(conversation)"
                            >
                                <Icon icon="lucide:eye" />
                            </button>
                            <button
                                class="btn-icon btn-delete"
                                :title="t('conversationHistory.delete')"
                                @click="
                                    handleDeleteConversation(conversation.id)
                                "
                            >
                                <Icon icon="lucide:trash-2" />
                            </button>
                        </div>
                    </div>
                    <div class="card-meta">
                        <span class="meta-tag">
                            <Icon icon="lucide:bot" class="meta-icon" />
                            {{ conversation.bot_name }}
                        </span>
                        <span
                            class="chat-type-badge"
                            :class="`type-${conversation.chat_type}`"
                        >
                            {{ chatTypeLabel(conversation.chat_type) }}
                        </span>
                        <span class="meta-tag">
                            <Icon icon="lucide:mail" class="meta-icon" />
                            {{ getMessageCount(conversation.id) }}
                            {{ t("conversationHistory.messages") }}
                        </span>
                        <span class="meta-tag">
                            <Icon icon="lucide:clock" class="meta-icon" />
                            {{ formatDateTime(conversation.updated_at) }}
                        </span>
                    </div>
                </div>

                <!-- 卡片内容：消息预览 -->
                <div class="card-preview">
                    <template v-if="getConversationPreview(conversation.id)">
                        <div
                            v-for="(line, idx) in getConversationPreview(
                                conversation.id,
                            ).split('\n')"
                            :key="idx"
                            class="preview-line"
                            :class="{
                                'preview-user': line.startsWith('👤'),
                                'preview-assistant': line.startsWith('🤖'),
                                'preview-tool': line.startsWith('🔧'),
                            }"
                        >
                            {{ line }}
                        </div>
                    </template>
                    <div v-else class="preview-empty">
                        {{ t("conversationHistory.noMessages") }}
                    </div>
                </div>

                <!-- 卡片底部：查看全部 -->
                <div class="card-footer">
                    <span class="view-all-link">
                        <Icon icon="lucide:arrow-right" class="meta-icon" />
                        {{ t("conversationHistory.view") }}
                    </span>
                </div>
            </div>
        </div>

        <!-- 空状态 -->
        <div v-else class="empty-state">
            <Icon icon="lucide:message-square-off" class="icon" />
            <h2>{{ t("conversationHistory.emptyTitle") }}</h2>
            <p>{{ t("conversationHistory.emptyDescription") }}</p>
        </div>

        <!-- 遮罩层 -->
        <Transition name="overlay">
            <div
                v-if="detailOpen"
                class="detail-overlay"
                @click="closeDetail"
            ></div>
        </Transition>

        <!-- 详情侧边面板 -->
        <Transition name="slide-over">
            <div v-if="detailOpen" class="detail-panel">
                <!-- 面板头部 -->
                <div class="detail-header">
                    <div class="detail-header-info">
                        <h2 class="detail-title">
                            {{
                                detailConversation?.title ||
                                t("conversationHistory.noTitle")
                            }}
                        </h2>
                        <div class="detail-meta">
                            <span
                                v-if="detailConversation?.bot_name"
                                class="detail-meta-item"
                            >
                                <Icon icon="lucide:bot" class="meta-icon" />
                                {{ detailConversation.bot_name }}
                            </span>
                            <span
                                v-if="detailConversation?.chat_type"
                                class="detail-meta-item"
                            >
                                <Icon
                                    icon="lucide:message-circle"
                                    class="meta-icon"
                                />
                                {{
                                    chatTypeLabel(detailConversation.chat_type)
                                }}
                            </span>
                            <span class="detail-meta-item">
                                <Icon icon="lucide:mail" class="meta-icon" />
                                {{ detailMessages.length }}
                                {{ t("conversationHistory.messages") }}
                            </span>
                        </div>
                    </div>
                    <button
                        class="btn-icon btn-close"
                        :title="t('conversationHistory.close')"
                        @click="closeDetail"
                    >
                        <Icon icon="lucide:x" />
                    </button>
                </div>

                <!-- 加载状态 -->
                <div v-if="detailLoading" class="detail-loading">
                    <Icon icon="lucide:loader-2" class="icon spin" />
                    <span>{{ t("conversationHistory.loadingMessages") }}</span>
                </div>

                <!-- 错误提示 -->
                <div v-else-if="detailError" class="detail-error">
                    <Icon icon="lucide:alert-circle" class="icon" />
                    <span>{{ detailError }}</span>
                </div>

                <!-- 无消息 -->
                <div
                    v-else-if="detailMessages.length === 0"
                    class="detail-empty"
                >
                    <Icon icon="lucide:message-square-off" class="icon" />
                    <span>{{ t("conversationHistory.noMessages") }}</span>
                </div>

                <!-- 消息列表 -->
                <div v-else ref="messagesContainer" class="detail-messages">
                    <div
                        v-for="message in detailMessages"
                        :key="message.id"
                        class="detail-message"
                        :class="`detail-message-${message.role}`"
                    >
                        <!-- 用户消息 -->
                        <div
                            v-if="message.role === 'user'"
                            class="msg msg-user"
                        >
                            <div class="msg-body msg-body-user">
                                <div class="msg-header msg-header-user">
                                    <span class="msg-role">{{
                                        roleLabel(message.role)
                                    }}</span>
                                    <span class="msg-time">{{
                                        formatTime(message.created_at)
                                    }}</span>
                                </div>
                                <div
                                    class="msg-content msg-content-user"
                                    v-html="renderMarkdown(message.content)"
                                ></div>
                            </div>
                            <div class="msg-avatar avatar-user">
                                <svg
                                    class="avatar-icon"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                >
                                    <circle
                                        cx="12"
                                        cy="8"
                                        r="4"
                                        fill="hsl(var(--primary))"
                                    />
                                    <path
                                        d="M4 20c0-4.418 3.582-8 8-8s8 3.582 8 8"
                                        stroke="hsl(var(--primary))"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                    />
                                </svg>
                            </div>
                        </div>

                        <!-- 助手消息 -->
                        <div
                            v-else-if="message.role === 'assistant'"
                            class="msg msg-assistant"
                        >
                            <div class="msg-avatar avatar-assistant">
                                <Icon
                                    icon="lucide:sparkles"
                                    class="avatar-icon-inner"
                                />
                            </div>
                            <div class="msg-body msg-body-assistant">
                                <div class="msg-header msg-header-assistant">
                                    <span class="msg-role msg-role-assistant">{{
                                        roleLabel(message.role)
                                    }}</span>
                                    <span class="msg-time">{{
                                        formatTime(message.created_at)
                                    }}</span>
                                </div>
                                <div
                                    class="msg-content msg-content-assistant"
                                    v-html="renderMarkdown(message.content)"
                                ></div>
                            </div>
                        </div>

                        <!-- 系统消息 -->
                        <div
                            v-else-if="message.role === 'system'"
                            class="msg msg-system"
                        >
                            <div class="msg-body msg-body-system">
                                <div class="msg-header msg-header-system">
                                    <span class="msg-role msg-role-system">{{
                                        roleLabel(message.role)
                                    }}</span>
                                    <span class="msg-time">{{
                                        formatTime(message.created_at)
                                    }}</span>
                                </div>
                                <div
                                    class="msg-content msg-content-system"
                                    v-html="renderMarkdown(message.content)"
                                ></div>
                            </div>
                        </div>

                        <!-- 工具消息 -->
                        <div
                            v-else-if="message.role === 'tool'"
                            class="msg msg-tool"
                        >
                            <div class="msg-avatar avatar-tool">
                                <Icon
                                    icon="lucide:wrench"
                                    class="avatar-icon-inner"
                                />
                            </div>
                            <div class="msg-body msg-body-tool">
                                <div class="msg-header msg-header-tool">
                                    <span class="msg-role msg-role-tool">{{
                                        roleLabel(message.role)
                                    }}</span>
                                    <span class="msg-time">{{
                                        formatTime(message.created_at)
                                    }}</span>
                                </div>
                                <div
                                    class="msg-content msg-content-tool"
                                    v-html="renderMarkdown(message.content)"
                                ></div>
                            </div>
                        </div>

                        <!-- 其他角色 -->
                        <div v-else class="msg msg-other">
                            <div class="msg-avatar avatar-other">
                                <Icon
                                    icon="lucide:user"
                                    class="avatar-icon-inner"
                                />
                            </div>
                            <div class="msg-body">
                                <div class="msg-header">
                                    <span class="msg-role">{{
                                        roleLabel(message.role)
                                    }}</span>
                                    <span class="msg-time">{{
                                        formatTime(message.created_at)
                                    }}</span>
                                </div>
                                <div
                                    class="msg-content"
                                    v-html="renderMarkdown(message.content)"
                                ></div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </Transition>
    </div>
</template>

<style scoped>
.conversation-history-view {
    padding: 24px;
    max-width: 100%;
    margin: 0 auto;
}

.page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
    gap: 1rem;
}

.header-content {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.header-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 0.75rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.2) 0%,
        hsl(var(--primary) / 0.1) 100%
    );
    color: hsl(var(--primary));
    flex-shrink: 0;
}

.header-icon svg {
    width: 1.25rem;
    height: 1.25rem;
}

.header-text {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
}

.header-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: hsl(var(--foreground));
    margin: 0;
    line-height: 1.2;
}

.header-desc {
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
    margin: 0;
}

/* 筛选工具栏 */
.filter-toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-bottom: 24px;
    padding: 16px;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
}

.filter-item {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 200px;
    flex: 1;
}

.filter-item label {
    font-size: 12px;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

.filter-item input,
.filter-item select {
    padding: 8px 12px;
    border: 1px solid hsl(var(--border));
    border-radius: 6px;
    background: hsl(var(--background));
    color: hsl(var(--foreground));
    font-size: 14px;
    transition: all 0.2s;
}

.filter-item input:focus,
.filter-item select:focus {
    outline: none;
    border-color: hsl(var(--primary));
    box-shadow: 0 0 0 2px hsl(var(--primary) / 0.1);
}

.filter-actions {
    display: flex;
    gap: 8px;
    align-items: flex-end;
}

/* 按钮样式 */
.btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    white-space: nowrap;
}

.btn-primary {
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
}

.btn-primary:hover {
    background: hsl(var(--primary) / 0.9);
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.2);
}

.btn-secondary {
    background: hsl(var(--secondary));
    color: hsl(var(--foreground));
}

.btn-secondary:hover {
    background: hsl(var(--secondary) / 0.8);
}

.icon {
    width: 16px;
    height: 16px;
}

/* 错误横幅 */
.error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: hsl(var(--destructive) / 0.1);
    border: 1px solid hsl(var(--destructive) / 0.2);
    border-radius: 8px;
    color: hsl(var(--destructive));
    margin-bottom: 24px;
}

.error-banner .icon {
    width: 20px;
    height: 20px;
}

/* 加载状态 */
.loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 64px 24px;
    color: hsl(var(--muted-foreground));
}

.loading-state .icon {
    width: 48px;
    height: 48px;
}

.spin {
    animation: spin 1s linear infinite;
}

@keyframes spin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

/* 对话卡片列表 */
.conversation-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 16px;
}

.conversation-card {
    display: flex;
    flex-direction: column;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    overflow: hidden;
    cursor: pointer;
    transition: all 0.2s ease;
}

.conversation-card:hover {
    border-color: hsl(var(--primary) / 0.4);
    box-shadow: 0 4px 16px hsl(var(--primary) / 0.08);
    transform: translateY(-1px);
}

.conversation-card:active {
    transform: translateY(0);
}

/* 卡片头部 */
.card-header {
    padding: 16px 16px 8px;
}

.card-title-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
}

.card-title {
    font-size: 1rem;
    font-weight: 700;
    color: hsl(var(--foreground));
    margin: 0;
    line-height: 1.3;
    word-break: break-word;
    flex: 1;
    min-width: 0;
}

.card-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
}

.card-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
}

.meta-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
}

.meta-tag .meta-icon {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
}

/* 卡片内容预览 */
.card-preview {
    padding: 8px 16px;
    border-top: 1px solid hsl(var(--border) / 0.5);
    background: hsl(var(--muted) / 0.15);
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-height: 80px;
    max-height: 140px;
    overflow: hidden;
}

.preview-line {
    font-size: 0.8125rem;
    line-height: 1.5;
    color: hsl(var(--muted-foreground));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.preview-user {
    color: hsl(var(--primary));
    font-weight: 500;
}

.preview-assistant {
    color: hsl(var(--foreground) / 0.8);
}

.preview-tool {
    color: hsl(38 92% 50%);
    font-size: 0.75rem;
}

.preview-empty {
    font-size: 0.8125rem;
    color: hsl(var(--muted-foreground) / 0.6);
    font-style: italic;
    padding: 8px 0;
}

/* 卡片底部 */
.card-footer {
    padding: 8px 16px;
    border-top: 1px solid hsl(var(--border) / 0.3);
    display: flex;
    justify-content: flex-end;
}

.view-all-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    color: hsl(var(--primary));
    text-transform: uppercase;
    letter-spacing: 0.025em;
    transition: gap 0.2s ease;
}

.conversation-card:hover .view-all-link {
    gap: 8px;
}

.view-all-link .meta-icon {
    width: 14px;
    height: 14px;
}

/* 聊天类型徽章 */
.chat-type-badge {
    display: inline-block;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
}

.chat-type-badge.type-group {
    background: hsl(var(--secondary) / 0.2);
    color: hsl(var(--secondary));
    border: 1px solid hsl(var(--secondary) / 0.3);
}

.chat-type-badge.type-private {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
    border: 1px solid hsl(var(--primary) / 0.2);
}

/* 按钮图标 */
.btn-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s;
}

.btn-icon:hover {
    background: hsl(var(--muted) / 0.2);
    color: hsl(var(--foreground));
}

.btn-icon.btn-view:hover {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
}

.btn-icon.btn-delete:hover {
    background: hsl(var(--destructive) / 0.1);
    color: hsl(var(--destructive));
}

.btn-icon.btn-close {
    width: 36px;
    height: 36px;
    border-radius: 8px;
}

.btn-icon.btn-close:hover {
    background: hsl(var(--muted) / 0.3);
    color: hsl(var(--foreground));
}

/* 空状态 */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 64px 24px;
    text-align: center;
    color: hsl(var(--muted-foreground));
}

.empty-state .icon {
    width: 64px;
    height: 64px;
    opacity: 0.5;
}

.empty-state h2 {
    font-size: 20px;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

.empty-state p {
    font-size: 14px;
    margin: 0;
}

/* ======== 遮罩层 ======== */
.detail-overlay {
    position: fixed;
    inset: 0;
    background: hsl(var(--foreground) / 0.4);
    z-index: 40;
    transition: opacity 0.3s ease;
}

.overlay-enter-active,
.overlay-leave-active {
    transition: opacity 0.3s ease;
}

.overlay-enter-from,
.overlay-leave-to {
    opacity: 0;
}

/* ======== 详情侧边面板 ======== */
.detail-panel {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(560px, 90vw);
    background: hsl(var(--background));
    border-left: 1px solid hsl(var(--border));
    z-index: 50;
    display: flex;
    flex-direction: column;
    box-shadow: -8px 0 24px hsl(var(--foreground) / 0.1);
}

.slide-over-enter-active,
.slide-over-leave-active {
    transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}

.slide-over-enter-from,
.slide-over-leave-to {
    transform: translateX(100%);
}

/* 面板头部 */
.detail-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid hsl(var(--border));
    background: hsl(var(--card));
    flex-shrink: 0;
    gap: 12px;
}

.detail-header-info {
    flex: 1;
    min-width: 0;
}

.detail-title {
    font-size: 1.125rem;
    font-weight: 700;
    color: hsl(var(--foreground));
    margin: 0 0 8px 0;
    line-height: 1.3;
    word-break: break-word;
}

.detail-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
}

.detail-meta-item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
}

.meta-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
}

/* 面板加载/错误/空状态 */
.detail-loading,
.detail-error,
.detail-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 64px 24px;
    color: hsl(var(--muted-foreground));
    flex: 1;
    justify-content: center;
}

.detail-loading .icon,
.detail-empty .icon {
    width: 32px;
    height: 32px;
}

.detail-error {
    color: hsl(var(--destructive));
}

.detail-error .icon {
    width: 24px;
    height: 24px;
}

/* ======== 消息列表 ======== */
.detail-messages {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
}

.detail-messages::-webkit-scrollbar {
    width: 6px;
}

.detail-messages::-webkit-scrollbar-track {
    background: transparent;
}

.detail-messages::-webkit-scrollbar-thumb {
    background: hsl(var(--border));
    border-radius: 3px;
}

.detail-messages::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--muted-foreground) / 0.3);
}

/* 消息容器 */
.detail-message {
    display: flex;
    width: 100%;
}

/* 用户消息 - 右对齐 */
.detail-message-user {
    justify-content: flex-end;
}

/* 助手/工具/系统消息 - 左对齐 */
.detail-message-assistant,
.detail-message-tool,
.detail-message-system,
.detail-message-other {
    justify-content: flex-start;
}

/* 消息行 */
.msg {
    display: flex;
    gap: 10px;
    max-width: 85%;
}

.msg-user {
    flex-direction: row-reverse;
}

.msg-system {
    max-width: 100%;
    justify-content: center;
}

/* 头像 */
.msg-avatar {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 2px;
}

.avatar-user {
    background: hsl(var(--primary) / 0.15);
    border: 1px solid hsl(var(--primary) / 0.3);
}

.avatar-user .avatar-icon {
    width: 16px;
    height: 16px;
}

.avatar-assistant {
    background: hsl(var(--primary) / 0.15);
    border: 2px solid hsl(var(--primary));
}

.avatar-icon-inner {
    width: 16px;
    height: 16px;
    color: hsl(var(--primary));
}

.avatar-tool {
    background: hsl(38 92% 50% / 0.1);
    border: 1px solid hsl(38 92% 50% / 0.3);
    border-radius: 0.5rem;
}

.avatar-tool .avatar-icon-inner {
    color: hsl(38 92% 50%);
}

.avatar-other {
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
}

/* 消息体 */
.msg-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
}

.msg-body-user {
    align-items: flex-end;
}

.msg-body-system {
    align-items: center;
}

/* 消息头部 */
.msg-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 4px;
}

.msg-header-user {
    flex-direction: row-reverse;
}

.msg-header-system {
    justify-content: center;
}

.msg-role {
    font-size: 0.625rem;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.msg-role-assistant {
    color: hsl(var(--primary));
}

.msg-role-system {
    color: hsl(200 80% 50%);
}

.msg-role-tool {
    color: hsl(38 92% 50%);
}

.msg-time {
    font-size: 0.625rem;
    color: hsl(var(--muted-foreground) / 0.7);
}

/* 消息内容 */
.msg-content {
    padding: 10px 14px;
    font-size: 0.875rem;
    line-height: 1.6;
    word-break: break-word;
    overflow-wrap: break-word;
}

/* 用户消息样式 */
.msg-content-user {
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(280 70% 60%) 100%
    );
    color: white;
    border-radius: 1rem 0.25rem 1rem 1rem;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.15);
}

/* 助手消息样式 */
.msg-content-assistant {
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 0.25rem 1rem 1rem 1rem;
    color: hsl(var(--foreground));
    box-shadow: 0 1px 3px hsl(var(--primary) / 0.04);
}

/* 系统消息样式 */
.msg-content-system {
    background: hsl(200 80% 50% / 0.08);
    border: 1px dashed hsl(200 80% 50% / 0.3);
    border-radius: 0.5rem;
    color: hsl(200 80% 40%);
    font-size: 0.8125rem;
    text-align: center;
    max-width: 400px;
}

.dark .msg-content-system {
    color: hsl(200 80% 70%);
    border-color: hsl(200 80% 50% / 0.4);
}

/* 工具消息样式 */
.msg-content-tool {
    background: hsl(38 92% 50% / 0.08);
    border: 1px solid hsl(38 92% 50% / 0.25);
    border-radius: 1rem 1rem 1rem 0.25rem;
    color: hsl(38 92% 30%);
    font-size: 0.8125rem;
}

.dark .msg-content-tool {
    color: hsl(38 92% 70%);
}

/* 其他消息 */
.detail-message-other .msg-content {
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 0.75rem;
    color: hsl(var(--foreground));
}

/* ======== Markdown 渲染样式 ======== */
.msg-content :deep(p) {
    margin: 0.25rem 0;
}

.msg-content :deep(p:first-child) {
    margin-top: 0;
}

.msg-content :deep(p:last-child) {
    margin-bottom: 0;
}

.msg-content :deep(pre) {
    background: hsl(var(--muted) / 0.5);
    border: 1px solid hsl(var(--border));
    border-radius: 0.5rem;
    padding: 0.75rem;
    overflow-x: auto;
    margin: 0.5rem 0;
}

.msg-content :deep(code) {
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;
    font-size: 0.8125rem;
}

.msg-content :deep(:not(pre) > code) {
    background: hsl(var(--muted) / 0.5);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
}

.msg-content :deep(pre code) {
    background: none;
    padding: 0;
}

.msg-content :deep(ul),
.msg-content :deep(ol) {
    margin: 0.25rem 0;
    padding-left: 1.5rem;
}

.msg-content :deep(li) {
    margin: 0.125rem 0;
}

.msg-content :deep(blockquote) {
    border-left: 3px solid hsl(var(--primary) / 0.5);
    padding-left: 0.75rem;
    margin: 0.5rem 0;
    color: hsl(var(--muted-foreground));
}

.msg-content :deep(a) {
    color: hsl(var(--primary));
    text-decoration: underline;
    text-underline-offset: 2px;
}

.msg-content :deep(a:hover) {
    text-decoration-thickness: 2px;
}

.msg-content :deep(hr) {
    border: none;
    border-top: 1px solid hsl(var(--border));
    margin: 0.75rem 0;
}

.msg-content :deep(table) {
    border-collapse: collapse;
    width: 100%;
    margin: 0.5rem 0;
}

.msg-content :deep(th),
.msg-content :deep(td) {
    border: 1px solid hsl(var(--border));
    padding: 0.375rem 0.625rem;
    text-align: left;
}

.msg-content :deep(th) {
    background: hsl(var(--muted) / 0.3);
    font-weight: 600;
}

.msg-content :deep(img) {
    max-width: 100%;
    border-radius: 0.5rem;
    margin: 0.25rem 0;
}

.msg-content :deep(h1),
.msg-content :deep(h2),
.msg-content :deep(h3),
.msg-content :deep(h4),
.msg-content :deep(h5),
.msg-content :deep(h6) {
    margin: 0.5rem 0 0.25rem;
    font-weight: 600;
    line-height: 1.3;
}

.msg-content :deep(h1) {
    font-size: 1.25rem;
}

.msg-content :deep(h2) {
    font-size: 1.125rem;
}

.msg-content :deep(h3) {
    font-size: 1rem;
}

/* 用户消息中的链接需要确保可读 */
.msg-content-user :deep(a) {
    color: white;
    text-decoration: underline;
}

.msg-content-user :deep(code) {
    background: hsl(0 0% 100% / 0.2);
}

.msg-content-user :deep(pre) {
    background: hsl(0 0% 100% / 0.15);
    border-color: hsl(0 0% 100% / 0.2);
}

.msg-content-user :deep(pre code) {
    background: none;
}

.msg-content-user :deep(blockquote) {
    border-left-color: hsl(0 0% 100% / 0.5);
    color: hsl(0 0% 100% / 0.85);
}

/* 响应式 */
@media (max-width: 768px) {
    .conversation-history-view {
        padding: 16px;
    }

    .filter-toolbar {
        flex-direction: column;
    }

    .filter-item {
        min-width: 100%;
    }

    .filter-actions {
        width: 100%;
    }

    .filter-actions .btn {
        flex: 1;
    }

    .conversation-cards {
        grid-template-columns: 1fr;
    }

    .detail-panel {
        width: 100vw;
    }
}

@media (max-width: 640px) {
    .page-header {
        flex-direction: column;
        align-items: flex-start;
    }

    .detail-header {
        padding: 16px;
    }

    .detail-messages {
        padding: 12px;
    }

    .msg {
        max-width: 92%;
    }
}
</style>

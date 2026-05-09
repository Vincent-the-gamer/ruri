<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useI18n } from "vue-i18n";
import {
    listConversations,
    deleteConversation,
    type Conversation,
    type ConversationFilter,
} from "../api/conversations";
import { Icon } from "@iconify/vue";

const { t } = useI18n();

// 对话列表
const conversations = ref<Conversation[]>([]);
// 加载状态
const loading = ref(false);
// 错误信息
const error = ref<string | null>(null);

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

// 加载对话列表
async function loadConversations() {
    loading.value = true;
    error.value = null;

    try {
        conversations.value = await listConversations(filter.value);
    } catch (err: any) {
        console.error("Failed to load conversations:", err);
        error.value =
            err.response?.data?.error || err.message || t("common.error");
    } finally {
        loading.value = false;
    }
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
        await loadConversations();
    } catch (err: any) {
        console.error("Failed to delete conversation:", err);
        error.value =
            err.response?.data?.error || err.message || t("common.error");
    }
}

// 格式化日期时间
function formatDateTime(dateString: string) {
    return new Date(dateString).toLocaleString();
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

        <!-- 对话列表表格 -->
        <div v-else-if="conversations.length > 0" class="table-container">
            <table class="conversations-table">
                <thead>
                    <tr>
                        <th>{{ t("conversationHistory.table.title") }}</th>
                        <th>{{ t("conversationHistory.table.botName") }}</th>
                        <th>{{ t("conversationHistory.table.chatType") }}</th>
                        <th>{{ t("conversationHistory.table.chatId") }}</th>
                        <th>{{ t("conversationHistory.table.createdAt") }}</th>
                        <th>{{ t("conversationHistory.table.updatedAt") }}</th>
                        <th>{{ t("conversationHistory.table.actions") }}</th>
                    </tr>
                </thead>
                <tbody>
                    <tr
                        v-for="conversation in conversations"
                        :key="conversation.id"
                    >
                        <td class="title-cell">
                            {{
                                conversation.title ||
                                t("conversationHistory.noTitle")
                            }}
                        </td>
                        <td>{{ conversation.bot_name }}</td>
                        <td>
                            <span
                                class="chat-type-badge"
                                :class="`type-${conversation.chat_type}`"
                            >
                                {{ chatTypeLabel(conversation.chat_type) }}
                            </span>
                        </td>
                        <td class="chat-id-cell">{{ conversation.chat_id }}</td>
                        <td>{{ formatDateTime(conversation.created_at) }}</td>
                        <td>{{ formatDateTime(conversation.updated_at) }}</td>
                        <td class="actions-cell">
                            <button
                                class="btn-icon btn-delete"
                                :title="t('conversationHistory.delete')"
                                @click="
                                    handleDeleteConversation(conversation.id)
                                "
                            >
                                <Icon icon="lucide:trash-2" />
                            </button>
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>

        <!-- 空状态 -->
        <div v-else class="empty-state">
            <Icon icon="lucide:message-square-off" class="icon" />
            <h2>{{ t("conversationHistory.emptyTitle") }}</h2>
            <p>{{ t("conversationHistory.emptyDescription") }}</p>
        </div>
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

/* 表格容器 */
.table-container {
    overflow-x: auto;
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    background: hsl(var(--card));
}

.conversations-table {
    width: 100%;
    border-collapse: collapse;
    min-width: 800px;
}

.conversations-table thead {
    background: hsl(var(--muted) / 0.3);
}

.conversations-table th {
    padding: 12px 16px;
    text-align: left;
    font-size: 12px;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: 1px solid hsl(var(--border));
}

.conversations-table td {
    padding: 12px 16px;
    border-bottom: 1px solid hsl(var(--border));
    color: hsl(var(--foreground));
    font-size: 14px;
}

.conversations-table tbody tr:hover {
    background: hsl(var(--muted) / 0.2);
}

.conversations-table tbody tr:last-child td {
    border-bottom: none;
}

/* 特殊单元格 */
.title-cell {
    font-weight: 600;
    color: hsl(var(--foreground));
}

.chat-id-cell {
    font-family: "Monaco", "Courier New", monospace;
    font-size: 12px;
    color: hsl(var(--muted-foreground));
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

/* 操作单元格 */
.actions-cell {
    text-align: right;
    white-space: nowrap;
}

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

.btn-icon.btn-delete:hover {
    background: hsl(var(--destructive) / 0.1);
    color: hsl(var(--destructive));
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
}

@media (max-width: 640px) {
    .page-header {
        flex-direction: column;
        align-items: flex-start;
    }
}
</style>

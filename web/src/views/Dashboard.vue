<script setup lang="ts">
import { onMounted, computed } from "vue";
import { useRouter } from "vue-router";
import { useAgentStore } from "../stores/agent";
import { useProviderStore } from "../stores/provider";
import { useSkillStore } from "../stores/skill";
import { useToolStore } from "../stores/tool";
import { useChatStore } from "../stores/chat";
import StatusBar from "../components/StatusBar.vue";

const router = useRouter();
const agentStore = useAgentStore();
const providerStore = useProviderStore();
const skillStore = useSkillStore();
const toolStore = useToolStore();
const chatStore = useChatStore();

onMounted(async () => {
    await Promise.all([
        agentStore.fetchStatus(),
        providerStore.fetchProviders(),
        skillStore.fetchSkills(),
        toolStore.fetchTools(),
        chatStore.fetchHistory(),
    ]);
});

const statusClass = computed(() => {
    switch (agentStore.status.status) {
        case "running":
            return "status-running";
        case "error":
            return "status-error";
        default:
            return "status-stopped";
    }
});

const recentMessages = computed(() => {
    const msgs = chatStore.messages.filter((m) => m.role !== "system");
    return msgs.slice(-5).reverse();
});

const providerTypeStyle = (type: string) => {
    switch (type) {
        case "openai":
            return { class: "type-openai", label: "OpenAI" };
        case "anthropic":
            return { class: "type-anthropic", label: "Anthropic" };
        case "custom":
            return { class: "type-custom", label: "Custom" };
        default:
            return { class: "type-default", label: "Other" };
    }
};
</script>

<template>
    <div class="dashboard">
        <!-- Header -->
        <header class="dashboard-header">
            <h1 class="title">仪表盘</h1>
            <p class="subtitle">Ruri AI 智能体总览</p>
        </header>

        <!-- Status Card -->
        <div class="status-card" :class="statusClass">
            <div class="status-content">
                <div class="status-info">
                    <div class="status-indicator">
                        <svg
                            v-if="agentStore.status.status === 'running'"
                            class="status-icon"
                            viewBox="0 0 24 24"
                            fill="currentColor"
                        >
                            <circle cx="12" cy="12" r="8" />
                        </svg>
                        <svg
                            v-else-if="agentStore.status.status === 'error'"
                            class="status-icon"
                            viewBox="0 0 24 24"
                            fill="currentColor"
                        >
                            <circle cx="12" cy="12" r="8" />
                        </svg>
                        <svg
                            v-else
                            class="status-icon"
                            viewBox="0 0 24 24"
                            fill="currentColor"
                        >
                            <circle cx="12" cy="12" r="8" />
                        </svg>
                    </div>
                    <div class="status-text">
                        <h2 class="status-title">
                            {{
                                agentStore.status.status === "running"
                                    ? "运行中"
                                    : agentStore.status.status === "error"
                                      ? "错误"
                                      : "已停止"
                            }}
                        </h2>
                        <StatusBar />
                    </div>
                </div>
                <div class="status-meta">
                    <div class="meta-item">
                        <span class="meta-label">运行时间</span>
                        <span class="meta-value">{{
                            agentStore.formatUptime(
                                agentStore.status.uptime_secs,
                            )
                        }}</span>
                    </div>
                    <div class="meta-item">
                        <span class="meta-label">消息数</span>
                        <span class="meta-value">{{
                            agentStore.status.message_count
                        }}</span>
                    </div>
                </div>
            </div>
        </div>

        <!-- Stats Grid -->
        <div class="stats-grid">
            <!-- Providers -->
            <div class="stat-card">
                <div class="stat-header">
                    <span class="stat-label">供应商</span>
                    <svg
                        class="stat-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path
                            d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"
                        />
                    </svg>
                </div>
                <div class="stat-value">
                    {{ providerStore.providers.length }}
                </div>
                <div
                    v-if="providerStore.activeProvider"
                    class="stat-detail accent"
                >
                    活跃：{{ providerStore.activeProvider.name }}
                </div>
                <div v-else class="stat-detail warning">暂无活跃供应商</div>
            </div>

            <!-- Skills -->
            <div class="stat-card">
                <div class="stat-header">
                    <span class="stat-label">技能</span>
                    <svg
                        class="stat-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <polygon
                            points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"
                        />
                    </svg>
                </div>
                <div class="stat-value">{{ skillStore.skills.length }}</div>
                <div class="stat-detail success">
                    {{ skillStore.skills.filter((s) => s.is_active).length }}
                    已启用
                </div>
            </div>

            <!-- Tools -->
            <div class="stat-card">
                <div class="stat-header">
                    <span class="stat-label">工具</span>
                    <svg
                        class="stat-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path
                            d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"
                        />
                    </svg>
                </div>
                <div class="stat-value">{{ toolStore.tools.length }}</div>
                <div class="stat-detail">可用工具</div>
            </div>
        </div>

        <!-- Two Column Layout -->
        <div class="two-column">
            <!-- Providers List -->
            <section class="panel">
                <div class="panel-header">
                    <h3 class="panel-title">已配置供应商</h3>
                    <button class="link-btn" @click="router.push('/providers')">
                        管理
                        <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <path d="M5 12h14M12 5l7 7-7 7" />
                        </svg>
                    </button>
                </div>
                <div
                    v-if="providerStore.providers.length === 0"
                    class="empty-state"
                >
                    暂未配置供应商
                </div>
                <div v-else class="provider-list">
                    <div
                        v-for="p in providerStore.providers"
                        :key="p.id"
                        class="provider-item"
                        :class="{ active: p.is_active }"
                    >
                        <span
                            class="provider-dot"
                            :class="providerTypeStyle(p.provider_type).class"
                        />
                        <div class="provider-info">
                            <div class="provider-name">{{ p.name }}</div>
                            <div class="provider-model">
                                {{ (p.config as any).default_model }}
                            </div>
                        </div>
                        <span v-if="p.is_active" class="active-badge"
                            >已启用</span
                        >
                    </div>
                </div>
            </section>

            <!-- Recent Messages -->
            <section class="panel">
                <div class="panel-header">
                    <h3 class="panel-title">最近消息</h3>
                    <button class="link-btn" @click="router.push('/chat')">
                        打开对话
                        <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <path d="M5 12h14M12 5l7 7-7 7" />
                        </svg>
                    </button>
                </div>
                <div v-if="recentMessages.length === 0" class="empty-state">
                    暂无消息
                </div>
                <div v-else class="message-list">
                    <div
                        v-for="(msg, i) in recentMessages"
                        :key="i"
                        class="message-item"
                    >
                        <div class="message-header">
                            <svg
                                v-if="msg.role === 'user'"
                                class="message-icon user"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <path
                                    d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"
                                />
                                <circle cx="12" cy="7" r="4" />
                            </svg>
                            <svg
                                v-else
                                class="message-icon assistant"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <rect
                                    x="3"
                                    y="11"
                                    width="18"
                                    height="10"
                                    rx="2"
                                />
                                <circle cx="12" cy="5" r="2" />
                                <path d="M12 7v4" />
                                <circle cx="8" cy="16" r="1" />
                                <circle cx="16" cy="16" r="1" />
                            </svg>
                            <span class="message-role" :class="msg.role">
                                {{ msg.role === "user" ? "用户" : "助手" }}
                            </span>
                        </div>
                        <div class="message-content">{{ msg.content }}</div>
                    </div>
                </div>
            </section>
        </div>

        <!-- Quick Actions -->
        <div class="actions">
            <button class="btn btn-accent" @click="router.push('/chat')">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <path
                        d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"
                    />
                </svg>
                开始对话
            </button>
            <button class="btn" @click="router.push('/providers')">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <path
                        d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"
                    />
                </svg>
                配置供应商
            </button>
            <button class="btn btn-ghost" @click="router.push('/api-test')">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <path
                        d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"
                    />
                    <polyline points="14 2 14 8 20 8" />
                    <path d="M12 18v-6" />
                    <path d="M9 15l3 3 3-3" />
                </svg>
                接口测试
            </button>
        </div>
    </div>
</template>

<style scoped>
/* ═══════════════════════════════════════════════════════════════
 * Dashboard - Raycast-inspired frosted glass design
 * ═══════════════════════════════════════════════════════════════ */
.dashboard {
    padding: 1.5rem;
    max-width: 72rem;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    animation: fadeIn var(--transition-normal) cubic-bezier(0.25, 0.1, 0.25, 1);
}

/* Header */
.dashboard-header {
    margin-bottom: 0.5rem;
}

.title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-text);
    line-height: 1.3;
    text-shadow: 0 1px 3px rgba(139, 92, 246, 0.1);
}

.subtitle {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    margin-top: 0.25rem;
}

/* Status Card - Glass effect */
.status-card {
    background: linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.9) 0%,
        rgba(250, 245, 255, 0.85) 100%
    );
    backdrop-filter: blur(16px) saturate(160%);
    -webkit-backdrop-filter: blur(16px) saturate(160%);
    border: 1px solid rgba(255, 255, 255, 0.5);
    border-right: 1px solid rgba(216, 180, 254, 0.2);
    border-bottom: 1px solid rgba(216, 180, 254, 0.2);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
    transition: all var(--transition-fast);
    box-shadow: var(--shadow-sm);
    position: relative;
    overflow: hidden;
}
.status-card::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(
        90deg,
        var(--color-accent),
        var(--color-primary)
    );
    opacity: 0;
    transition: opacity var(--transition-fast);
}
.status-card:hover::before {
    opacity: 1;
}

.status-card.status-running {
    border-color: rgba(34, 197, 94, 0.35);
    box-shadow:
        0 2px 12px rgba(16, 185, 129, 0.1),
        var(--shadow-sm);
}
.status-card.status-running::before {
    background: linear-gradient(90deg, #10b981, #34d399);
    opacity: 1;
}

.status-card.status-error {
    border-color: rgba(239, 68, 68, 0.35);
    box-shadow:
        0 2px 12px rgba(239, 68, 68, 0.1),
        var(--shadow-sm);
}
.status-card.status-error::before {
    background: linear-gradient(90deg, #ef4444, #f87171);
    opacity: 1;
}

.status-card.status-stopped {
    border-color: rgba(216, 180, 254, 0.25);
}

.status-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
}

.status-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.status-indicator {
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--color-bg-mute);
}

.status-card.status-running .status-indicator {
    background-color: var(--color-success-soft);
}

.status-card.status-error .status-indicator {
    background-color: var(--color-danger-soft);
}

.status-icon {
    width: 0.625rem;
    height: 0.625rem;
}

.status-card.status-running .status-icon {
    color: var(--color-success);
}

.status-card.status-error .status-icon {
    color: var(--color-danger);
}

.status-card.status-stopped .status-icon {
    color: var(--color-text-muted);
}

.status-text {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
}

.status-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--color-text);
    text-transform: capitalize;
}

.status-meta {
    display: flex;
    gap: 1.5rem;
    text-align: right;
}

.meta-item {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
}

.meta-label {
    font-size: 0.75rem;
    color: var(--color-text-muted);
}

.meta-value {
    font-size: 0.875rem;
    color: var(--color-text-secondary);
}

/* Stats Grid */
.stats-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1rem;
}

/* Stat Card - Glass effect */
.stat-card {
    background: linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.88) 0%,
        rgba(250, 245, 255, 0.82) 100%
    );
    backdrop-filter: blur(12px) saturate(150%);
    -webkit-backdrop-filter: blur(12px) saturate(150%);
    border: 1px solid rgba(255, 255, 255, 0.45);
    border-right: 1px solid rgba(216, 180, 254, 0.18);
    border-bottom: 1px solid rgba(216, 180, 254, 0.18);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
    transition: all var(--transition-fast);
    box-shadow: var(--shadow-sm);
    position: relative;
    overflow: hidden;
}

.stat-card:hover {
    border-color: rgba(216, 180, 254, 0.35);
    transform: translateY(-2px);
    box-shadow: var(--shadow-md);
}

.stat-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
}

.stat-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-text-secondary);
}

.stat-icon {
    width: 1.25rem;
    height: 1.25rem;
    color: var(--color-text-muted);
}

.stat-value {
    font-size: 2rem;
    font-weight: 600;
    color: var(--color-text);
    line-height: 1.2;
}

.stat-detail {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    margin-top: 0.5rem;
}

.stat-detail.accent {
    color: var(--color-accent);
}

.stat-detail.success {
    color: var(--color-success);
}

.stat-detail.warning {
    color: var(--color-warning);
}

/* Two Column Layout */
.two-column {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
}

/* Panel - Glass effect */
.panel {
    background: linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.85) 0%,
        rgba(250, 245, 255, 0.8) 100%
    );
    backdrop-filter: blur(12px) saturate(150%);
    -webkit-backdrop-filter: blur(12px) saturate(150%);
    border: 1px solid rgba(255, 255, 255, 0.4);
    border-right: 1px solid rgba(216, 180, 254, 0.18);
    border-bottom: 1px solid rgba(216, 180, 254, 0.18);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
    box-shadow: var(--shadow-sm);
    transition: all var(--transition-fast);
}
.panel:hover {
    box-shadow: var(--shadow-md);
}

.panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
}

.panel-title {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-text);
}

.link-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--color-text-muted);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    transition: color var(--transition-fast);
}

.link-btn:hover {
    color: var(--color-accent);
}

.link-btn svg {
    width: 0.875rem;
    height: 0.875rem;
}

.empty-state {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    text-align: center;
    padding: 2rem 0;
}

/* Provider List */
.provider-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

/* Provider Item - Glass effect */
.provider-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.625rem 0.75rem;
    background: rgba(255, 255, 255, 0.5);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    border: 1px solid rgba(255, 255, 255, 0.35);
    border-radius: var(--radius-md);
    transition: all var(--transition-fast);
}
.provider-item:hover {
    background: rgba(255, 255, 255, 0.7);
    border-color: rgba(216, 180, 254, 0.25);
    box-shadow: 0 2px 8px rgba(139, 92, 246, 0.06);
}

.provider-item.active {
    background: linear-gradient(
        135deg,
        rgba(236, 72, 153, 0.08) 0%,
        rgba(139, 92, 246, 0.06) 100%
    );
    border-color: rgba(236, 72, 153, 0.25);
    box-shadow: 0 2px 10px rgba(236, 72, 153, 0.08);
}

.provider-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    flex-shrink: 0;
}

.provider-dot.type-openai {
    background-color: #22c55e;
}

.provider-dot.type-anthropic {
    background-color: #f59e0b;
}

.provider-dot.type-custom {
    background-color: var(--color-accent);
}

.provider-dot.type-default {
    background-color: var(--color-text-muted);
}

.provider-info {
    flex: 1;
    min-width: 0;
}

.provider-name {
    font-size: 0.875rem;
    color: var(--color-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.provider-model {
    font-size: 0.75rem;
    color: var(--color-text-muted);
}

.active-badge {
    font-size: 0.6875rem;
    font-weight: 500;
    color: var(--color-accent);
    padding: 0.125rem 0.5rem;
    background-color: var(--color-accent-soft);
    border-radius: var(--radius-sm);
}

/* Message List */
.message-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

/* Message Item - Glass effect */
.message-item {
    padding: 0.625rem 0.75rem;
    background: rgba(255, 255, 255, 0.5);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    border: 1px solid rgba(255, 255, 255, 0.3);
    border-radius: var(--radius-md);
    transition: all var(--transition-fast);
}
.message-item:hover {
    background: rgba(255, 255, 255, 0.65);
    border-color: rgba(216, 180, 254, 0.2);
}

.message-header {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    margin-bottom: 0.25rem;
}

.message-icon {
    width: 0.875rem;
    height: 0.875rem;
}

.message-icon.user {
    color: var(--color-info);
}

.message-icon.assistant {
    color: var(--color-accent);
}

.message-role {
    font-size: 0.6875rem;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.message-role.user {
    color: var(--color-info);
}

.message-role.assistant {
    color: var(--color-accent);
}

.message-content {
    font-size: 0.75rem;
    color: var(--color-text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
}

/* Actions */
.actions {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.75rem;
}

.actions .btn {
    padding: 0.75rem 1rem;
}

.actions .btn svg {
    width: 1rem;
    height: 1rem;
}

/* Responsive */
@media (max-width: 768px) {
    .stats-grid {
        grid-template-columns: 1fr;
    }

    .two-column {
        grid-template-columns: 1fr;
    }

    .actions {
        grid-template-columns: 1fr;
    }

    .status-content {
        flex-direction: column;
        align-items: flex-start;
        gap: 1rem;
    }

    .status-meta {
        text-align: left;
    }
}
</style>

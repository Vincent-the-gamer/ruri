<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { useAgentStore } from "../stores/agent";
import { useProviderStore } from "../stores/provider";
import { useSkillStore } from "../stores/skill";
import { useToolStore } from "../stores/tool";
import { useChatStore } from "../stores/chat";

const router = useRouter();
const { t } = useI18n();
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

const statusText = computed(() => {
    switch (agentStore.status.status) {
        case "running":
            return t("dashboard.status.running");
        case "error":
            return t("dashboard.status.error");
        default:
            return t("dashboard.status.stopped");
    }
});

const recentMessages = computed(() => {
    const msgs = chatStore.messages.filter((m) => m.role !== "system");
    return msgs.slice(-5).reverse();
});

const activeProvider = computed(() => {
    return providerStore.providers.find((p) => p.is_active);
});

// Feature cards data
const features = computed(() => [
    {
        id: "multi-provider",
        icon: "server",
        title: t("dashboard.features.multiProvider.title"),
        description: t("dashboard.features.multiProvider.description"),
        link: "/providers",
        linkText: t("dashboard.features.multiProvider.linkText"),
        color: "primary",
        stats: `${providerStore.providers.length} ${t("dashboard.stats.providers")}`,
    },
    {
        id: "tool-framework",
        icon: "wrench",
        title: t("dashboard.features.toolFramework.title"),
        description: t("dashboard.features.toolFramework.description"),
        link: "/tools",
        linkText: t("dashboard.features.toolFramework.linkText"),
        color: "accent",
        stats: `${toolStore.tools.length} ${t("dashboard.stats.tools")}`,
    },
    {
        id: "skill-system",
        icon: "sparkles",
        title: t("dashboard.features.skillSystem.title"),
        description: t("dashboard.features.skillSystem.description"),
        link: "/skills",
        linkText: t("dashboard.features.skillSystem.linkText"),
        color: "purple",
        stats: `${skillStore.skills.length} ${t("dashboard.stats.skills")}`,
    },
    {
        id: "acp-protocol",
        icon: "plug",
        title: t("dashboard.features.acpProtocol.title"),
        description: t("dashboard.features.acpProtocol.description"),
        link: "/acp-config",
        linkText: t("dashboard.features.acpProtocol.linkText"),
        color: "cyan",
        stats: t("dashboard.features.acpProtocol.stats"),
    },
]);

const getIconSvg = (icon: string) => {
    const icons: Record<string, string> = {
        server: '<path d="M2 20a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2Z"/><path d="M6 12h.01M6 16h.01M6 8h.01"/><path d="M10 16h8M10 12h8M10 8h8"/>',
        wrench: '<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>',
        sparkles:
            '<path d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275L12 3Z"/><path d="M5 3v4"/><path d="M19 17v4"/><path d="M3 5h4"/><path d="M17 19h4"/>',
        plug: '<path d="M12 22v-5"/><path d="M9 7V2"/><path d="M15 7V2"/><path d="M6 13H4a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v2a2 2 0 0 1-2 2h-2"/><path d="M6 13v6a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2v-6"/>',
    };
    return icons[icon] || "";
};
</script>

<template>
    <div class="dashboard-container">
        <!-- Header section -->
        <header class="dashboard-header">
            <div class="header-content">
                <div class="title-section">
                    <h1 class="page-title">
                        {{ t("dashboard.overviewTitle") }}
                    </h1>
                    <p class="page-subtitle">
                        {{ t("dashboard.overviewSubtitle") }}
                    </p>
                </div>

                <!-- Status indicator -->
                <div class="status-badge" :class="statusClass">
                    <span class="status-dot"></span>
                    <span class="status-label">{{ statusText }}</span>
                </div>
            </div>
        </header>

        <!-- Quick stats bar -->
        <div class="quick-stats">
            <div class="quick-stat">
                <div class="stat-icon providers">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <rect x="2" y="2" width="20" height="8" rx="2" />
                        <rect x="2" y="14" width="20" height="8" rx="2" />
                        <line x1="6" y1="6" x2="6.01" y2="6" />
                        <line x1="6" y1="18" x2="6.01" y2="18" />
                    </svg>
                </div>
                <div class="stat-info">
                    <span class="stat-value">{{
                        providerStore.providers.length
                    }}</span>
                    <span class="stat-label">{{
                        t("dashboard.stats.providers")
                    }}</span>
                </div>
            </div>

            <div class="quick-stat">
                <div class="stat-icon skills">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <polygon
                            points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
                        />
                    </svg>
                </div>
                <div class="stat-info">
                    <span class="stat-value">{{
                        skillStore.skills.length
                    }}</span>
                    <span class="stat-label">{{
                        t("dashboard.stats.skills")
                    }}</span>
                </div>
            </div>

            <div class="quick-stat">
                <div class="stat-icon tools">
                    <svg
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
                <div class="stat-info">
                    <span class="stat-value">{{ toolStore.tools.length }}</span>
                    <span class="stat-label">{{
                        t("dashboard.stats.tools")
                    }}</span>
                </div>
            </div>

            <div class="quick-stat">
                <div class="stat-icon messages">
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
                </div>
                <div class="stat-info">
                    <span class="stat-value">{{
                        chatStore.messages.length
                    }}</span>
                    <span class="stat-label">{{
                        t("dashboard.messageCount")
                    }}</span>
                </div>
            </div>
        </div>

        <!-- Feature cards -->
        <section class="features-section">
            <h2 class="section-title">{{ t("dashboard.featuresSection") }}</h2>
            <div class="features-grid">
                <div
                    v-for="feature in features"
                    :key="feature.id"
                    class="feature-card"
                    :class="`feature-${feature.color}`"
                    @click="router.push(feature.link)"
                >
                    <div class="feature-header">
                        <div class="feature-icon">
                            <svg
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                v-html="getIconSvg(feature.icon)"
                            />
                        </div>
                        <span class="feature-stats">{{ feature.stats }}</span>
                    </div>
                    <h3 class="feature-title">{{ feature.title }}</h3>
                    <p class="feature-description">{{ feature.description }}</p>
                    <div class="feature-link">
                        <span>{{ feature.linkText }}</span>
                        <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <path d="M5 12h14M12 5l7 7-7 7" />
                        </svg>
                    </div>
                </div>
            </div>
        </section>

        <!-- Two column section: Active Provider & Recent Messages -->
        <section class="two-column-section">
            <!-- Active Provider Panel -->
            <div class="panel">
                <div class="panel-header">
                    <h3 class="panel-title">
                        {{ t("dashboard.activeProvider") }}
                    </h3>
                    <button class="link-btn" @click="router.push('/providers')">
                        {{ t("dashboard.manage") }}
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

                <div v-if="activeProvider" class="provider-card">
                    <div class="provider-status">
                        <span class="status-indicator active"></span>
                        <span class="provider-name">{{
                            activeProvider.name
                        }}</span>
                    </div>
                    <div class="provider-details">
                        <div class="detail-item">
                            <span class="detail-label">{{
                                t(
                                    "providers.type." +
                                        activeProvider.provider_type,
                                )
                            }}</span>
                        </div>
                        <div
                            v-if="activeProvider.config.default_model"
                            class="detail-item"
                        >
                            <span class="detail-label">{{
                                activeProvider.config.default_model
                            }}</span>
                        </div>
                    </div>
                </div>

                <div v-else class="empty-state">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <circle cx="12" cy="12" r="10" />
                        <line x1="12" y1="8" x2="12" y2="12" />
                        <line x1="12" y1="16" x2="12.01" y2="16" />
                    </svg>
                    <p>{{ t("dashboard.noActiveProvider") }}</p>
                    <button
                        class="btn btn-primary"
                        @click="router.push('/providers')"
                    >
                        {{ t("providers.addFirstProvider") }}
                    </button>
                </div>
            </div>

            <!-- Recent Messages Panel -->
            <div class="panel">
                <div class="panel-header">
                    <h3 class="panel-title">
                        {{ t("dashboard.recentMessages") }}
                    </h3>
                    <button class="link-btn" @click="router.push('/chat')">
                        {{ t("dashboard.viewAll") }}
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

                <div v-if="recentMessages.length > 0" class="messages-list">
                    <div
                        v-for="(msg, index) in recentMessages"
                        :key="index"
                        class="message-item"
                    >
                        <div class="message-role" :class="msg.role">
                            {{
                                msg.role === "user"
                                    ? t("chat.userMessage")
                                    : t("chat.assistantMessage")
                            }}
                        </div>
                        <div class="message-preview">
                            {{ msg.content.substring(0, 100)
                            }}{{ msg.content.length > 100 ? "..." : "" }}
                        </div>
                    </div>
                </div>

                <div v-else class="empty-state">
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
                    <p>{{ t("dashboard.noMessages") }}</p>
                    <button
                        class="btn btn-primary"
                        @click="router.push('/chat')"
                    >
                        {{ t("dashboard.startChat") }}
                    </button>
                </div>
            </div>
        </section>

        <!-- Quick Actions -->
        <section class="actions-section">
            <h2 class="section-title">{{ t("dashboard.quickActions") }}</h2>
            <div class="actions-grid">
                <button class="action-card" @click="router.push('/chat')">
                    <div class="action-icon chat">
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
                    </div>
                    <span class="action-label">{{
                        t("dashboard.startChat")
                    }}</span>
                </button>

                <button class="action-card" @click="router.push('/providers')">
                    <div class="action-icon providers">
                        <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <rect x="2" y="2" width="20" height="8" rx="2" />
                            <rect x="2" y="14" width="20" height="8" rx="2" />
                            <line x1="6" y1="6" x2="6.01" y2="6" />
                            <line x1="6" y1="18" x2="6.01" y2="18" />
                        </svg>
                    </div>
                    <span class="action-label">{{
                        t("dashboard.configureProvider")
                    }}</span>
                </button>

                <button class="action-card" @click="router.push('/skills')">
                    <div class="action-icon skills">
                        <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <polygon
                                points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
                            />
                        </svg>
                    </div>
                    <span class="action-label">{{ t("skills.addSkill") }}</span>
                </button>

                <button class="action-card" @click="router.push('/api-test')">
                    <div class="action-icon api">
                        <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <path d="M16 18l6-6-6-6" />
                            <path d="M8 6l-6 6 6 6" />
                        </svg>
                    </div>
                    <span class="action-label">{{
                        t("dashboard.apiTest")
                    }}</span>
                </button>
            </div>
        </section>
    </div>
</template>

<style scoped>
.dashboard-container {
    min-height: 100%;
    padding: 1.5rem;
    max-width: 1400px;
    margin: 0 auto;
}

/* Header */
.dashboard-header {
    margin-bottom: 1.5rem;
}

.header-content {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: 1rem;
}

.page-title {
    font-size: 2rem;
    font-weight: 700;
    color: hsl(var(--foreground));
    margin-bottom: 0.25rem;
}

.page-subtitle {
    color: hsl(var(--muted-foreground));
    font-size: 1rem;
}

.status-badge {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: 9999px;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    font-size: 0.875rem;
    font-weight: 500;
}

.status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: hsl(var(--muted-foreground));
}

.status-badge.status-running .status-dot {
    background: #10b981;
    animation: pulse 2s ease-in-out infinite;
}

.status-badge.status-error .status-dot {
    background: #ef4444;
}

.status-badge.status-stopped .status-dot {
    background: hsl(var(--muted-foreground));
}

@keyframes pulse {
    0%,
    100% {
        opacity: 1;
    }
    50% {
        opacity: 0.5;
    }
}

/* Quick Stats */
.quick-stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
    margin-bottom: 2rem;
}

.quick-stat {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    transition: all 0.2s ease;
}

.quick-stat:hover {
    border-color: hsl(var(--primary) / 0.5);
    transform: translateY(-2px);
}

.stat-icon {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
}

.stat-icon svg {
    width: 20px;
    height: 20px;
}

.stat-icon.providers {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
}

.stat-icon.skills {
    background: hsl(280 70% 60% / 0.1);
    color: hsl(280 70% 60%);
}

.stat-icon.tools {
    background: hsl(150 70% 50% / 0.1);
    color: hsl(150 70% 50%);
}

.stat-icon.messages {
    background: hsl(200 70% 60% / 0.1);
    color: hsl(200 70% 60%);
}

.stat-info {
    display: flex;
    flex-direction: column;
}

.stat-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: hsl(var(--foreground));
}

.quick-stat .stat-label {
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
}

/* Section Title */
.section-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin-bottom: 1rem;
}

/* Feature Cards */
.features-section {
    margin-bottom: 2rem;
}

.features-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
}

.feature-card {
    padding: 1.25rem;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 16px;
    cursor: pointer;
    transition: all 0.2s ease;
    position: relative;
    overflow: hidden;
}

.feature-card::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: transparent;
    transition: background 0.2s ease;
}

.feature-card:hover {
    border-color: hsl(var(--primary) / 0.5);
    transform: translateY(-4px);
    box-shadow: 0 8px 24px hsl(var(--foreground) / 0.1);
}

.feature-card:hover::before {
    background: hsl(var(--primary));
}

.feature-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 0.75rem;
}

.feature-icon {
    width: 48px;
    height: 48px;
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.2s ease;
}

.feature-card:hover .feature-icon {
    transform: scale(1.1);
}

.feature-icon svg {
    width: 24px;
    height: 24px;
}

.feature-primary .feature-icon {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
}

.feature-accent .feature-icon {
    background: hsl(150 70% 50% / 0.1);
    color: hsl(150 70% 50%);
}

.feature-purple .feature-icon {
    background: hsl(280 70% 60% / 0.1);
    color: hsl(280 70% 60%);
}

.feature-cyan .feature-icon {
    background: hsl(190 70% 55% / 0.1);
    color: hsl(190 70% 55%);
}

.feature-stats {
    font-size: 0.75rem;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
    padding: 0.25rem 0.5rem;
    background: hsl(var(--secondary));
    border-radius: 9999px;
}

.feature-title {
    font-size: 1rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin-bottom: 0.375rem;
}

.feature-description {
    font-size: 0.8125rem;
    color: hsl(var(--muted-foreground));
    line-height: 1.5;
    margin-bottom: 0.75rem;
}

.feature-link {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: hsl(var(--primary));
    transition: gap 0.2s ease;
}

.feature-card:hover .feature-link {
    gap: 0.5rem;
}

.feature-link svg {
    width: 16px;
    height: 16px;
}

/* Two Column Section */
.two-column-section {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
    margin-bottom: 2rem;
}

.panel {
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 16px;
    padding: 1.25rem;
}

.panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
}

.panel-title {
    font-size: 1rem;
    font-weight: 600;
    color: hsl(var(--foreground));
}

.link-btn {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: hsl(var(--primary));
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    transition: gap 0.2s ease;
}

.link-btn:hover {
    gap: 0.5rem;
}

.link-btn svg {
    width: 16px;
    height: 16px;
}

/* Provider Card */
.provider-card {
    background: hsl(var(--secondary) / 0.5);
    border-radius: 12px;
    padding: 1rem;
}

.provider-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
}

.status-indicator {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: hsl(var(--muted-foreground));
}

.status-indicator.active {
    background: #10b981;
    box-shadow: 0 0 8px rgba(16, 185, 129, 0.5);
}

.provider-name {
    font-weight: 600;
    color: hsl(var(--foreground));
}

.provider-details {
    display: flex;
    gap: 0.75rem;
}

.detail-item {
    padding: 0.25rem 0.5rem;
    background: hsl(var(--background));
    border-radius: 6px;
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
}

/* Messages List */
.messages-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.message-item {
    padding: 0.75rem;
    background: hsl(var(--secondary) / 0.5);
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.2s ease;
}

.message-item:hover {
    background: hsl(var(--secondary));
}

.message-role {
    font-size: 0.75rem;
    font-weight: 500;
    margin-bottom: 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.message-role.user {
    color: hsl(var(--primary));
}

.message-role.assistant {
    color: hsl(280 70% 60%);
}

.message-preview {
    font-size: 0.8125rem;
    color: hsl(var(--muted-foreground));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* Empty State */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    text-align: center;
}

.empty-state svg {
    width: 48px;
    height: 48px;
    color: hsl(var(--muted-foreground) / 0.5);
    margin-bottom: 1rem;
}

.empty-state p {
    color: hsl(var(--muted-foreground));
    margin-bottom: 1rem;
}

/* Actions Section */
.actions-section {
    margin-bottom: 2rem;
}

.actions-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
}

.action-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 1rem;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    cursor: pointer;
    transition: all 0.2s ease;
}

.action-card:hover {
    border-color: hsl(var(--primary) / 0.5);
    transform: translateY(-2px);
}

.action-icon {
    width: 48px;
    height: 48px;
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
}

.action-icon svg {
    width: 24px;
    height: 24px;
}

.action-icon.chat {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
}

.action-icon.providers {
    background: hsl(200 70% 60% / 0.1);
    color: hsl(200 70% 60%);
}

.action-icon.skills {
    background: hsl(280 70% 60% / 0.1);
    color: hsl(280 70% 60%);
}

.action-icon.api {
    background: hsl(150 70% 50% / 0.1);
    color: hsl(150 70% 50%);
}

.action-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: hsl(var(--foreground));
}

/* Responsive */
@media (max-width: 1280px) {
    .features-grid {
        grid-template-columns: repeat(2, 1fr);
    }

    .actions-grid {
        grid-template-columns: repeat(2, 1fr);
    }
}

@media (max-width: 1024px) {
    .quick-stats {
        grid-template-columns: repeat(2, 1fr);
    }

    .two-column-section {
        grid-template-columns: 1fr;
    }
}

@media (max-width: 768px) {
    .dashboard-container {
        padding: 1rem;
    }

    .page-title {
        font-size: 1.5rem;
    }

    .features-grid {
        grid-template-columns: 1fr;
    }

    .actions-grid {
        grid-template-columns: 1fr;
    }

    .quick-stats {
        grid-template-columns: repeat(2, 1fr);
    }
}

@media (max-width: 480px) {
    .quick-stats {
        grid-template-columns: 1fr;
    }
}
</style>

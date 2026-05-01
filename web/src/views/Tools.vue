<script setup lang="ts">
import { onMounted } from "vue";
import { useToolStore } from "../stores/tool";

const toolStore = useToolStore();

onMounted(() => {
    toolStore.fetchTools();
});

const paramTypeColor = (type: string) => {
    switch (type) {
        case "string":
            return "param-string";
        case "number":
            return "param-number";
        case "integer":
            return "param-number";
        case "boolean":
            return "param-boolean";
        case "array":
            return "param-array";
        case "object":
            return "param-object";
        default:
            return "param-default";
    }
};
</script>

<template>
    <div class="tools-view">
        <!-- Header -->
        <div class="header">
            <h1 class="header-title">工具</h1>
            <p class="header-desc">智能体可用的已注册工具</p>
        </div>

        <!-- Error -->
        <div v-if="toolStore.error" class="error-banner">
            <svg
                class="error-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
            >
                <circle cx="12" cy="12" r="10" />
                <line x1="12" y1="8" x2="12" y2="12" />
                <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <span>{{ toolStore.error }}</span>
        </div>

        <!-- Loading -->
        <div
            v-if="toolStore.loading && toolStore.tools.length === 0"
            class="loading-state"
        >
            <div class="loading-spinner"></div>
            <span class="loading-text">加载中...</span>
        </div>

        <!-- Empty State -->
        <div v-else-if="toolStore.tools.length === 0" class="empty-state">
            <div class="empty-icon-wrapper">
                <svg
                    class="empty-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <path
                        d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"
                    />
                </svg>
            </div>
            <h3 class="empty-title">暂无注册工具</h3>
            <p class="empty-desc">工具在智能体初始化时通过代码注册</p>
        </div>

        <!-- Tool Cards -->
        <div v-else class="tool-list">
            <div
                v-for="tool in toolStore.tools"
                :key="tool.name"
                class="tool-card"
            >
                <div class="tool-card-content">
                    <div class="tool-icon-wrapper">
                        <svg
                            class="tool-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <path
                                d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"
                            />
                        </svg>
                    </div>
                    <div class="tool-info">
                        <div class="tool-name-row">
                            <h3 class="tool-name">{{ tool.name }}</h3>
                        </div>
                        <p class="tool-desc">{{ tool.description }}</p>

                        <!-- Parameters -->
                        <div
                            v-if="tool.parameters.length > 0"
                            class="params-section"
                        >
                            <h4 class="params-label">参数</h4>
                            <div class="params-list">
                                <div
                                    v-for="param in tool.parameters"
                                    :key="param.name"
                                    class="param-row"
                                >
                                    <span class="param-name">{{
                                        param.name
                                    }}</span>
                                    <span
                                        class="param-type-badge"
                                        :class="
                                            paramTypeColor(param.param_type)
                                        "
                                    >
                                        {{ param.param_type }}
                                    </span>
                                    <span
                                        v-if="param.required"
                                        class="param-required-badge"
                                    >
                                        必填
                                    </span>
                                    <span
                                        v-if="param.description"
                                        class="param-desc"
                                    >
                                        — {{ param.description }}
                                    </span>
                                </div>
                            </div>
                        </div>
                        <div v-else class="no-params">无参数</div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.tools-view {
    padding: 1.5rem;
    max-width: 72rem;
    margin: 0 auto;
    animation: fadeIn 0.3s ease;
}

@keyframes fadeIn {
    from {
        opacity: 0;
        transform: translateY(4px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

.header {
    margin-bottom: 1.5rem;
}

.header-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-text);
    line-height: 1.3;
}

.header-desc {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    margin-top: 0.25rem;
}

.error-banner {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    margin-bottom: 1rem;
    background: var(--color-danger-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    color: var(--color-danger);
}

.error-icon {
    width: 1rem;
    height: 1rem;
    flex-shrink: 0;
}

.loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 3rem 0;
    color: var(--color-text-muted);
}

.loading-spinner {
    width: 1.25rem;
    height: 1.25rem;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

.loading-text {
    font-size: 0.875rem;
    color: var(--color-text-muted);
}

.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 5rem 0;
}

.empty-icon-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 4.5rem;
    height: 4.5rem;
    border-radius: var(--radius-xl);
    background: var(--color-bg-mute);
    border: 1px solid var(--color-border);
    margin-bottom: 1.25rem;
}

.empty-icon {
    width: 2rem;
    height: 2rem;
    color: var(--color-text-dim);
}

.empty-title {
    font-size: 1.125rem;
    font-weight: 500;
    color: var(--color-text-secondary);
    margin-bottom: 0.5rem;
}

.empty-desc {
    font-size: 0.875rem;
    color: var(--color-text-dim);
}

.tool-list {
    display: grid;
    gap: 0.75rem;
}

.tool-card {
    background: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
    transition: border-color 0.2s ease;
}

.tool-card:hover {
    border-color: var(--color-border-hover);
}

.tool-card-content {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
}

.tool-icon-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.25rem;
    height: 2.25rem;
    border-radius: var(--radius-md);
    background: var(--color-accent-soft);
    flex-shrink: 0;
}

.tool-icon {
    width: 1.125rem;
    height: 1.125rem;
    color: var(--color-accent);
}

.tool-info {
    flex: 1;
    min-width: 0;
}

.tool-name-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.tool-name {
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--color-text);
    font-family: var(--font-mono, ui-monospace, monospace);
}

.tool-desc {
    font-size: 0.875rem;
    color: var(--color-text-secondary);
    margin-top: 0.125rem;
}

.params-section {
    margin-top: 0.75rem;
}

.params-label {
    font-size: 0.6875rem;
    font-weight: 500;
    color: var(--color-text-dim);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 0.5rem;
}

.params-list {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
}

.param-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
}

.param-name {
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--color-text);
    font-size: 0.8125rem;
}

.param-type-badge {
    padding: 0.125rem 0.375rem;
    font-size: 0.6875rem;
    border-radius: var(--radius-sm);
    font-weight: 500;
    font-family: var(--font-mono, ui-monospace, monospace);
}

.param-type-badge.param-string {
    color: var(--color-success);
    background: var(--color-success-soft);
}

.param-type-badge.param-number {
    color: var(--color-info);
    background: var(--color-info-soft);
}

.param-type-badge.param-boolean {
    color: var(--color-warning);
    background: var(--color-warning-soft);
}

.param-type-badge.param-array {
    color: var(--color-accent);
    background: var(--color-accent-soft);
}

.param-type-badge.param-object {
    color: var(--color-info);
    background: var(--color-info-soft);
}

.param-type-badge.param-default {
    color: var(--color-text-muted);
    background: var(--color-bg-mute);
}

.param-required-badge {
    padding: 0.125rem 0.375rem;
    font-size: 0.6875rem;
    border-radius: var(--radius-sm);
    font-weight: 500;
    color: var(--color-danger);
    background: var(--color-danger-soft);
}

.param-desc {
    font-size: 0.75rem;
    color: var(--color-text-dim);
}

.no-params {
    margin-top: 0.75rem;
    font-size: 0.75rem;
    color: var(--color-text-dim);
}
</style>

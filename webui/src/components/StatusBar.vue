<script setup lang="ts">
import { computed } from "vue";
import { useAgentStore } from "../stores/agent";

const agentStore = useAgentStore();

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
            return "运行中";
        case "error":
            return "错误";
        default:
            return "已停止";
    }
});
</script>

<template>
    <div class="status-bar" :class="statusClass">
        <span class="status-dot pulse-dot"></span>
        <span class="status-label">{{ statusText }}</span>
        <template v-if="agentStore.status.active_provider">
            <span class="status-divider">·</span>
            <span class="status-provider">{{
                agentStore.status.active_provider
            }}</span>
        </template>
        <template v-if="agentStore.status.active_model">
            <span class="status-divider">·</span>
            <span class="status-model">{{
                agentStore.status.active_model
            }}</span>
        </template>
    </div>
</template>

<style scoped>
.status-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    line-height: 1;
}

.status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
}

.status-divider {
    color: var(--color-text-muted);
}

.status-provider {
    color: var(--color-accent);
    font-weight: 500;
}

.status-model {
    color: var(--color-text-secondary);
}

/* Status variants */
.status-running .status-dot {
    background: var(--color-success);
}

.status-running .status-label {
    color: var(--color-success);
}

.status-error .status-dot {
    background: var(--color-danger);
}

.status-error .status-label {
    color: var(--color-danger);
}

.status-stopped .status-dot {
    background: var(--color-text-muted);
}

.status-stopped .status-label {
    color: var(--color-text-muted);
}
</style>

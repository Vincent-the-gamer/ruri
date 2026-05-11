<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useAgentStore } from "../stores/agent";

const { t } = useI18n();
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
            return t("dashboard.status.running");
        case "error":
            return t("dashboard.status.error");
        default:
            return t("dashboard.status.stopped");
    }
});
</script>

<template>
    <div class="status-bar" :class="statusClass">
        <span class="status-dot"></span>
        <span class="status-label">{{ statusText }}</span>
        <template v-if="agentStore.status.active_provider">
            <span class="status-divider"></span>
            <span class="status-provider">{{
                agentStore.status.active_provider
            }}</span>
        </template>
        <template v-if="agentStore.status.active_model">
            <span class="status-divider"></span>
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
    gap: 8px;
    font-size: 12px;
    line-height: 1;
    padding: 5px 12px;
    border-radius: 9999px;
    background: hsl(var(--card) / 0.6);
    border: 1px solid hsl(var(--border) / 0.5);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    white-space: nowrap;
    max-width: 320px;
    overflow: hidden;
}

.status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
    transition: background 0.3s ease;
}

.status-label {
    font-weight: 600;
    font-size: 11px;
    letter-spacing: 0.02em;
    transition: color 0.3s ease;
}

.status-divider {
    width: 1px;
    height: 10px;
    background: hsl(var(--border));
    flex-shrink: 0;
}

.status-provider {
    color: hsl(var(--primary));
    font-weight: 600;
    font-size: 11px;
}

.status-model {
    color: hsl(var(--muted-foreground));
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* Status variants */
.status-running .status-dot {
    background: #10b981;
    box-shadow: 0 0 6px #10b981 / 0.5;
    animation: pulse-dot 2s ease-in-out infinite;
}

.status-running .status-label {
    color: #10b981;
}

.status-error .status-dot {
    background: #ef4444;
    box-shadow: 0 0 6px #ef4444 / 0.4;
}

.status-error .status-label {
    color: #ef4444;
}

.status-stopped .status-dot {
    background: hsl(var(--muted-foreground));
}

.status-stopped .status-label {
    color: hsl(var(--muted-foreground));
}

@keyframes pulse-dot {
    0%,
    100% {
        opacity: 1;
    }
    50% {
        opacity: 0.5;
    }
}

/* Responsive: hide model name on small screens */
@media (max-width: 768px) {
    .status-model {
        display: none;
    }
    .status-bar {
        max-width: 200px;
    }
}

@media (max-width: 480px) {
    .status-provider {
        display: none;
    }
    .status-bar {
        max-width: 120px;
    }
}
</style>

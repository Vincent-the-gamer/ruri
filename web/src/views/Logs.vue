<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick, watch } from "vue";
import * as api from "../api";
import type { LogEntry, LogLevel } from "../types";

const logs = ref<LogEntry[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const ws = ref<WebSocket | null>(null);
const autoScroll = ref(true);
const filterLevel = ref<LogLevel | "all">("all");
const filterTarget = ref("");
const searchQuery = ref("");
const logContainer = ref<HTMLElement | null>(null);

const filteredLogs = computed(() => {
    const levels: LogLevel[] = ["error", "warn", "info", "debug", "trace"];
    return logs.value.filter((log) => {
        // Filter by level
        if (filterLevel.value !== "all") {
            const currentLevelIndex = levels.indexOf(log.level);
            const filterLevelIndex = levels.indexOf(
                filterLevel.value as LogLevel,
            );
            if (
                filterLevelIndex !== -1 &&
                currentLevelIndex < filterLevelIndex
            ) {
                return false;
            }
        }

        // Filter by target
        if (
            filterTarget.value &&
            !log.target.toLowerCase().includes(filterTarget.value.toLowerCase())
        ) {
            return false;
        }

        // Filter by search query
        if (
            searchQuery.value &&
            !log.message.toLowerCase().includes(searchQuery.value.toLowerCase())
        ) {
            return false;
        }

        return true;
    });
});

async function fetchLogs() {
    loading.value = true;
    error.value = null;
    try {
        logs.value = await api.getLogs();
        await nextTick();
        if (autoScroll.value) {
            scrollToBottom();
        }
    } catch (e: unknown) {
        error.value = e instanceof Error ? e.message : "Failed to fetch logs";
    } finally {
        loading.value = false;
    }
}

async function clearLogs() {
    if (confirm("确定要清空所有日志吗？")) {
        try {
            await api.clearLogs();
            logs.value = [];
        } catch (e: unknown) {
            alert(e instanceof Error ? e.message : "Failed to clear logs");
        }
    }
}

function scrollToBottom() {
    if (logContainer.value) {
        logContainer.value.scrollTop = logContainer.value.scrollHeight;
    }
}

function handleScroll() {
    if (logContainer.value) {
        const { scrollTop, scrollHeight, clientHeight } = logContainer.value;
        autoScroll.value = scrollTop + clientHeight >= scrollHeight - 10;
    }
}

function openLogsStream() {
    if (ws.value) {
        return;
    }

    try {
        ws.value = api.openLogsStream();
        ws.value.onmessage = (event) => {
            try {
                const logEntry: LogEntry = JSON.parse(event.data);
                logs.value.push(logEntry);
                if (autoScroll.value) {
                    nextTick(() => scrollToBottom());
                }
            } catch (e) {
                console.error("Failed to parse log entry:", e);
            }
        };

        ws.value.onerror = () => {
            ws.value?.close();
            ws.value = null;
        };

        ws.value.onclose = () => {
            ws.value = null;
        };
    } catch (e) {
        console.error("Failed to open logs stream:", e);
    }
}

function closeLogsStream() {
    if (ws.value) {
        ws.value.close();
        ws.value = null;
    }
}

function getLevelClass(level: LogLevel): string {
    switch (level) {
        case "error":
            return "level-error";
        case "warn":
            return "level-warn";
        case "info":
            return "level-info";
        case "debug":
            return "level-debug";
        case "trace":
            return "level-trace";
        default:
            return "";
    }
}

function formatTimestamp(timestamp: number): string {
    const date = new Date(timestamp);
    const hours = date.getHours().toString().padStart(2, "0");
    const minutes = date.getMinutes().toString().padStart(2, "0");
    const seconds = date.getSeconds().toString().padStart(2, "0");
    const ms = date.getMilliseconds().toString().padStart(3, "0");
    return `${hours}:${minutes}:${seconds}.${ms}`;
}

onMounted(() => {
    fetchLogs();
    openLogsStream();
});

onUnmounted(() => {
    closeLogsStream();
});
</script>

<template>
    <div class="logs-page">
        <div class="page-header">
            <div class="header-content">
                <div>
                    <h1 class="page-title">系统日志</h1>
                    <p class="page-subtitle">查看和管理系统运行日志</p>
                </div>
                <button class="btn btn-danger" @click="clearLogs">
                    <svg
                        class="btn-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <polyline points="3 6 5 6 21 6" />
                        <path
                            d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                        />
                    </svg>
                    清空日志
                </button>
            </div>
        </div>

        <div class="logs-container">
            <div class="logs-sidebar">
                <div class="filter-section">
                    <h3 class="filter-title">筛选</h3>

                    <!-- Level Filter -->
                    <div class="filter-group">
                        <label class="filter-label">日志级别</label>
                        <select v-model="filterLevel" class="filter-select">
                            <option value="all">全部级别</option>
                            <option value="error">Error</option>
                            <option value="warn">Warn</option>
                            <option value="info">Info</option>
                            <option value="debug">Debug</option>
                            <option value="trace">Trace</option>
                        </select>
                    </div>

                    <!-- Target Filter -->
                    <div class="filter-group">
                        <label class="filter-label">目标模块</label>
                        <input
                            v-model="filterTarget"
                            type="text"
                            class="filter-input"
                            placeholder="输入模块名称..."
                        />
                    </div>

                    <!-- Search Filter -->
                    <div class="filter-group">
                        <label class="filter-label">搜索</label>
                        <input
                            v-model="searchQuery"
                            type="text"
                            class="filter-input"
                            placeholder="搜索日志内容..."
                        />
                    </div>

                    <!-- Auto Scroll Toggle -->
                    <div class="filter-group">
                        <label class="checkbox-label">
                            <input
                                v-model="autoScroll"
                                type="checkbox"
                                class="checkbox-input"
                            />
                            <span>自动滚动到底部</span>
                        </label>
                    </div>
                </div>

                <!-- Statistics -->
                <div class="stats-section">
                    <h3 class="filter-title">统计</h3>
                    <div class="stats-grid">
                        <div class="stat-item">
                            <span class="stat-value">{{
                                filteredLogs.length
                            }}</span>
                            <span class="stat-label">总日志数</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-value">{{
                                logs.filter((l) => l.level === "error").length
                            }}</span>
                            <span class="stat-label stat-label-error"
                                >错误</span
                            >
                        </div>
                        <div class="stat-item">
                            <span class="stat-value">{{
                                logs.filter((l) => l.level === "warn").length
                            }}</span>
                            <span class="stat-label stat-label-warn">警告</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-value">{{
                                logs.filter((l) => l.level === "info").length
                            }}</span>
                            <span class="stat-label stat-label-info">信息</span>
                        </div>
                    </div>
                    <div class="websocket-status">
                        <span
                            class="status-indicator"
                            :class="{ connected: ws !== null }"
                        ></span>
                        <span>实时日志流</span>
                    </div>
                </div>
            </div>

            <!-- Logs Display -->
            <div class="logs-main">
                <div
                    ref="logContainer"
                    class="logs-display"
                    @scroll="handleScroll"
                >
                    <div v-if="error" class="logs-error">
                        {{ error }}
                    </div>

                    <div
                        v-else-if="loading && filteredLogs.length === 0"
                        class="logs-loading"
                    >
                        加载中...
                    </div>

                    <div
                        v-else-if="filteredLogs.length === 0"
                        class="logs-empty"
                    >
                        没有符合条件的日志
                    </div>

                    <div v-else class="logs-list">
                        <div
                            v-for="(log, index) in filteredLogs"
                            :key="index"
                            class="log-entry"
                            :class="getLevelClass(log.level)"
                        >
                            <div class="log-header">
                                <span class="log-timestamp">{{
                                    formatTimestamp(log.timestamp)
                                }}</span>
                                <span
                                    class="log-level level-badge"
                                    :class="getLevelClass(log.level)"
                                >
                                    {{ log.level.toUpperCase() }}
                                </span>
                                <span class="log-target">{{ log.target }}</span>
                                <span v-if="log.file" class="log-location">
                                    {{ log.file }}:{{ log.line }}
                                </span>
                            </div>
                            <div class="log-message">{{ log.message }}</div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.logs-page {
    padding: 0;
}

.page-header {
    padding: 1.5rem 2rem;
    background: var(--color-bg-mute);
    border-bottom: 1px solid var(--color-border);
}

.header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
}

.page-title {
    font-size: 2rem;
    font-weight: 700;
    margin: 0 0 0.25rem 0;
    background: linear-gradient(
        135deg,
        var(--color-accent) 0%,
        var(--color-primary) 100%
    );
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.page-subtitle {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    margin: 0;
}

.logs-container {
    display: flex;
    height: calc(100vh - 100px);
    min-height: 600px;
}

.logs-sidebar {
    width: 280px;
    padding: 1.5rem;
    background: var(--color-bg-mute);
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    overflow-y: auto;
    flex-shrink: 0;
}

.filter-section,
.stats-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
}

.filter-title {
    font-size: 0.875rem;
    font-weight: 700;
    color: var(--color-text-secondary);
    margin: 0;
}

.filter-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.filter-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-text-muted);
}

.filter-select,
.filter-input {
    padding: 0.625rem 0.875rem;
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    color: var(--color-text-secondary);
    transition: all var(--transition-fast);
}

.filter-select:hover,
.filter-input:hover {
    border-color: var(--color-border-hover);
}

.filter-select:focus,
.filter-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.1);
}

.checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    font-size: 0.875rem;
    color: var(--color-text-secondary);
    cursor: pointer;
}

.checkbox-input {
    cursor: pointer;
}

.stats-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.75rem;
}

.stat-item {
    background: var(--color-bg);
    padding: 0.75rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
}

.stat-value {
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--color-text-primary);
}

.stat-label {
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--color-text-muted);
}

.stat-label-error {
    color: var(--color-danger);
}

.stat-label-warn {
    color: #f59e0b;
}

.stat-label-info {
    color: var(--color-success);
}

.websocket-status {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.75rem;
    background: var(--color-bg);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    font-size: 0.875rem;
    color: var(--color-text-secondary);
}

.status-indicator {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--color-text-muted);
    transition: all var(--transition-fast);
}

.status-indicator.connected {
    background: var(--color-success);
    box-shadow: 0 0 8px rgba(34, 197, 94, 0.5);
}

.logs-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.logs-display {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    background: var(--color-bg);
}

.logs-error,
.logs-loading,
.logs-empty {
    padding: 3rem;
    text-align: center;
    color: var(--color-text-muted);
}

.logs-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.log-entry {
    padding: 0.875rem 1rem;
    background: var(--color-bg-mute);
    border-radius: var(--radius-md);
    border-left: 4px solid var(--color-border);
    transition: all var(--transition-fast);
}

.log-entry:hover {
    border-color: var(--color-border-hover);
}

.log-entry.level-error {
    border-left-color: var(--color-danger);
    background: rgba(239, 68, 68, 0.05);
}

.log-entry.level-warn {
    border-left-color: #f59e0b;
    background: rgba(245, 158, 11, 0.05);
}

.log-entry.level-info {
    border-left-color: var(--color-success);
}

.log-entry.level-debug {
    border-left-color: var(--color-primary);
}

.log-entry.level-trace {
    border-left-color: var(--color-text-muted);
}

.log-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
    font-size: 0.75rem;
}

.log-timestamp {
    font-family: "Monaco", "Menlo", monospace;
    color: var(--color-text-muted);
    font-weight: 600;
}

.level-badge {
    padding: 0.125rem 0.5rem;
    border-radius: 4px;
    font-weight: 700;
    font-size: 0.625rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.level-badge.level-error {
    background: var(--color-danger);
    color: white;
}

.level-badge.level-warn {
    background: #f59e0b;
    color: white;
}

.level-badge.level-info {
    background: var(--color-success);
    color: white;
}

.level-badge.level-debug {
    background: var(--color-primary);
    color: white;
}

.level-badge.level-trace {
    background: var(--color-text-muted);
    color: white;
}

.log-target {
    color: var(--color-text-secondary);
    font-weight: 600;
}

.log-location {
    color: var(--color-text-muted);
    font-family: "Monaco", "Menlo", monospace;
}

.log-message {
    margin: 0;
    color: var(--color-text-primary);
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 0.875rem;
    line-height: 1.5;
}

.btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 1.25rem;
    font-size: 0.875rem;
    font-weight: 600;
    border-radius: var(--radius-md);
    border: none;
    cursor: pointer;
    transition: all var(--transition-fast);
}

.btn-danger {
    background: var(--color-danger);
    color: white;
}

.btn-danger:hover {
    background: #dc2626;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(239, 68, 68, 0.3);
}

.btn-icon {
    width: 16px;
    height: 16px;
}

/* Scrollbar */
.logs-display::-webkit-scrollbar,
.logs-sidebar::-webkit-scrollbar {
    width: 8px;
}

.logs-display::-webkit-scrollbar-track,
.logs-sidebar::-webkit-scrollbar-track {
    background: var(--color-bg-mute);
}

.logs-display::-webkit-scrollbar-thumb,
.logs-sidebar::-webkit-scrollbar-thumb {
    background: var(--color-border);
    border-radius: 4px;
}

.logs-display::-webkit-scrollbar-thumb:hover,
.logs-sidebar::-webkit-scrollbar-thumb:hover {
    background: var(--color-border-hover);
}

/* Responsive */
@media (max-width: 1024px) {
    .logs-container {
        flex-direction: column;
        height: auto;
    }

    .logs-sidebar {
        width: 100%;
        border-right: none;
        border-bottom: 1px solid var(--color-border);
    }

    .logs-display {
        min-height: 600px;
    }
}
</style>

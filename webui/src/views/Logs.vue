<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick } from "vue";
import { useI18n } from "vue-i18n";
import * as api from "../api";
import type { LogEntry, LogLevel } from "../types";

const { t } = useI18n();
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

        if (
            filterTarget.value &&
            !log.target.toLowerCase().includes(filterTarget.value.toLowerCase())
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
        if (autoScroll.value) scrollToBottom();
    } catch (e: unknown) {
        error.value = e instanceof Error ? e.message : "Failed to fetch logs";
    } finally {
        loading.value = false;
    }
}

async function clearLogs() {
    if (confirm(t("logs.clearConfirm"))) {
        try {
            await api.clearLogs();
            logs.value = [];
        } catch (e: unknown) {
            alert(e instanceof Error ? e.message : t("errors.unknown"));
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
    if (ws.value) return;

    try {
        ws.value = api.openLogsStream();
        ws.value.onmessage = (event) => {
            try {
                const logEntry: LogEntry = JSON.parse(event.data);
                logs.value.push(logEntry);
                if (autoScroll.value) nextTick(() => scrollToBottom());
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

// Highlight search query in text - search in original, highlight in escaped
function highlightText(text: string): string {
    const query = searchQuery.value;
    if (!query) return escapeHtml(text);
    // Escape the query for regex special characters
    const regexSafeQuery = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    // Build regex to find the original query in the original text
    const regex = new RegExp(regexSafeQuery, "gi");
    // Split by matches, escape each part, then wrap matches in highlight
    const parts = text.split(regex);
    const result = parts.map((part, i) => {
        const escaped = escapeHtml(part);
        // Even indices are non-matches, odd indices are matches
        return i % 2 === 1
            ? `<mark class="search-highlight">${escaped}</mark>`
            : escaped;
    });
    return result.join("");
}

// Format a log line exactly like a terminal would
function formatLogLine(log: LogEntry): string {
    const ts = formatTimestamp(log.timestamp);
    const level = log.level.toUpperCase().padStart(5);
    const target = log.target;
    const msg = highlightText(log.message);
    return `${ts} ${level} ${target}: ${msg}`;
}

function getLevelClass(level: LogLevel): string {
    switch (level) {
        case "error":
            return "lvl-error";
        case "warn":
            return "lvl-warn";
        case "info":
            return "lvl-info";
        case "debug":
            return "lvl-debug";
        case "trace":
            return "lvl-trace";
        default:
            return "";
    }
}

function formatTimestamp(timestamp: number): string {
    const date = new Date(timestamp);
    const pad = (n: number, len: number) => n.toString().padStart(len, "0");
    const year = date.getFullYear();
    const month = pad(date.getMonth() + 1, 2);
    const day = pad(date.getDate(), 2);
    const hours = pad(date.getHours(), 2);
    const minutes = pad(date.getMinutes(), 2);
    const seconds = pad(date.getSeconds(), 2);
    const ms = pad(date.getMilliseconds(), 3);
    return `${year}-${month}-${day}T${hours}:${minutes}:${seconds}.${ms}Z`;
}

function escapeHtml(text: string): string {
    return text
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");
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
    <div class="terminal-page">
        <!-- Page Header -->
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
                        <path
                            d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                        />
                        <path d="M14 2v6h6" />
                        <line x1="16" x2="8" y1="13" y2="13" />
                        <line x1="16" x2="8" y1="17" y2="17" />
                        <line x1="10" x2="8" y1="9" y2="9" />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">{{ t("logs.title") }}</h1>
                    <p class="header-desc">{{ t("logs.subtitle") }}</p>
                </div>
            </div>
        </div>

        <!-- Terminal Toolbar -->
        <div class="terminal-toolbar">
            <div class="toolbar-left">
                <svg
                    class="toolbar-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <polyline points="4 17 10 11 4 5" />
                    <line x1="12" y1="18" x2="20" y2="18" />
                </svg>
                <span class="toolbar-title">{{ t("logs.terminal") }}</span>
                <span
                    class="toolbar-dot"
                    :class="{ online: ws !== null }"
                ></span>
            </div>

            <div class="toolbar-center">
                <select v-model="filterLevel" class="toolbar-select">
                    <option value="all">{{ t("logs.level.all") }}</option>
                    <option value="error">{{ t("logs.level.error") }}</option>
                    <option value="warn">{{ t("logs.level.warning") }}</option>
                    <option value="info">{{ t("logs.level.info") }}</option>
                    <option value="debug">{{ t("logs.level.debug") }}</option>
                    <option value="trace">{{ t("logs.level.trace") }}</option>
                </select>
                <input
                    v-model="filterTarget"
                    type="text"
                    class="toolbar-input"
                    :placeholder="t('logs.filterModule')"
                />
                <input
                    v-model="searchQuery"
                    type="text"
                    class="toolbar-input"
                    :placeholder="t('logs.searchLogs')"
                />
            </div>

            <div class="toolbar-right">
                <label class="toolbar-toggle">
                    <input v-model="autoScroll" type="checkbox" />
                    <span class="toggle-track"
                        ><span class="toggle-thumb"></span
                    ></span>
                    <span class="toggle-label">{{ t("logs.autoScroll") }}</span>
                </label>
                <button
                    class="toolbar-btn"
                    @click="clearLogs"
                    :title="t('logs.clearLogs')"
                >
                    <svg
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
                </button>
                <button
                    class="toolbar-btn"
                    @click="fetchLogs"
                    :title="t('logs.refresh')"
                >
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <polyline points="23 4 23 10 17 10" />
                        <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
                    </svg>
                </button>
            </div>
        </div>

        <!-- Terminal Output - Pure text lines -->
        <div ref="logContainer" class="terminal-output" @scroll="handleScroll">
            <!-- Error state -->
            <div v-if="error" class="term-line">
                <span class="term-text term-error">{{ error }}</span>
            </div>

            <!-- Loading state -->
            <div
                v-else-if="loading && filteredLogs.length === 0"
                class="term-line"
            >
                <span class="term-text term-dim">{{
                    t("common.loading")
                }}</span>
            </div>

            <!-- Empty state -->
            <div v-else-if="filteredLogs.length === 0" class="term-line">
                <span class="term-text term-dim">{{
                    t("logs.noMatchingLogs")
                }}</span>
            </div>

            <!-- Log lines - pure text -->
            <template v-else>
                <div
                    v-for="(log, index) in filteredLogs"
                    :key="index"
                    class="term-line"
                >
                    <span
                        class="term-text"
                        :class="getLevelClass(log.level)"
                        v-html="formatLogLine(log)"
                    ></span>
                </div>
            </template>
        </div>

        <!-- Status Bar -->
        <div class="terminal-status">
            <span class="status-chunk">Ln {{ filteredLogs.length }}</span>
            <span class="status-chunk status-error"
                >E:{{ logs.filter((l) => l.level === "error").length }}</span
            >
            <span class="status-chunk status-warn"
                >W:{{ logs.filter((l) => l.level === "warn").length }}</span
            >
            <span class="status-chunk status-info"
                >I:{{ logs.filter((l) => l.level === "info").length }}</span
            >
            <span class="status-chunk status-debug"
                >D:{{ logs.filter((l) => l.level === "debug").length }}</span
            >
            <span class="status-chunk status-trace"
                >T:{{ logs.filter((l) => l.level === "trace").length }}</span
            >
            <span class="status-spacer"></span>
            <span class="status-chunk">{{
                ws !== null
                    ? t("logs.realtimeConnected")
                    : t("logs.disconnected")
            }}</span>
        </div>
    </div>
</template>

<style scoped>
.terminal-page {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #0c0c0c;
    font-family:
        "Cascadia Code", "Fira Code", "JetBrains Mono", "Consolas", "Monaco",
        "Menlo", "Courier New", monospace;
}

/* ── Page Header ── */
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

/* ── Toolbar ── */
.terminal-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    background: #1a1a1a;
    border-bottom: 1px solid #2a2a2a;
    gap: 8px;
    flex-shrink: 0;
    user-select: none;
}

.toolbar-left {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
}

.toolbar-icon {
    width: 16px;
    height: 16px;
    color: #888;
}

.toolbar-title {
    font-size: 13px;
    color: #ccc;
    font-weight: 600;
    letter-spacing: 0.5px;
}

.toolbar-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #555;
    transition: background 0.2s;
}

.toolbar-dot.online {
    background: #4ec9b0;
    box-shadow: 0 0 6px rgba(78, 201, 176, 0.4);
}

.toolbar-center {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    max-width: 560px;
}

.toolbar-select,
.toolbar-input {
    padding: 4px 8px;
    background: #0c0c0c;
    border: 1px solid #2a2a2a;
    border-radius: 3px;
    font-size: 12px;
    color: #ccc;
    font-family: inherit;
    outline: none;
    transition: border-color 0.15s;
}

.toolbar-select {
    min-width: 70px;
}

.toolbar-input {
    flex: 1;
    min-width: 100px;
}

.toolbar-select:focus,
.toolbar-input:focus {
    border-color: #007acc;
}

.toolbar-right {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
}

/* Toggle */
.toolbar-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    user-select: none;
}

.toolbar-toggle input {
    display: none;
}

.toggle-track {
    width: 28px;
    height: 14px;
    background: #333;
    border-radius: 7px;
    position: relative;
    transition: background 0.2s;
}

.toggle-thumb {
    width: 10px;
    height: 10px;
    background: #888;
    border-radius: 50%;
    position: absolute;
    top: 2px;
    left: 2px;
    transition: transform 0.2s;
}

.toolbar-toggle input:checked + .toggle-track {
    background: #007acc;
}

.toolbar-toggle input:checked + .toggle-track .toggle-thumb {
    transform: translateX(14px);
}

.toggle-label {
    font-size: 11px;
    color: #888;
    letter-spacing: 0.3px;
}

/* Buttons */
.toolbar-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    background: transparent;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    color: #888;
    transition:
        background 0.15s,
        color 0.15s;
}

.toolbar-btn:hover {
    background: #2a2a2a;
    color: #ccc;
}

.toolbar-btn svg {
    width: 14px;
    height: 14px;
}

/* ── Terminal Output ── */
.terminal-output {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 8px 12px;
    background: #0c0c0c;
    font-size: 13px;
    line-height: 1.6;
}

/* Each line is pure text with preserved whitespace */
.term-line {
    white-space: pre;
    line-height: 1.6;
}

.term-text {
    color: #cccccc;
}

/* Level colors */
.term-text.lvl-error {
    color: #f44747;
}

.term-text.lvl-warn {
    color: #cca700;
}

.term-text.lvl-info {
    color: #4ec9b0;
}

.term-text.lvl-debug {
    color: #569cd6;
}

.term-text.lvl-trace {
    color: #6a9955;
}

/* Special states */
.term-error {
    color: #f44747;
}

.term-dim {
    color: #555;
}

/* Search highlight */
.search-highlight {
    background: #613a0b;
    color: #ffa657;
    padding: 0 2px;
    border-radius: 2px;
}

/* ── Status Bar ── */
.terminal-status {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 3px 12px;
    background: #007acc;
    color: #fff;
    font-size: 12px;
    flex-shrink: 0;
    user-select: none;
}

.status-chunk {
    display: flex;
    align-items: center;
    gap: 4px;
    white-space: nowrap;
}

.status-spacer {
    flex: 1;
}

.status-error {
    color: #ffd7d7;
}

.status-warn {
    color: #fff5cc;
}

.status-info {
    color: #ccffd8;
}

.status-debug {
    color: #cce5ff;
}

.status-trace {
    color: #e0ffe0;
}

/* ── Scrollbar ── */
.terminal-output::-webkit-scrollbar {
    width: 8px;
}

.terminal-output::-webkit-scrollbar-track {
    background: #0c0c0c;
}

.terminal-output::-webkit-scrollbar-thumb {
    background: #2a2a2a;
    border-radius: 4px;
    border: 1px solid #0c0c0c;
}

.terminal-output::-webkit-scrollbar-thumb:hover {
    background: #3a3a3a;
}

/* ── Responsive ── */
@media (max-width: 768px) {
    .toolbar-center {
        display: none;
    }

    .terminal-output {
        font-size: 11px;
    }

    .terminal-status {
        gap: 8px;
        font-size: 11px;
    }
}

@media (max-width: 640px) {
    .page-header {
        flex-direction: column;
        align-items: flex-start;
    }
}
</style>

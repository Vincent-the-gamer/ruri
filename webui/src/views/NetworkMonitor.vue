<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch, nextTick } from "vue";
import { useI18n } from "vue-i18n";
import { Chart, registerables, type TooltipItem } from "chart.js";
import "chartjs-adapter-date-fns";
import axios from "axios";

Chart.register(...registerables);

const { t, locale } = useI18n();

// ─── Types ──────────────────────────────────────────────────────

interface TimeSeriesPoint {
    timestamp: number;
    value: number;
}

interface RequestMetrics {
    days: number;
    total: number;
    series: TimeSeriesPoint[];
}

interface TrafficMetrics {
    days: number;
    total_in: number;
    total_out: number;
    series_in: TimeSeriesPoint[];
    series_out: TimeSeriesPoint[];
}

interface TokenSeriesProvider {
    provider_name: string;
    total_tokens: number;
    points: TimeSeriesPoint[];
}

interface TokenMetrics {
    days: number;
    total_tokens: number;
    token_series: TokenSeriesProvider[];
    tokens_by_provider: { provider_name: string; tokens: number }[];
    tokens_by_source: { source: string; tokens: number }[];
}

// ─── State ──────────────────────────────────────────────────────

type TimeRange = 1 | 3 | 7;

const selectedRange = ref<TimeRange>(1);
const loading = ref(true);
const error = ref("");
const lastUpdated = ref<Date | null>(null);

const requestMetrics = ref<RequestMetrics | null>(null);
const trafficMetrics = ref<TrafficMetrics | null>(null);
const tokenMetrics = ref<TokenMetrics | null>(null);

// Chart canvases
const requestChartCanvas = ref<HTMLCanvasElement | null>(null);
const trafficChartCanvas = ref<HTMLCanvasElement | null>(null);
const tokenChartCanvas = ref<HTMLCanvasElement | null>(null);

let requestChart: Chart | null = null;
let trafficChart: Chart | null = null;
let tokenChart: Chart | null = null;
let ws: WebSocket | null = null;
let wsReconnectTimer: number | null = null;

// ─── Computed ───────────────────────────────────────────────────

const rangeLabel = computed(() => {
    return `${selectedRange.value} ${t("networkMonitor.day")}`;
});

const rangeOptions: TimeRange[] = [1, 3, 7];

const lastUpdatedLabel = computed(() => {
    if (!lastUpdated.value) return t("networkMonitor.notUpdated");
    const loc = locale.value === "en-US" ? "en-US" : "zh-CN";
    return lastUpdated.value.toLocaleTimeString(loc, {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
    });
});

// ─── Format helpers ─────────────────────────────────────────────

function formatNumber(n: number): string {
    return new Intl.NumberFormat("zh-CN").format(n);
}

function formatBytes(bytes: number): string {
    if (bytes >= 1073741824) return (bytes / 1073741824).toFixed(2) + " GB";
    if (bytes >= 1048576) return (bytes / 1048576).toFixed(2) + " MB";
    if (bytes >= 1024) return (bytes / 1024).toFixed(1) + " KB";
    return bytes + " B";
}

function formatTokens(n: number): string {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + "M";
    if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
    return n.toString();
}

function sourceLabel(source: string): string {
    if (source === "debug_session")
        return t("networkMonitor.sourceDebugSession");
    if (source.startsWith("profile:")) {
        const name = source.slice(8);
        return `${t("networkMonitor.sourceProfile")} (${name})`;
    }
    if (source === "acp") return t("networkMonitor.sourceAcp");
    if (source === "unknown") return t("networkMonitor.sourceUnknown");
    return source;
}

// ─── Data fetching ──────────────────────────────────────────────

async function fetchAllMetrics() {
    try {
        error.value = "";
        const days = selectedRange.value;
        const [reqRes, trafRes, tokRes] = await Promise.all([
            axios.get("/api/metrics/requests", { params: { days } }),
            axios.get("/api/metrics/traffic", { params: { days } }),
            axios.get("/api/metrics/tokens", { params: { days } }),
        ]);
        requestMetrics.value = reqRes.data;
        trafficMetrics.value = trafRes.data;
        tokenMetrics.value = tokRes.data;
        lastUpdated.value = new Date();
    } catch (e: unknown) {
        error.value =
            e instanceof Error ? e.message : t("networkMonitor.loadFailed");
    } finally {
        loading.value = false;
    }
}

// ─── Chart rendering ────────────────────────────────────────────

const chartColors = [
    "#5F7E9B",
    "#708865",
    "#9A7557",
    "#786696",
    "#5D8985",
    "#9C6674",
    "#80844F",
    "#69788D",
    "#A0526E",
    "#5A8A7A",
];

function getChartColors(dark: boolean): string[] {
    return dark
        ? [
              "#6F8FAF",
              "#7E9A73",
              "#A78468",
              "#8A78A8",
              "#6B9995",
              "#B07A87",
              "#8C8F62",
              "#7C8798",
          ]
        : chartColors;
}

function createRequestChart() {
    if (!requestChartCanvas.value || !requestMetrics.value) return;
    if (requestChart) requestChart.destroy();

    const data = requestMetrics.value.series.map((p) => ({
        x: p.timestamp * 1000,
        y: p.value,
    }));
    const isDark = document.documentElement.classList.contains("dark");

    requestChart = new Chart(requestChartCanvas.value, {
        type: "line",
        data: {
            datasets: [
                {
                    label: t("networkMonitor.totalRequests"),
                    data,
                    borderColor: getChartColors(isDark)[0],
                    backgroundColor: getChartColors(isDark)[0] + "20",
                    fill: true,
                    tension: 0.3,
                    pointRadius: 0,
                    borderWidth: 2,
                },
            ],
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            interaction: { intersect: false, mode: "index" },
            plugins: {
                legend: { display: false },
                tooltip: {
                    callbacks: {
                        title(items: TooltipItem<"line">[]) {
                            const x = items[0]?.parsed?.x;
                            return x != null
                                ? new Date(x as number).toLocaleString("zh-CN")
                                : "";
                        },
                    },
                },
            },
            scales: {
                x: {
                    type: "time",
                    time: {
                        unit:
                            selectedRange.value === 1
                                ? "hour"
                                : selectedRange.value === 3
                                  ? "hour"
                                  : "day",
                    },
                    grid: { color: isDark ? "#ffffff15" : "#00000010" },
                },
                y: {
                    beginAtZero: true,
                    ticks: {
                        callback: (v) =>
                            typeof v === "number" ? formatNumber(v) : v,
                    },
                    grid: { color: isDark ? "#ffffff15" : "#00000010" },
                },
            },
        },
    });
}

function createTrafficChart() {
    if (!trafficChartCanvas.value || !trafficMetrics.value) return;
    if (trafficChart) trafficChart.destroy();

    const dataIn = trafficMetrics.value.series_in.map((p) => ({
        x: p.timestamp * 1000,
        y: p.value,
    }));
    const dataOut = trafficMetrics.value.series_out.map((p) => ({
        x: p.timestamp * 1000,
        y: p.value,
    }));
    const isDark = document.documentElement.classList.contains("dark");

    trafficChart = new Chart(trafficChartCanvas.value, {
        type: "line",
        data: {
            datasets: [
                {
                    label: t("networkMonitor.received"),
                    data: dataIn,
                    borderColor: getChartColors(isDark)[1],
                    backgroundColor: getChartColors(isDark)[1] + "20",
                    fill: true,
                    tension: 0.3,
                    pointRadius: 0,
                    borderWidth: 2,
                },
                {
                    label: t("networkMonitor.sent"),
                    data: dataOut,
                    borderColor: getChartColors(isDark)[2],
                    backgroundColor: getChartColors(isDark)[2] + "20",
                    fill: true,
                    tension: 0.3,
                    pointRadius: 0,
                    borderWidth: 2,
                },
            ],
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            interaction: { intersect: false, mode: "index" },
            plugins: {
                legend: { position: "top" },
                tooltip: {
                    callbacks: {
                        title: (items) =>
                            items[0]?.parsed?.x
                                ? new Date(items[0].parsed.x).toLocaleString(
                                      "zh-CN",
                                  )
                                : "",
                        label: (item) =>
                            `${item.dataset.label}: ${formatBytes(item.parsed.y ?? 0)}`,
                    },
                },
            },
            scales: {
                x: {
                    type: "time",
                    time: {
                        unit:
                            selectedRange.value === 1
                                ? "hour"
                                : selectedRange.value === 3
                                  ? "hour"
                                  : "day",
                    },
                    grid: { color: isDark ? "#ffffff15" : "#00000010" },
                },
                y: {
                    beginAtZero: true,
                    ticks: {
                        callback: (v) =>
                            typeof v === "number" ? formatBytes(v) : v,
                    },
                    grid: { color: isDark ? "#ffffff15" : "#00000010" },
                },
            },
        },
    });
}

function createTokenChart() {
    if (!tokenChartCanvas.value || !tokenMetrics.value) return;
    if (tokenChart) tokenChart.destroy();

    const providers = tokenMetrics.value.token_series;
    const isDark = document.documentElement.classList.contains("dark");
    const colors = getChartColors(isDark);

    // Collect all timestamps for x-axis
    const allTimestamps = new Set<number>();
    for (const p of providers) {
        for (const pt of p.points) allTimestamps.add(pt.timestamp * 1000);
    }
    const sortedTimestamps = [...allTimestamps].sort((a, b) => a - b);

    const datasets = providers.map((p, i) => {
        const pointMap = new Map(
            p.points.map((pt) => [pt.timestamp * 1000, pt.value]),
        );
        return {
            label: p.provider_name,
            data: sortedTimestamps.map((ts) => ({
                x: ts,
                y: pointMap.get(ts) || 0,
            })),
            backgroundColor: colors[i % colors.length],
            borderColor: colors[i % colors.length],
            borderWidth: 0,
            borderRadius: 2,
        };
    });

    tokenChart = new Chart(tokenChartCanvas.value, {
        type: "bar",
        data: { datasets },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            interaction: { intersect: false, mode: "index" },
            plugins: {
                legend: { position: "top" },
                tooltip: {
                    callbacks: {
                        title: (items) =>
                            items[0]?.parsed?.x
                                ? new Date(items[0].parsed.x).toLocaleString(
                                      "zh-CN",
                                  )
                                : "",
                        label: (item) =>
                            `${item.dataset.label}: ${formatTokens(item.parsed.y ?? 0)} tokens`,
                    },
                },
            },
            scales: {
                x: {
                    type: "time",
                    stacked: true,
                    time: {
                        unit:
                            selectedRange.value === 1
                                ? "hour"
                                : selectedRange.value === 3
                                  ? "hour"
                                  : "day",
                    },
                    grid: { color: isDark ? "#ffffff15" : "#00000010" },
                },
                y: {
                    stacked: true,
                    beginAtZero: true,
                    ticks: {
                        callback: (v) =>
                            typeof v === "number" ? formatTokens(v) : v,
                    },
                    grid: { color: isDark ? "#ffffff15" : "#00000010" },
                },
            },
        },
    });
}

function renderCharts() {
    nextTick(() => {
        createRequestChart();
        createTrafficChart();
        createTokenChart();
    });
}

// ─── WebSocket connection ───────────────────────────────────────

function connectWebSocket() {
    if (ws && ws.readyState === WebSocket.OPEN) return;

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${protocol}//${window.location.host}/api/metrics/ws`;
    ws = new WebSocket(url);

    ws.onmessage = async (event) => {
        try {
            const data = JSON.parse(event.data);
            if (data.type === "metrics_updated") {
                await fetchAllMetrics();
                renderCharts();
            }
        } catch {
            // Ignore malformed messages
        }
    };

    ws.onclose = () => {
        // Reconnect after 3 seconds
        wsReconnectTimer = window.setTimeout(() => {
            connectWebSocket();
        }, 3000);
    };

    ws.onerror = () => {
        ws?.close();
    };
}

// ─── Watch & lifecycle ──────────────────────────────────────────

watch(selectedRange, async () => {
    await fetchAllMetrics();
    renderCharts();
});

onMounted(async () => {
    await fetchAllMetrics();
    renderCharts();
    connectWebSocket();
});

onUnmounted(() => {
    if (ws) {
        ws.onclose = null;
        ws.close();
        ws = null;
    }
    if (wsReconnectTimer !== null) window.clearTimeout(wsReconnectTimer);
    if (requestChart) requestChart.destroy();
    if (trafficChart) trafficChart.destroy();
    if (tokenChart) tokenChart.destroy();
});
</script>

<template>
    <div class="monitor-container">
        <!-- Header -->
        <header class="monitor-header">
            <div>
                <h1 class="page-title">{{ t("networkMonitor.title") }}</h1>
                <p class="page-subtitle">
                    {{ t("networkMonitor.subtitle") }}
                </p>
            </div>
            <div class="header-meta">
                <span class="update-badge">
                    <svg
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <polyline points="23 4 23 10 17 10" />
                        <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
                    </svg>
                    {{ lastUpdatedLabel }}
                </span>
            </div>
        </header>

        <!-- Error -->
        <div v-if="error" class="error-banner">
            {{ error }}
        </div>

        <!-- Loading -->
        <div v-if="loading && !requestMetrics" class="loading-wrap">
            <div class="spinner" />
        </div>

        <template v-else>
            <!-- Overview Cards -->
            <div class="overview-grid">
                <div class="overview-card">
                    <div class="card-icon requests">
                        <svg
                            width="18"
                            height="18"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
                        </svg>
                    </div>
                    <div class="card-label">
                        {{ t("networkMonitor.totalRequests") }}
                    </div>
                    <div class="card-value">
                        {{ formatNumber(requestMetrics?.total ?? 0) }}
                    </div>
                    <div class="card-note">
                        {{ t("networkMonitor.past") }} {{ rangeLabel }}
                    </div>
                </div>
                <div class="overview-card">
                    <div class="card-icon traffic">
                        <svg
                            width="18"
                            height="18"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <polyline
                                points="22 12 18 12 15 21 9 3 6 12 2 12"
                            />
                        </svg>
                    </div>
                    <div class="card-label">
                        {{ t("networkMonitor.networkTraffic") }}
                    </div>
                    <div class="card-value">
                        {{
                            formatBytes(
                                (trafficMetrics?.total_in ?? 0) +
                                    (trafficMetrics?.total_out ?? 0),
                            )
                        }}
                    </div>
                    <div class="card-note">
                        {{ t("networkMonitor.received") }}
                        {{ formatBytes(trafficMetrics?.total_in ?? 0) }} /
                        {{ t("networkMonitor.sent") }}
                        {{ formatBytes(trafficMetrics?.total_out ?? 0) }}
                    </div>
                </div>
                <div class="overview-card">
                    <div class="card-icon tokens">
                        <svg
                            width="18"
                            height="18"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <circle cx="12" cy="12" r="10" />
                            <polyline points="12 6 12 12 16 14" />
                        </svg>
                    </div>
                    <div class="card-label">
                        {{ t("networkMonitor.tokenConsumption") }}
                    </div>
                    <div class="card-value">
                        {{ formatTokens(tokenMetrics?.total_tokens ?? 0) }}
                    </div>
                    <div class="card-note">
                        {{ t("networkMonitor.past") }} {{ rangeLabel }}
                    </div>
                </div>
                <div
                    class="overview-card"
                    v-if="tokenMetrics?.tokens_by_provider?.length"
                >
                    <div class="card-icon providers">
                        <svg
                            width="18"
                            height="18"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <rect
                                x="2"
                                y="2"
                                width="20"
                                height="8"
                                rx="2"
                                ry="2"
                            />
                            <rect
                                x="2"
                                y="14"
                                width="20"
                                height="8"
                                rx="2"
                                ry="2"
                            />
                            <line x1="6" y1="6" x2="6.01" y2="6" />
                            <line x1="6" y1="18" x2="6.01" y2="18" />
                        </svg>
                    </div>
                    <div class="card-label">
                        {{ t("networkMonitor.activeProviders") }}
                    </div>
                    <div class="card-value">
                        {{ tokenMetrics.tokens_by_provider.length }}
                    </div>
                    <div class="card-note">
                        {{
                            tokenMetrics.tokens_by_provider
                                .map((p) => p.provider_name)
                                .join(", ")
                        }}
                    </div>
                </div>
            </div>

            <!-- Range Selector -->
            <div class="section-toolbar">
                <div>
                    <h2 class="section-title">
                        {{ t("networkMonitor.requestTrend") }}
                    </h2>
                    <p class="section-subtitle">
                        {{ t("networkMonitor.requestTrendDesc") }}
                    </p>
                </div>
                <div class="range-switch">
                    <button
                        v-for="r in rangeOptions"
                        :key="r"
                        :class="{ active: selectedRange === r }"
                        class="range-chip"
                        @click="selectedRange = r as TimeRange"
                    >
                        {{
                            r === 1
                                ? "1 " + t("networkMonitor.day")
                                : r === 3
                                  ? "3 " + t("networkMonitor.day")
                                  : "7 " + t("networkMonitor.day")
                        }}
                    </button>
                </div>
            </div>

            <!-- Request Chart -->
            <div class="chart-card">
                <canvas ref="requestChartCanvas" height="280"></canvas>
            </div>

            <!-- Traffic Chart -->
            <div class="section-header">
                <h2 class="section-title">
                    {{ t("networkMonitor.trafficTrend") }}
                </h2>
                <p class="section-subtitle">
                    {{ t("networkMonitor.trafficTrendDesc") }}
                </p>
            </div>
            <div class="chart-card">
                <canvas ref="trafficChartCanvas" height="280"></canvas>
            </div>

            <!-- Token Chart -->
            <div class="section-header">
                <h2 class="section-title">
                    {{ t("networkMonitor.tokenTrend") }}
                </h2>
                <p class="section-subtitle">
                    {{ t("networkMonitor.tokenTrendDesc") }}
                </p>
            </div>
            <div class="chart-card">
                <canvas ref="tokenChartCanvas" height="320"></canvas>
            </div>

            <!-- Provider Ranking -->
            <div
                v-if="tokenMetrics?.tokens_by_provider?.length"
                class="section-header"
            >
                <h2 class="section-title">
                    {{ t("networkMonitor.providerRanking") }}
                </h2>
                <p class="section-subtitle">
                    {{
                        t("networkMonitor.providerRankingDesc", {
                            range: rangeLabel,
                        })
                    }}
                </p>
            </div>
            <div
                v-if="tokenMetrics?.tokens_by_provider?.length"
                class="ranking-card"
            >
                <div
                    v-for="(p, i) in [...tokenMetrics.tokens_by_provider].sort(
                        (a, b) => b.tokens - a.tokens,
                    )"
                    :key="p.provider_name"
                    class="ranking-row"
                >
                    <span class="ranking-index">{{ i + 1 }}</span>
                    <span class="ranking-name">{{ p.provider_name }}</span>
                    <div class="ranking-bar-wrap">
                        <div
                            class="ranking-bar"
                            :style="{
                                width:
                                    tokenMetrics.tokens_by_provider.length >
                                        0 && p.tokens > 0
                                        ? (p.tokens /
                                              Math.max(
                                                  ...tokenMetrics.tokens_by_provider.map(
                                                      (x) => x.tokens,
                                                  ),
                                              )) *
                                              100 +
                                          '%'
                                        : '0%',
                                backgroundColor:
                                    chartColors[i % chartColors.length],
                            }"
                        />
                    </div>
                    <span class="ranking-value">{{
                        formatTokens(p.tokens)
                    }}</span>
                </div>
            </div>
            <!-- Source Ranking -->
            <div
                v-if="tokenMetrics?.tokens_by_source?.length"
                class="section-header"
            >
                <h2 class="section-title">
                    {{ t("networkMonitor.sourceRanking") }}
                </h2>
                <p class="section-subtitle">
                    {{
                        t("networkMonitor.sourceRankingDesc", {
                            range: rangeLabel,
                        })
                    }}
                </p>
            </div>
            <div
                v-if="tokenMetrics?.tokens_by_source?.length"
                class="ranking-card"
            >
                <div
                    v-for="(s, i) in [...tokenMetrics.tokens_by_source].sort(
                        (a, b) => b.tokens - a.tokens,
                    )"
                    :key="s.source"
                    class="ranking-row"
                >
                    <span class="ranking-index">{{ i + 1 }}</span>
                    <span class="ranking-name">{{
                        sourceLabel(s.source)
                    }}</span>
                    <div class="ranking-bar-wrap">
                        <div
                            class="ranking-bar"
                            :style="{
                                width:
                                    tokenMetrics.tokens_by_source.length > 0 &&
                                    s.tokens > 0
                                        ? (s.tokens /
                                              Math.max(
                                                  ...tokenMetrics.tokens_by_source.map(
                                                      (x) => x.tokens,
                                                  ),
                                              )) *
                                              100 +
                                          '%'
                                        : '0%',
                                backgroundColor:
                                    chartColors[i % chartColors.length],
                            }"
                        />
                    </div>
                    <span class="ranking-value">{{
                        formatTokens(s.tokens)
                    }}</span>
                </div>
            </div>
        </template>
    </div>
</template>

<style scoped>
.monitor-container {
    max-width: 1280px;
    margin: 0 auto;
    padding: 24px;
    font-family:
        "SF Pro Display",
        "SF Pro Text",
        -apple-system,
        BlinkMacSystemFont,
        "Segoe UI",
        sans-serif;
}

.monitor-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 24px;
    margin-bottom: 24px;
}

.page-title {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--c-text, #1a1a2e);
}

.page-subtitle {
    margin: 4px 0 0;
    font-size: 0.875rem;
    color: var(--c-muted, #666);
}

.header-meta {
    display: flex;
    gap: 12px;
}

.update-badge {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border: 1px solid var(--c-border, #e0e0e0);
    border-radius: 999px;
    font-size: 13px;
    color: var(--c-muted, #666);
}

.error-banner {
    padding: 12px 16px;
    margin-bottom: 16px;
    border-radius: 8px;
    background: #fef2f2;
    color: #dc2626;
    border: 1px solid #fecaca;
    font-size: 14px;
}

.loading-wrap {
    display: flex;
    justify-content: center;
    padding: 80px 0;
}

.spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--c-border, #e0e0e0);
    border-top-color: #5f7e9b;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

/* Overview Cards */
.overview-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 16px;
    margin-bottom: 28px;
}

.overview-card {
    padding: 18px 20px;
    border: 1px solid var(--c-border, #e0e0e0);
    border-radius: 14px;
    background: var(--c-surface, #fff);
}

.card-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    margin-bottom: 8px;
}

.card-icon.requests {
    background: #eef2ff;
    color: #5f7e9b;
}
.card-icon.traffic {
    background: #ecfdf5;
    color: #708865;
}
.card-icon.tokens {
    background: #fef9e7;
    color: #9a7557;
}
.card-icon.providers {
    background: #f3e8ff;
    color: #786696;
}

.card-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--c-muted, #666);
}

.card-value {
    margin-top: 4px;
    font-size: 26px;
    font-weight: 700;
    color: var(--c-text, #1a1a2e);
    line-height: 1.2;
}

.card-note {
    margin-top: 4px;
    font-size: 12px;
    color: var(--c-muted, #999);
}

/* Section Toolbar */
.section-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 16px;
    margin-bottom: 12px;
}

.section-header {
    margin-top: 28px;
    margin-bottom: 12px;
}

.section-title {
    font-size: 18px;
    font-weight: 650;
    margin: 0;
    color: var(--c-text, #1a1a2e);
}

.section-subtitle {
    font-size: 13px;
    color: var(--c-muted, #666);
    margin: 4px 0 0;
}

.range-switch {
    display: inline-flex;
    gap: 6px;
    padding: 4px;
    border: 1px solid var(--c-border, #e0e0e0);
    border-radius: 999px;
    background: var(--c-surface, #fff);
}

.range-chip {
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--c-muted, #666);
    padding: 7px 14px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.18s ease;
}

.range-chip.active {
    background: #5f7e9b15;
    color: #5f7e9b;
}

.range-chip:hover:not(.active) {
    background: var(--c-hover, #f5f5f5);
}

/* Charts */
.chart-card {
    padding: 20px;
    border: 1px solid var(--c-border, #e0e0e0);
    border-radius: 14px;
    background: var(--c-surface, #fff);
    margin-bottom: 8px;
}

.chart-card canvas {
    max-height: 320px;
}

/* Ranking */
.ranking-card {
    padding: 16px 20px;
    border: 1px solid var(--c-border, #e0e0e0);
    border-radius: 14px;
    background: var(--c-surface, #fff);
}

.ranking-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
    border-bottom: 1px solid var(--c-border, #f0f0f0);
}

.ranking-row:last-child {
    border-bottom: 0;
}

.ranking-index {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    background: var(--c-hover, #f5f5f5);
    font-size: 12px;
    font-weight: 700;
    color: var(--c-muted, #999);
    flex-shrink: 0;
}

.ranking-name {
    width: 100px;
    font-size: 14px;
    font-weight: 500;
    color: var(--c-text, #1a1a2e);
    flex-shrink: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.ranking-bar-wrap {
    flex: 1;
    height: 8px;
    background: var(--c-hover, #f0f0f0);
    border-radius: 4px;
    overflow: hidden;
}

.ranking-bar {
    height: 100%;
    border-radius: 4px;
    transition: width 0.3s ease;
}

.ranking-value {
    font-size: 13px;
    font-weight: 600;
    color: var(--c-muted, #666);
    flex-shrink: 0;
    min-width: 50px;
    text-align: right;
}

/* Dark mode */
html.dark .monitor-container {
    color-scheme: dark;
}

@media (max-width: 768px) {
    .overview-grid {
        grid-template-columns: repeat(2, 1fr);
    }

    .monitor-header {
        flex-direction: column;
        align-items: flex-start;
    }

    .section-toolbar {
        flex-direction: column;
        align-items: flex-start;
    }
}

@media (max-width: 480px) {
    .overview-grid {
        grid-template-columns: 1fr;
    }
}
</style>

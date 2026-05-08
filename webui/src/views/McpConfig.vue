<script setup lang="ts">
import { onMounted, ref, reactive, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useMcpStore } from "../stores/mcp";
import type {
    McpServerConfig,
    TransportType,
    TransportConfig,
    CreateMcpServerRequest,
    UpdateMcpServerRequest,
} from "../types";

const { t } = useI18n();
const mcpStore = useMcpStore();

const showForm = ref(false);
const editingServer = ref<McpServerConfig | null>(null);

const formData = reactive({
    name: "",
    transport_type: "stdio" as TransportType,
    // Stdio fields
    command: "",
    args: "",
    envEntries: [] as { key: string; value: string }[],
    // URL-based fields (SSE, WebSocket, HTTP)
    url: "",
    headerEntries: [] as { key: string; value: string }[],
    enabled: true,
});

const isStdio = computed(() => formData.transport_type === "stdio");
const isUrlBased = computed(() =>
    ["sse", "websocket", "http"].includes(formData.transport_type),
);

onMounted(() => {
    mcpStore.fetchServers();
});

function resetForm() {
    formData.name = "";
    formData.transport_type = "stdio";
    formData.command = "";
    formData.args = "";
    formData.envEntries = [];
    formData.url = "";
    formData.headerEntries = [];
    formData.enabled = true;
}

function openCreate() {
    editingServer.value = null;
    resetForm();
    showForm.value = true;
}

function openEdit(server: McpServerConfig) {
    editingServer.value = server;
    formData.name = server.name;
    formData.transport_type = server.transport_type;
    formData.enabled = server.enabled ?? true;

    // Parse transport config
    if (server.transport_type === "stdio") {
        const cfg = server.transport_config as Extract<
            TransportConfig,
            { type: "stdio" }
        >;
        formData.command = cfg.command;
        formData.args = cfg.args?.join(" ") ?? "";
        formData.envEntries = cfg.env
            ? Object.entries(cfg.env).map(([key, value]) => ({ key, value }))
            : [];
        formData.url = "";
        formData.headerEntries = [];
    } else {
        const cfg = server.transport_config as Extract<
            TransportConfig,
            { type: "sse" } | { type: "websocket" } | { type: "http" }
        >;
        formData.url = cfg.url;
        formData.headerEntries = cfg.headers
            ? Object.entries(cfg.headers).map(([key, value]) => ({
                  key,
                  value,
              }))
            : [];
        formData.command = "";
        formData.args = "";
        formData.envEntries = [];
    }

    showForm.value = true;
}

function buildTransportConfig(): TransportConfig {
    if (formData.transport_type === "stdio") {
        const args = formData.args.trim()
            ? formData.args.trim().split(/\s+/).filter(Boolean)
            : undefined;
        const env =
            formData.envEntries.length > 0
                ? Object.fromEntries(
                      formData.envEntries
                          .filter((e) => e.key.trim())
                          .map((e) => [e.key.trim(), e.value]),
                  )
                : undefined;
        return {
            type: "stdio",
            command: formData.command,
            ...(args ? { args } : {}),
            ...(env ? { env } : {}),
        };
    } else {
        const headers =
            formData.headerEntries.length > 0
                ? Object.fromEntries(
                      formData.headerEntries
                          .filter((e) => e.key.trim())
                          .map((e) => [e.key.trim(), e.value]),
                  )
                : undefined;
        const base = {
            url: formData.url,
            ...(headers ? { headers } : {}),
        };
        switch (formData.transport_type) {
            case "sse":
                return { type: "sse", ...base };
            case "websocket":
                return { type: "websocket", ...base };
            case "http":
                return { type: "http", ...base };
            default:
                return { type: "sse", ...base };
        }
    }
}

async function handleSave() {
    try {
        const transportConfig = buildTransportConfig();
        if (editingServer.value) {
            await mcpStore.updateServer(editingServer.value.id, {
                name: formData.name,
                transport_type: formData.transport_type,
                transport_config: transportConfig,
                enabled: formData.enabled,
            } as UpdateMcpServerRequest);
        } else {
            await mcpStore.createServer({
                name: formData.name,
                transport_type: formData.transport_type,
                transport_config: transportConfig,
                enabled: formData.enabled,
            } as CreateMcpServerRequest);
        }
        showForm.value = false;
        editingServer.value = null;
    } catch {
        // error is in store
    }
}

function handleCancel() {
    showForm.value = false;
    editingServer.value = null;
}

async function handleDelete(id: string) {
    if (!confirm(t("common.deleteConfirm"))) return;
    try {
        await mcpStore.deleteServer(id);
        await mcpStore.fetchServers();
    } catch {
        // error is in store
    }
}

async function handleToggle(server: McpServerConfig) {
    try {
        await mcpStore.toggleServer(server.id);
        await mcpStore.fetchServers();
    } catch {
        // error is in store
    }
}

function addEnvEntry() {
    formData.envEntries.push({ key: "", value: "" });
}

function removeEnvEntry(index: number) {
    formData.envEntries.splice(index, 1);
}

function addHeaderEntry() {
    formData.headerEntries.push({ key: "", value: "" });
}

function removeHeaderEntry(index: number) {
    formData.headerEntries.splice(index, 1);
}

function getTransportLabel(type: TransportType): string {
    switch (type) {
        case "stdio":
            return "STDIO";
        case "sse":
            return "SSE";
        case "websocket":
            return "WebSocket";
        case "http":
            return "HTTP";
        default:
            return type;
    }
}

function getTransportIcon(type: TransportType): string {
    switch (type) {
        case "stdio":
            return "⌨️";
        case "sse":
            return "📡";
        case "websocket":
            return "🔌";
        case "http":
            return "🌐";
        default:
            return "🔗";
    }
}

function getServerSummary(server: McpServerConfig): string {
    if (server.transport_type === "stdio") {
        const cfg = server.transport_config as Extract<
            TransportConfig,
            { type: "stdio" }
        >;
        let summary = cfg.command;
        if (cfg.args && cfg.args.length > 0) {
            summary += " " + cfg.args.join(" ");
        }
        return summary;
    } else {
        const cfg = server.transport_config as Extract<
            TransportConfig,
            { type: "sse" } | { type: "websocket" } | { type: "http" }
        >;
        return cfg.url;
    }
}

function truncateText(text: string, maxLen: number = 80): string {
    if (!text) return "";
    return text.length > maxLen ? text.slice(0, maxLen) + "..." : text;
}

function formatDate(dateString: string): string {
    return new Date(dateString).toLocaleDateString();
}

function onTransportTypeChange() {
    // Reset relevant fields when transport type changes
    if (formData.transport_type === "stdio") {
        formData.url = "";
        formData.headerEntries = [];
    } else {
        formData.command = "";
        formData.args = "";
        formData.envEntries = [];
    }
}
</script>

<template>
    <div class="page">
        <!-- Header -->
        <div class="page-header">
            <div class="header-content">
                <div class="header-icon">
                    <svg
                        width="24"
                        height="24"
                        viewBox="0 0 24 24"
                        fill="none"
                        xmlns="http://www.w3.org/2000/svg"
                    >
                        <path
                            d="M12 2L2 7l10 5 10-5-10-5z"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <path
                            d="M2 17l10 5 10-5"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <path
                            d="M2 12l10 5 10-5"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">{{ t("mcpConfig.title") }}</h1>
                    <p class="header-desc">{{ t("mcpConfig.subtitle") }}</p>
                </div>
            </div>
            <button class="btn btn-accent" @click="openCreate">
                <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    xmlns="http://www.w3.org/2000/svg"
                >
                    <path
                        d="M12 5v14M5 12h14"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                    />
                </svg>
                {{ t("mcpConfig.addServer") }}
            </button>
        </div>

        <!-- Info Banner -->
        <div class="info-banner">
            <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
            >
                <circle cx="12" cy="12" r="10" />
                <line x1="12" y1="16" x2="12" y2="12" />
                <line x1="12" y1="8" x2="12.01" y2="8" />
            </svg>
            <span>{{ t("mcpConfig.infoBanner") }}</span>
        </div>

        <!-- Error -->
        <div v-if="mcpStore.error" class="error-banner">
            <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
            >
                <circle cx="12" cy="12" r="10" />
                <line x1="15" y1="9" x2="9" y2="15" />
                <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
            {{ mcpStore.error }}
        </div>

        <!-- Loading -->
        <div
            v-if="mcpStore.loading && mcpStore.servers.length === 0"
            class="loading-state"
        >
            <div class="loading-spinner"></div>
            <span class="loading-text">{{ t("common.loading") }}</span>
        </div>

        <!-- Empty State -->
        <div v-else-if="mcpStore.servers.length === 0" class="empty-state">
            <div class="empty-illustration">
                <div class="empty-icon-wrapper">
                    <svg
                        width="48"
                        height="48"
                        viewBox="0 0 24 24"
                        fill="none"
                        xmlns="http://www.w3.org/2000/svg"
                    >
                        <path
                            d="M12 2L2 7l10 5 10-5-10-5z"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <path
                            d="M2 17l10 5 10-5"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <path
                            d="M2 12l10 5 10-5"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                    </svg>
                </div>
                <div class="empty-decoration">
                    <span class="deco-dot deco-dot-1"></span>
                    <span class="deco-dot deco-dot-2"></span>
                    <span class="deco-dot deco-dot-3"></span>
                </div>
            </div>
            <h3 class="empty-title">{{ t("mcpConfig.noServers") }}</h3>
            <button class="btn btn-accent" @click="openCreate">
                <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                >
                    <path d="M12 5v14M5 12h14" />
                </svg>
                {{ t("mcpConfig.addServer") }}
            </button>
        </div>

        <!-- Server Cards -->
        <div v-else class="card-list">
            <div
                v-for="(server, index) in mcpStore.servers"
                :key="server.id"
                class="mcp-card"
                :class="{ 'mcp-card--enabled': server.enabled }"
                :style="{ animationDelay: `${index * 50}ms` }"
            >
                <div
                    class="card-glow"
                    :class="{ 'card-glow--active': server.enabled }"
                ></div>
                <div class="card-content">
                    <div class="card-info">
                        <div class="card-icon">
                            <span class="icon-emoji">{{
                                getTransportIcon(server.transport_type)
                            }}</span>
                        </div>
                        <div class="card-details">
                            <div class="card-title-row">
                                <h3 class="card-title">{{ server.name }}</h3>
                                <span class="transport-badge">{{
                                    getTransportLabel(server.transport_type)
                                }}</span>
                                <span
                                    v-if="server.enabled"
                                    class="status-badge status-badge--active"
                                >
                                    <span class="status-dot"></span>
                                    {{ t("common.enabled") }}
                                </span>
                                <span
                                    v-else
                                    class="status-badge status-badge--inactive"
                                >
                                    <span class="status-dot"></span>
                                    {{ t("common.disabled") }}
                                </span>
                            </div>
                            <div class="card-summary">
                                <span class="summary-label">🔗</span>
                                <span class="summary-text">
                                    {{ truncateText(getServerSummary(server)) }}
                                </span>
                            </div>
                            <div class="card-meta">
                                {{ t("mcpConfig.createdAt") }}:
                                {{ formatDate(server.created_at) }}
                            </div>
                        </div>
                    </div>

                    <div class="card-actions">
                        <button
                            class="btn btn-ghost btn-sm"
                            @click="handleToggle(server)"
                            :title="
                                server.enabled
                                    ? t('common.disabled')
                                    : t('common.enabled')
                            "
                        >
                            <svg
                                v-if="server.enabled"
                                width="14"
                                height="14"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <circle cx="12" cy="12" r="10" />
                                <line x1="4.93" y1="12" x2="19.07" y2="12" />
                            </svg>
                            <svg
                                v-else
                                width="14"
                                height="14"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path d="M22 11.08V12a10 10 0 1 1 -5.93-9.14" />
                                <polyline points="22 4 12 14.01 9 11.01" />
                            </svg>
                            {{
                                server.enabled
                                    ? t("common.disabled")
                                    : t("common.enabled")
                            }}
                        </button>
                        <button
                            class="btn btn-ghost btn-sm"
                            @click="openEdit(server)"
                            :title="t('common.edit')"
                        >
                            <svg
                                width="14"
                                height="14"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"
                                />
                                <path
                                    d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"
                                />
                            </svg>
                            {{ t("common.edit") }}
                        </button>
                        <button
                            class="btn btn-ghost btn-sm btn-danger-ghost"
                            @click="handleDelete(server.id)"
                            :title="t('common.delete')"
                        >
                            <svg
                                width="14"
                                height="14"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <polyline points="3 6 5 6 21 6" />
                                <path
                                    d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                                />
                            </svg>
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Form Modal -->
        <Teleport to="body">
            <div v-if="showForm" class="persona-modal-overlay">
                <div class="persona-modal-content glass" @click.stop>
                    <div class="modal-header">
                        <h2 class="modal-title">
                            {{
                                editingServer
                                    ? t("mcpConfig.editServer")
                                    : t("mcpConfig.createServer")
                            }}
                        </h2>
                        <button class="modal-close" @click="handleCancel">
                            <svg
                                width="18"
                                height="18"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                            >
                                <path d="M18 6L6 18M6 6l12 12" />
                            </svg>
                        </button>
                    </div>
                    <div class="modal-body">
                        <!-- Server Name -->
                        <div class="form-group">
                            <label class="form-label">{{
                                t("mcpConfig.serverName")
                            }}</label>
                            <input
                                v-model="formData.name"
                                type="text"
                                class="form-input"
                                :placeholder="
                                    t('mcpConfig.serverNamePlaceholder')
                                "
                            />
                        </div>

                        <!-- Transport Type -->
                        <div class="form-group">
                            <label class="form-label">{{
                                t("mcpConfig.transportType")
                            }}</label>
                            <select
                                v-model="formData.transport_type"
                                class="form-input"
                                @change="onTransportTypeChange"
                            >
                                <option value="stdio">STDIO</option>
                                <option value="sse">SSE</option>
                                <option value="websocket">WebSocket</option>
                                <option value="http">HTTP</option>
                            </select>
                        </div>

                        <!-- STDIO Config -->
                        <template v-if="isStdio">
                            <div class="form-section-title">
                                {{ t("mcpConfig.stdioConfig") }}
                            </div>
                            <div class="form-group">
                                <label class="form-label">{{
                                    t("mcpConfig.command")
                                }}</label>
                                <input
                                    v-model="formData.command"
                                    type="text"
                                    class="form-input"
                                    :placeholder="
                                        t('mcpConfig.commandPlaceholder')
                                    "
                                />
                                <span class="form-hint">{{
                                    t("mcpConfig.commandHint")
                                }}</span>
                            </div>
                            <div class="form-group">
                                <label class="form-label">{{
                                    t("mcpConfig.args")
                                }}</label>
                                <input
                                    v-model="formData.args"
                                    type="text"
                                    class="form-input"
                                    :placeholder="t('mcpConfig.argPlaceholder')"
                                />
                                <span class="form-hint">{{
                                    t("mcpConfig.argsHint")
                                }}</span>
                            </div>
                            <div class="form-group">
                                <label class="form-label">{{
                                    t("mcpConfig.env")
                                }}</label>
                                <div class="entry-list">
                                    <div
                                        v-for="(
                                            _, index
                                        ) in formData.envEntries"
                                        :key="index"
                                        class="entry-row"
                                    >
                                        <input
                                            v-model="
                                                formData.envEntries[index].key
                                            "
                                            type="text"
                                            class="form-input entry-input"
                                            :placeholder="
                                                t('mcpConfig.envKeyPlaceholder')
                                            "
                                        />
                                        <input
                                            v-model="
                                                formData.envEntries[index].value
                                            "
                                            type="text"
                                            class="form-input entry-input"
                                            :placeholder="
                                                t(
                                                    'mcpConfig.envValuePlaceholder',
                                                )
                                            "
                                        />
                                        <button
                                            class="btn btn-ghost btn-sm entry-remove"
                                            @click="removeEnvEntry(index)"
                                            type="button"
                                        >
                                            <svg
                                                width="12"
                                                height="12"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="2"
                                                stroke-linecap="round"
                                            >
                                                <path
                                                    d="M18 6L6 18M6 6l12 12"
                                                />
                                            </svg>
                                        </button>
                                    </div>
                                    <button
                                        class="btn btn-ghost btn-sm"
                                        @click="addEnvEntry"
                                        type="button"
                                    >
                                        <svg
                                            width="12"
                                            height="12"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                            stroke-linecap="round"
                                        >
                                            <path d="M12 5v14M5 12h14" />
                                        </svg>
                                        {{ t("common.add") }}
                                    </button>
                                </div>
                            </div>
                        </template>

                        <!-- URL-based Config (SSE, WebSocket, HTTP) -->
                        <template v-if="isUrlBased">
                            <div class="form-section-title">
                                {{
                                    formData.transport_type === "sse"
                                        ? t("mcpConfig.sseConfig")
                                        : formData.transport_type ===
                                            "websocket"
                                          ? t("mcpConfig.websocketConfig")
                                          : t("mcpConfig.httpConfig")
                                }}
                            </div>
                            <div class="form-group">
                                <label class="form-label">{{
                                    t("mcpConfig.url")
                                }}</label>
                                <input
                                    v-model="formData.url"
                                    type="text"
                                    class="form-input"
                                    :placeholder="t('mcpConfig.urlPlaceholder')"
                                />
                            </div>
                            <div class="form-group">
                                <label class="form-label">{{
                                    t("mcpConfig.headers")
                                }}</label>
                                <div class="entry-list">
                                    <div
                                        v-for="(
                                            _, index
                                        ) in formData.headerEntries"
                                        :key="index"
                                        class="entry-row"
                                    >
                                        <input
                                            v-model="
                                                formData.headerEntries[index]
                                                    .key
                                            "
                                            type="text"
                                            class="form-input entry-input"
                                            :placeholder="
                                                t(
                                                    'mcpConfig.headerKeyPlaceholder',
                                                )
                                            "
                                        />
                                        <input
                                            v-model="
                                                formData.headerEntries[index]
                                                    .value
                                            "
                                            type="text"
                                            class="form-input entry-input"
                                            :placeholder="
                                                t(
                                                    'mcpConfig.headerValuePlaceholder',
                                                )
                                            "
                                        />
                                        <button
                                            class="btn btn-ghost btn-sm entry-remove"
                                            @click="removeHeaderEntry(index)"
                                            type="button"
                                        >
                                            <svg
                                                width="12"
                                                height="12"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="2"
                                                stroke-linecap="round"
                                            >
                                                <path
                                                    d="M18 6L6 18M6 6l12 12"
                                                />
                                            </svg>
                                        </button>
                                    </div>
                                    <button
                                        class="btn btn-ghost btn-sm"
                                        @click="addHeaderEntry"
                                        type="button"
                                    >
                                        <svg
                                            width="12"
                                            height="12"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                            stroke-linecap="round"
                                        >
                                            <path d="M12 5v14M5 12h14" />
                                        </svg>
                                        {{ t("common.add") }}
                                    </button>
                                </div>
                            </div>
                        </template>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-ghost" @click="handleCancel">
                            {{ t("common.cancel") }}
                        </button>
                        <button
                            class="btn btn-accent"
                            @click="handleSave"
                            :disabled="
                                !formData.name.trim() ||
                                (isStdio && !formData.command.trim()) ||
                                (isUrlBased && !formData.url.trim())
                            "
                        >
                            {{ t("common.save") }}
                        </button>
                    </div>
                </div>
            </div>
        </Teleport>
    </div>
</template>

<style scoped>
.page {
    padding: 1.5rem;
    max-width: 960px;
    margin: 0 auto;
    animation: fadeIn 0.4s ease-out;
}

@keyframes fadeIn {
    from {
        opacity: 0;
    }
    to {
        opacity: 1;
    }
}

/* Page Header */
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
    border: 1px solid hsl(var(--primary) / 0.2);
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

/* Info Banner */
.info-banner {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: hsl(var(--primary) / 0.06);
    border: 1px solid hsl(var(--primary) / 0.15);
    border-radius: 0.5rem;
    color: hsl(var(--muted-foreground));
    font-size: 0.8rem;
    margin-bottom: 1rem;
    line-height: 1.5;
}

.info-banner svg {
    flex-shrink: 0;
    margin-top: 1px;
    color: hsl(var(--primary));
}

/* Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: 0.5rem;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    border: 1px solid transparent;
}

.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-accent {
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(var(--primary) / 0.9) 100%
    );
    color: hsl(var(--primary-foreground));
    border-color: hsl(var(--primary) / 0.3);
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.2);
}

.btn-accent:hover:not(:disabled) {
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.3);
    transform: translateY(-1px);
}

.btn-ghost {
    background: transparent;
    color: hsl(var(--muted-foreground));
    border-color: transparent;
}

.btn-ghost:hover {
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--foreground));
}

.btn-danger-ghost:hover {
    background: hsl(var(--destructive) / 0.1);
    color: hsl(var(--destructive));
}

.btn-sm {
    padding: 0.35rem 0.65rem;
    font-size: 0.8rem;
}

/* Transport Badge */
.transport-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.15rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--muted-foreground));
    border: 1px solid hsl(var(--border) / 0.3);
}

/* Status Badge */
.status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.6rem;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 600;
}

.status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
}

.status-badge--active {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
    border: 1px solid hsl(var(--primary) / 0.2);
}

.status-badge--active .status-dot {
    background: hsl(var(--primary));
    box-shadow: 0 0 6px hsl(var(--primary) / 0.5);
}

.status-badge--inactive {
    background: hsl(var(--muted) / 0.3);
    color: hsl(var(--muted-foreground));
    border: 1px solid hsl(var(--border) / 0.3);
}

.status-badge--inactive .status-dot {
    background: hsl(var(--muted-foreground));
}

/* Error Banner */
.error-banner {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: hsl(var(--destructive) / 0.1);
    border: 1px solid hsl(var(--destructive) / 0.2);
    border-radius: 0.5rem;
    color: hsl(var(--destructive));
    font-size: 0.875rem;
    margin-bottom: 1rem;
}

/* Loading State */
.loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 3rem;
    color: hsl(var(--muted-foreground));
}

.loading-spinner {
    width: 2rem;
    height: 2rem;
    border: 3px solid hsl(var(--primary) / 0.2);
    border-top-color: hsl(var(--primary));
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

.loading-text {
    font-size: 0.875rem;
}

/* Empty State */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 3rem 1rem;
    text-align: center;
}

.empty-illustration {
    position: relative;
    margin-bottom: 0.5rem;
}

.empty-icon-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 5rem;
    height: 5rem;
    border-radius: 1rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.15) 0%,
        hsl(var(--primary) / 0.05) 100%
    );
    color: hsl(var(--primary));
    border: 1px solid hsl(var(--primary) / 0.15);
}

.empty-decoration {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
}

.deco-dot {
    position: absolute;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: hsl(var(--primary) / 0.4);
    animation: float 3s ease-in-out infinite;
}

.deco-dot-1 {
    top: 10%;
    right: 15%;
    animation-delay: 0s;
}

.deco-dot-2 {
    bottom: 15%;
    left: 10%;
    animation-delay: 1s;
}

.deco-dot-3 {
    top: 50%;
    right: 5%;
    animation-delay: 2s;
}

@keyframes float {
    0%,
    100% {
        transform: translateY(0);
        opacity: 0.6;
    }
    50% {
        transform: translateY(-8px);
        opacity: 1;
    }
}

.empty-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

/* Card List */
.card-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}

.mcp-card {
    position: relative;
    border-radius: 0.75rem;
    overflow: hidden;
    animation: slideUp 0.4s ease-out both;
}

@keyframes slideUp {
    from {
        opacity: 0;
        transform: translateY(12px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

.card-glow {
    position: absolute;
    inset: 0;
    border-radius: 0.75rem;
    padding: 1px;
    background: linear-gradient(
        135deg,
        hsl(var(--border) / 0.3) 0%,
        transparent 50%,
        hsl(var(--border) / 0.2) 100%
    );
    -webkit-mask:
        linear-gradient(#fff 0 0) content-box,
        linear-gradient(#fff 0 0);
    mask:
        linear-gradient(#fff 0 0) content-box,
        linear-gradient(#fff 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
}

.card-glow--active {
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.4) 0%,
        transparent 50%,
        hsl(var(--primary) / 0.3) 100%
    );
}

.mcp-card:hover {
    transform: translateY(-2px);
    transition: transform 0.2s ease;
}

.mcp-card--enabled {
    box-shadow: 0 4px 16px hsl(var(--primary) / 0.1);
}

.card-content {
    position: relative;
    background: linear-gradient(
        180deg,
        hsl(var(--card) / 0.95) 0%,
        hsl(var(--card) / 0.85) 100%
    );
    backdrop-filter: blur(12px);
    padding: 1rem;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
}

.card-info {
    display: flex;
    gap: 0.75rem;
    flex: 1;
    min-width: 0;
}

.card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 0.5rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.15) 0%,
        hsl(var(--primary) / 0.08) 100%
    );
    flex-shrink: 0;
}

.icon-emoji {
    font-size: 1.25rem;
}

.card-details {
    flex: 1;
    min-width: 0;
}

.card-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
}

.card-title {
    font-size: 1rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

.card-summary {
    display: flex;
    align-items: flex-start;
    gap: 0.35rem;
    margin-top: 0.5rem;
    padding: 0.4rem 0.6rem;
    background: hsl(var(--muted) / 0.2);
    border-radius: 0.35rem;
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
}

.summary-label {
    flex-shrink: 0;
}

.summary-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.card-meta {
    margin-top: 0.3rem;
    font-size: 0.7rem;
    color: hsl(var(--muted-foreground) / 0.7);
}

/* Card Actions */
.card-actions {
    display: flex;
    gap: 0.35rem;
    flex-shrink: 0;
}

/* Modal */
.persona-modal-overlay {
    position: fixed;
    inset: 0;
    background: hsl(var(--background) / 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    padding: 1rem;
    animation: fadeIn 0.2s ease-out;
}

.persona-modal-overlay .persona-modal-content {
    width: 100%;
    max-width: 520px;
    max-height: 90vh;
    border-radius: 1rem;
    background: hsl(var(--background));
    border: 1px solid hsl(var(--border));
    box-shadow: 0 25px 50px -12px hsl(var(--foreground) / 0.25);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: modalSlideIn 0.25s ease-out;
}

@keyframes modalSlideIn {
    from {
        opacity: 0;
        transform: scale(0.95) translateY(10px);
    }
    to {
        opacity: 1;
        transform: scale(1) translateY(0);
    }
}

.modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid hsl(var(--border) / 0.2);
}

.modal-title {
    font-size: 1.1rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

.modal-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.75rem;
    height: 1.75rem;
    border-radius: 0.375rem;
    background: transparent;
    border: none;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s ease;
}

.modal-close:hover {
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--foreground));
}

.modal-body {
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    overflow-y: auto;
}

.form-group {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
}

.form-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
}

.form-hint {
    font-size: 0.7rem;
    color: hsl(var(--muted-foreground) / 0.7);
}

.form-input,
.form-textarea {
    padding: 0.6rem 0.75rem;
    border-radius: 0.5rem;
    border: 1px solid hsl(var(--border) / 0.4);
    background: hsl(var(--background) / 0.5);
    color: hsl(var(--foreground));
    font-size: 0.875rem;
    outline: none;
    transition: all 0.2s ease;
}

.form-input:focus,
.form-textarea:focus {
    border-color: hsl(var(--primary) / 0.5);
    box-shadow: 0 0 0 2px hsl(var(--primary) / 0.1);
}

select.form-input {
    appearance: none;
    background-image: url("data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 20 20'%3e%3cpath stroke='%236b7280' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.5' d='M6 8l4 4 4-4'/%3e%3c/svg%3e");
    background-position: right 0.5rem center;
    background-repeat: no-repeat;
    background-size: 1.5em 1.5em;
    padding-right: 2.5rem;
}

.form-section-title {
    font-size: 0.85rem;
    font-weight: 700;
    color: hsl(var(--foreground));
    padding-bottom: 0.35rem;
    border-bottom: 1px solid hsl(var(--border) / 0.2);
}

/* Entry List (env vars, headers) */
.entry-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.entry-row {
    display: flex;
    gap: 0.35rem;
    align-items: center;
}

.entry-input {
    flex: 1;
    min-width: 0;
}

.entry-remove {
    flex-shrink: 0;
    padding: 0.35rem;
    color: hsl(var(--muted-foreground));
}

.entry-remove:hover {
    color: hsl(var(--destructive));
}

.modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 1rem 1.25rem;
    border-top: 1px solid hsl(var(--border) / 0.2);
}

/* Responsive */
@media (max-width: 640px) {
    .page {
        padding: 1rem;
    }

    .page-header {
        flex-direction: column;
        align-items: flex-start;
    }

    .card-content {
        flex-direction: column;
    }

    .card-actions {
        width: 100%;
        justify-content: flex-end;
    }
}
</style>

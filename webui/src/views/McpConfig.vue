<script setup lang="ts">
import { onMounted, ref, computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useMcpStore } from "../stores/mcp";
import { Icon } from "@iconify/vue";
import type { McpServerConfig, TransportType, TransportConfig } from "../types";

const { t } = useI18n();
const mcpStore = useMcpStore();

// ── Modal state ──────────────────────────────────────────────────
const showModal = ref(false);
const isEditMode = ref(false);
const editingServer = ref<McpServerConfig | null>(null);
const isSaving = ref(false);
const saveSuccess = ref(false);
const saveError = ref<string | null>(null);

// ── Form fields ──────────────────────────────────────────────────
const formName = ref("");
const formTransportType = ref<TransportType>("stdio");
const formEnabled = ref(true);
// STDIO fields
const formCommand = ref("");
const formArgs = ref("");
const formEnv = ref("");
// URL-based fields (SSE / WebSocket / HTTP)
const formUrl = ref("");
const formHeaders = ref("");

const formErrors = ref<Record<string, string>>({});

// ── Computed ─────────────────────────────────────────────────────
const isUrlBasedTransport = computed(
    () =>
        formTransportType.value === "sse" ||
        formTransportType.value === "websocket" ||
        formTransportType.value === "http",
);

const transportOptions: {
    value: TransportType;
    label: string;
    icon: string;
}[] = [
    { value: "stdio", label: "STDIO", icon: "lucide:terminal" },
    { value: "sse", label: "SSE", icon: "lucide:radio" },
    { value: "websocket", label: "WebSocket", icon: "lucide:plug" },
    { value: "http", label: "HTTP", icon: "lucide:globe" },
];

// ── Lifecycle ────────────────────────────────────────────────────
onMounted(() => {
    mcpStore.fetchServers();
});

// ── Helpers ──────────────────────────────────────────────────────
function getTransportTypeLabel(type: TransportType): string {
    const map: Record<TransportType, string> = {
        stdio: "STDIO",
        sse: "SSE",
        websocket: "WebSocket",
        http: "HTTP",
    };
    return map[type] ?? type.toUpperCase();
}

function getTransportTypeIcon(type: TransportType): string {
    const map: Record<TransportType, string> = {
        stdio: "lucide:terminal",
        sse: "lucide:radio",
        websocket: "lucide:plug",
        http: "lucide:globe",
    };
    return map[type] ?? "lucide:hard-drive";
}

function getTransportConfigDisplay(server: McpServerConfig): string {
    if (server.transport_config.type === "stdio") {
        const cmd = server.transport_config.command;
        const args = server.transport_config.args?.join(" ") ?? "";
        return args ? `${cmd} ${args}` : cmd;
    }
    return "url" in server.transport_config ? server.transport_config.url : "";
}

function formatDate(dateStr: string): string {
    try {
        return new Date(dateStr).toLocaleString();
    } catch {
        return dateStr;
    }
}

// ── Form helpers ─────────────────────────────────────────────────
function parseArgs(text: string): string[] | undefined {
    const arr = text
        .trim()
        .split(/\s+/)
        .filter((s) => s.length > 0);
    return arr.length > 0 ? arr : undefined;
}

function parseEnv(text: string): Record<string, string> | undefined {
    const result: Record<string, string> = {};
    text.split("\n").forEach((line) => {
        const trimmed = line.trim();
        if (!trimmed) return;
        const eqIdx = trimmed.indexOf("=");
        if (eqIdx > 0) {
            const key = trimmed.slice(0, eqIdx).trim();
            const val = trimmed.slice(eqIdx + 1).trim();
            if (key) result[key] = val;
        }
    });
    return Object.keys(result).length > 0 ? result : undefined;
}

function parseHeaders(text: string): Record<string, string> | undefined {
    const result: Record<string, string> = {};
    text.split("\n").forEach((line) => {
        const trimmed = line.trim();
        if (!trimmed) return;
        const sepIdx = trimmed.indexOf(":");
        if (sepIdx > 0) {
            const key = trimmed.slice(0, sepIdx).trim();
            const val = trimmed.slice(sepIdx + 1).trim();
            if (key) result[key] = val;
        }
    });
    return Object.keys(result).length > 0 ? result : undefined;
}

function buildTransportConfig(): TransportConfig {
    if (formTransportType.value === "stdio") {
        return {
            type: "stdio",
            command: formCommand.value.trim(),
            args: parseArgs(formArgs.value),
            env: parseEnv(formEnv.value),
        };
    }
    if (formTransportType.value === "sse") {
        return {
            type: "sse",
            url: formUrl.value.trim(),
            headers: parseHeaders(formHeaders.value),
        };
    }
    if (formTransportType.value === "websocket") {
        return {
            type: "websocket",
            url: formUrl.value.trim(),
            headers: parseHeaders(formHeaders.value),
        };
    }
    // http
    return {
        type: "http",
        url: formUrl.value.trim(),
        headers: parseHeaders(formHeaders.value),
    };
}

// ── Validation ───────────────────────────────────────────────────
function validateForm(): boolean {
    formErrors.value = {};
    if (!formName.value.trim()) {
        formErrors.value.name =
            t("mcpConfig.serverName") + " " + t("common.required");
    }
    if (formTransportType.value === "stdio") {
        if (!formCommand.value.trim()) {
            formErrors.value.command =
                t("mcpConfig.command") + " " + t("common.required");
        }
    } else {
        if (!formUrl.value.trim()) {
            formErrors.value.url =
                t("mcpConfig.url") + " " + t("common.required");
        }
    }
    return Object.keys(formErrors.value).length === 0;
}

// ── Modal actions ────────────────────────────────────────────────
function openCreateModal() {
    isEditMode.value = false;
    editingServer.value = null;
    resetForm();
    showModal.value = true;
}

function openEditModal(server: McpServerConfig) {
    isEditMode.value = true;
    editingServer.value = server;

    formName.value = server.name;
    formTransportType.value = server.transport_type;
    formEnabled.value = server.enabled ?? true;

    if (server.transport_config.type === "stdio") {
        formCommand.value = server.transport_config.command;
        formArgs.value = server.transport_config.args?.join(" ") ?? "";
        formEnv.value = server.transport_config.env
            ? Object.entries(server.transport_config.env)
                  .map(([k, v]) => `${k}=${v}`)
                  .join("\n")
            : "";
        formUrl.value = "";
        formHeaders.value = "";
    } else {
        formUrl.value =
            "url" in server.transport_config ? server.transport_config.url : "";
        formHeaders.value =
            "headers" in server.transport_config &&
            server.transport_config.headers
                ? Object.entries(server.transport_config.headers)
                      .map(([k, v]) => `${k}: ${v}`)
                      .join("\n")
                : "";
        formCommand.value = "";
        formArgs.value = "";
        formEnv.value = "";
    }

    formErrors.value = {};
    saveSuccess.value = false;
    saveError.value = null;
    showModal.value = true;
}

function closeModal() {
    showModal.value = false;
    resetForm();
}

function resetForm() {
    formName.value = "";
    formTransportType.value = "stdio";
    formEnabled.value = true;
    formCommand.value = "";
    formArgs.value = "";
    formEnv.value = "";
    formUrl.value = "";
    formHeaders.value = "";
    formErrors.value = {};
    saveSuccess.value = false;
    saveError.value = null;
}

// ── Save / Delete / Toggle ───────────────────────────────────────
async function handleSave() {
    if (!validateForm()) return;

    isSaving.value = true;
    saveSuccess.value = false;
    saveError.value = null;

    try {
        const serverData = {
            name: formName.value.trim(),
            transport_type: formTransportType.value,
            transport_config: buildTransportConfig(),
            enabled: formEnabled.value,
        };

        if (isEditMode.value && editingServer.value) {
            await mcpStore.updateServer(editingServer.value.id, serverData);
        } else {
            await mcpStore.createServer(serverData);
        }

        saveSuccess.value = true;
        setTimeout(() => closeModal(), 1200);
    } catch (err: unknown) {
        saveError.value =
            err instanceof Error ? err.message : t("common.saveFailed");
    } finally {
        isSaving.value = false;
    }
}

async function handleDelete(server: McpServerConfig) {
    if (!confirm(`${t("common.deleteConfirm")}: ${server.name}?`)) return;
    try {
        await mcpStore.deleteServer(server.id);
    } catch (err: unknown) {
        console.error("Failed to delete server:", err);
    }
}

async function handleToggle(server: McpServerConfig) {
    try {
        await mcpStore.toggleServerEnabled(server.id);
    } catch (err: unknown) {
        console.error("Failed to toggle server:", err);
    }
}

// Close modal on Escape
watch(showModal, (val) => {
    if (val) {
        document.body.style.overflow = "hidden";
        const handler = (e: KeyboardEvent) => {
            if (e.key === "Escape") closeModal();
        };
        document.addEventListener("keydown", handler);
        // auto-cleanup when modal closes
        const stop = watch(showModal, () => {
            document.removeEventListener("keydown", handler);
            document.body.style.overflow = "";
            stop();
        });
    }
});
</script>

<template>
    <div class="mcp-page">
        <!-- Header -->
        <div class="page-header">
            <div class="header-content">
                <h1 class="page-title">{{ t("mcpConfig.title") }}</h1>
                <p class="page-subtitle">{{ t("mcpConfig.subtitle") }}</p>
            </div>
            <button @click="openCreateModal" class="btn btn-primary">
                <Icon icon="lucide:plus" class="btn-icon" />
                {{ t("mcpConfig.addServer") }}
            </button>
        </div>

        <!-- Info Banner -->
        <div class="info-banner">
            <div class="info-content">
                <Icon icon="lucide:info" class="info-icon" />
                <span>{{ t("mcpConfig.infoBanner") }}</span>
            </div>
            <a
                href="https://modelcontextprotocol.io"
                target="_blank"
                rel="noopener noreferrer"
                class="learn-more-link"
            >
                {{ t("mcpConfig.learnMore") }}
                <Icon icon="lucide:external-link" class="link-icon" />
            </a>
        </div>

        <!-- Error Banner -->
        <div v-if="mcpStore.error" class="error-banner">
            <Icon icon="lucide:alert-circle" class="error-icon" />
            <span>{{ mcpStore.error }}</span>
        </div>

        <!-- Loading -->
        <div v-if="mcpStore.loading" class="loading-state">
            <Icon icon="lucide:loader-2" class="spin-icon" />
            <span>{{ t("common.loading") }}</span>
        </div>

        <!-- Empty State -->
        <div v-else-if="mcpStore.servers.length === 0" class="empty-state">
            <Icon icon="lucide:server" class="empty-icon" />
            <p class="empty-title">{{ t("mcpConfig.noServers") }}</p>
            <button @click="openCreateModal" class="btn btn-accent">
                <Icon icon="lucide:plus" class="btn-icon" />
                {{ t("mcpConfig.addServer") }}
            </button>
        </div>

        <!-- Server List -->
        <div v-else class="server-list">
            <div
                v-for="server in mcpStore.servers"
                :key="server.id"
                class="server-card card"
                :class="{ 'server-card--disabled': !server.enabled }"
            >
                <div class="server-header">
                    <div class="server-info">
                        <div class="server-name-row">
                            <Icon
                                :icon="
                                    getTransportTypeIcon(server.transport_type)
                                "
                                class="transport-icon"
                            />
                            <h3 class="server-name">{{ server.name }}</h3>
                            <span
                                :class="[
                                    'badge',
                                    `badge-transport`,
                                    `badge-transport--${server.transport_type}`,
                                ]"
                            >
                                {{
                                    getTransportTypeLabel(server.transport_type)
                                }}
                            </span>
                        </div>
                        <code class="server-endpoint">{{
                            getTransportConfigDisplay(server)
                        }}</code>
                    </div>
                    <div class="server-actions">
                        <button
                            @click="handleToggle(server)"
                            class="btn btn-ghost btn-sm"
                            :title="
                                server.enabled
                                    ? t('common.disable')
                                    : t('common.enable')
                            "
                        >
                            <Icon
                                :icon="
                                    server.enabled
                                        ? 'lucide:toggle-right'
                                        : 'lucide:toggle-left'
                                "
                                :class="
                                    server.enabled
                                        ? 'text-primary'
                                        : 'text-muted'
                                "
                                class="action-icon"
                            />
                        </button>
                        <button
                            @click="openEditModal(server)"
                            class="btn btn-ghost btn-sm"
                            :title="t('common.edit')"
                        >
                            <Icon icon="lucide:pencil" class="action-icon" />
                        </button>
                        <button
                            @click="handleDelete(server)"
                            class="btn btn-danger-ghost btn-sm"
                            :title="t('common.delete')"
                        >
                            <Icon icon="lucide:trash-2" class="action-icon" />
                        </button>
                    </div>
                </div>
                <div class="server-meta">
                    <span class="meta-item">
                        <Icon icon="lucide:clock" class="meta-icon" />
                        {{ formatDate(server.created_at) }}
                    </span>
                    <span
                        :class="[
                            'status-indicator',
                            server.enabled
                                ? 'status-indicator--on'
                                : 'status-indicator--off',
                        ]"
                    >
                        <span class="status-dot" />
                        {{
                            server.enabled
                                ? t("common.enabled")
                                : t("common.disabled")
                        }}
                    </span>
                </div>
            </div>
        </div>

        <!-- ── Modal ────────────────────────────────────────────── -->
        <div v-if="showModal" class="modal-overlay" @click.self="closeModal">
            <div class="modal-content" @click.stop>
                <!-- Modal Header -->
                <div class="modal-header">
                    <h2 class="modal-title">
                        {{
                            isEditMode
                                ? t("mcpConfig.editServer")
                                : t("mcpConfig.createServer")
                        }}
                    </h2>
                    <button @click="closeModal" class="modal-close">
                        <Icon icon="lucide:x" />
                    </button>
                </div>

                <!-- Modal Body -->
                <div class="modal-body">
                    <!-- Success / Error alerts -->
                    <div v-if="saveSuccess" class="alert alert-success">
                        <Icon icon="lucide:check-circle" />
                        <span>{{ t("common.saveSuccess") }}</span>
                    </div>
                    <div v-if="saveError" class="alert alert-error">
                        <Icon icon="lucide:alert-circle" />
                        <span>{{ saveError }}</span>
                    </div>

                    <!-- Server Name -->
                    <div
                        class="form-group"
                        :class="{ 'has-error': formErrors.name }"
                    >
                        <label class="form-label">
                            {{ t("mcpConfig.serverName") }}
                            <span class="required">*</span>
                        </label>
                        <input
                            v-model="formName"
                            class="form-input"
                            :placeholder="t('mcpConfig.serverName')"
                            @input="formErrors.name = ''"
                        />
                        <span v-if="formErrors.name" class="form-error">{{
                            formErrors.name
                        }}</span>
                    </div>

                    <!-- Transport Type Tabs -->
                    <div class="form-group">
                        <label class="form-label">{{
                            t("mcpConfig.transportType")
                        }}</label>
                        <div class="transport-tabs">
                            <button
                                v-for="opt in transportOptions"
                                :key="opt.value"
                                :class="[
                                    'tab',
                                    {
                                        active: formTransportType === opt.value,
                                    },
                                ]"
                                @click="formTransportType = opt.value"
                            >
                                <Icon :icon="opt.icon" class="tab-icon" />
                                {{ opt.label }}
                            </button>
                        </div>
                    </div>

                    <!-- STDIO Config -->
                    <div
                        v-if="formTransportType === 'stdio'"
                        class="config-section"
                    >
                        <h3 class="section-title">
                            <Icon icon="lucide:terminal" />
                            STDIO {{ t("mcpConfig.transportConfig") }}
                        </h3>
                        <div
                            class="form-group"
                            :class="{ 'has-error': formErrors.command }"
                        >
                            <label class="form-label">
                                {{ t("mcpConfig.command") }}
                                <span class="required">*</span>
                            </label>
                            <input
                                v-model="formCommand"
                                class="form-input"
                                placeholder="e.g., npx"
                                @input="formErrors.command = ''"
                            />
                            <p class="form-hint">
                                {{ t("mcpConfig.commandHint") }}
                            </p>
                            <span
                                v-if="formErrors.command"
                                class="form-error"
                                >{{ formErrors.command }}</span
                            >
                        </div>
                        <div class="form-group">
                            <label class="form-label">{{
                                t("mcpConfig.args")
                            }}</label>
                            <input
                                v-model="formArgs"
                                class="form-input"
                                placeholder="-y @modelcontextprotocol/server-filesystem /tmp"
                            />
                            <p class="form-hint">
                                {{ t("mcpConfig.argPlaceholder") }}
                            </p>
                        </div>
                        <div class="form-group">
                            <label class="form-label">{{
                                t("mcpConfig.transportConfig")
                            }}</label>
                            <textarea
                                v-model="formEnv"
                                class="form-textarea"
                                rows="3"
                                placeholder="KEY=value"
                            />
                            <p class="form-hint">KEY=VALUE per line</p>
                        </div>
                    </div>

                    <!-- URL-based Config (SSE / WebSocket / HTTP) -->
                    <div v-if="isUrlBasedTransport" class="config-section">
                        <h3 class="section-title">
                            <Icon
                                :icon="getTransportTypeIcon(formTransportType)"
                            />
                            {{ getTransportTypeLabel(formTransportType) }}
                            {{ t("mcpConfig.transportConfig") }}
                        </h3>
                        <div
                            class="form-group"
                            :class="{ 'has-error': formErrors.url }"
                        >
                            <label class="form-label">
                                {{ t("mcpConfig.url") }}
                                <span class="required">*</span>
                            </label>
                            <input
                                v-model="formUrl"
                                class="form-input"
                                :placeholder="t('mcpConfig.urlPlaceholder')"
                                @input="formErrors.url = ''"
                            />
                            <span v-if="formErrors.url" class="form-error">{{
                                formErrors.url
                            }}</span>
                        </div>
                        <div class="form-group">
                            <label class="form-label">{{
                                t("mcpConfig.headers")
                            }}</label>
                            <textarea
                                v-model="formHeaders"
                                class="form-textarea"
                                rows="3"
                                :placeholder="
                                    t('mcpConfig.headerKeyPlaceholder') +
                                    ': ' +
                                    t('mcpConfig.headerValuePlaceholder')
                                "
                            />
                            <p class="form-hint">Header-Name: value per line</p>
                        </div>
                    </div>

                    <!-- Enabled Toggle -->
                    <div class="form-group toggle-row">
                        <label class="toggle-switch">
                            <input v-model="formEnabled" type="checkbox" />
                            <span class="toggle-track">
                                <span class="toggle-thumb" />
                            </span>
                        </label>
                        <span class="toggle-text">
                            {{
                                formEnabled
                                    ? t("common.enabled")
                                    : t("common.disabled")
                            }}
                        </span>
                    </div>
                </div>

                <!-- Modal Footer -->
                <div class="modal-footer">
                    <button @click="closeModal" class="btn btn-secondary">
                        {{ t("common.cancel") }}
                    </button>
                    <button
                        @click="handleSave"
                        class="btn btn-primary"
                        :disabled="isSaving"
                    >
                        <Icon
                            v-if="isSaving"
                            icon="lucide:loader-2"
                            class="spin-icon btn-icon"
                        />
                        <Icon v-else icon="lucide:save" class="btn-icon" />
                        <span>{{
                            isEditMode ? t("common.update") : t("common.create")
                        }}</span>
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
/* ── Page Layout ──────────────────────────────────────────────── */
.mcp-page {
    max-width: 960px;
    margin: 0 auto;
    animation: fadeIn 0.4s ease-out;
}

@keyframes fadeIn {
    from {
        opacity: 0;
        transform: translateY(8px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

/* ── Header ───────────────────────────────────────────────────── */
.page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1.5rem;
    gap: 1rem;
}

.header-content {
    flex: 1;
}

.page-title {
    font-size: 1.75rem;
    font-weight: 700;
    margin: 0 0 0.375rem 0;
    color: hsl(var(--foreground));
}

.page-subtitle {
    font-size: 0.9375rem;
    color: hsl(var(--muted-foreground));
    margin: 0;
}

/* ── Info Banner ──────────────────────────────────────────────── */
.info-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.875rem 1.125rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.08) 0%,
        hsl(var(--primary) / 0.04) 100%
    );
    border: 1px solid hsl(var(--primary) / 0.15);
    border-radius: var(--radius-lg);
    margin-bottom: 1.5rem;
    gap: 1rem;
}

.info-content {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    color: hsl(var(--foreground));
    font-size: 0.9375rem;
}

.info-icon {
    flex-shrink: 0;
    color: hsl(var(--primary));
    font-size: 1.25rem;
}

.learn-more-link {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    color: hsl(var(--primary));
    text-decoration: none;
    font-size: 0.875rem;
    font-weight: 500;
    white-space: nowrap;
    transition: all 0.2s;
    padding: 0.375rem 0.75rem;
    border-radius: var(--radius-md);
}

.learn-more-link:hover {
    background: hsl(var(--primary) / 0.1);
}

.link-icon {
    font-size: 0.875rem;
}

/* ── Error Banner ─────────────────────────────────────────────── */
.error-banner {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.75rem 1rem;
    background: hsl(var(--destructive) / 0.08);
    border: 1px solid hsl(var(--destructive) / 0.15);
    border-radius: var(--radius-lg);
    color: hsl(var(--destructive));
    margin-bottom: 1.5rem;
    font-size: 0.875rem;
}

.error-icon {
    flex-shrink: 0;
    font-size: 1.125rem;
}

/* ── Loading ──────────────────────────────────────────────────── */
.loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 3rem;
    color: hsl(var(--muted-foreground));
}

.spin-icon {
    animation: spin 1s linear infinite;
    font-size: 1.25rem;
}

@keyframes spin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

/* ── Empty State ──────────────────────────────────────────────── */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    text-align: center;
}

.empty-icon {
    font-size: 3.5rem;
    color: hsl(var(--muted-foreground) / 0.4);
    margin-bottom: 1rem;
}

.empty-title {
    font-size: 1.0625rem;
    color: hsl(var(--muted-foreground));
    margin: 0 0 1.5rem;
}

/* ── Server List ──────────────────────────────────────────────── */
.server-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}

.server-card {
    padding: 1.25rem;
    transition: all 0.2s ease;
}

.server-card:hover {
    border-color: hsl(var(--border));
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
}

.server-card--disabled {
    opacity: 0.6;
}

.server-card--disabled:hover {
    opacity: 0.8;
}

/* ── Server Card Header ───────────────────────────────────────── */
.server-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 0.75rem;
}

.server-info {
    flex: 1;
    min-width: 0;
}

.server-name-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.375rem;
    flex-wrap: wrap;
}

.transport-icon {
    font-size: 1.125rem;
    color: hsl(var(--primary));
    flex-shrink: 0;
}

.server-name {
    font-size: 1.0625rem;
    font-weight: 600;
    margin: 0;
    color: hsl(var(--foreground));
}

.badge-transport {
    font-size: 0.6875rem;
    font-weight: 600;
    padding: 0.125rem 0.5rem;
    border-radius: 9999px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.badge-transport--stdio {
    background: hsl(220 70% 50% / 0.12);
    color: hsl(220 70% 55%);
}

.badge-transport--sse {
    background: hsl(280 70% 50% / 0.12);
    color: hsl(280 70% 55%);
}

.badge-transport--websocket {
    background: hsl(160 70% 40% / 0.12);
    color: hsl(160 70% 45%);
}

.badge-transport--http {
    background: hsl(30 80% 50% / 0.12);
    color: hsl(30 80% 50%);
}

:global(.dark) .badge-transport--stdio {
    background: hsl(220 70% 60% / 0.15);
    color: hsl(220 70% 70%);
}

:global(.dark) .badge-transport--sse {
    background: hsl(280 70% 60% / 0.15);
    color: hsl(280 70% 70%);
}

:global(.dark) .badge-transport--websocket {
    background: hsl(160 70% 55% / 0.15);
    color: hsl(160 70% 65%);
}

:global(.dark) .badge-transport--http {
    background: hsl(30 80% 60% / 0.15);
    color: hsl(30 80% 65%);
}

.server-endpoint {
    display: block;
    font-size: 0.8125rem;
    color: hsl(var(--muted-foreground));
    background: hsl(var(--muted) / 0.4);
    padding: 0.25rem 0.5rem;
    border-radius: var(--radius-md);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
}

/* ── Server Actions ───────────────────────────────────────────── */
.server-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex-shrink: 0;
}

.action-icon {
    font-size: 1.125rem;
}

.text-primary {
    color: hsl(var(--primary));
}

.text-muted {
    color: hsl(var(--muted-foreground));
}

/* ── Server Meta ──────────────────────────────────────────────── */
.server-meta {
    display: flex;
    align-items: center;
    gap: 1.25rem;
    font-size: 0.8125rem;
    color: hsl(var(--muted-foreground));
    padding-top: 0.75rem;
    border-top: 1px solid hsl(var(--border) / 0.3);
}

.meta-item {
    display: flex;
    align-items: center;
    gap: 0.375rem;
}

.meta-icon {
    font-size: 0.875rem;
}

.status-indicator {
    display: flex;
    align-items: center;
    gap: 0.375rem;
}

.status-indicator--on {
    color: hsl(142 70% 45%);
}

.status-indicator--off {
    color: hsl(var(--muted-foreground));
}

.status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: hsl(var(--muted-foreground) / 0.4);
}

.status-indicator--on .status-dot {
    background: hsl(142 70% 45%);
    box-shadow: 0 0 6px hsl(142 70% 45% / 0.5);
}

.status-indicator--off .status-dot {
    background: hsl(var(--muted-foreground) / 0.3);
}

/* ── Modal Overlay ────────────────────────────────────────────── */
.modal-overlay {
    position: fixed;
    inset: 0;
    background: hsl(var(--background) / 0.8);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 1.5rem;
}

.modal-content {
    width: 100%;
    max-width: 560px;
    max-height: 90vh;
    border-radius: 1rem;
    background: hsl(var(--background));
    border: 1px solid hsl(var(--border));
    box-shadow: 0 25px 50px -12px hsl(var(--foreground) / 0.25);
    overflow: hidden;
    display: flex;
    flex-direction: column;
}

.modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid hsl(var(--border));
}

.modal-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

.modal-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    border: none;
    background: transparent;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s;
    font-size: 1.25rem;
}

.modal-close:hover {
    background: hsl(var(--muted) / 0.5);
    color: hsl(var(--foreground));
}

.modal-body {
    padding: 1.5rem;
    overflow-y: auto;
    flex: 1;
}

.modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    border-top: 1px solid hsl(var(--border));
}

/* ── Form ─────────────────────────────────────────────────────── */
.form-group {
    margin-bottom: 1rem;
}

.form-group.has-error .form-input,
.form-group.has-error .form-textarea {
    border-color: hsl(var(--destructive) / 0.5);
}

.form-group.has-error .form-input:focus,
.form-group.has-error .form-textarea:focus {
    border-color: hsl(var(--destructive));
    box-shadow: 0 0 0 3px hsl(var(--destructive) / 0.1);
}

.form-label {
    display: block;
    font-size: 0.8125rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin-bottom: 0.375rem;
}

.required {
    color: hsl(var(--destructive));
    margin-left: 0.125rem;
}

.form-input,
.form-textarea {
    width: 100%;
    padding: 0.5625rem 0.75rem;
    font-size: 0.875rem;
    border: 1px solid hsl(var(--border));
    border-radius: var(--radius-md);
    background: hsl(var(--background));
    color: hsl(var(--foreground));
    outline: none;
    transition: all 0.2s;
}

.form-input:focus,
.form-textarea:focus {
    border-color: hsl(var(--primary));
    box-shadow: 0 0 0 3px hsl(var(--primary) / 0.1);
}

.form-input::placeholder,
.form-textarea::placeholder {
    color: hsl(var(--muted-foreground) / 0.5);
}

.form-textarea {
    resize: vertical;
    min-height: 60px;
    font-family: var(--font-mono);
    font-size: 0.8125rem;
}

.form-hint {
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground) / 0.7);
    margin: 0.25rem 0 0;
}

.form-error {
    display: block;
    font-size: 0.75rem;
    color: hsl(var(--destructive));
    margin-top: 0.25rem;
}

/* ── Transport Tabs ───────────────────────────────────────────── */
.transport-tabs {
    display: flex;
    gap: 0.375rem;
    flex-wrap: wrap;
}

.tab {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.4375rem 0.875rem;
    font-size: 0.8125rem;
    font-weight: 600;
    border: 1px solid hsl(var(--border));
    border-radius: var(--radius-md);
    background: transparent;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s;
}

.tab:hover {
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--foreground));
}

.tab.active {
    background: hsl(var(--primary) / 0.12);
    border-color: hsl(var(--primary) / 0.3);
    color: hsl(var(--primary));
}

.tab-icon {
    font-size: 0.875rem;
}

/* ── Config Section ───────────────────────────────────────────── */
.config-section {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid hsl(var(--border) / 0.3);
}

.section-title {
    font-size: 0.9375rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0 0 0.75rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

/* ── Toggle Row ───────────────────────────────────────────────── */
.toggle-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-top: 1.25rem;
}

.toggle-switch {
    position: relative;
    display: inline-flex;
    cursor: pointer;
}

.toggle-switch input[type="checkbox"] {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
}

.toggle-track {
    width: 44px;
    height: 24px;
    background: hsl(var(--muted));
    border-radius: 12px;
    transition: background 0.2s ease;
    position: relative;
}

.toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    background: white;
    border-radius: 50%;
    transition: transform 0.2s ease;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

.toggle-switch input:checked + .toggle-track {
    background: hsl(var(--primary));
}

.toggle-switch input:checked + .toggle-track .toggle-thumb {
    transform: translateX(20px);
}

.toggle-switch:hover .toggle-track {
    background: hsl(var(--muted-foreground) / 0.3);
}

.toggle-switch:hover input:checked + .toggle-track {
    background: hsl(var(--primary) / 0.85);
}

.toggle-text {
    font-size: 0.875rem;
    font-weight: 500;
    color: hsl(var(--foreground));
}

/* ── Alert ────────────────────────────────────────────────────── */
.alert {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 0.875rem;
    border-radius: var(--radius-md);
    font-size: 0.8125rem;
    font-weight: 500;
    margin-bottom: 1rem;
}

.alert-success {
    background: hsl(142 70% 40% / 0.1);
    border: 1px solid hsl(142 70% 40% / 0.2);
    color: hsl(142 70% 40%);
}

.alert-error {
    background: hsl(var(--destructive) / 0.08);
    border: 1px solid hsl(var(--destructive) / 0.15);
    color: hsl(var(--destructive));
}

/* ── Button Icon ──────────────────────────────────────────────── */
.btn-icon {
    font-size: 1rem;
}

/* ── Responsive ───────────────────────────────────────────────── */
@media (max-width: 768px) {
    .mcp-page {
        padding: 0;
    }

    .page-header {
        flex-direction: column;
    }

    .page-title {
        font-size: 1.375rem;
    }

    .server-header {
        flex-direction: column;
    }

    .server-actions {
        align-self: flex-end;
    }

    .info-banner {
        flex-direction: column;
        align-items: flex-start;
    }

    .modal-content {
        max-width: 100%;
        max-height: 95vh;
    }

    .modal-body {
        padding: 1rem;
    }

    .modal-footer {
        flex-direction: column;
    }

    .modal-footer .btn {
        width: 100%;
        justify-content: center;
    }
}

@media (max-width: 480px) {
    .page-title {
        font-size: 1.25rem;
    }

    .server-name-row {
        flex-wrap: wrap;
    }

    .server-meta {
        flex-direction: column;
        align-items: flex-start;
        gap: 0.5rem;
    }

    .transport-tabs {
        flex-direction: column;
    }

    .tab {
        justify-content: center;
    }
}
</style>

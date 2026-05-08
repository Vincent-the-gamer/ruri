<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import axios from "axios";

const { t } = useI18n();
const activeTab = ref("chat");
const responseOutput = ref("");
const loading = ref(false);

// Chat test
const chatMessage = ref("");
const chatProviderId = ref("");
const chatTemperature = ref(0.7);
const chatMaxTokens = ref(4096);

// Custom request
const customMethod = ref("GET");
const customPath = ref("/api/agent/status");
const customBody = ref("{}");

const endpoints = [
    {
        method: "POST",
        path: "/api/chat",
        desc: "向智能体发送消息",
        body: true,
    },
    { method: "GET", path: "/api/chat/history", desc: "获取聊天记录" },
    { method: "DELETE", path: "/api/chat/history", desc: "清空聊天记录" },
    { method: "GET", path: "/api/providers", desc: "列出所有供应商" },
    {
        method: "POST",
        path: "/api/providers",
        desc: "创建新供应商",
        body: true,
    },
    { method: "GET", path: "/api/providers/:id", desc: "获取供应商详情" },
    {
        method: "PUT",
        path: "/api/providers/:id",
        desc: "更新供应商",
        body: true,
    },
    { method: "DELETE", path: "/api/providers/:id", desc: "删除供应商" },
    {
        method: "POST",
        path: "/api/providers/:id/activate",
        desc: "设为活跃供应商",
    },
    { method: "GET", path: "/api/skills", desc: "列出所有技能" },
    { method: "POST", path: "/api/skills", desc: "添加技能", body: true },
    { method: "DELETE", path: "/api/skills/:name", desc: "移除技能" },
    { method: "GET", path: "/api/tools", desc: "列出所有已注册工具" },
    { method: "GET", path: "/api/agent/status", desc: "获取智能体状态" },
];

const tabs = [
    { key: "chat", label: t("apiTest.tabs.chat") },
    { key: "custom", label: t("apiTest.tabs.custom") },
    { key: "docs", label: t("apiTest.tabs.docs") },
];

async function sendChat() {
    if (!chatMessage.value.trim()) return;
    loading.value = true;
    responseOutput.value = "";
    try {
        const body: Record<string, unknown> = {
            message: chatMessage.value,
            temperature: chatTemperature.value,
            max_tokens: chatMaxTokens.value,
        };
        if (chatProviderId.value.trim()) {
            body.provider_id = chatProviderId.value.trim();
        }
        const res = await axios.post("/api/chat", body);
        responseOutput.value = JSON.stringify(res.data, null, 2);
    } catch (e: unknown) {
        responseOutput.value = `Error: ${e instanceof Error ? e.message : "Unknown error"}`;
    } finally {
        loading.value = false;
    }
}

async function sendCustom() {
    loading.value = true;
    responseOutput.value = "";
    try {
        let body = undefined;
        if (["POST", "PUT", "PATCH"].includes(customMethod.value)) {
            try {
                body = JSON.parse(customBody.value);
            } catch {
                body = customBody.value;
            }
        }
        const config: Record<string, unknown> = {
            method: customMethod.value.toLowerCase(),
            url: customPath.value,
        };
        if (body) config.data = body;
        const res = await axios(config);
        responseOutput.value = JSON.stringify(res.data, null, 2);
    } catch (e: unknown) {
        if (axios.isAxiosError(e) && e.response) {
            responseOutput.value = `HTTP ${e.response.status}\n\n${JSON.stringify(e.response.data, null, 2)}`;
        } else {
            responseOutput.value = `Error: ${e instanceof Error ? e.message : "Unknown error"}`;
        }
    } finally {
        loading.value = false;
    }
}

function selectEndpoint(ep: (typeof endpoints)[0]) {
    customMethod.value = ep.method;
    customPath.value = ep.path.replace(/:id|:name/g, "");
    if (ep.body) {
        if (ep.path === "/api/chat") {
            customBody.value = JSON.stringify({ message: "Hello!" }, null, 2);
        } else if (ep.path === "/api/providers") {
            customBody.value = JSON.stringify(
                {
                    name: "My Provider",
                    provider_type: "openai",
                    config: {
                        type: "openai",
                        base_url: "https://api.openai.com/v1",
                        api_key: "sk-...",
                        default_model: "gpt-4o",
                    },
                },
                null,
                2,
            );
        } else if (ep.path === "/api/skills") {
            customBody.value = JSON.stringify(
                {
                    skill_type: "system_prompt",
                    config: { prompt: "You are a helpful assistant." },
                },
                null,
                2,
            );
        } else {
            customBody.value = "{}";
        }
    }
    activeTab.value = "custom";
}

const methodColor = (method: string) => {
    switch (method) {
        case "GET":
            return "method-get";
        case "POST":
            return "method-post";
        case "PUT":
            return "method-put";
        case "DELETE":
            return "method-delete";
        case "PATCH":
            return "method-patch";
        default:
            return "method-default";
    }
};
</script>

<template>
    <div class="api-test-view">
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
                        <path d="M9 3h1v3H9z" />
                        <path d="M14 3h1v3h-1z" />
                        <path d="M8 14h8" />
                        <path d="M8 18h5" />
                        <path d="M8 10h8" />
                        <path d="M3 6h18" />
                        <path
                            d="M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2"
                        />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">{{ t("apiTest.title") }}</h1>
                    <p class="header-desc">{{ t("apiTest.subtitle") }}</p>
                </div>
            </div>
        </div>

        <!-- Tabs -->
        <div class="tab-bar">
            <button
                v-for="tab in tabs"
                :key="tab.key"
                @click="activeTab = tab.key"
                class="tab-btn"
                :class="{ 'tab-btn-active': activeTab === tab.key }"
            >
                {{ tab.label }}
            </button>
        </div>

        <div class="main-grid">
            <!-- Left: Input -->
            <div>
                <!-- Chat Tab -->
                <template v-if="activeTab === 'chat'">
                    <div class="input-panel">
                        <h3 class="panel-title">POST /api/chat</h3>
                        <div class="form-group">
                            <label class="form-label">{{
                                t("apiTest.message")
                            }}</label>
                            <textarea
                                v-model="chatMessage"
                                rows="3"
                                placeholder="Hello, how are you?"
                                class="form-textarea"
                            ></textarea>
                        </div>
                        <div class="form-grid-2">
                            <div class="form-group">
                                <label class="form-label">{{
                                    t("apiTest.providerIdOptional")
                                }}</label>
                                <input
                                    v-model="chatProviderId"
                                    type="text"
                                    :placeholder="
                                        t('apiTest.useActiveProvider')
                                    "
                                    class="form-input"
                                />
                            </div>
                            <div class="form-group">
                                <label class="form-label">{{
                                    t("apiTest.temperature")
                                }}</label>
                                <input
                                    v-model.number="chatTemperature"
                                    type="number"
                                    step="0.1"
                                    min="0"
                                    max="2"
                                    class="form-input"
                                />
                            </div>
                        </div>
                        <button
                            @click="sendChat"
                            :disabled="loading || !chatMessage.trim()"
                            class="btn-primary"
                        >
                            {{
                                loading
                                    ? t("apiTest.sending")
                                    : t("apiTest.sendMessage")
                            }}
                        </button>
                    </div>
                </template>

                <!-- Custom Request Tab -->
                <template v-if="activeTab === 'custom'">
                    <div class="input-panel">
                        <h3 class="panel-title">
                            {{ t("apiTest.customRequest") }}
                        </h3>
                        <div class="custom-request-row">
                            <select v-model="customMethod" class="form-select">
                                <option>GET</option>
                                <option>POST</option>
                                <option>PUT</option>
                                <option>PATCH</option>
                                <option>DELETE</option>
                            </select>
                            <input
                                v-model="customPath"
                                type="text"
                                placeholder="/api/..."
                                class="form-input form-input-mono"
                            />
                        </div>
                        <div
                            v-if="
                                ['POST', 'PUT', 'PATCH'].includes(customMethod)
                            "
                            class="form-group"
                        >
                            <label class="form-label">{{
                                t("apiTest.requestBody")
                            }}</label>
                            <textarea
                                v-model="customBody"
                                rows="10"
                                class="form-textarea form-textarea-mono"
                            ></textarea>
                        </div>
                        <button
                            @click="sendCustom"
                            :disabled="loading"
                            class="btn-primary"
                        >
                            {{
                                loading
                                    ? t("apiTest.sending")
                                    : t("apiTest.sendRequest")
                            }}
                        </button>
                    </div>
                </template>

                <!-- API Docs Tab -->
                <template v-if="activeTab === 'docs'">
                    <div class="docs-panel">
                        <div
                            v-for="ep in endpoints"
                            :key="ep.method + ep.path"
                            @click="selectEndpoint(ep)"
                            class="endpoint-row"
                        >
                            <span
                                class="method-badge"
                                :class="methodColor(ep.method)"
                            >
                                {{ ep.method }}
                            </span>
                            <span class="endpoint-path">{{ ep.path }}</span>
                            <span class="endpoint-desc">{{ ep.desc }}</span>
                        </div>
                    </div>
                </template>
            </div>

            <!-- Right: Response -->
            <div>
                <div class="response-panel">
                    <div class="response-header">
                        <h3 class="panel-title">{{ t("apiTest.response") }}</h3>
                        <button
                            v-if="responseOutput"
                            @click="
                                (() => {
                                    try {
                                        navigator.clipboard.writeText(
                                            responseOutput,
                                        );
                                    } catch (e) {
                                        console.error(e);
                                    }
                                })()
                            "
                            class="btn-copy"
                        >
                            <svg
                                class="copy-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <rect
                                    x="9"
                                    y="9"
                                    width="13"
                                    height="13"
                                    rx="2"
                                    ry="2"
                                />
                                <path
                                    d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
                                />
                            </svg>
                            {{ t("apiTest.copy") }}
                        </button>
                    </div>
                    <div class="response-body">
                        <pre v-if="responseOutput" class="response-output">{{
                            responseOutput
                        }}</pre>
                        <div v-else class="response-placeholder">
                            {{ t("apiTest.sendToSeeResponse") }}
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <!-- Curl Examples -->
        <div class="curl-section">
            <h3 class="curl-title">
                <svg
                    class="curl-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <path
                        d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"
                    />
                    <polyline points="15 3 21 3 21 9" />
                    <line x1="10" y1="14" x2="21" y2="3" />
                </svg>
                {{ t("apiTest.externalApiCall") }}
            </h3>
            <p class="curl-desc">{{ t("apiTest.externalApiDesc") }}</p>
            <div class="curl-code-block">
                <div class="curl-example">
                    <span class="curl-comment"
                        ># {{ t("apiTest.curlSendMessage") }}</span
                    ><br />
                    <span class="curl-cmd">curl</span>
                    <span class="curl-flag">-X POST</span>
                    http://localhost:3000/api/chat \<br />
                    &nbsp;&nbsp;<span class="curl-flag">-H</span>
                    <span class="curl-string"
                        >"Content-Type: application/json"</span
                    >
                    \<br />
                    &nbsp;&nbsp;<span class="curl-flag">-d</span>
                    <span class="curl-string">
                        '{"message": "Hello, who are you?"}'</span
                    >
                </div>
                <div class="curl-example">
                    <span class="curl-comment"
                        ># {{ t("apiTest.curlGetStatus") }}</span
                    ><br />
                    <span class="curl-cmd">curl</span>
                    http://localhost:3000/api/agent/status
                </div>
                <div class="curl-example">
                    <span class="curl-comment"
                        ># {{ t("apiTest.curlListProviders") }}</span
                    ><br />
                    <span class="curl-cmd">curl</span>
                    http://localhost:3000/api/providers
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.api-test-view {
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

/* Tabs */
.tab-bar {
    display: inline-flex;
    gap: 0.25rem;
    margin-bottom: 1.5rem;
    background: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 3px;
}

.tab-btn {
    padding: 0.375rem 1rem;
    font-size: 0.8125rem;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: all 0.15s ease;
    font-weight: 400;
}

.tab-btn:hover {
    color: var(--color-text-secondary);
    background: var(--color-bg-hover);
}

.tab-btn-active {
    background: var(--color-accent-soft);
    color: var(--color-accent);
    font-weight: 500;
    border-color: transparent;
}

/* Main grid */
.main-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
}

/* Panels */
.input-panel,
.docs-panel,
.response-panel {
    background: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
}

.input-panel {
    display: flex;
    flex-direction: column;
    gap: 1rem;
}

.panel-title {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-text-secondary);
}

/* Form elements */
.form-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
}

.form-label {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    font-weight: 400;
}

.form-input,
.form-textarea,
.form-select {
    width: 100%;
    background: var(--color-bg-mute);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 0.5rem 0.75rem;
    font-size: 0.8125rem;
    color: var(--color-text);
    transition: border-color 0.15s ease;
    outline: none;
}

.form-input::placeholder,
.form-textarea::placeholder {
    color: var(--color-text-dim);
}

.form-input:focus,
.form-textarea:focus,
.form-select:focus {
    border-color: var(--color-accent);
}

.form-textarea {
    resize: vertical;
    min-height: 2.5rem;
}

.form-input-mono,
.form-textarea-mono {
    font-family: var(--font-mono, ui-monospace, monospace);
}

.form-select {
    cursor: pointer;
    -webkit-appearance: none;
    appearance: none;
    padding-right: 2rem;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2371717a' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.625rem center;
}

.form-grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
}

.custom-request-row {
    display: flex;
    gap: 0.625rem;
}

.custom-request-row .form-select {
    width: auto;
    min-width: 5.5rem;
    flex-shrink: 0;
}

.custom-request-row .form-input {
    flex: 1;
}

/* Buttons */
.btn-primary {
    width: 100%;
    padding: 0.5rem 1rem;
    background: var(--color-accent);
    color: #fff;
    font-size: 0.8125rem;
    font-weight: 500;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: background 0.15s ease;
}

.btn-primary:hover:not(:disabled) {
    background: var(--color-accent-hover);
}

.btn-primary:disabled {
    background: var(--color-bg-mute);
    color: var(--color-text-dim);
    cursor: not-allowed;
}

.btn-copy {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    color: var(--color-text-muted);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
}

.btn-copy:hover {
    color: var(--color-text-secondary);
    background: var(--color-bg-hover);
}

.copy-icon {
    width: 0.875rem;
    height: 0.875rem;
}

/* Method badges */
.method-badge {
    display: inline-block;
    padding: 0.125rem 0.5rem;
    font-size: 0.6875rem;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-weight: 500;
    min-width: 3.25rem;
    text-align: center;
}

.method-badge.method-get {
    color: var(--color-success);
    background: var(--color-success-soft);
}

.method-badge.method-post {
    color: var(--color-info);
    background: var(--color-info-soft);
}

.method-badge.method-put {
    color: var(--color-warning);
    background: var(--color-warning-soft);
}

.method-badge.method-delete {
    color: var(--color-danger);
    background: var(--color-danger-soft);
}

.method-badge.method-patch {
    color: var(--color-accent);
    background: var(--color-accent-soft);
}

.method-badge.method-default {
    color: var(--color-text-muted);
    background: var(--color-bg-mute);
}

/* Docs panel */
.docs-panel {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
}

.endpoint-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: background 0.15s ease;
    border: 1px solid transparent;
}

.endpoint-row:hover {
    background: var(--color-bg-hover);
    border-color: var(--color-border);
}

.endpoint-path {
    font-size: 0.8125rem;
    color: var(--color-text);
    font-family: var(--font-mono, ui-monospace, monospace);
    flex: 1;
}

.endpoint-desc {
    font-size: 0.6875rem;
    color: var(--color-text-dim);
}

/* Response panel */
.response-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
}

.response-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
}

.response-body {
    background: var(--color-bg-mute);
    border-radius: var(--radius-md);
    padding: 1rem;
    min-height: 18.75rem;
    max-height: 37.5rem;
    overflow: auto;
    border: 1px solid var(--color-border);
    flex: 1;
}

.response-output {
    font-size: 0.8125rem;
    color: var(--color-text);
    font-family: var(--font-mono, ui-monospace, monospace);
    white-space: pre-wrap;
    word-break: break-all;
    line-height: 1.6;
    margin: 0;
}

.response-placeholder {
    font-size: 0.8125rem;
    color: var(--color-text-dim);
    text-align: center;
    padding: 3rem 0;
}

/* Curl section */
.curl-section {
    margin-top: 2rem;
    background: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
}

.curl-title {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-text-secondary);
    margin-bottom: 0.75rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.curl-icon {
    width: 1rem;
    height: 1rem;
    color: var(--color-accent);
}

.curl-desc {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    margin-bottom: 0.75rem;
}

.curl-code-block {
    background: var(--color-bg-mute);
    border-radius: var(--radius-md);
    padding: 1.25rem;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.75rem;
    color: var(--color-text-secondary);
    overflow-x: auto;
    border: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    gap: 1rem;
}

.curl-example {
    line-height: 1.6;
}

.curl-example:not(:last-child) {
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--color-border);
}

.curl-comment {
    color: var(--color-text-dim);
}

.curl-cmd {
    color: var(--color-info);
}

.curl-flag {
    color: var(--color-warning);
}

.curl-string {
    color: var(--color-success);
}

/* Responsive */
@media (max-width: 768px) {
    .main-grid {
        grid-template-columns: 1fr;
    }
}

@media (max-width: 640px) {
    .page-header {
        flex-direction: column;
        align-items: flex-start;
    }
}
</style>

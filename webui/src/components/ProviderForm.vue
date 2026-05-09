<script setup lang="ts">
import { ref, reactive } from "vue";
import type {
    ProviderType,
    ProviderConfig,
    OpenAIProviderConfig,
    AnthropicProviderConfig,
    LmStudioProviderConfig,
    CustomProviderConfig,
    Provider,
} from "../types";

const props = defineProps<{
    provider?: Provider | null;
}>();

const emit = defineEmits<{
    save: [
        data: {
            name: string;
            provider_type: ProviderType;
            config: ProviderConfig;
        },
    ];
    cancel: [];
}>();

const name = ref(props.provider?.name || "");
const providerType = ref<ProviderType>(
    props.provider?.provider_type || "openai",
);
const showApiKey = ref(false);

const openaiConfig = reactive<OpenAIProviderConfig>({
    type: "openai",
    base_url: "https://api.openai.com/v1",
    api_key: "",
    default_model: "gpt-4o",
    ...(props.provider?.provider_type === "openai"
        ? (props.provider.config as OpenAIProviderConfig)
        : {}),
});

const anthropicConfig = reactive<AnthropicProviderConfig>({
    type: "anthropic",
    base_url: "https://api.anthropic.com",
    api_key: "",
    default_model: "claude-sonnet-4-20250514",
    api_version: "2023-06-01",
    ...(props.provider?.provider_type === "anthropic"
        ? (props.provider.config as AnthropicProviderConfig)
        : {}),
});

const lmStudioConfig = reactive<LmStudioProviderConfig>({
    type: "lm_studio",
    host: "localhost",
    port: 1234,
    api_key: "",
    default_model: "default",
    ...(props.provider?.provider_type === "lm_studio"
        ? (props.provider.config as LmStudioProviderConfig)
        : {}),
});

const customConfig = reactive<CustomProviderConfig>({
    type: "custom",
    base_url: "http://localhost:11434",
    chat_path: "/v1/chat/completions",
    method: "POST",
    auth_header: "Authorization",
    auth_prefix: "Bearer ",
    extra_headers: {},
    request_template: null,
    response_content_path: "choices.0.message.content",
    response_tool_calls_path: "choices.0.message.tool_calls",
    response_model_path: "model",
    response_finish_reason_path: "choices.0.finish_reason",
    default_model: "default",
    use_openai_format: true,
    ...(props.provider?.provider_type === "custom"
        ? (props.provider.config as CustomProviderConfig)
        : {}),
});

const extraHeadersText = ref(
    props.provider?.provider_type === "custom"
        ? JSON.stringify(
              (props.provider.config as CustomProviderConfig).extra_headers ||
                  {},
              null,
              2,
          )
        : "{}",
);

function handleSave() {
    if (!name.value.trim()) return;

    let config: ProviderConfig;
    switch (providerType.value) {
        case "openai":
            config = { ...openaiConfig };
            break;
        case "anthropic":
            config = { ...anthropicConfig };
            break;
        case "lm_studio":
            config = { ...lmStudioConfig };
            break;
        case "custom":
            try {
                customConfig.extra_headers = JSON.parse(
                    extraHeadersText.value || "{}",
                );
            } catch {
                customConfig.extra_headers = {};
            }
            config = { ...customConfig };
            break;
    }

    emit("save", {
        name: name.value,
        provider_type: providerType.value,
        config,
    });
}
</script>

<template>
    <div class="modal-backdrop">
        <div class="modal-card">
            <!-- Header -->
            <div class="modal-header">
                <h2 class="modal-title">
                    {{ provider ? "编辑供应商" : "添加供应商" }}
                </h2>
                <button @click="emit('cancel')" class="btn-close">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <line x1="18" y1="6" x2="6" y2="18" />
                        <line x1="6" y1="6" x2="18" y2="18" />
                    </svg>
                </button>
            </div>

            <!-- Body -->
            <div class="modal-body">
                <!-- Name -->
                <div class="form-group">
                    <label class="form-label">供应商名称</label>
                    <input
                        v-model="name"
                        type="text"
                        placeholder="例如：我的 OpenAI、Claude Pro"
                        class="form-input"
                    />
                </div>

                <!-- Type Selector -->
                <div class="form-group">
                    <label class="form-label">供应商类型</label>
                    <div class="type-selector">
                        <button
                            v-for="t in [
                                'openai',
                                'anthropic',
                                'lm_studio',
                                'custom',
                            ] as ProviderType[]"
                            :key="t"
                            @click="providerType = t"
                            class="type-btn"
                            :class="{ 'type-btn-active': providerType === t }"
                        >
                            <svg
                                v-if="t === 'openai'"
                                class="type-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <circle cx="12" cy="12" r="3" />
                                <path
                                    d="M12 2v4m0 12v4m-7.07-3.93l2.83-2.83m8.49-8.49l2.83-2.83M2 12h4m12 0h4M4.93 4.93l2.83 2.83m8.49 8.49l2.83 2.83"
                                />
                            </svg>
                            <svg
                                v-if="t === 'anthropic'"
                                class="type-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
                                />
                            </svg>
                            <svg
                                v-if="t === 'custom'"
                                class="type-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <circle cx="12" cy="12" r="3" />
                                <path
                                    d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
                                />
                            </svg>
                            <svg
                                v-if="t === 'lm_studio'"
                                class="type-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <rect
                                    x="2"
                                    y="3"
                                    width="20"
                                    height="14"
                                    rx="2"
                                    ry="2"
                                />
                                <line x1="8" y1="21" x2="16" y2="21" />
                                <line x1="12" y1="17" x2="12" y2="21" />
                            </svg>
                            {{
                                t === "openai"
                                    ? "OpenAI"
                                    : t === "anthropic"
                                      ? "Anthropic"
                                      : t === "lm_studio"
                                        ? "LM Studio"
                                        : "Custom"
                            }}
                        </button>
                    </div>
                </div>

                <!-- OpenAI Config -->
                <template v-if="providerType === 'openai'">
                    <div class="form-group">
                        <label class="form-label">接口地址</label>
                        <input
                            v-model="openaiConfig.base_url"
                            type="text"
                            class="form-input"
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label">API 密钥</label>
                        <div class="input-with-action">
                            <input
                                v-model="openaiConfig.api_key"
                                :type="showApiKey ? 'text' : 'password'"
                                placeholder="sk-..."
                                class="form-input"
                            />
                            <button
                                @click="showApiKey = !showApiKey"
                                class="btn-eye"
                            >
                                <svg
                                    v-if="showApiKey"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path
                                        d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"
                                    />
                                    <line x1="1" y1="1" x2="23" y2="23" />
                                </svg>
                                <svg
                                    v-else
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path
                                        d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"
                                    />
                                    <circle cx="12" cy="12" r="3" />
                                </svg>
                            </button>
                        </div>
                    </div>
                    <div class="form-group">
                        <label class="form-label">默认模型</label>
                        <input
                            v-model="openaiConfig.default_model"
                            type="text"
                            placeholder="gpt-4o"
                            class="form-input"
                        />
                    </div>
                </template>

                <!-- Anthropic Config -->
                <template v-if="providerType === 'anthropic'">
                    <div class="form-group">
                        <label class="form-label">接口地址</label>
                        <input
                            v-model="anthropicConfig.base_url"
                            type="text"
                            class="form-input"
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label">API 密钥</label>
                        <div class="input-with-action">
                            <input
                                v-model="anthropicConfig.api_key"
                                :type="showApiKey ? 'text' : 'password'"
                                placeholder="sk-ant-..."
                                class="form-input"
                            />
                            <button
                                @click="showApiKey = !showApiKey"
                                class="btn-eye"
                            >
                                <svg
                                    v-if="showApiKey"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path
                                        d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"
                                    />
                                    <line x1="1" y1="1" x2="23" y2="23" />
                                </svg>
                                <svg
                                    v-else
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path
                                        d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"
                                    />
                                    <circle cx="12" cy="12" r="3" />
                                </svg>
                            </button>
                        </div>
                    </div>
                    <div class="form-group">
                        <label class="form-label">默认模型</label>
                        <input
                            v-model="anthropicConfig.default_model"
                            type="text"
                            placeholder="claude-sonnet-4-20250514"
                            class="form-input"
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label">API 版本</label>
                        <input
                            v-model="anthropicConfig.api_version"
                            type="text"
                            placeholder="2023-06-01"
                            class="form-input"
                        />
                    </div>
                </template>

                <!-- LM Studio Config -->
                <template v-if="providerType === 'lm_studio'">
                    <div class="section-title">LM Studio Configuration</div>
                    <div class="form-group">
                        <label class="form-label">Host</label>
                        <input
                            v-model="lmStudioConfig.host"
                            type="text"
                            class="form-input"
                            placeholder="e.g., localhost"
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label">Port</label>
                        <input
                            v-model.number="lmStudioConfig.port"
                            type="number"
                            class="form-input"
                            placeholder="e.g., 1234"
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label">API Key</label>
                        <input
                            v-model="lmStudioConfig.api_key"
                            type="text"
                            class="form-input"
                            placeholder="Optional"
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label">Default Model</label>
                        <input
                            v-model="lmStudioConfig.default_model"
                            type="text"
                            class="form-input"
                            placeholder="e.g., default"
                        />
                    </div>
                </template>

                <!-- Custom Config -->
                <template v-if="providerType === 'custom'">
                    <div class="form-grid-2">
                        <div class="form-group">
                            <label class="form-label">接口地址</label>
                            <input
                                v-model="customConfig.base_url"
                                type="text"
                                class="form-input"
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label">对话路径</label>
                            <input
                                v-model="customConfig.chat_path"
                                type="text"
                                class="form-input"
                            />
                        </div>
                    </div>
                    <div class="form-grid-2">
                        <div class="form-group">
                            <label class="form-label">请求方式</label>
                            <select
                                v-model="customConfig.method"
                                class="form-select"
                            >
                                <option value="POST">POST</option>
                                <option value="GET">GET</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label class="form-label">默认模型</label>
                            <input
                                v-model="customConfig.default_model"
                                type="text"
                                class="form-input"
                            />
                        </div>
                    </div>
                    <div class="form-grid-2">
                        <div class="form-group">
                            <label class="form-label">认证请求头</label>
                            <input
                                v-model="customConfig.auth_header"
                                type="text"
                                placeholder="Authorization"
                                class="form-input"
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label">认证前缀</label>
                            <input
                                v-model="customConfig.auth_prefix"
                                type="text"
                                placeholder="Bearer "
                                class="form-input"
                            />
                        </div>
                    </div>
                    <div class="form-group">
                        <label class="form-label">API 密钥（可选）</label>
                        <div class="input-with-action">
                            <input
                                v-model="customConfig.api_key"
                                :type="showApiKey ? 'text' : 'password'"
                                placeholder="如不需要请留空"
                                class="form-input"
                            />
                            <button
                                @click="showApiKey = !showApiKey"
                                class="btn-eye"
                            >
                                <svg
                                    v-if="showApiKey"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path
                                        d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"
                                    />
                                    <line x1="1" y1="1" x2="23" y2="23" />
                                </svg>
                                <svg
                                    v-else
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path
                                        d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"
                                    />
                                    <circle cx="12" cy="12" r="3" />
                                </svg>
                            </button>
                        </div>
                    </div>

                    <!-- Response path mapping -->
                    <div class="section-divider">
                        <h4 class="section-title">响应路径映射</h4>
                    </div>
                    <div class="form-grid-2">
                        <div class="form-group">
                            <label class="form-label-sm">内容路径</label>
                            <input
                                v-model="customConfig.response_content_path"
                                type="text"
                                placeholder="choices.0.message.content"
                                class="form-input-sm"
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label-sm">工具调用路径</label>
                            <input
                                v-model="customConfig.response_tool_calls_path"
                                type="text"
                                placeholder="choices.0.message.tool_calls"
                                class="form-input-sm"
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label-sm">模型路径</label>
                            <input
                                v-model="customConfig.response_model_path"
                                type="text"
                                placeholder="model"
                                class="form-input-sm"
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label-sm">结束原因路径</label>
                            <input
                                v-model="
                                    customConfig.response_finish_reason_path
                                "
                                type="text"
                                placeholder="choices.0.finish_reason"
                                class="form-input-sm"
                            />
                        </div>
                    </div>

                    <!-- Toggle -->
                    <div class="toggle-row">
                        <label class="form-label">使用 OpenAI 格式</label>
                        <button
                            @click="
                                customConfig.use_openai_format =
                                    !customConfig.use_openai_format
                            "
                            class="toggle"
                            :class="{
                                'toggle-active': customConfig.use_openai_format,
                            }"
                        >
                            <span
                                class="toggle-thumb"
                                :class="{
                                    'toggle-thumb-active':
                                        customConfig.use_openai_format,
                                }"
                            ></span>
                        </button>
                    </div>

                    <!-- Extra headers -->
                    <div class="form-group">
                        <label class="form-label">额外请求头 (JSON)</label>
                        <textarea
                            v-model="extraHeadersText"
                            rows="3"
                            class="form-textarea-sm"
                            placeholder='{"X-Custom-Header": "value"}'
                        ></textarea>
                    </div>
                </template>
            </div>

            <!-- Footer -->
            <div class="modal-footer">
                <button @click="emit('cancel')" class="btn-ghost">取消</button>
                <button
                    @click="handleSave"
                    :disabled="!name.trim()"
                    class="btn-accent"
                >
                    {{ provider ? "更新" : "创建" }}
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
}

.modal-card {
    background: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xl);
    width: 100%;
    max-width: 42rem;
    max-height: 90vh;
    overflow-y: auto;
}

.modal-header {
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--color-border);
    display: flex;
    align-items: center;
    justify-content: space-between;
}

.modal-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text);
}

.btn-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.75rem;
    height: 1.75rem;
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: all 0.15s ease;
}

.btn-close:hover {
    color: var(--color-text);
    background: var(--color-bg-hover);
}

.btn-close svg {
    width: 1.125rem;
    height: 1.125rem;
}

.modal-body {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
}

.modal-footer {
    padding: 1rem 1.5rem;
    border-top: 1px solid var(--color-border);
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
}

/* Form elements */
.form-group {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
}

.form-label {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-text-secondary);
}

.form-label-sm {
    font-size: 0.6875rem;
    font-weight: 500;
    color: var(--color-text-muted);
}

.form-input,
.form-input-sm,
.form-select,
.form-textarea-sm {
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

.form-input-sm {
    padding: 0.375rem 0.75rem;
    font-size: 0.75rem;
}

.form-input::placeholder,
.form-input-sm::placeholder,
.form-textarea-sm::placeholder {
    color: var(--color-text-dim);
}

.form-input:focus,
.form-input-sm:focus,
.form-select:focus,
.form-textarea-sm:focus {
    border-color: var(--color-accent);
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

.form-textarea-sm {
    resize: vertical;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.75rem;
    min-height: 2.5rem;
}

.form-grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
}

/* Input with eye button */
.input-with-action {
    position: relative;
}

.input-with-action .form-input {
    padding-right: 2.5rem;
}

.btn-eye {
    position: absolute;
    right: 0.5rem;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    color: var(--color-text-muted);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: color 0.15s ease;
}

.btn-eye:hover {
    color: var(--color-text-secondary);
}

.btn-eye svg {
    width: 1rem;
    height: 1rem;
}

/* Type selector */
.type-selector {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.5rem;
}

.type-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    padding: 0.5rem 0.75rem;
    border-radius: var(--radius-md);
    font-size: 0.8125rem;
    font-weight: 500;
    border: 1px solid var(--color-border);
    background: var(--color-bg-mute);
    color: var(--color-text-muted);
    cursor: pointer;
    transition: all 0.15s ease;
}

.type-btn:hover {
    color: var(--color-text-secondary);
    border-color: var(--color-border-hover);
}

.type-btn-active {
    background: var(--color-accent-soft);
    color: var(--color-accent);
    border-color: transparent;
}

.type-icon {
    width: 0.875rem;
    height: 0.875rem;
}

/* Section divider */
.section-divider {
    padding-top: 0.5rem;
    border-top: 1px solid var(--color-border);
}

.section-title {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-text-secondary);
    margin-bottom: 0;
}

/* Toggle */
.toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
}

.toggle {
    position: relative;
    width: 2.25rem;
    height: 1.25rem;
    border-radius: 9999px;
    background: var(--color-bg-mute);
    border: 1px solid var(--color-border);
    cursor: pointer;
    transition: all 0.15s ease;
}

.toggle-active {
    background: var(--color-accent);
    border-color: var(--color-accent);
}

.toggle-thumb {
    position: absolute;
    top: 1px;
    left: 1px;
    width: 0.875rem;
    height: 0.875rem;
    background: var(--color-text-muted);
    border-radius: 50%;
    transition: all 0.15s ease;
}

.toggle-thumb-active {
    left: calc(100% - 1px);
    transform: translateX(-100%);
    background: #fff;
}

/* Footer buttons */
.btn-ghost {
    padding: 0.5rem 1.25rem;
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    background: transparent;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.15s ease;
}

.btn-ghost:hover {
    color: var(--color-text);
    border-color: var(--color-border-hover);
    background: var(--color-bg-hover);
}

.btn-accent {
    padding: 0.5rem 1.25rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: #fff;
    background: var(--color-accent);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: background 0.15s ease;
}

.btn-accent:hover:not(:disabled) {
    background: var(--color-accent-hover);
}

.btn-accent:disabled {
    background: var(--color-bg-mute);
    color: var(--color-text-dim);
    cursor: not-allowed;
}
</style>

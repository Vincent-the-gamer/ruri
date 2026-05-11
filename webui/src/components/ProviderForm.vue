<script setup lang="ts">
import { ref, reactive } from "vue";
import { useI18n } from "vue-i18n";
import type {
    ProviderType,
    ProviderConfig,
    OpenAIProviderConfig,
    AnthropicProviderConfig,
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

const { t } = useI18n();

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
    supports_multimodal: true,
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
    supports_multimodal: true,
    ...(props.provider?.provider_type === "anthropic"
        ? (props.provider.config as AnthropicProviderConfig)
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
    supports_multimodal: false,
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
                    {{
                        provider
                            ? t("providers.editProvider")
                            : t("providers.addProvider")
                    }}
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
                    <label class="form-label">{{
                        t("providers.form.name")
                    }}</label>
                    <input
                        v-model="name"
                        type="text"
                        :placeholder="t('providers.form.namePlaceholder')"
                        class="form-input"
                    />
                </div>

                <!-- Type Selector -->
                <div class="form-group">
                    <label class="form-label">{{
                        t("providers.form.type")
                    }}</label>
                    <div class="type-selector">
                        <button
                            v-for="typeItem in [
                                'openai',
                                'anthropic',
                                'custom',
                            ] as ProviderType[]"
                            :key="typeItem"
                            @click="providerType = typeItem"
                            class="type-btn"
                            :class="{
                                'type-btn-active': providerType === typeItem,
                            }"
                        >
                            <svg
                                v-if="typeItem === 'openai'"
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
                                v-if="typeItem === 'anthropic'"
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
                                v-if="typeItem === 'custom'"
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
                            {{
                                typeItem === "openai"
                                    ? t("providers.type.openai")
                                    : typeItem === "anthropic"
                                      ? t("providers.type.anthropic")
                                      : t("providers.type.custom")
                            }}
                        </button>
                    </div>
                </div>

                <!-- OpenAI Config -->
                <template v-if="providerType === 'openai'">
                    <div class="form-group">
                        <label class="form-label">{{
                            t("providers.form.baseUrl")
                        }}</label>
                        <input
                            v-model="openaiConfig.base_url"
                            type="text"
                            class="form-input"
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label">{{
                            t("providers.form.apiKey")
                        }}</label>
                        <div class="input-with-action">
                            <input
                                v-model="openaiConfig.api_key"
                                :type="showApiKey ? 'text' : 'password'"
                                :placeholder="
                                    t('providers.form.apiKeyPlaceholder')
                                "
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
                        <label class="form-label">{{
                            t("providers.form.defaultModel")
                        }}</label>
                        <input
                            v-model="openaiConfig.default_model"
                            type="text"
                            :placeholder="
                                t('providers.form.defaultModelPlaceholder')
                            "
                            class="form-input"
                        />
                    </div>
                    <!-- Multimodal toggle -->
                    <div class="toggle-row">
                        <label class="form-label">{{
                            t("providers.form.supportsMultimodal")
                        }}</label>
                        <button
                            @click="
                                openaiConfig.supports_multimodal =
                                    !openaiConfig.supports_multimodal
                            "
                            class="toggle"
                            :class="{
                                'toggle-active':
                                    openaiConfig.supports_multimodal,
                            }"
                        >
                            <span
                                class="toggle-thumb"
                                :class="{
                                    'toggle-thumb-active':
                                        openaiConfig.supports_multimodal,
                                }"
                            ></span>
                        </button>
                    </div>
                </template>

                <!-- Anthropic Config -->
                <template v-if="providerType === 'anthropic'">
                    <div class="form-group">
                        <label class="form-label">{{
                            t("providers.form.baseUrl")
                        }}</label>
                        <input
                            v-model="anthropicConfig.base_url"
                            type="text"
                            class="form-input"
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label">{{
                            t("providers.form.apiKey")
                        }}</label>
                        <div class="input-with-action">
                            <input
                                v-model="anthropicConfig.api_key"
                                :type="showApiKey ? 'text' : 'password'"
                                :placeholder="
                                    t('providers.form.apiKeyPlaceholder')
                                "
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
                        <label class="form-label">{{
                            t("providers.form.defaultModel")
                        }}</label>
                        <input
                            v-model="anthropicConfig.default_model"
                            type="text"
                            :placeholder="
                                t('providers.form.defaultModelPlaceholder')
                            "
                            class="form-input"
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label">{{
                            t("providers.form.apiVersion")
                        }}</label>
                        <input
                            v-model="anthropicConfig.api_version"
                            type="text"
                            :placeholder="
                                t('providers.form.apiVersionPlaceholder')
                            "
                            class="form-input"
                        />
                    </div>
                    <!-- Multimodal toggle -->
                    <div class="toggle-row">
                        <label class="form-label">{{
                            t("providers.form.supportsMultimodal")
                        }}</label>
                        <button
                            @click="
                                anthropicConfig.supports_multimodal =
                                    !anthropicConfig.supports_multimodal
                            "
                            class="toggle"
                            :class="{
                                'toggle-active':
                                    anthropicConfig.supports_multimodal,
                            }"
                        >
                            <span
                                class="toggle-thumb"
                                :class="{
                                    'toggle-thumb-active':
                                        anthropicConfig.supports_multimodal,
                                }"
                            ></span>
                        </button>
                    </div>
                </template>

                <template v-if="providerType === 'custom'">
                    <div class="form-grid-2">
                        <div class="form-group">
                            <label class="form-label">{{
                                t("providers.form.baseUrl")
                            }}</label>
                            <input
                                v-model="customConfig.base_url"
                                type="text"
                                class="form-input"
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label">{{
                                t("providers.form.chatPath")
                            }}</label>
                            <input
                                v-model="customConfig.chat_path"
                                type="text"
                                class="form-input"
                            />
                        </div>
                    </div>
                    <div class="form-grid-2">
                        <div class="form-group">
                            <label class="form-label">{{
                                t("providers.form.method")
                            }}</label>
                            <select
                                v-model="customConfig.method"
                                class="form-select"
                            >
                                <option value="POST">POST</option>
                                <option value="GET">GET</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label class="form-label">{{
                                t("providers.form.defaultModel")
                            }}</label>
                            <input
                                v-model="customConfig.default_model"
                                type="text"
                                class="form-input"
                            />
                        </div>
                    </div>
                    <div class="form-grid-2">
                        <div class="form-group">
                            <label class="form-label">{{
                                t("providers.form.authHeader")
                            }}</label>
                            <input
                                v-model="customConfig.auth_header"
                                type="text"
                                :placeholder="
                                    t('providers.form.authHeaderPlaceholder')
                                "
                                class="form-input"
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label">{{
                                t("providers.form.authPrefix")
                            }}</label>
                            <input
                                v-model="customConfig.auth_prefix"
                                type="text"
                                :placeholder="
                                    t('providers.form.authPrefixPlaceholder')
                                "
                                class="form-input"
                            />
                        </div>
                    </div>
                    <div class="form-group">
                        <label class="form-label">{{
                            t("providers.form.apiKeyOptional")
                        }}</label>
                        <div class="input-with-action">
                            <input
                                v-model="customConfig.api_key"
                                :type="showApiKey ? 'text' : 'password'"
                                :placeholder="
                                    t(
                                        'providers.form.apiKeyOptionalPlaceholder',
                                    )
                                "
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
                        <h4 class="section-title">
                            {{ t("providers.form.responsePathMapping") }}
                        </h4>
                    </div>
                    <div class="form-grid-2">
                        <div class="form-group">
                            <label class="form-label-sm">{{
                                t("providers.form.contentPath")
                            }}</label>
                            <input
                                v-model="customConfig.response_content_path"
                                type="text"
                                placeholder="choices.0.message.content"
                                class="form-input-sm"
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label-sm">{{
                                t("providers.form.toolCallsPath")
                            }}</label>
                            <input
                                v-model="customConfig.response_tool_calls_path"
                                type="text"
                                placeholder="choices.0.message.tool_calls"
                                class="form-input-sm"
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label-sm">{{
                                t("providers.form.modelPath")
                            }}</label>
                            <input
                                v-model="customConfig.response_model_path"
                                type="text"
                                placeholder="model"
                                class="form-input-sm"
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label-sm">{{
                                t("providers.form.finishReasonPath")
                            }}</label>
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
                        <label class="form-label">{{
                            t("providers.form.useOpenaiFormat")
                        }}</label>
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

                    <!-- Multimodal toggle -->
                    <div class="toggle-row">
                        <label class="form-label">{{
                            t("providers.form.supportsMultimodal")
                        }}</label>
                        <button
                            @click="
                                customConfig.supports_multimodal =
                                    !customConfig.supports_multimodal
                            "
                            class="toggle"
                            :class="{
                                'toggle-active':
                                    customConfig.supports_multimodal,
                            }"
                        >
                            <span
                                class="toggle-thumb"
                                :class="{
                                    'toggle-thumb-active':
                                        customConfig.supports_multimodal,
                                }"
                            ></span>
                        </button>
                    </div>

                    <!-- Extra headers -->
                    <div class="form-group">
                        <label class="form-label">{{
                            t("providers.form.extraHeaders")
                        }}</label>
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
                <button @click="emit('cancel')" class="btn-ghost">
                    {{ t("common.cancel") }}
                </button>
                <button
                    @click="handleSave"
                    :disabled="!name.trim()"
                    class="btn-accent"
                >
                    {{
                        provider
                            ? t("providers.form.update")
                            : t("providers.form.create")
                    }}
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.modal-backdrop {
    position: fixed;
    inset: 0;
    background: hsl(var(--background) / 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
    padding: 1rem;
    animation: fadeIn 0.2s ease-out;
}

@keyframes fadeIn {
    from {
        opacity: 0;
    }
    to {
        opacity: 1;
    }
}

.modal-card {
    background: hsl(var(--background));
    border: 1px solid hsl(var(--border));
    border-radius: 1rem;
    width: 100%;
    max-width: 42rem;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 25px 50px -12px hsl(var(--foreground) / 0.25);
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
    padding: 1rem 1.25rem;
    border-bottom: 1px solid hsl(var(--border) / 0.2);
    display: flex;
    align-items: center;
    justify-content: space-between;
}

.modal-title {
    font-size: 1.1rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

.btn-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.75rem;
    height: 1.75rem;
    border-radius: 0.375rem;
    color: hsl(var(--muted-foreground));
    background: transparent;
    border: none;
    cursor: pointer;
    transition: all 0.2s ease;
}

.btn-close:hover {
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--foreground));
}

.btn-close svg {
    width: 1.125rem;
    height: 1.125rem;
}

.modal-body {
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    overflow-y: auto;
    flex: 1;
}

.modal-footer {
    padding: 1rem 1.25rem;
    border-top: 1px solid hsl(var(--border) / 0.2);
    display: flex;
    justify-content: flex-end;
    gap: 0.625rem;
}

/* Form elements */
.form-group {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
}

.form-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
}

.form-label-sm {
    font-size: 0.6875rem;
    font-weight: 600;
    color: hsl(var(--muted-foreground) / 0.8);
}

.form-input,
.form-input-sm,
.form-select,
.form-textarea-sm {
    width: 100%;
    background: hsl(var(--background) / 0.5);
    border: 1px solid hsl(var(--border) / 0.4);
    border-radius: 0.5rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.8125rem;
    color: hsl(var(--foreground));
    transition: all 0.2s ease;
    outline: none;
}

.form-input-sm {
    padding: 0.375rem 0.75rem;
    font-size: 0.75rem;
}

.form-input::placeholder,
.form-input-sm::placeholder,
.form-textarea-sm::placeholder {
    color: hsl(var(--muted-foreground) / 0.5);
}

.form-input:focus,
.form-input-sm:focus,
.form-select:focus,
.form-textarea-sm:focus {
    border-color: hsl(var(--primary) / 0.5);
    box-shadow: 0 0 0 2px hsl(var(--primary) / 0.1);
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
    font-family:
        ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
    font-size: 0.75rem;
    min-height: 2.5rem;
}

.form-grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.875rem;
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
    color: hsl(var(--muted-foreground));
    background: transparent;
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    transition: color 0.2s ease;
}

.btn-eye:hover {
    color: hsl(var(--foreground));
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
    border-radius: 0.5rem;
    font-size: 0.8125rem;
    font-weight: 500;
    border: 1px solid hsl(var(--border) / 0.4);
    background: hsl(var(--background) / 0.5);
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s ease;
}

.type-btn:hover {
    color: hsl(var(--foreground));
    border-color: hsl(var(--border) / 0.6);
}

.type-btn-active {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
    border-color: hsl(var(--primary) / 0.4);
}

.type-icon {
    width: 0.875rem;
    height: 0.875rem;
}

/* Section divider */
.section-divider {
    padding-top: 0.5rem;
    border-top: 1px solid hsl(var(--border) / 0.2);
}

.section-title {
    font-size: 0.8125rem;
    font-weight: 500;
    color: hsl(var(--muted-foreground));
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
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border) / 0.5);
    cursor: pointer;
    transition: all 0.2s ease;
}

.toggle-active {
    background: hsl(var(--primary));
    border-color: hsl(var(--primary));
}

.toggle-thumb {
    position: absolute;
    top: 1px;
    left: 1px;
    width: 0.875rem;
    height: 0.875rem;
    background: hsl(var(--muted-foreground));
    border-radius: 50%;
    transition: all 0.2s ease;
}

.toggle-thumb-active {
    left: calc(100% - 1px);
    transform: translateX(-100%);
    background: white;
}

/* Footer buttons - consistent sizing */
.btn-ghost {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    white-space: nowrap;
    padding: 0.5rem 1.125rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: hsl(var(--muted-foreground));
    background: transparent;
    border: 1px solid hsl(var(--border) / 0.5);
    border-radius: 0.5rem;
    cursor: pointer;
    transition: all 0.2s ease;
    min-height: 2.125rem;
    line-height: 1.4;
}

.btn-ghost:hover {
    color: hsl(var(--foreground));
    border-color: hsl(var(--border));
    background: hsl(var(--secondary) / 0.5);
}

.btn-accent {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    white-space: nowrap;
    padding: 0.5rem 1.125rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: hsl(var(--primary-foreground));
    background: linear-gradient(
        135deg,
        hsl(var(--primary)),
        hsl(var(--primary) / 0.9)
    );
    border: none;
    border-radius: 0.5rem;
    cursor: pointer;
    transition: all 0.2s ease;
    min-height: 2.125rem;
    line-height: 1.4;
    box-shadow: 0 1px 4px hsl(var(--primary) / 0.25);
}

.btn-accent:hover:not(:disabled) {
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.95),
        hsl(var(--primary) / 0.85)
    );
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.35);
    transform: translateY(-1px);
}

.btn-accent:disabled {
    background: hsl(var(--secondary));
    color: hsl(var(--muted-foreground));
    cursor: not-allowed;
    opacity: 0.5;
}

/* Scrollbar */
.modal-body::-webkit-scrollbar {
    width: 5px;
}

.modal-body::-webkit-scrollbar-track {
    background: transparent;
}

.modal-body::-webkit-scrollbar-thumb {
    background: hsl(var(--muted) / 0.5);
    border-radius: 3px;
}

.modal-body::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--muted-foreground) / 0.4);
}

/* Responsive */
@media (max-width: 640px) {
    .form-grid-2 {
        grid-template-columns: 1fr;
    }
}
</style>

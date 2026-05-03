<script setup lang="ts">
import { onMounted, ref, computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useWebSearchStore } from "../stores/webSearch";
import type { SearchEngine } from "../types";

const { t } = useI18n();
const webSearchStore = useWebSearchStore();

const selectedEngine = ref<SearchEngine>("duckduckgo");
const apiKey = ref<string>("");
const maxResults = ref<number>(10);
const enabled = ref<boolean>(true);

const saveSuccess = ref(false);
const saveError = ref<string | null>(null);

onMounted(async () => {
    await webSearchStore.fetchConfig();
    syncFromStore();
});

function syncFromStore() {
    if (webSearchStore.config) {
        selectedEngine.value = webSearchStore.config.search_engine;
        apiKey.value = webSearchStore.config.api_key || "";
        maxResults.value = webSearchStore.config.max_results;
        enabled.value = webSearchStore.config.enabled;
    }
}

watch([selectedEngine, apiKey, maxResults, enabled], () => {
    clearMessages();
});

function clearMessages() {
    saveSuccess.value = false;
    saveError.value = null;
}

async function handleSave() {
    saveSuccess.value = false;
    saveError.value = null;

    try {
        await webSearchStore.updateConfig({
            search_engine: selectedEngine.value,
            api_key: apiKey.value || null,
            max_results: maxResults.value,
            enabled: enabled.value,
        });
        saveSuccess.value = true;
    } catch (e: unknown) {
        saveError.value =
            e instanceof Error ? e.message : t("webSearchConfig.saveFailed");
    }
}

const hasChanges = computed(() => {
    if (!webSearchStore.config) return false;

    return (
        selectedEngine.value !== webSearchStore.config.search_engine ||
        apiKey.value !== (webSearchStore.config.api_key || "") ||
        maxResults.value !== webSearchStore.config.max_results ||
        enabled.value !== webSearchStore.config.enabled
    );
});

const needsApiKey = computed(() => {
    return selectedEngine.value !== "duckduckgo";
});

const searchEngines = computed(() => [
    {
        value: "duckduckgo",
        label: t("webSearchConfig.engines.duckduckgo"),
        desc: t("webSearchConfig.engines.duckduckgoDesc"),
    },
    {
        value: "tavily",
        label: t("webSearchConfig.engines.tavily"),
        desc: t("webSearchConfig.engines.tavilyDesc"),
    },
    {
        value: "bocha",
        label: t("webSearchConfig.engines.bocha"),
        desc: t("webSearchConfig.engines.bochaDesc"),
    },
    {
        value: "baidu",
        label: t("webSearchConfig.engines.baidu"),
        desc: t("webSearchConfig.engines.baiduDesc"),
    },
    {
        value: "brave",
        label: t("webSearchConfig.engines.brave"),
        desc: t("webSearchConfig.engines.braveDesc"),
    },
]);
</script>

<template>
    <div class="page">
        <div class="page-header">
            <div class="header-info">
                <h1 class="header-title">{{ t("webSearchConfig.title") }}</h1>
                <p class="header-desc">{{ t("webSearchConfig.subtitle") }}</p>
            </div>
        </div>

        <div v-if="webSearchStore.error" class="error-banner">
            {{ webSearchStore.error }}
        </div>
        <div v-if="saveSuccess" class="success-banner">
            {{ t("webSearchConfig.saveSuccess") }}
        </div>
        <div v-if="saveError" class="error-banner">
            {{ saveError }}
        </div>

        <div
            v-if="webSearchStore.loading && !webSearchStore.config"
            class="loading-state"
        >
            {{ t("webSearchConfig.loading") }}
        </div>
        <template v-else-if="webSearchStore.config">
            <!-- Enable/Disable Toggle -->
            <section class="config-section">
                <h2 class="section-title">{{ t("webSearchConfig.status") }}</h2>
                <p class="section-desc">
                    {{ t("webSearchConfig.statusDesc") }}
                </p>
                <div class="toggle-row">
                    <label class="toggle-container">
                        <input
                            type="checkbox"
                            v-model="enabled"
                            class="toggle-input"
                        />
                        <span class="toggle" :class="{ 'toggle--on': enabled }">
                            <span class="toggle-thumb"></span>
                        </span>
                    </label>
                    <span class="toggle-text">{{
                        enabled
                            ? t("webSearchConfig.enabled")
                            : t("webSearchConfig.disabled")
                    }}</span>
                </div>
            </section>

            <!-- Search Engine Selection -->
            <section class="config-section">
                <h2 class="section-title">
                    {{ t("webSearchConfig.searchEngine") }}
                </h2>
                <p class="section-desc">
                    {{ t("webSearchConfig.searchEngineDesc") }}
                </p>
                <div class="engine-options">
                    <button
                        v-for="engine in searchEngines"
                        :key="engine.value"
                        @click="selectedEngine = engine.value as SearchEngine"
                        class="engine-option"
                        :class="{
                            'engine-option--selected':
                                selectedEngine === engine.value,
                        }"
                    >
                        <div class="engine-radio">
                            <span
                                class="radio-dot"
                                :class="{
                                    'radio-dot--selected':
                                        selectedEngine === engine.value,
                                }"
                            >
                                <span class="radio-dot-inner"></span>
                            </span>
                        </div>
                        <div class="engine-info">
                            <div class="engine-name">{{ engine.label }}</div>
                            <div class="engine-desc">{{ engine.desc }}</div>
                        </div>
                    </button>
                </div>
            </section>

            <!-- API Configuration -->
            <section class="config-section">
                <h2 class="section-title">
                    {{ t("webSearchConfig.apiConfiguration") }}
                </h2>

                <!-- API Key -->
                <div class="form-field">
                    <label class="input-label">{{
                        t("webSearchConfig.apiKey")
                    }}</label>
                    <input
                        v-model="apiKey"
                        type="password"
                        class="text-input"
                        :placeholder="
                            needsApiKey
                                ? t('webSearchConfig.apiKeyPlaceholder')
                                : t('webSearchConfig.notRequired')
                        "
                        :disabled="!needsApiKey"
                    />
                    <p class="input-hint">
                        <span v-if="selectedEngine === 'tavily'">
                            {{ t("webSearchConfig.getApiKey") }}
                            <a
                                href="https://tavily.com"
                                target="_blank"
                                class="learn-more-link"
                                >{{ t("webSearchConfig.tavilyLink") }}</a
                            >
                        </span>
                        <span v-else-if="selectedEngine === 'bocha'">
                            {{ t("webSearchConfig.getApiKey") }}
                            <a
                                href="https://bocha.io"
                                target="_blank"
                                class="learn-more-link"
                                >{{ t("webSearchConfig.bochaLink") }}</a
                            >
                        </span>
                        <span v-else-if="selectedEngine === 'baidu'">
                            {{ t("webSearchConfig.getApiKey") }}
                            <a
                                href="https://cloud.baidu.com"
                                target="_blank"
                                class="learn-more-link"
                                >{{ t("webSearchConfig.baiduLink") }}</a
                            >
                        </span>
                        <span v-else-if="selectedEngine === 'brave'">
                            {{ t("webSearchConfig.getApiKey") }}
                            <a
                                href="https://brave.com/search/api"
                                target="_blank"
                                class="learn-more-link"
                                >{{ t("webSearchConfig.braveLink") }}</a
                            >
                        </span>
                        <span v-else>{{
                            t("webSearchConfig.duckduckgoFree")
                        }}</span>
                    </p>
                </div>

                <!-- Max Results -->
                <div class="form-field">
                    <label class="input-label">{{
                        t("webSearchConfig.maxResults")
                    }}</label>
                    <input
                        v-model.number="maxResults"
                        type="number"
                        min="1"
                        max="20"
                        class="text-input"
                        placeholder="10"
                    />
                    <p class="input-hint">
                        {{ t("webSearchConfig.maxResultsHint") }}
                    </p>
                </div>
            </section>

            <!-- Save Button -->
            <div class="save-row">
                <button
                    @click="handleSave"
                    class="btn btn-accent"
                    :disabled="webSearchStore.loading || !hasChanges"
                >
                    <svg
                        v-if="webSearchStore.loading"
                        class="btn-icon spin"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <circle cx="12" cy="12" r="10" />
                        <path d="M12 6v6l4 2" />
                    </svg>
                    <svg
                        v-else-if="saveSuccess"
                        class="btn-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                        <path d="M9 12l2 2 4-4" />
                    </svg>
                    <svg
                        v-else
                        class="btn-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path
                            d="M19 21H5a2 2 0 01-2-2V5a2 2 0 012-2h11l5 5v11a2 2 0 01-2 2z"
                        />
                        <polyline points="17 21 17 13 7 13 7 21" />
                        <polyline points="7 3 7 8 15 8" />
                    </svg>
                    {{
                        webSearchStore.loading
                            ? t("webSearchConfig.saving")
                            : saveSuccess
                              ? t("webSearchConfig.saved")
                              : t("webSearchConfig.save")
                    }}
                </button>
                <span v-if="hasChanges" class="change-hint">{{
                    t("webSearchConfig.unsavedChanges")
                }}</span>
            </div>

            <!-- Info Banner -->
            <div class="info-banner">
                <svg
                    class="info-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <circle cx="12" cy="12" r="10" />
                    <line x1="12" y1="16" x2="12" y2="12" />
                    <line x1="12" y1="8" x2="12.01" y2="8" />
                </svg>
                <span>{{ t("webSearchConfig.infoBanner") }}</span>
            </div>
        </template>

        <div class="learn-more">
            <h3 class="learn-more-title">
                {{ t("webSearchConfig.learnMore") }}
            </h3>
            <p class="learn-more-desc">
                {{ t("webSearchConfig.learnMoreDesc") }}
                <a
                    href="https://docs.astrbot.app/use/websearch.html"
                    target="_blank"
                    class="learn-more-link"
                    >{{ t("webSearchConfig.viewDocs") }}</a
                >
            </p>
        </div>
    </div>
</template>

<style scoped>
.page {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
}

.page-header {
    margin-bottom: 2rem;
}

.header-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-text);
    margin-bottom: 0.5rem;
    letter-spacing: -0.01em;
}

.header-desc {
    color: var(--color-text-muted);
    font-size: 0.875rem;
    margin-top: 0.25rem;
}

.config-section {
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1.5rem;
    margin-bottom: 1.5rem;
}

.section-title {
    font-size: 1.0625rem;
    font-weight: 600;
    color: var(--color-text);
    margin-bottom: 0.25rem;
}

.section-desc {
    color: var(--color-text-muted);
    font-size: 0.8125rem;
    margin-bottom: 1rem;
}

.error-banner {
    background-color: var(--color-danger-soft);
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: var(--color-danger);
    padding: 0.75rem 1rem;
    border-radius: var(--radius-md);
    margin-bottom: 1rem;
    font-size: 0.875rem;
}

.success-banner {
    background-color: var(--color-accent-soft);
    border: 1px solid rgba(134, 59, 255, 0.2);
    color: var(--color-accent-hover);
    padding: 0.75rem 1rem;
    border-radius: var(--radius-md);
    margin-bottom: 1rem;
    font-size: 0.875rem;
}

.loading-state {
    text-align: center;
    padding: 3rem;
    color: var(--color-text-muted);
    font-size: 0.875rem;
}

.engine-options {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.engine-option {
    display: flex;
    align-items: flex-start;
    padding: 1rem 1.25rem;
    border: 2px solid var(--color-border);
    border-radius: var(--radius-lg);
    background-color: var(--color-bg-soft);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
    width: 100%;
}

.engine-option:hover {
    border-color: var(--color-primary);
    background-color: var(--color-bg-hover);
}

.engine-option--selected {
    border-color: var(--color-accent);
    background-color: var(--color-accent-soft);
}

.engine-option--selected:hover {
    border-color: var(--color-accent-hover);
}

.engine-radio {
    margin-right: 1rem;
    margin-top: 0.2rem;
}

.radio-dot {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border: 2px solid var(--color-border);
    border-radius: 50%;
    transition: all 0.2s;
    background-color: var(--color-bg);
}

.radio-dot--selected {
    border-color: var(--color-accent);
}

.radio-dot-inner {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--color-accent);
    transition: all 0.2s;
}

.engine-info {
    flex: 1;
}

.engine-name {
    font-weight: 500;
    color: var(--color-text);
    margin-bottom: 0.25rem;
    font-size: 0.9375rem;
}

.engine-desc {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
}

.toggle-row {
    display: flex;
    align-items: center;
    gap: 1rem;
}

.toggle-container {
    display: inline-flex;
    align-items: center;
    cursor: pointer;
}

.toggle-input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
}

.toggle {
    position: relative;
    width: 2.75rem;
    height: 1.5rem;
    background-color: var(--color-bg-soft);
    border: 2px solid var(--color-border);
    border-radius: 1.5rem;
    transition: all 0.2s ease;
    flex-shrink: 0;
}

.toggle:hover {
    border-color: var(--color-primary);
}

.toggle--on {
    background-color: var(--color-accent);
    border-color: var(--color-accent);
}

.toggle--on:hover {
    border-color: var(--color-accent-hover);
    background-color: var(--color-accent-hover);
}

.toggle-thumb {
    position: absolute;
    top: 0.125rem;
    left: 0.125rem;
    width: 1rem;
    height: 1rem;
    background-color: var(--color-text);
    border-radius: 50%;
    transition: transform 0.2s ease;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.toggle--on .toggle-thumb {
    transform: translateX(1.25rem);
}

.toggle-text {
    font-weight: 500;
    color: var(--color-text);
}

.form-field {
    margin-bottom: 1.5rem;
}

.form-field:last-child {
    margin-bottom: 0;
}

.input-label {
    display: block;
    font-weight: 500;
    color: var(--color-text);
    margin-bottom: 0.5rem;
}

.text-input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: 0.9375rem;
    transition: all 0.2s;
    background-color: var(--color-bg);
    color: var(--color-text);
}

.text-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
}

.text-input:disabled {
    background-color: var(--color-bg-mute);
    cursor: not-allowed;
}

.text-input::placeholder {
    color: var(--color-text-muted);
}

.input-hint {
    margin-top: 0.5rem;
    font-size: 0.8125rem;
    color: var(--color-text-muted);
}

.btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: var(--radius-md);
    font-size: 0.9375rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
}

.btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}

.btn-icon {
    width: 18px;
    height: 18px;
}

.spin {
    animation: spin 1s linear infinite;
}

@keyframes spin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

.btn-accent {
    background-color: var(--color-accent);
    color: var(--color-primary-foreground, #fff);
}

.btn-accent:hover:not(:disabled) {
    background-color: var(--color-accent-hover);
}

.save-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.5rem;
}

.change-hint {
    color: var(--color-text-muted);
    font-size: 0.875rem;
}

.info-banner {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    background-color: var(--color-info-soft);
    border: 1px solid var(--color-info);
    padding: 1rem;
    border-radius: var(--radius-md);
    margin-bottom: 1.5rem;
}

.info-icon {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    margin-top: 0.1rem;
    color: var(--color-info-text);
}

.info-banner > span {
    flex: 1;
    font-size: 0.875rem;
    color: var(--color-text);
}

.learn-more {
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1.5rem;
}

.learn-more-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text);
    margin-bottom: 0.75rem;
}

.learn-more-desc {
    color: var(--color-text-muted);
    font-size: 0.875rem;
    line-height: 1.6;
}

.learn-more-link {
    color: var(--color-accent);
    text-decoration: none;
    font-weight: 500;
}

.learn-more-link:hover {
    text-decoration: underline;
}

@media (max-width: 640px) {
    .page {
        padding: 1rem;
    }

    .save-row {
        flex-direction: column;
        align-items: stretch;
    }
}
</style>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from "vue";
import { useI18n } from "vue-i18n";
import type { ModelInfo } from "../types";
import * as api from "../api";

const props = defineProps<{
    modelValue: string;
    providerType: string;
    baseUrl: string;
    apiKey: string;
    placeholder?: string;
}>();

const emit = defineEmits<{
    "update:modelValue": [value: string];
}>();

const { t } = useI18n();

const models = ref<ModelInfo[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const dropdownOpen = ref(false);
const searchQuery = ref("");
const searchInputRef = ref<HTMLInputElement | null>(null);
const selectorRef = ref<HTMLDivElement | null>(null);

const filteredModels = computed(() => {
    const query = searchQuery.value.toLowerCase().trim();
    if (!query) return models.value;
    return models.value.filter(
        (m) =>
            m.id.toLowerCase().includes(query) ||
            (m.name && m.name.toLowerCase().includes(query)),
    );
});

async function fetchModels() {
    if (!props.baseUrl && !props.apiKey) return;
    loading.value = true;
    error.value = null;
    try {
        const res = await api.fetchProviderModels({
            provider_type: props.providerType,
            base_url: props.baseUrl,
            api_key: props.apiKey,
        });
        models.value = res.models;
    } catch (e: unknown) {
        error.value =
            e instanceof Error
                ? e.message
                : t("providers.form.fetchModelsError");
    } finally {
        loading.value = false;
    }
}

function openDropdown() {
    dropdownOpen.value = true;
    nextTick(() => {
        searchInputRef.value?.focus();
    });
}

function closeDropdown() {
    dropdownOpen.value = false;
    searchQuery.value = "";
}

function toggleDropdown() {
    if (dropdownOpen.value) {
        closeDropdown();
    } else {
        openDropdown();
    }
}

async function handleFetchClick() {
    if (loading.value) return;
    await fetchModels();
    if (!dropdownOpen.value) {
        openDropdown();
    }
}

function selectModel(modelId: string) {
    emit("update:modelValue", modelId);
    closeDropdown();
}

function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
        closeDropdown();
    } else if (e.key === "Enter") {
        const first = filteredModels.value[0];
        if (first) {
            selectModel(first.id);
        }
    }
}

// Close dropdown on outside click
function handleClickOutside(e: MouseEvent) {
    const target = e.target as Node;
    const el = selectorRef.value;
    if (el && !el.contains(target)) {
        closeDropdown();
    }
}

onMounted(() => {
    document.addEventListener("mousedown", handleClickOutside);
});

onBeforeUnmount(() => {
    document.removeEventListener("mousedown", handleClickOutside);
});
</script>

<template>
    <div class="form-group">
        <label class="form-label">{{ t("providers.form.defaultModel") }}</label>
        <div class="model-selector" ref="selectorRef">
            <div class="model-input-row">
                <input
                    type="text"
                    class="form-input model-text-input"
                    :value="modelValue"
                    @input="
                        emit(
                            'update:modelValue',
                            ($event.target as HTMLInputElement).value,
                        )
                    "
                    :placeholder="
                        placeholder ||
                        t('providers.form.defaultModelPlaceholder')
                    "
                />
                <button
                    type="button"
                    class="btn-fetch"
                    @click="handleFetchClick"
                    :disabled="loading"
                    :title="t('providers.form.fetchModels')"
                >
                    <svg
                        v-if="loading"
                        class="spin-icon"
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path d="M21 12a9 9 0 1 1-6.219-8.56" />
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
                        <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                    </svg>
                </button>
                <button
                    v-if="models.length > 0"
                    type="button"
                    class="btn-toggle"
                    @click="toggleDropdown"
                    :title="t('providers.form.selectModel')"
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
                        <polyline points="6 9 12 15 18 9" />
                    </svg>
                </button>
            </div>

            <!-- Dropdown -->
            <Transition name="dropdown">
                <div
                    v-if="dropdownOpen && models.length > 0"
                    class="model-dropdown"
                >
                    <div class="model-search">
                        <svg
                            class="search-icon"
                            width="13"
                            height="13"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <circle cx="11" cy="11" r="8" />
                            <line x1="21" y1="21" x2="16.65" y2="16.65" />
                        </svg>
                        <input
                            ref="searchInputRef"
                            v-model="searchQuery"
                            type="text"
                            class="search-input"
                            :placeholder="t('providers.form.searchModels')"
                            @keydown="handleSearchKeydown"
                        />
                        <button
                            v-if="searchQuery"
                            class="search-clear"
                            @click="searchQuery = ''"
                        >
                            <svg
                                width="12"
                                height="12"
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

                    <div class="model-list">
                        <div
                            v-if="filteredModels.length === 0"
                            class="model-list-empty"
                        >
                            {{ t("providers.form.noModelsFound") }}
                        </div>
                        <button
                            v-else
                            v-for="model in filteredModels"
                            :key="model.id"
                            class="model-item"
                            :class="{ active: model.id === modelValue }"
                            @click="selectModel(model.id)"
                        >
                            <span class="model-item-id">{{ model.id }}</span>
                            <span
                                v-if="model.name && model.name !== model.id"
                                class="model-item-name"
                            >
                                {{ model.name }}
                            </span>
                            <svg
                                v-if="model.id === modelValue"
                                class="model-item-check"
                                width="14"
                                height="14"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <polyline points="20 6 9 17 4 12" />
                            </svg>
                        </button>
                    </div>

                    <div class="model-dropdown-footer">
                        <button class="btn-refresh" @click="fetchModels">
                            <svg
                                width="12"
                                height="12"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <polyline points="23 4 23 10 17 10" />
                                <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
                            </svg>
                            {{ t("providers.form.refreshModels") }}
                        </button>
                    </div>
                </div>
            </Transition>

            <!-- Error hint below the input -->
            <p v-if="error" class="model-error-hint">
                {{ error }}
                <button class="btn-retry" @click="handleFetchClick">
                    {{ t("providers.form.retry") }}
                </button>
            </p>
        </div>
    </div>
</template>

<style scoped>
.model-selector {
    position: relative;
}

/* ── Input row ── */
.model-input-row {
    display: flex;
    align-items: stretch;
    gap: 0;
}

.model-text-input {
    flex: 1;
    border-radius: 0.5rem 0 0 0.5rem !important;
    border-right: none !important;
}

.btn-fetch {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 0.5rem;
    border: 1px solid hsl(var(--border) / 0.4);
    border-left: none;
    background: hsl(var(--background) / 0.5);
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s ease;
}

.btn-fetch:last-child {
    border-radius: 0 0.5rem 0.5rem 0;
}

.btn-fetch:not(:last-child) {
    border-right: none;
}

.btn-fetch:hover:not(:disabled) {
    color: hsl(var(--primary));
    background: hsl(var(--primary) / 0.05);
}

.btn-fetch:disabled {
    cursor: not-allowed;
    opacity: 0.6;
}

.btn-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 0.5rem;
    border: 1px solid hsl(var(--border) / 0.4);
    border-left: none;
    border-radius: 0 0.5rem 0.5rem 0;
    background: hsl(var(--background) / 0.5);
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s ease;
}

.btn-toggle:hover {
    color: hsl(var(--foreground));
    background: hsl(var(--background) / 0.8);
}

.spin-icon {
    animation: spin 0.8s linear infinite;
}

@keyframes spin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

/* ── Error hint ── */
.model-error-hint {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0.25rem 0 0;
    font-size: 0.75rem;
    color: hsl(var(--destructive));
}

.btn-retry {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.5rem;
    border: 1px solid hsl(var(--border) / 0.4);
    border-radius: 0.25rem;
    background: none;
    color: hsl(var(--foreground));
    cursor: pointer;
    font-size: 0.6875rem;
    transition: background 0.15s;
}

.btn-retry:hover {
    background: hsl(var(--background) / 0.8);
}

/* ── Dropdown ── */
.model-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 1000;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border) / 0.4);
    border-radius: 0.5rem;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
    overflow: hidden;
}

/* Dropdown transition */
.dropdown-enter-active,
.dropdown-leave-active {
    transition:
        opacity 0.15s ease,
        transform 0.15s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
    opacity: 0;
    transform: translateY(-4px);
}

/* ── Search ── */
.model-search {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 0.625rem;
    border-bottom: 1px solid hsl(var(--border) / 0.2);
}

.search-icon {
    color: hsl(var(--muted-foreground));
    flex-shrink: 0;
}

.search-input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    color: hsl(var(--foreground));
    font-size: 0.8125rem;
}

.search-input::placeholder {
    color: hsl(var(--muted-foreground) / 0.5);
}

.search-clear {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    padding: 0.125rem;
    border-radius: 0.25rem;
}

.search-clear:hover {
    color: hsl(var(--foreground));
    background: hsl(var(--background) / 0.5);
}

/* ── Model list ── */
.model-list {
    overflow-y: auto;
    max-height: 200px;
    padding: 0.25rem;
}

.model-list::-webkit-scrollbar {
    width: 4px;
}

.model-list::-webkit-scrollbar-track {
    background: transparent;
}

.model-list::-webkit-scrollbar-thumb {
    background: hsl(var(--border) / 0.4);
    border-radius: 2px;
}

.model-list-empty {
    padding: 1rem 0.75rem;
    color: hsl(var(--muted-foreground));
    font-size: 0.8125rem;
    text-align: center;
}

.model-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.4375rem 0.625rem;
    background: none;
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    text-align: left;
    color: hsl(var(--foreground));
    font-size: 0.8125rem;
    transition: background 0.12s;
}

.model-item:hover {
    background: hsl(var(--background) / 0.8);
}

.model-item.active {
    background: hsl(var(--primary) / 0.08);
}

.model-item-id {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
}

.model-item-name {
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
    white-space: nowrap;
    flex-shrink: 0;
}

.model-item-check {
    color: hsl(var(--primary));
    flex-shrink: 0;
}

/* ── Footer ── */
.model-dropdown-footer {
    padding: 0.375rem 0.5rem;
    border-top: 1px solid hsl(var(--border) / 0.2);
}

.btn-refresh {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    background: none;
    border: none;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    padding: 0.25rem 0.5rem;
    border-radius: 0.375rem;
    font-size: 0.75rem;
    width: 100%;
    transition: all 0.15s;
}

.btn-refresh:hover {
    color: hsl(var(--foreground));
    background: hsl(var(--background) / 0.5);
}
</style>

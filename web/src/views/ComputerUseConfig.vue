<script setup lang="ts">
import { onMounted, ref, computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useComputerUseStore } from "../stores/computerUse";
import type { SandboxConfig, SandboxDriver } from "../types";

const { t } = useI18n();
const computerUseStore = useComputerUseStore();

const selectedRuntime = ref<string>("none");
const requireAdmin = ref(true);
const adminIds = ref<string[]>([]);
const allowedPaths = ref<string[]>([]);
const newAdminId = ref("");
const newAllowedPath = ref("");

// Sandbox config with default values
const sandboxConfig = ref<SandboxConfig>({
    driver: "shipyard_neo",
    endpoint: undefined,
    access_token: undefined,
    profile: undefined,
    ttl_secs: 3600,
    enable_browser: false,
    // CUA defaults
    cua_image: "linux",
    cua_os_type: "linux",
    cua_sandbox_ttl: 3600,
    cua_telemetry_enabled: false,
    cua_local_runtime: true,
    cua_api_key: undefined,
});

const saveSuccess = ref(false);
const saveError = ref<string | null>(null);

onMounted(async () => {
    await computerUseStore.fetchConfig();
    syncFromStore();
});

function syncFromStore() {
    if (computerUseStore.config) {
        selectedRuntime.value = computerUseStore.config.runtime;
        requireAdmin.value = computerUseStore.config.require_admin;
        adminIds.value = [...computerUseStore.config.admin_ids];
        allowedPaths.value = [...computerUseStore.config.allowed_paths];
        // Sync sandbox config if exists
        if (computerUseStore.config.sandbox_config) {
            sandboxConfig.value = { ...computerUseStore.config.sandbox_config };
        }
    }
}

// Watch for runtime changes to provide default sandbox config
watch(selectedRuntime, (newRuntime) => {
    if (newRuntime === "sandbox" && !sandboxConfig.value.driver) {
        sandboxConfig.value = {
            driver: "shipyard_neo",
            endpoint: undefined,
            access_token: undefined,
            profile: undefined,
            ttl_secs: 3600,
            enable_browser: false,
            cua_image: "linux",
            cua_os_type: "linux",
            cua_sandbox_ttl: 3600,
            cua_telemetry_enabled: false,
            cua_local_runtime: true,
            cua_api_key: undefined,
        };
    }
    clearMessages();
});

function clearMessages() {
    saveSuccess.value = false;
    saveError.value = null;
}

function addAdminId() {
    if (
        newAdminId.value.trim() &&
        !adminIds.value.includes(newAdminId.value.trim())
    ) {
        adminIds.value.push(newAdminId.value.trim());
        newAdminId.value = "";
        clearMessages();
    }
}

function removeAdminId(id: string) {
    const idx = adminIds.value.indexOf(id);
    if (idx !== -1) {
        adminIds.value.splice(idx, 1);
        clearMessages();
    }
}

function addAllowedPath() {
    if (
        newAllowedPath.value.trim() &&
        !allowedPaths.value.includes(newAllowedPath.value.trim())
    ) {
        allowedPaths.value.push(newAllowedPath.value.trim());
        newAllowedPath.value = "";
        clearMessages();
    }
}

function removeAllowedPath(path: string) {
    const idx = allowedPaths.value.indexOf(path);
    if (idx !== -1) {
        allowedPaths.value.splice(idx, 1);
        clearMessages();
    }
}

async function handleSave() {
    clearMessages();
    try {
        await computerUseStore.updateConfig({
            runtime: selectedRuntime.value as "none" | "local" | "sandbox",
            require_admin: requireAdmin.value,
            admin_ids: adminIds.value,
            allowed_paths: allowedPaths.value,
            // Include sandbox config only if runtime is sandbox
            sandbox_config:
                selectedRuntime.value === "sandbox"
                    ? sandboxConfig.value
                    : undefined,
        });
        saveSuccess.value = true;
        setTimeout(() => {
            saveSuccess.value = false;
        }, 3000);
    } catch (e: unknown) {
        saveError.value = e instanceof Error ? e.message : t("errors.unknown");
    }
}

const hasChanges = computed(() => {
    if (!computerUseStore.config) return false;
    const sandboxChanged =
        selectedRuntime.value === "sandbox" &&
        JSON.stringify(sandboxConfig.value) !==
            JSON.stringify(computerUseStore.config.sandbox_config);
    return (
        selectedRuntime.value !== computerUseStore.config.runtime ||
        requireAdmin.value !== computerUseStore.config.require_admin ||
        JSON.stringify(adminIds.value) !==
            JSON.stringify(computerUseStore.config.admin_ids) ||
        JSON.stringify(allowedPaths.value) !==
            JSON.stringify(computerUseStore.config.allowed_paths) ||
        sandboxChanged
    );
});
</script>

<template>
    <div class="page">
        <!-- Header -->
        <div class="page-header">
            <div class="header-info">
                <h1 class="header-title">{{ t("computerUseConfig.title") }}</h1>
                <p class="header-desc">{{ t("computerUseConfig.subtitle") }}</p>
            </div>
        </div>

        <!-- Error/Success Messages -->
        <div v-if="computerUseStore.error" class="error-banner">
            {{ computerUseStore.error }}
        </div>

        <div v-if="saveSuccess" class="success-banner">
            {{ t("computerUseConfig.saveSuccess") }}
        </div>

        <div v-if="saveError" class="error-banner">
            {{ saveError }}
        </div>

        <!-- Loading State -->
        <div
            v-if="computerUseStore.loading && !computerUseStore.config"
            class="loading-state"
        >
            {{ t("common.loading") }}
        </div>

        <!-- Configuration Form -->
        <template v-else-if="computerUseStore.config">
            <!-- Runtime Selection -->
            <section class="config-section">
                <h2 class="section-title">
                    {{ t("computerUseConfig.runtime") }}
                </h2>
                <p class="section-desc">
                    {{ t("computerUseConfig.runtimeDesc[selectedRuntime]") }}
                </p>

                <div class="runtime-options">
                    <button
                        v-for="runtime in ['none', 'local', 'sandbox']"
                        :key="runtime"
                        @click="selectedRuntime = runtime"
                        :class="[
                            'runtime-option',
                            {
                                'runtime-option--selected':
                                    selectedRuntime === runtime,
                            },
                        ]"
                    >
                        <div class="runtime-radio">
                            <span
                                :class="[
                                    'radio-dot',
                                    {
                                        'radio-dot--selected':
                                            selectedRuntime === runtime,
                                    },
                                ]"
                            >
                                <span
                                    v-if="selectedRuntime === runtime"
                                    class="radio-dot-inner"
                                ></span>
                            </span>
                        </div>
                        <div class="runtime-info">
                            <div class="runtime-name">
                                {{
                                    t(
                                        `computerUseConfig.runtime${runtime.charAt(0).toUpperCase() + runtime.slice(1)}`,
                                    )
                                }}
                            </div>
                        </div>
                    </button>
                </div>
            </section>

            <!-- Admin Configuration -->
            <section v-if="selectedRuntime !== 'none'" class="config-section">
                <h2 class="section-title">
                    {{ t("computerUseConfig.requireAdmin") }}
                </h2>
                <p class="section-desc">
                    {{ t("computerUseConfig.requireAdminDesc") }}
                </p>

                <div class="toggle-row">
                    <label class="toggle-container">
                        <input
                            type="checkbox"
                            v-model="requireAdmin"
                            class="toggle-input"
                            @change="clearMessages"
                        />
                        <span
                            :class="['toggle', { 'toggle--on': requireAdmin }]"
                        >
                            <span class="toggle-thumb"></span>
                        </span>
                    </label>
                </div>

                <!-- Admin IDs -->
                <div class="admin-section">
                    <label class="input-label">{{
                        t("computerUseConfig.adminIds")
                    }}</label>
                    <div class="input-row">
                        <input
                            v-model="newAdminId"
                            type="text"
                            :placeholder="t('computerUseConfig.adminIdsDesc')"
                            class="text-input"
                            @keyup.enter="addAdminId"
                        />
                        <button @click="addAdminId" class="btn btn-secondary">
                            {{ t("common.add") }}
                        </button>
                    </div>
                    <div class="tag-list">
                        <span
                            v-for="id in adminIds"
                            :key="id"
                            class="tag tag-primary"
                        >
                            {{ id }}
                            <button
                                @click="removeAdminId(id)"
                                class="tag-remove"
                            >
                                ×
                            </button>
                        </span>
                    </div>
                </div>

                <!-- Allowed Paths -->
                <div class="admin-section">
                    <label class="input-label">{{
                        t("computerUseConfig.allowedPaths")
                    }}</label>
                    <div class="input-row">
                        <input
                            v-model="newAllowedPath"
                            type="text"
                            :placeholder="
                                t('computerUseConfig.allowedPathsDesc')
                            "
                            class="text-input"
                            @keyup.enter="addAllowedPath"
                        />
                        <button
                            @click="addAllowedPath"
                            class="btn btn-secondary"
                        >
                            {{ t("common.add") }}
                        </button>
                    </div>
                    <div class="tag-list">
                        <span
                            v-for="path in allowedPaths"
                            :key="path"
                            class="tag tag-secondary"
                        >
                            {{ path }}
                            <button
                                @click="removeAllowedPath(path)"
                                class="tag-remove"
                            >
                                ×
                            </button>
                        </span>
                    </div>
                </div>
            </section>

            <!-- Sandbox Configuration (only when runtime is sandbox) -->
            <section
                v-if="selectedRuntime === 'sandbox'"
                class="config-section"
            >
                <h2 class="section-title">
                    {{ t("computerUseConfig.sandboxConfig") }}
                </h2>

                <!-- Driver Selection -->
                <div class="driver-selection">
                    <label class="input-label">{{
                        t("computerUseConfig.sandboxDriver")
                    }}</label>
                    <div class="driver-options">
                        <button
                            v-for="driver in ['shipyard_neo', 'cua']"
                            :key="driver"
                            @click="
                                sandboxConfig.driver = driver as SandboxDriver;
                                clearMessages();
                            "
                            :class="[
                                'driver-option',
                                {
                                    'driver-option--selected':
                                        sandboxConfig.driver === driver,
                                },
                            ]"
                        >
                            <div class="driver-radio">
                                <span
                                    :class="[
                                        'radio-dot',
                                        {
                                            'radio-dot--selected':
                                                sandboxConfig.driver === driver,
                                        },
                                    ]"
                                >
                                    <span
                                        v-if="sandboxConfig.driver === driver"
                                        class="radio-dot-inner"
                                    ></span>
                                </span>
                            </div>
                            <div class="driver-info">
                                <div class="driver-name">
                                    {{
                                        driver === "shipyard_neo"
                                            ? "Shipyard Neo"
                                            : "CUA"
                                    }}
                                </div>
                                <div class="driver-desc">
                                    {{
                                        t(
                                            `computerUseConfig.sandboxDriverDesc.${driver}`,
                                        )
                                    }}
                                </div>
                            </div>
                        </button>
                    </div>
                </div>

                <!-- Shipyard Neo Configuration -->
                <div
                    v-if="sandboxConfig.driver === 'shipyard_neo'"
                    class="driver-config"
                >
                    <h3 class="config-subtitle">
                        {{ t("computerUseConfig.shipyardNeoTitle") }}
                    </h3>

                    <div class="form-field">
                        <label class="input-label">{{
                            t("computerUseConfig.shipyardEndpoint")
                        }}</label>
                        <input
                            v-model="sandboxConfig.endpoint"
                            type="text"
                            :placeholder="
                                t(
                                    'computerUseConfig.shipyardEndpointPlaceholder',
                                )
                            "
                            class="text-input"
                            @change="clearMessages"
                        />
                        <p class="input-hint">
                            {{ t("computerUseConfig.shipyardEndpointDesc") }}
                        </p>
                    </div>

                    <div class="form-field">
                        <label class="input-label">{{
                            t("computerUseConfig.shipyardAccessToken")
                        }}</label>
                        <input
                            v-model="sandboxConfig.access_token"
                            type="password"
                            :placeholder="
                                t(
                                    'computerUseConfig.shipyardAccessTokenPlaceholder',
                                )
                            "
                            class="text-input"
                            @change="clearMessages"
                        />
                        <p class="input-hint">
                            {{ t("computerUseConfig.shipyardAccessTokenDesc") }}
                        </p>
                    </div>

                    <div class="form-field">
                        <label class="input-label">{{
                            t("computerUseConfig.shipyardProfile")
                        }}</label>
                        <input
                            v-model="sandboxConfig.profile"
                            type="text"
                            :placeholder="
                                t(
                                    'computerUseConfig.shipyardProfilePlaceholder',
                                )
                            "
                            class="text-input"
                            @change="clearMessages"
                        />
                        <p class="input-hint">
                            {{ t("computerUseConfig.shipyardProfileDesc") }}
                        </p>
                    </div>

                    <div class="form-field">
                        <label class="input-label">{{
                            t("computerUseConfig.shipyardTtl")
                        }}</label>
                        <input
                            v-model.number="sandboxConfig.ttl_secs"
                            type="number"
                            min="60"
                            max="86400"
                            placeholder="3600"
                            class="text-input"
                            @change="clearMessages"
                        />
                        <p class="input-hint">
                            {{ t("computerUseConfig.shipyardTtlDesc") }}
                        </p>
                    </div>
                </div>

                <!-- CUA Configuration -->
                <div
                    v-if="sandboxConfig.driver === 'cua'"
                    class="driver-config"
                >
                    <h3 class="config-subtitle">
                        {{ t("computerUseConfig.cuaTitle") }}
                    </h3>

                    <div class="form-field">
                        <label class="input-label">{{
                            t("computerUseConfig.cuaImage")
                        }}</label>
                        <input
                            v-model="sandboxConfig.cua_image"
                            type="text"
                            :placeholder="
                                t('computerUseConfig.cuaImagePlaceholder')
                            "
                            class="text-input"
                            @change="clearMessages"
                        />
                        <p class="input-hint">
                            {{ t("computerUseConfig.cuaImageDesc") }}
                        </p>
                    </div>

                    <div class="form-field">
                        <label class="input-label">{{
                            t("computerUseConfig.cuaOsType")
                        }}</label>
                        <input
                            v-model="sandboxConfig.cua_os_type"
                            type="text"
                            :placeholder="
                                t('computerUseConfig.cuaOsTypePlaceholder')
                            "
                            class="text-input"
                            @change="clearMessages"
                        />
                        <p class="input-hint">
                            {{ t("computerUseConfig.cuaOsTypeDesc") }}
                        </p>
                    </div>

                    <div class="form-field">
                        <label class="input-label">{{
                            t("computerUseConfig.cuaTtl")
                        }}</label>
                        <input
                            v-model.number="sandboxConfig.cua_sandbox_ttl"
                            type="number"
                            min="60"
                            max="86400"
                            placeholder="3600"
                            class="text-input"
                            @change="clearMessages"
                        />
                        <p class="input-hint">
                            {{ t("computerUseConfig.cuaTtlDesc") }}
                        </p>
                    </div>

                    <div class="form-field">
                        <label class="toggle-label">
                            <input
                                type="checkbox"
                                v-model="sandboxConfig.cua_local_runtime"
                                class="toggle-input"
                                @change="clearMessages"
                            />
                            <span
                                :class="[
                                    'toggle',
                                    {
                                        'toggle--on':
                                            sandboxConfig.cua_local_runtime,
                                    },
                                ]"
                            >
                                <span class="toggle-thumb"></span>
                            </span>
                            <span class="toggle-text">{{
                                t("computerUseConfig.cuaLocalRuntime")
                            }}</span>
                        </label>
                        <p class="input-hint">
                            {{ t("computerUseConfig.cuaLocalRuntimeDesc") }}
                        </p>
                    </div>

                    <div class="form-field">
                        <label class="toggle-label">
                            <input
                                type="checkbox"
                                v-model="sandboxConfig.cua_telemetry_enabled"
                                class="toggle-input"
                                @change="clearMessages"
                            />
                            <span
                                :class="[
                                    'toggle',
                                    {
                                        'toggle--on':
                                            sandboxConfig.cua_telemetry_enabled,
                                    },
                                ]"
                            >
                                <span class="toggle-thumb"></span>
                            </span>
                            <span class="toggle-text">{{
                                t("computerUseConfig.cuaTelemetry")
                            }}</span>
                        </label>
                        <p class="input-hint">
                            {{ t("computerUseConfig.cuaTelemetryDesc") }}
                        </p>
                    </div>

                    <div
                        v-if="!sandboxConfig.cua_local_runtime"
                        class="form-field"
                    >
                        <label class="input-label">{{
                            t("computerUseConfig.cuaApiKey")
                        }}</label>
                        <input
                            v-model="sandboxConfig.cua_api_key"
                            type="password"
                            :placeholder="
                                t('computerUseConfig.cuaApiKeyPlaceholder')
                            "
                            class="text-input"
                            @change="clearMessages"
                        />
                        <p class="input-hint">
                            {{ t("computerUseConfig.cuaApiKeyDesc") }}
                        </p>
                    </div>
                </div>

                <!-- Enable Browser (common) -->
                <div class="form-field">
                    <label class="toggle-label">
                        <input
                            type="checkbox"
                            v-model="sandboxConfig.enable_browser"
                            class="toggle-input"
                            @change="clearMessages"
                        />
                        <span
                            :class="[
                                'toggle',
                                { 'toggle--on': sandboxConfig.enable_browser },
                            ]"
                        >
                            <span class="toggle-thumb"></span>
                        </span>
                        <span class="toggle-text">{{
                            t("computerUseConfig.enableBrowser")
                        }}</span>
                    </label>
                    <p class="input-hint">
                        {{ t("computerUseConfig.enableBrowserDesc") }}
                    </p>
                </div>
            </section>

            <!-- Save Button -->
            <div class="save-row">
                <button
                    @click="handleSave"
                    :disabled="!hasChanges || computerUseStore.loading"
                    class="btn btn-accent"
                >
                    <svg
                        v-if="computerUseStore.loading"
                        class="btn-icon spin"
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
                    {{ t("computerUseConfig.save") }}
                </button>
                <span v-if="hasChanges" class="change-hint">{{
                    t("computerUseConfig.unsavedChanges")
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
                <span>{{ t("computerUseConfig.infoBanner") }}</span>
            </div>
        </template>

        <!-- Learn More Section -->
        <div class="learn-more">
            <h3 class="learn-more-title">
                {{ t("computerUseConfig.learnMore") }}
            </h3>
            <p class="learn-more-desc">
                {{ t("computerUseConfig.learnMoreDesc") }}
                <a
                    href="https://docs.astrbot.app/use/astrbot-agent-sandbox.html"
                    target="_blank"
                    class="learn-more-link"
                >
                    {{ t("computerUseConfig.viewDocs") }}
                </a>
            </p>
        </div>
    </div>
</template>

<style scoped>
.page {
    padding: 1.5rem;
    max-width: 56rem;
    margin: 0 auto;
    animation: fadeIn var(--transition-normal) ease-out;
}

/* Header */
.page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.5rem;
}

.header-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-text);
    letter-spacing: -0.01em;
}

.header-desc {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    margin-top: 0.25rem;
}

/* Section */
.config-section {
    margin-bottom: 2rem;
}

.section-title {
    font-size: 1.0625rem;
    font-weight: 600;
    color: var(--color-text);
    margin-bottom: 0.25rem;
}

.section-desc {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    margin-bottom: 1rem;
}

.config-subtitle {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text);
    margin-bottom: 1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--color-border);
}

/* Error & Success */
.error-banner {
    margin-bottom: 1rem;
    padding: 0.75rem 1rem;
    background-color: var(--color-danger-soft);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    color: var(--color-danger);
}

.success-banner {
    margin-bottom: 1rem;
    padding: 0.75rem 1rem;
    background-color: var(--color-accent-soft);
    border: 1px solid rgba(134, 59, 255, 0.2);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    color: var(--color-accent-hover);
}

/* Loading */
.loading-state {
    text-align: center;
    padding: 3rem 0;
    color: var(--color-text-muted);
    font-size: 0.875rem;
}

/* Runtime Options */
.runtime-options {
    display: grid;
    gap: 0.5rem;
}

.runtime-option {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    background-color: var(--color-bg-soft);
    border: 2px solid var(--color-border);
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
    width: 100%;
}

.runtime-option:hover {
    border-color: var(--color-primary);
    background-color: var(--color-bg-soft-hover);
}

.runtime-option--selected {
    border-color: var(--color-accent);
    background-color: var(--color-bg-soft-hover);
}

.runtime-option--selected:hover {
    border-color: var(--color-accent);
}

/* Radio */
.runtime-radio {
    flex-shrink: 0;
    padding-top: 0.125rem;
}

.radio-dot {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    border: 2px solid var(--color-border);
    border-radius: 50%;
    transition: all 0.2s ease;
    background-color: var(--color-bg);
}

.radio-dot--selected {
    border-color: var(--color-accent);
}

.radio-dot-inner {
    width: 0.625rem;
    height: 0.625rem;
    border-radius: 50%;
    background-color: var(--color-accent);
}

.runtime-info {
    flex: 1;
}

.runtime-name {
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--color-text);
}

/* Toggle */
.toggle-row {
    margin-bottom: 1.5rem;
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
    display: inline-flex;
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

/* Admin Section */
.admin-section {
    margin-top: 1.5rem;
}

.input-label {
    display: block;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-text-secondary);
    margin-bottom: 0.5rem;
}

.input-row {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
}

.text-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    font-size: 0.875rem;
    background-color: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    color: var(--color-text);
    transition: border-color 0.2s ease;
}

.text-input:focus {
    outline: none;
    border-color: var(--color-primary);
}

.text-input::placeholder {
    color: var(--color-text-muted);
}

.input-hint {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    margin-top: 0.25rem;
}

/* Tags */
.tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
}

.tag {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.75rem;
    border-radius: var(--radius-full);
    font-size: 0.875rem;
    font-weight: 500;
}

.tag-primary {
    background-color: var(--color-accent-soft);
    color: var(--color-accent-hover);
}

.tag-secondary {
    background-color: var(--color-bg-soft);
    color: var(--color-text-secondary);
}

.tag-remove {
    background: none;
    border: none;
    font-size: 1.25rem;
    line-height: 1;
    cursor: pointer;
    opacity: 0.6;
    transition: opacity 0.2s ease;
    padding: 0;
    color: inherit;
}

.tag-remove:hover {
    opacity: 1;
}

/* Driver Selection */
.driver-selection {
    margin-bottom: 1.5rem;
}

.driver-options {
    display: grid;
    gap: 0.5rem;
}

.driver-option {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    background-color: var(--color-bg-soft);
    border: 2px solid var(--color-border);
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
    width: 100%;
}

.driver-option:hover {
    border-color: var(--color-primary);
    background-color: var(--color-bg-soft-hover);
}

.driver-option--selected {
    border-color: var(--color-accent);
    background-color: var(--color-bg-soft-hover);
}

.driver-option--selected:hover {
    border-color: var(--color-accent);
}

.driver-radio {
    flex-shrink: 0;
    padding-top: 0.125rem;
}

.driver-info {
    flex: 1;
}

.driver-name {
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--color-text);
    margin-bottom: 0.25rem;
}

.driver-desc {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
}

.driver-config {
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
    margin-bottom: 1.5rem;
}

/* Form Field */
.form-field {
    margin-bottom: 1.25rem;
}

.form-field:last-child {
    margin-bottom: 0;
}

.toggle-label {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    cursor: pointer;
}

.toggle-text {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-text);
}

/* Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    font-weight: 500;
    border-radius: var(--radius-md);
    border: none;
    cursor: pointer;
    transition: all 0.2s ease;
}

.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-icon {
    width: 1rem;
    height: 1rem;
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

.btn-secondary {
    background-color: var(--color-bg-soft);
    color: var(--color-text);
    border: 1px solid var(--color-border);
}

.btn-secondary:hover:not(:disabled) {
    background-color: var(--color-bg-soft-hover);
    border-color: var(--color-primary);
}

/* Save Row */
.save-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding-top: 1rem;
    margin-bottom: 1.5rem;
}

.btn-accent {
    background-color: var(--color-accent);
    color: white;
    font-weight: 600;
    padding: 0.625rem 1.5rem;
}

.btn-accent:hover:not(:disabled) {
    background-color: var(--color-accent-hover);
}

.change-hint {
    font-size: 0.875rem;
    color: var(--color-accent-hover);
    font-weight: 500;
}

/* Info Banner */
.info-banner {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 1rem;
    background-color: var(--color-primary-soft);
    border: 1px solid rgba(59, 130, 246, 0.2);
    border-radius: var(--radius-lg);
    margin-bottom: 1.5rem;
}

.info-icon {
    width: 1.25rem;
    height: 1.25rem;
    color: var(--color-primary);
    flex-shrink: 0;
    margin-top: 0.0625rem;
}

.info-banner > span {
    font-size: 0.875rem;
    color: var(--color-text-secondary);
    line-height: 1.5;
}

/* Learn More */
.learn-more {
    margin-top: 2rem;
    padding: 1rem 1.25rem;
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
}

.learn-more-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--color-text);
    margin-bottom: 0.5rem;
}

.learn-more-desc {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    line-height: 1.6;
}

.learn-more-link {
    color: var(--color-accent);
    text-decoration: underline;
    transition: color 0.2s ease;
}

.learn-more-link:hover {
    color: var(--color-accent-hover);
}

/* Responsive */
@media (max-width: 640px) {
    .page {
        padding: 1rem;
    }

    .input-row {
        flex-direction: column;
    }

    .btn-secondary {
        width: 100%;
    }
}
</style>

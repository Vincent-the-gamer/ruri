<script setup lang="ts">
import { onMounted, ref, computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useComputerUseStore } from "../stores/computerUse";
import type { AioSandboxConfig } from "../types";

const { t } = useI18n();
const computerUseStore = useComputerUseStore();

const selectedRuntime = ref<string>("none");

const runtimeLabelKeys: Record<string, string> = {
    none: "computerUseConfig.runtimeNone",
    local: "computerUseConfig.runtimeLocal",
    aio_sandbox: "computerUseConfig.runtimeAioSandbox",
};
const requireAdmin = ref(true);
const adminIds = ref<string[]>([]);
const allowedPaths = ref<string[]>([]);
const newAdminId = ref("");
const newAllowedPath = ref("");

const aioSandboxConfig = ref<AioSandboxConfig>({
    endpoint: "http://localhost:8080",
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
        if (computerUseStore.config.aio_sandbox_config) {
            aioSandboxConfig.value = {
                ...computerUseStore.config.aio_sandbox_config,
            };
        }
    }
}

// Watch for runtime changes to provide default aio sandbox config
watch(selectedRuntime, (newRuntime) => {
    if (newRuntime === "aio_sandbox" && !aioSandboxConfig.value.endpoint) {
        aioSandboxConfig.value = {
            endpoint: "http://localhost:8080",
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
            runtime: selectedRuntime.value as "none" | "local" | "aio_sandbox",
            require_admin: requireAdmin.value,
            admin_ids: adminIds.value,
            allowed_paths: allowedPaths.value,
            // Include aio sandbox config only if runtime is aio_sandbox
            aio_sandbox_config:
                selectedRuntime.value === "aio_sandbox"
                    ? aioSandboxConfig.value
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
    const aioSandboxChanged =
        selectedRuntime.value === "aio_sandbox" &&
        JSON.stringify(aioSandboxConfig.value) !==
            JSON.stringify(computerUseStore.config.aio_sandbox_config);
    return (
        selectedRuntime.value !== computerUseStore.config.runtime ||
        requireAdmin.value !== computerUseStore.config.require_admin ||
        JSON.stringify(adminIds.value) !==
            JSON.stringify(computerUseStore.config.admin_ids) ||
        JSON.stringify(allowedPaths.value) !==
            JSON.stringify(computerUseStore.config.allowed_paths) ||
        aioSandboxChanged
    );
});
</script>

<template>
    <div class="page">
        <!-- Header -->
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
                        <rect width="20" height="14" x="2" y="3" rx="2" />
                        <line x1="8" x2="16" y1="21" y2="21" />
                        <line x1="12" x2="12" y1="17" y2="21" />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">
                        {{ t("computerUseConfig.title") }}
                    </h1>
                    <p class="header-desc">
                        {{ t("computerUseConfig.subtitle") }}
                    </p>
                </div>
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
                        v-for="runtime in ['none', 'local', 'aio_sandbox']"
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
                                {{ t(runtimeLabelKeys[runtime]) }}
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

            <!-- AIO Sandbox Configuration (only when runtime is aio_sandbox) -->
            <section
                v-if="selectedRuntime === 'aio_sandbox'"
                class="config-section"
            >
                <h2 class="section-title">
                    {{ t("computerUseConfig.aioSandboxConfig") }}
                </h2>

                <div class="form-field">
                    <label class="input-label">{{
                        t("computerUseConfig.aioSandboxEndpoint")
                    }}</label>
                    <input
                        v-model="aioSandboxConfig.endpoint"
                        type="text"
                        :placeholder="
                            t('computerUseConfig.aioSandboxEndpointPlaceholder')
                        "
                        class="text-input"
                        @change="clearMessages"
                    />
                    <p class="input-hint">
                        {{ t("computerUseConfig.aioSandboxEndpointDesc") }}
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
                    href="https://ruri.vince-g.xyz/computer-use"
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

    .page-header {
        flex-direction: column;
        align-items: flex-start;
    }

    .input-row {
        flex-direction: column;
    }

    .btn-secondary {
        width: 100%;
    }
}
</style>

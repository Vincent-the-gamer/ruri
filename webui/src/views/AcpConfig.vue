<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useAcpStore } from "../stores/acp";
import { useKnowledgeBaseStore } from "../stores/knowledgeBase";

const { t } = useI18n();
const acpStore = useAcpStore();
const kbStore = useKnowledgeBaseStore();

const selectedProviderId = ref<string | null>(null);
const selectedSkillNames = ref<string[]>([]);
const selectedKbIds = ref<string[]>([]);
const saveSuccess = ref(false);
const saveError = ref<string | null>(null);

onMounted(async () => {
    await Promise.all([acpStore.fetchConfig(), kbStore.fetchKnowledgeBases()]);
    syncFromStore();
});

function syncFromStore() {
    if (acpStore.config) {
        selectedProviderId.value = acpStore.config.active_provider_id;
        selectedSkillNames.value = [...acpStore.config.active_skill_names];
        selectedKbIds.value = [
            ...(acpStore.config.active_knowledge_base_ids || []),
        ];
    }
}

function selectProvider(id: string) {
    selectedProviderId.value = id;
    clearMessages();
}

function toggleSkill(name: string) {
    const idx = selectedSkillNames.value.indexOf(name);
    if (idx === -1) {
        selectedSkillNames.value.push(name);
    } else {
        selectedSkillNames.value.splice(idx, 1);
    }
    clearMessages();
}

function isSkillSelected(name: string): boolean {
    return selectedSkillNames.value.includes(name);
}

function clearMessages() {
    saveSuccess.value = false;
    saveError.value = null;
}

function toggleKb(id: string) {
    const idx = selectedKbIds.value.indexOf(id);
    if (idx === -1) {
        selectedKbIds.value.push(id);
    } else {
        selectedKbIds.value.splice(idx, 1);
    }
    clearMessages();
}

function isKbSelected(id: string): boolean {
    return selectedKbIds.value.includes(id);
}

async function handleSave() {
    clearMessages();
    try {
        await acpStore.updateConfig({
            active_provider_id: selectedProviderId.value,
            active_skill_names: selectedSkillNames.value,
            active_knowledge_base_ids: selectedKbIds.value,
        });
        saveSuccess.value = true;
        setTimeout(() => {
            saveSuccess.value = false;
        }, 3000);
    } catch (e: unknown) {
        saveError.value = e instanceof Error ? e.message : t("errors.unknown");
    }
}

const providerTypeLabel = (type: string) => {
    switch (type) {
        case "openai":
            return "OpenAI";
        case "anthropic":
            return "Anthropic";
        case "custom":
            return "Custom";
        default:
            return type;
    }
};

const hasChanges = computed(() => {
    if (!acpStore.config) return false;
    return (
        selectedProviderId.value !== acpStore.config.active_provider_id ||
        JSON.stringify(selectedSkillNames.value) !==
            JSON.stringify(acpStore.config.active_skill_names) ||
        JSON.stringify(selectedKbIds.value) !==
            JSON.stringify(acpStore.config.active_knowledge_base_ids || [])
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
                        <path d="M12 8V4H8" />
                        <rect width="16" height="12" x="4" y="8" rx="2" />
                        <path d="M2 14h2" />
                        <path d="M20 14h2" />
                        <path d="M15 13v2" />
                        <path d="M9 13v2" />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">{{ t("acpConfig.title") }}</h1>
                    <p class="header-desc">{{ t("acpConfig.subtitle") }}</p>
                </div>
            </div>
        </div>

        <!-- Error -->
        <div v-if="acpStore.error" class="error-banner">
            {{ acpStore.error }}
        </div>

        <!-- Success -->
        <div v-if="saveSuccess" class="success-banner">
            ✅ {{ t("acpConfig.saveSuccess") }}
        </div>

        <!-- Save Error -->
        <div v-if="saveError" class="error-banner">
            {{ saveError }}
        </div>

        <!-- Loading -->
        <div v-if="acpStore.loading && !acpStore.config" class="loading-state">
            {{ t("common.loading") }}
        </div>

        <template v-else-if="acpStore.config">
            <!-- Provider Section -->
            <section class="config-section">
                <h2 class="section-title">{{ t("acpConfig.providers") }}</h2>
                <p class="section-desc">{{ t("acpConfig.selectProvider") }}</p>

                <div
                    v-if="acpStore.config.available_providers.length === 0"
                    class="empty-section"
                >
                    <p class="empty-section-text">
                        {{ t("acpConfig.noProviders") }}
                    </p>
                    <p class="empty-section-hint">
                        {{ t("acpConfig.addProviderHint") }}
                    </p>
                </div>

                <div v-else class="provider-list">
                    <button
                        v-for="provider in acpStore.config.available_providers"
                        :key="provider.id"
                        class="provider-option"
                        :class="{
                            'provider-option--selected':
                                selectedProviderId === provider.id,
                        }"
                        @click="selectProvider(provider.id)"
                    >
                        <div class="provider-radio">
                            <span
                                class="radio-dot"
                                :class="{
                                    'radio-dot--selected':
                                        selectedProviderId === provider.id,
                                }"
                            >
                                <span
                                    v-if="selectedProviderId === provider.id"
                                    class="radio-dot-inner"
                                ></span>
                            </span>
                        </div>
                        <div class="provider-info">
                            <div class="provider-name">{{ provider.name }}</div>
                            <div class="provider-meta">
                                <span
                                    class="type-badge"
                                    :class="`type-badge--${provider.provider_type}`"
                                >
                                    {{
                                        providerTypeLabel(
                                            provider.provider_type,
                                        )
                                    }}
                                </span>
                                <span class="provider-model">{{
                                    provider.default_model
                                }}</span>
                            </div>
                        </div>
                    </button>
                </div>
            </section>

            <!-- Skills Section -->
            <section class="config-section">
                <h2 class="section-title">{{ t("acpConfig.skills") }}</h2>
                <p class="section-desc">{{ t("acpConfig.selectSkills") }}</p>

                <div
                    v-if="acpStore.config.available_skills.length === 0"
                    class="empty-section"
                >
                    <p class="empty-section-text">
                        {{ t("acpConfig.noSkills") }}
                    </p>
                    <p class="empty-section-hint">
                        {{ t("acpConfig.addSkillHint") }}
                    </p>
                </div>

                <div v-else class="skill-list">
                    <div
                        v-for="skill in acpStore.config.available_skills"
                        :key="skill.name"
                        class="skill-option"
                        :class="{
                            'skill-option--selected': isSkillSelected(
                                skill.name,
                            ),
                        }"
                    >
                        <div class="skill-info">
                            <div class="skill-name">{{ skill.name }}</div>
                            <div class="skill-desc">
                                {{ skill.description }}
                            </div>
                        </div>
                        <button
                            class="toggle"
                            :class="{
                                'toggle--on': isSkillSelected(skill.name),
                            }"
                            @click="toggleSkill(skill.name)"
                            role="switch"
                            :aria-checked="isSkillSelected(skill.name)"
                        >
                            <span class="toggle-thumb"></span>
                        </button>
                    </div>
                </div>
            </section>

            <!-- Knowledge Base Section -->
            <section class="config-section">
                <h2 class="section-title">
                    {{ t("acpConfig.knowledgeBases", "Knowledge Bases") }}
                </h2>
                <p class="section-desc">
                    {{
                        t(
                            "acpConfig.selectKnowledgeBases",
                            "Select knowledge bases to enable in ACP mode",
                        )
                    }}
                </p>

                <div
                    v-if="kbStore.knowledgeBases.length === 0"
                    class="empty-section"
                >
                    <p class="empty-section-text">
                        {{
                            t(
                                "acpConfig.noKnowledgeBases",
                                "No knowledge bases configured",
                            )
                        }}
                    </p>
                    <p class="empty-section-hint">
                        {{
                            t(
                                "acpConfig.addKnowledgeBaseHint",
                                "Go to Knowledge Base page to create one",
                            )
                        }}
                    </p>
                </div>

                <div v-else class="skill-list">
                    <div
                        v-for="kb in kbStore.knowledgeBases"
                        :key="kb.id"
                        class="skill-option"
                        :class="{
                            'skill-option--selected': isKbSelected(kb.id),
                        }"
                    >
                        <div class="skill-info">
                            <div class="skill-name">{{ kb.name }}</div>
                            <div class="skill-desc">
                                {{
                                    kb.description ||
                                    t(
                                        "acpConfig.noDescription",
                                        "No description",
                                    )
                                }}
                            </div>
                        </div>
                        <button
                            class="toggle"
                            :class="{
                                'toggle--on': isKbSelected(kb.id),
                            }"
                            @click="toggleKb(kb.id)"
                            role="switch"
                            :aria-checked="isKbSelected(kb.id)"
                        >
                            <span class="toggle-thumb"></span>
                        </button>
                    </div>
                </div>
            </section>

            <!-- Save Button -->
            <div class="save-row">
                <button
                    class="btn btn-accent"
                    :disabled="!hasChanges || acpStore.loading"
                    @click="handleSave"
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
                            d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"
                        />
                        <polyline points="17 21 17 13 7 13 7 21" />
                        <polyline points="7 3 7 8 15 8" />
                    </svg>
                    {{ t("acpConfig.save") }}
                </button>
                <span v-if="hasChanges" class="change-hint">{{
                    t("acpConfig.unsavedChanges")
                }}</span>
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
                    class="info-icon"
                >
                    <circle cx="12" cy="12" r="10" />
                    <line x1="12" y1="16" x2="12" y2="12" />
                    <line x1="12" y1="8" x2="12.01" y2="8" />
                </svg>
                <span>
                    {{ t("acpConfig.infoBanner") }}
                </span>
            </div>
        </template>
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

/* Empty Section */
.empty-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 2rem 0;
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
}

.empty-section-text {
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--color-text-secondary);
    margin-bottom: 0.25rem;
}

.empty-section-hint {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
}

/* Provider List */
.provider-list {
    display: grid;
    gap: 0.5rem;
}

.provider-option {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition:
        border-color var(--transition-fast),
        background-color var(--transition-fast);
    text-align: left;
    width: 100%;
    font-family: inherit;
}

.provider-option:hover {
    border-color: var(--color-border-hover);
}

.provider-option--selected {
    background-color: var(--color-accent-soft);
    border-color: rgba(134, 59, 255, 0.25);
    border-left: 3px solid var(--color-accent);
}

.provider-option--selected:hover {
    border-color: rgba(134, 59, 255, 0.4);
    border-left: 3px solid var(--color-accent);
}

/* Radio */
.provider-radio {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
}

.radio-dot {
    width: 1.125rem;
    height: 1.125rem;
    border-radius: 50%;
    border: 2px solid var(--color-border);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: border-color var(--transition-fast);
}

.radio-dot--selected {
    border-color: var(--color-accent);
}

.radio-dot-inner {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background-color: var(--color-accent);
}

.provider-info {
    min-width: 0;
}

.provider-name {
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--color-text);
}

.provider-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.25rem;
}

.type-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.0625rem 0.375rem;
    font-size: 0.6875rem;
    font-weight: 500;
    border-radius: var(--radius-sm);
    background-color: var(--color-bg-mute);
    color: var(--color-text-secondary);
}

.type-badge--openai {
    background-color: rgba(16, 185, 129, 0.1);
    color: var(--color-success);
}

.type-badge--anthropic {
    background-color: var(--color-accent-soft);
    color: var(--color-accent-hover);
}

.type-badge--custom {
    background-color: var(--color-bg-mute);
    color: var(--color-text-muted);
}

.provider-model {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
}

/* Skill List */
.skill-list {
    display: grid;
    gap: 0.5rem;
}

.skill-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.875rem 1.25rem;
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    transition:
        border-color var(--transition-fast),
        background-color var(--transition-fast);
}

.skill-option:hover {
    border-color: var(--color-border-hover);
}

.skill-option--selected {
    background-color: var(--color-accent-soft);
    border-color: rgba(134, 59, 255, 0.25);
}

.skill-option--selected:hover {
    border-color: rgba(134, 59, 255, 0.4);
}

.skill-info {
    min-width: 0;
}

.skill-name {
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--color-text);
}

.skill-desc {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    margin-top: 0.125rem;
}

/* Toggle Switch */
.toggle {
    position: relative;
    width: 2.25rem;
    height: 1.25rem;
    border-radius: 9999px;
    border: none;
    background-color: var(--color-border);
    cursor: pointer;
    transition: background-color var(--transition-fast);
    flex-shrink: 0;
    padding: 0;
}

.toggle:hover {
    background-color: var(--color-border-hover);
}

.toggle--on {
    background-color: var(--color-accent);
}

.toggle--on:hover {
    background-color: var(--color-accent-hover);
}

.toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: calc(1.25rem - 4px);
    height: calc(1.25rem - 4px);
    border-radius: 50%;
    background-color: white;
    transition: transform var(--transition-fast);
    display: block;
}

.toggle--on .toggle-thumb {
    transform: translateX(1rem);
}

/* Save Row */
.save-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.5rem;
}

/* Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    font-weight: 500;
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    background-color: var(--color-bg-mute);
    color: var(--color-text);
    cursor: pointer;
    transition: all var(--transition-fast);
    white-space: nowrap;
}

.btn:hover {
    background-color: var(--color-bg-hover);
    border-color: var(--color-border);
}

.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-accent {
    background-color: var(--color-accent);
    color: white;
    border-color: transparent;
}

.btn-accent:hover:not(:disabled) {
    background-color: var(--color-accent-hover);
    border-color: transparent;
}

.change-hint {
    font-size: 0.8125rem;
    color: var(--color-accent);
}

/* Info Banner */
.info-banner {
    display: flex;
    align-items: flex-start;
    gap: 0.625rem;
    padding: 0.875rem 1rem;
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: 0.8125rem;
    color: var(--color-text-secondary);
    line-height: 1.5;
}

.info-icon {
    flex-shrink: 0;
    margin-top: 0.0625rem;
    color: var(--color-text-muted);
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

    .skill-option {
        flex-direction: column;
        align-items: flex-start;
    }
}
</style>

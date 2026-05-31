<script setup lang="ts">
import { onMounted, reactive, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useSubAgentStore } from "../stores/subagent";
import { useProviderStore } from "../stores/provider";
import { usePersonaStore } from "../stores/persona";
import { useToast } from "../composables/useToast";
import type {
    SubAgentDefinition,
    SubAgentOrchestratorConfig,
    ModelInfo,
} from "../types";
import * as api from "../api";

const { t } = useI18n();
const store = useSubAgentStore();
const providerStore = useProviderStore();
const personaStore = usePersonaStore();
const toast = useToast();

const localConfig = reactive<SubAgentOrchestratorConfig>({
    main_enable: false,
    remove_main_duplicate_tools: false,
    router_system_prompt: "",
    agents: [],
});

onMounted(async () => {
    await Promise.all([
        store.fetchConfig(),
        providerStore.fetchProviders(),
        personaStore.fetchPersonas(),
    ]);
    Object.assign(localConfig, JSON.parse(JSON.stringify(store.config)));
});

watch(
    () => store.config,
    (val) => {
        Object.assign(localConfig, JSON.parse(JSON.stringify(val)));
    },
    { deep: true },
);

function addAgent() {
    localConfig.agents.push({
        name: "",
        enabled: true,
        system_prompt: "",
        description: "",
        model: null,
        provider_id: null,
        persona_id: null,
        max_tool_rounds: null,
    });
}

function removeAgent(index: number) {
    localConfig.agents.splice(index, 1);
}

// Per-agent model fetching state
const agentModels = reactive<Record<number, ModelInfo[]>>({});
const agentModelsLoading = reactive<Record<number, boolean>>({});
const agentModelsError = reactive<Record<number, string | null>>({});

function getProviderById(id: string | null | undefined) {
    if (!id) return null;
    return providerStore.providers.find((p) => p.id === id) ?? null;
}

async function fetchModelsForAgent(idx: number, agent: SubAgentDefinition) {
    const provider = getProviderById(agent.provider_id);
    if (!provider) return;
    agentModelsLoading[idx] = true;
    agentModelsError[idx] = null;
    try {
        const res = await api.fetchProviderModels({
            provider_type: provider.provider_type,
            base_url: provider.config.base_url,
            api_key: provider.config.api_key,
        });
        agentModels[idx] = res.models;
    } catch (e: unknown) {
        agentModelsError[idx] =
            e instanceof Error
                ? e.message
                : t("providers.form.fetchModelsError");
    } finally {
        agentModelsLoading[idx] = false;
    }
}

function onProviderChange(idx: number, agent: SubAgentDefinition) {
    // Clear model when provider changes
    agent.model = null;
    agentModels[idx] = [];
    agentModelsError[idx] = null;
    // Auto-fetch models
    fetchModelsForAgent(idx, agent);
}

function formatNullInt(val: number | null | undefined): string {
    if (val === null || val === undefined) return "";
    return String(val);
}

function parseNullInt(raw: string): number | null {
    const trimmed = raw.trim();
    if (trimmed === "") return null;
    const n = parseInt(trimmed, 10);
    return isNaN(n) ? null : n;
}

async function handleSave() {
    try {
        // Normalize empty strings to null before saving
        const data = JSON.parse(
            JSON.stringify(localConfig),
        ) as SubAgentOrchestratorConfig;
        for (const agent of data.agents) {
            if (agent.model === "") agent.model = null;
            if ((agent as any).provider_id === "")
                (agent as any).provider_id = null;
            if ((agent as any).persona_id === "")
                (agent as any).persona_id = null;
        }
        await store.updateConfig(data);
        toast.success(t("subagents.saveSuccess"));
    } catch {
        toast.error(t("subagents.saveFailed"));
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
                        width="32"
                        height="32"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
                        <circle cx="9" cy="7" r="4" />
                        <path d="M22 21v-2a4 4 0 0 0-3-3.87" />
                        <path d="M16 3.13a4 4 0 0 1 0 7.75" />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">{{ t("subagents.title") }}</h1>
                    <p class="header-desc">{{ t("subagents.subtitle") }}</p>
                </div>
            </div>
        </div>

        <!-- Error Banner -->
        <div v-if="store.error" class="error-banner">
            <svg
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="error-icon"
            >
                <circle cx="12" cy="12" r="10" />
                <line x1="15" y1="9" x2="9" y2="15" />
                <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
            <span>{{ store.error }}</span>
        </div>

        <!-- Loading -->
        <div v-if="store.loading" class="loading-state">
            <div class="loading-spinner"></div>
            <span class="loading-text">{{ t("common.loading") }}</span>
        </div>

        <div v-else class="config-body">
            <!-- Master Switch -->
            <div class="config-section glass">
                <h2 class="section-title">
                    {{ t("subagents.generalSettings") }}
                </h2>
                <div class="toggle-row">
                    <div class="toggle-info">
                        <label class="toggle-label">{{
                            t("subagents.mainEnable")
                        }}</label>
                        <p class="toggle-desc">
                            {{ t("subagents.mainEnableDesc") }}
                        </p>
                    </div>
                    <label class="switch">
                        <input
                            v-model="localConfig.main_enable"
                            type="checkbox"
                        />
                        <span class="slider"></span>
                    </label>
                </div>
                <div class="toggle-row">
                    <div class="toggle-info">
                        <label class="toggle-label">{{
                            t("subagents.removeDuplicateTools")
                        }}</label>
                        <p class="toggle-desc">
                            {{ t("subagents.removeDuplicateToolsDesc") }}
                        </p>
                    </div>
                    <label class="switch">
                        <input
                            v-model="localConfig.remove_main_duplicate_tools"
                            type="checkbox"
                        />
                        <span class="slider"></span>
                    </label>
                </div>
                <div class="form-group">
                    <label class="form-label">{{
                        t("subagents.routerSystemPrompt")
                    }}</label>
                    <textarea
                        v-model="localConfig.router_system_prompt"
                        class="form-textarea"
                        :placeholder="
                            t('subagents.routerSystemPromptPlaceholder')
                        "
                        rows="4"
                    ></textarea>
                </div>
            </div>

            <!-- Sub-Agent Cards -->
            <div class="config-section glass">
                <div class="section-header">
                    <h2 class="section-title">
                        {{ t("subagents.agentList") }}
                    </h2>
                    <button class="btn btn-accent btn-sm" @click="addAgent">
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
                            <path d="M12 5v14M5 12h14" />
                        </svg>
                        {{ t("subagents.addAgent") }}
                    </button>
                </div>

                <div v-if="localConfig.agents.length === 0" class="empty-state">
                    <div class="empty-illustration">
                        <div class="empty-icon-wrapper">
                            <svg
                                width="48"
                                height="48"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                class="empty-icon"
                            >
                                <path
                                    d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"
                                />
                                <circle cx="9" cy="7" r="4" />
                                <path d="M22 21v-2a4 4 0 0 0-3-3.87" />
                                <path d="M16 3.13a4 4 0 0 1 0 7.75" />
                            </svg>
                        </div>
                    </div>
                    <h3 class="empty-title">{{ t("subagents.noAgents") }}</h3>
                    <p class="empty-desc">{{ t("subagents.noAgentsDesc") }}</p>
                </div>

                <div v-else class="agent-cards">
                    <div
                        v-for="(agent, idx) in localConfig.agents"
                        :key="idx"
                        class="agent-card glass"
                    >
                        <div class="agent-card-header">
                            <div class="agent-name-row">
                                <input
                                    v-model="agent.name"
                                    class="agent-name-input"
                                    :placeholder="
                                        t('subagents.agentNamePlaceholder')
                                    "
                                />
                                <label class="switch switch-sm">
                                    <input
                                        v-model="agent.enabled"
                                        type="checkbox"
                                    />
                                    <span class="slider"></span>
                                </label>
                                <button
                                    class="btn btn-ghost btn-sm btn-danger-ghost"
                                    @click="removeAgent(idx)"
                                >
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
                                        <polyline points="3 6 5 6 21 6" />
                                        <path
                                            d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                                        />
                                    </svg>
                                </button>
                            </div>
                        </div>

                        <div class="agent-card-body">
                            <div class="form-group">
                                <label class="form-label">{{
                                    t("subagents.agentDescription")
                                }}</label>
                                <input
                                    v-model="agent.description"
                                    class="form-input"
                                    :placeholder="
                                        t(
                                            'subagents.agentDescriptionPlaceholder',
                                        )
                                    "
                                />
                            </div>

                            <div class="form-group">
                                <label class="form-label">{{
                                    t("subagents.agentSystemPrompt")
                                }}</label>
                                <textarea
                                    v-model="agent.system_prompt"
                                    class="form-textarea"
                                    :placeholder="
                                        t(
                                            'subagents.agentSystemPromptPlaceholder',
                                        )
                                    "
                                    rows="3"
                                ></textarea>
                            </div>

                            <!-- Provider + Persona row -->
                            <div class="agent-row-2">
                                <div class="form-group">
                                    <label class="form-label">{{
                                        t("subagents.agentProvider")
                                    }}</label>
                                    <select
                                        class="form-input"
                                        :value="agent.provider_id"
                                        @change="
                                            agent.provider_id =
                                                (
                                                    $event.target as HTMLSelectElement
                                                ).value || null;
                                            onProviderChange(idx, agent);
                                        "
                                    >
                                        <option value="">
                                            {{
                                                t(
                                                    "subagents.agentProviderPlaceholder",
                                                )
                                            }}
                                        </option>
                                        <option
                                            v-for="p in providerStore.providers"
                                            :key="p.id"
                                            :value="p.id"
                                        >
                                            {{ p.name }}
                                        </option>
                                    </select>
                                </div>

                                <div class="form-group">
                                    <label class="form-label">{{
                                        t("subagents.agentPersona")
                                    }}</label>
                                    <select
                                        v-model="agent.persona_id"
                                        class="form-input"
                                    >
                                        <option value="">
                                            {{
                                                t(
                                                    "subagents.agentPersonaPlaceholder",
                                                )
                                            }}
                                        </option>
                                        <option
                                            v-for="persona in personaStore.personas"
                                            :key="persona.id"
                                            :value="persona.id"
                                        >
                                            {{ persona.name }}
                                        </option>
                                    </select>
                                </div>
                            </div>

                            <!-- Model + Max Tool Rounds row -->
                            <div class="agent-row-2">
                                <div class="form-group">
                                    <label class="form-label">{{
                                        t("subagents.agentModel")
                                    }}</label>
                                    <div class="model-select-row">
                                        <select
                                            v-model="agent.model"
                                            class="form-input"
                                            :disabled="
                                                !agent.provider_id ||
                                                agentModelsLoading[idx]
                                            "
                                        >
                                            <option value="">
                                                {{
                                                    t(
                                                        "subagents.agentModelPlaceholder",
                                                    )
                                                }}
                                            </option>
                                            <option
                                                v-for="m in agentModels[idx]"
                                                :key="m.id"
                                                :value="m.id"
                                            >
                                                {{ m.id }}
                                            </option>
                                        </select>
                                        <button
                                            class="btn btn-ghost btn-sm btn-fetch-models"
                                            :disabled="
                                                !agent.provider_id ||
                                                agentModelsLoading[idx]
                                            "
                                            @click="
                                                fetchModelsForAgent(idx, agent)
                                            "
                                            :title="
                                                t('subagents.agentFetchModels')
                                            "
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
                                                :class="{
                                                    'spin-icon':
                                                        agentModelsLoading[idx],
                                                }"
                                            >
                                                <polyline
                                                    points="23 4 23 10 17 10"
                                                />
                                                <path
                                                    d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"
                                                />
                                            </svg>
                                        </button>
                                    </div>
                                    <p
                                        v-if="agentModelsError[idx]"
                                        class="model-error-hint"
                                    >
                                        {{ agentModelsError[idx] }}
                                    </p>
                                </div>

                                <div class="form-group">
                                    <label class="form-label">{{
                                        t("subagents.agentMaxToolRounds")
                                    }}</label>
                                    <input
                                        :value="
                                            formatNullInt(agent.max_tool_rounds)
                                        "
                                        @input="
                                            agent.max_tool_rounds =
                                                parseNullInt(
                                                    (
                                                        $event.target as HTMLInputElement
                                                    ).value,
                                                )
                                        "
                                        class="form-input"
                                        type="number"
                                        min="1"
                                        :placeholder="
                                            t(
                                                'subagents.agentMaxToolRoundsPlaceholder',
                                            )
                                        "
                                    />
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Save -->
            <div class="save-bar">
                <button class="btn btn-accent" @click="handleSave">
                    {{ t("common.saveChanges") }}
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.page {
    padding: 24px 32px;
    max-width: 960px;
    margin: 0 auto;
}

.page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
}

.header-content {
    display: flex;
    align-items: center;
    gap: 16px;
}

.header-icon {
    width: 56px;
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 14px;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.15),
        hsl(var(--primary) / 0.05)
    );
    color: hsl(var(--primary));
}

.header-title {
    font-size: 1.5rem;
    font-weight: 700;
    margin: 0;
    color: hsl(var(--foreground));
}

.header-desc {
    margin: 4px 0 0;
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
}

/* Error Banner */
.error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-radius: 10px;
    background: hsl(var(--destructive) / 0.1);
    color: hsl(var(--destructive));
    border: 1px solid hsl(var(--destructive) / 0.3);
    margin-bottom: 20px;
    font-size: 0.875rem;
}

.error-icon {
    flex-shrink: 0;
}

/* Loading */
.loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 0;
    gap: 12px;
}

.loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid hsl(var(--border));
    border-top-color: hsl(var(--primary));
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
}

.loading-text {
    color: hsl(var(--muted-foreground));
    font-size: 0.875rem;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

/* Config Body */
.config-body {
    display: flex;
    flex-direction: column;
    gap: 20px;
}

/* Glass */
.glass {
    background: hsl(var(--background) / 0.5);
    backdrop-filter: blur(12px);
    border: 1px solid hsl(var(--border) / 0.4);
    border-radius: 14px;
}

/* Config Section */
.config-section {
    padding: 20px;
}

.section-title {
    font-size: 1.1rem;
    font-weight: 700;
    margin: 0 0 16px;
    color: hsl(var(--foreground));
}

.section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
}

.section-header .section-title {
    margin-bottom: 0;
}

/* Toggle Row */
.toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 0;
    border-bottom: 1px solid hsl(var(--border) / 0.3);
}

.toggle-row:last-of-type {
    border-bottom: none;
    margin-bottom: 16px;
}

.toggle-info {
    flex: 1;
}

.toggle-label {
    font-size: 0.9rem;
    font-weight: 600;
    color: hsl(var(--foreground));
}

.toggle-desc {
    margin: 2px 0 0;
    font-size: 0.8rem;
    color: hsl(var(--muted-foreground));
}

/* Switch */
.switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
    flex-shrink: 0;
}

.switch-sm {
    width: 36px;
    height: 20px;
}

.switch input {
    opacity: 0;
    width: 0;
    height: 0;
}

.slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: hsl(var(--muted));
    border-radius: 24px;
    transition: background 0.2s;
}

.slider::before {
    content: "";
    position: absolute;
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background: #fff;
    border-radius: 50%;
    transition: transform 0.2s;
}

.switch-sm .slider::before {
    height: 14px;
    width: 14px;
    left: 3px;
    bottom: 3px;
}

input:checked + .slider {
    background: hsl(var(--primary));
}

input:checked + .slider::before {
    transform: translateX(20px);
}

.switch-sm input:checked + .slider::before {
    transform: translateX(16px);
}

/* Form */
.form-group {
    margin-bottom: 14px;
}

.form-label {
    display: block;
    font-size: 0.85rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin-bottom: 6px;
}

.form-input,
.form-textarea {
    width: 100%;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid hsl(var(--border));
    background: hsl(var(--background));
    color: hsl(var(--foreground));
    font-size: 0.875rem;
    font-family: inherit;
    transition: border-color 0.2s;
    box-sizing: border-box;
}

.form-input:focus,
.form-textarea:focus {
    outline: none;
    border-color: hsl(var(--ring));
    box-shadow: 0 0 0 2px hsl(var(--ring) / 0.2);
}

.form-textarea {
    resize: vertical;
}

/* Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 10px 18px;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid transparent;
    transition: all 0.2s;
    font-family: inherit;
}

.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-accent {
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    border-color: hsl(var(--primary));
}

.btn-accent:hover:not(:disabled) {
    filter: brightness(1.1);
}

.btn-ghost {
    background: transparent;
    color: hsl(var(--foreground));
    border-color: transparent;
}

.btn-ghost:hover {
    background: hsl(var(--secondary) / 0.5);
}

.btn-danger-ghost:hover {
    color: hsl(var(--destructive));
    background: hsl(var(--destructive) / 0.1);
}

.btn-sm {
    padding: 6px 12px;
    font-size: 0.8rem;
}

/* Empty State */
.empty-state {
    text-align: center;
    padding: 40px 20px;
}

.empty-illustration {
    margin-bottom: 16px;
}

.empty-icon-wrapper {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 80px;
    height: 80px;
    border-radius: 16px;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.1),
        hsl(var(--primary) / 0.03)
    );
    color: hsl(var(--primary) / 0.5);
}

.empty-title {
    font-size: 1rem;
    font-weight: 700;
    margin: 0 0 4px;
    color: hsl(var(--foreground));
}

.empty-desc {
    margin: 0;
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
}

/* Agent Cards */
.agent-cards {
    display: flex;
    flex-direction: column;
    gap: 14px;
}

.agent-card {
    padding: 16px;
    border-radius: 12px;
    background: hsl(var(--background) / 0.6);
    border: 1px solid hsl(var(--border) / 0.4);
}

.agent-card-header {
    margin-bottom: 12px;
}

.agent-name-row {
    display: flex;
    align-items: center;
    gap: 10px;
}

.agent-name-input {
    flex: 1;
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px solid hsl(var(--border));
    background: hsl(var(--background));
    color: hsl(var(--foreground));
    font-size: 0.9rem;
    font-weight: 600;
    font-family: inherit;
}

.agent-name-input:focus {
    outline: none;
    border-color: hsl(var(--ring));
    box-shadow: 0 0 0 2px hsl(var(--ring) / 0.2);
}

.agent-card-body {
    display: flex;
    flex-direction: column;
}

.agent-row-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
}

.model-select-row {
    display: flex;
    gap: 6px;
}

.model-select-row select {
    flex: 1;
}

.btn-fetch-models {
    flex-shrink: 0;
    padding: 6px 8px;
}

.spin-icon {
    animation: spin 0.8s linear infinite;
}

.model-error-hint {
    margin: 4px 0 0;
    font-size: 0.75rem;
    color: hsl(var(--destructive));
}

/* Save Bar */
.save-bar {
    display: flex;
    justify-content: flex-end;
}

@media (max-width: 640px) {
    .page {
        padding: 16px;
    }

    .page-header {
        flex-direction: column;
        align-items: flex-start;
    }

    .agent-row-2 {
        grid-template-columns: 1fr;
    }
}
</style>

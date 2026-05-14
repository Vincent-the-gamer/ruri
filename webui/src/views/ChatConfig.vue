<script setup lang="ts">
import { onMounted, ref, computed, reactive, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useConfigStore } from "../stores/config";
import { useKnowledgeBaseStore } from "../stores/knowledgeBase";
import { useProviderStore } from "../stores/provider";
import { useDebugSessionStore } from "../stores/debugSession";
import { useSkillStore } from "../stores/skill";
import { ProxyRuleTypeLabels } from "../types";
import type {
    ProxyConfig,
    ProxyMode,
    ProxyRule,
    ProxyRuleType,
} from "../types";

const { t } = useI18n();
const configStore = useConfigStore();
const kbStore = useKnowledgeBaseStore();
const providerStore = useProviderStore();
const debugSessionStore = useDebugSessionStore();
const skillStore = useSkillStore();

// ── Model Parameters ──
const temperature = ref(0.7);
const maxTokens = ref(4096);

// ── Provider Selection ──
const selectedProviderId = ref<string | null>(null);

// ── Persona Selection ──
const personaForm = ref<{
    name: string;
    description: string;
    prompt: string;
} | null>(null);
const activePersona = computed(() => {
    // Use local form, then debug session's embedded persona, then active config profile's
    return (
        personaForm.value ??
        debugSessionStore.embeddedPersona ??
        configStore.activeEmbeddedPersona
    );
});

// ── Custom Error Message ──
const customErrorMessage = ref("");

// ── Knowledge Base Selection ──
const selectedKbIds = ref<string[]>([]);
const knowledgeBases = computed(() => kbStore.knowledgeBases);
const allKbSelected = computed(
    () =>
        knowledgeBases.value.length > 0 &&
        selectedKbIds.value.length === knowledgeBases.value.length,
);
const noKbSelected = computed(() => selectedKbIds.value.length === 0);

function toggleKbSelection(kbId: string) {
    const idx = selectedKbIds.value.indexOf(kbId);
    if (idx === -1) {
        selectedKbIds.value.push(kbId);
    } else {
        selectedKbIds.value.splice(idx, 1);
    }
    debouncedSave();
}

function selectAllKb() {
    selectedKbIds.value = knowledgeBases.value.map((kb) => kb.id);
    debouncedSave();
}

function clearAllKb() {
    selectedKbIds.value = [];
    debouncedSave();
}

// ── Skill Selection ──
const selectedSkillNames = ref<string[]>([]);
const skills = computed(() => skillStore.skills);
const allSkillsSelected = computed(
    () =>
        skills.value.length > 0 &&
        selectedSkillNames.value.length === skills.value.length,
);
const noSkillsSelected = computed(() => selectedSkillNames.value.length === 0);

function toggleSkillSelection(skillName: string) {
    const idx = selectedSkillNames.value.indexOf(skillName);
    if (idx === -1) {
        selectedSkillNames.value.push(skillName);
    } else {
        selectedSkillNames.value.splice(idx, 1);
    }
    debouncedSave();
}

function selectAllSkills() {
    selectedSkillNames.value = skills.value.map((s) => s.name);
    debouncedSave();
}

function clearAllSkills() {
    selectedSkillNames.value = [];
    debouncedSave();
}

// ── Proxy Configuration ──
const proxyConfig = reactive<ProxyConfig>({
    enabled: false,
    url: "",
    mode: "global" as ProxyMode,
    proxy_domains: [],
    bypass_domains: [],
    username: null,
    password: null,
    bypass_localhost: true,
    rules: [],
});

const proxyDomainInput = ref("");
const bypassDomainInput = ref("");

function addProxyDomain() {
    const domain = proxyDomainInput.value.trim();
    if (domain && !proxyConfig.proxy_domains.includes(domain)) {
        proxyConfig.proxy_domains.push(domain);
        proxyDomainInput.value = "";
        debouncedSave();
    }
}

function removeProxyDomain(index: number) {
    proxyConfig.proxy_domains.splice(index, 1);
    debouncedSave();
}

function addBypassDomain() {
    const domain = bypassDomainInput.value.trim();
    if (domain && !proxyConfig.bypass_domains.includes(domain)) {
        proxyConfig.bypass_domains.push(domain);
        bypassDomainInput.value = "";
        debouncedSave();
    }
}

function removeBypassDomain(index: number) {
    proxyConfig.bypass_domains.splice(index, 1);
    debouncedSave();
}

// ── Proxy Rules Editor ──
const newRuleType = ref<ProxyRuleType>("domain");
const newRuleValue = ref("");

const ruleTypeOptions = computed(() => [
    { value: "domain", label: ProxyRuleTypeLabels.domain },
    { value: "domain-suffix", label: ProxyRuleTypeLabels["domain-suffix"] },
    { value: "domain-keyword", label: ProxyRuleTypeLabels["domain-keyword"] },
    { value: "ip-cidr", label: ProxyRuleTypeLabels["ip-cidr"] },
    { value: "geoip", label: ProxyRuleTypeLabels.geoip },
    { value: "match", label: ProxyRuleTypeLabels.match },
]);

function addRule() {
    const value = newRuleValue.value.trim();
    if (newRuleType.value === "match" || value) {
        const rule: ProxyRule = {
            rule_type: newRuleType.value,
            value: newRuleType.value === "match" ? "*" : value,
        };
        proxyConfig.rules.push(rule);
        newRuleValue.value = "";
        if (newRuleType.value === "match") {
            newRuleType.value = "domain";
        }
        debouncedSave();
    }
}

function removeRule(index: number) {
    proxyConfig.rules.splice(index, 1);
    debouncedSave();
}

function getRuleTypeColor(type: string): string {
    const colors: Record<string, string> = {
        domain: "#3b82f6",
        "domain-suffix": "#8b5cf6",
        "domain-keyword": "#f59e0b",
        "ip-cidr": "#ef4444",
        geoip: "#10b981",
        match: "#6b7280",
    };
    return colors[type] || "#6b7280";
}

// ── Save State ──
const saveSuccess = ref(false);
const saveError = ref<string | null>(null);
let saveTimer: ReturnType<typeof setTimeout> | null = null;

function clearMessages() {
    saveSuccess.value = false;
    saveError.value = null;
}

function debouncedSave() {
    clearMessages();
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
        handleSave();
    }, 600);
}

async function handleSave() {
    try {
        await debugSessionStore.updateDebugSessionConfig({
            embedded_persona: personaForm.value
                ? { ...personaForm.value }
                : null,
            temperature: temperature.value,
            max_tokens: maxTokens.value,
            custom_error_message: customErrorMessage.value || null,
            knowledge_base_ids: selectedKbIds.value,
            active_skill_names: selectedSkillNames.value,
            provider_id: selectedProviderId.value,
        });
        saveSuccess.value = true;
        setTimeout(() => {
            saveSuccess.value = false;
        }, 2000);
    } catch (e: unknown) {
        saveError.value =
            e instanceof Error ? e.message : t("chatConfig.saveFailed");
        setTimeout(() => {
            saveError.value = null;
        }, 3000);
    }
}

// ── Sync from debug session store ──
watch(
    () => debugSessionStore.debugSession,
    (session) => {
        if (session) {
            // Sync embedded persona
            if (session.embedded_persona) {
                personaForm.value = { ...session.embedded_persona };
            } else {
                personaForm.value = null;
            }
            selectedProviderId.value =
                session.provider_id || session.active_provider || null;
            temperature.value = session.temperature ?? 0.7;
            maxTokens.value = session.max_tokens ?? 4096;
            customErrorMessage.value = session.custom_error_message || "";
            selectedKbIds.value = [...(session.knowledge_base_ids || [])];
            selectedSkillNames.value = [...(session.active_skill_names || [])];
        }
    },
    { immediate: true },
);

// Also sync from config profile for proxy config (proxy is still profile-level)
watch(
    () => configStore.activeConfigProfile,
    (profile) => {
        if (profile?.proxy_config) {
            proxyConfig.enabled = profile.proxy_config.enabled;
            proxyConfig.url = profile.proxy_config.url;
            proxyConfig.mode = profile.proxy_config.mode;
            proxyConfig.proxy_domains = [...profile.proxy_config.proxy_domains];
            proxyConfig.bypass_domains = [
                ...profile.proxy_config.bypass_domains,
            ];
            proxyConfig.username = profile.proxy_config.username;
            proxyConfig.password = profile.proxy_config.password;
            proxyConfig.bypass_localhost =
                profile.proxy_config.bypass_localhost;
            proxyConfig.rules = profile.proxy_config.rules.map((r) => ({
                ...r,
            }));
        }
    },
    { immediate: true },
);

// ── Auto-save on change for simple fields ──
watch([temperature, maxTokens], () => {
    // These are client-side only (sent per-request), no server save needed
    // but if we wanted to persist we could call debouncedSave() here
});

watch([selectedProviderId, customErrorMessage], () => {
    debouncedSave();
});

watch(
    () => proxyConfig.enabled,
    () => {
        debouncedSave();
    },
);

watch(
    () => [
        proxyConfig.url,
        proxyConfig.mode,
        proxyConfig.bypass_localhost,
        proxyConfig.username,
        proxyConfig.password,
    ],
    () => {
        debouncedSave();
    },
);

// ── Fetch data on mount ──
onMounted(async () => {
    await Promise.all([
        debugSessionStore.fetchDebugSession(),
        configStore.fetchConfigProfiles(),
        kbStore.fetchKnowledgeBases(),
        skillStore.fetchSkills(),
        providerStore.fetchProviders(),
    ]);
});
</script>

<template>
    <div class="chat-config-page">
        <!-- Page Header -->
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
                        <path
                            d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
                        />
                        <circle cx="12" cy="12" r="3" />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">{{ t("chatConfig.title") }}</h1>
                    <p class="header-desc">{{ t("chatConfig.description") }}</p>
                </div>
            </div>
            <button class="save-btn" @click="handleSave">
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
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
                {{ t("chatConfig.save") }}
            </button>
        </div>

        <!-- Banners -->
        <div v-if="saveSuccess" class="success-banner">
            {{ t("chatConfig.saved") }}
        </div>
        <div v-if="saveError" class="error-banner">
            {{ saveError }}
        </div>

        <!-- Section 1: Model Parameters -->
        <section class="config-section">
            <h2 class="section-title">
                <span class="section-icon">🌡️</span>
                {{ t("chatConfig.modelParams") }}
            </h2>
            <p class="section-desc">
                {{
                    t(
                        "chatConfig.modelParamsDesc",
                        "Adjust model behavior for this chat session",
                    )
                }}
            </p>

            <div class="form-field">
                <label class="input-label">
                    {{ t("chatConfig.temperature") }}
                    <span class="label-value">{{
                        temperature.toFixed(1)
                    }}</span>
                </label>
                <input
                    v-model.number="temperature"
                    type="range"
                    min="0"
                    max="2"
                    step="0.1"
                    class="slider-input"
                />
                <div class="slider-labels">
                    <span>0</span>
                    <span>1</span>
                    <span>2</span>
                </div>
            </div>

            <div class="form-field">
                <label class="input-label">
                    {{ t("chatConfig.maxTokens") }}
                </label>
                <input
                    v-model.number="maxTokens"
                    type="number"
                    min="1"
                    max="128000"
                    class="text-input"
                />
                <p class="input-hint">
                    {{ t("chatConfig.maxTokensHint", "Range: 1 – 128000") }}
                </p>
            </div>
        </section>

        <!-- Section 2: Model Provider Selection -->
        <section class="config-section">
            <h2 class="section-title">
                <span class="section-icon">🤖</span>
                {{ t("chatConfig.modelProvider", "Model Provider") }}
            </h2>
            <p class="section-desc">
                {{
                    t(
                        "chatConfig.modelProviderDesc",
                        "Select which configured model provider to use for this chat. The provider from your active configuration profile is selected by default.",
                    )
                }}
            </p>

            <div class="form-field">
                <select v-model="selectedProviderId" class="select-input">
                    <option :value="null">
                        {{
                            t(
                                "chatConfig.providerDefault",
                                "Use profile default",
                            )
                        }}
                    </option>
                    <option
                        v-for="provider in providerStore.providers"
                        :key="provider.id"
                        :value="provider.id"
                    >
                        {{ provider.name }} ({{
                            (provider.config as any).default_model
                        }})
                    </option>
                </select>
            </div>

            <div
                v-if="providerStore.providers.length === 0"
                class="empty-hint"
                style="margin-top: 0.5rem"
            >
                {{
                    t(
                        "chatConfig.noProvidersHint",
                        "No providers configured yet. Add one in the Providers page first.",
                    )
                }}
            </div>
        </section>

        <!-- Section 3: Persona Selection -->
        <section class="config-section">
            <h2 class="section-title">
                <span class="section-icon">🎭</span>
                {{ t("chatConfig.persona") }}
            </h2>
            <p class="section-desc">
                {{ t("chatConfig.personaDesc") }}
            </p>

            <div class="form-field">
                <div v-if="personaForm" class="persona-editor">
                    <div class="persona-editor-header">
                        <input
                            v-model="personaForm.name"
                            class="text-input"
                            :placeholder="'Name'"
                        />
                        <button
                            type="button"
                            class="btn btn-sm btn-danger"
                            @click="
                                personaForm = null;
                                debouncedSave();
                            "
                        >
                            ✕
                        </button>
                    </div>
                    <input
                        v-model="personaForm.description"
                        class="text-input"
                        :placeholder="'Description'"
                    />
                    <textarea
                        v-model="personaForm.prompt"
                        class="text-input"
                        rows="3"
                        :placeholder="'System Prompt'"
                        @input="debouncedSave()"
                    ></textarea>
                </div>
                <button
                    v-else
                    type="button"
                    class="btn btn-sm btn-secondary"
                    @click="
                        personaForm = { name: '', description: '', prompt: '' };
                        debouncedSave();
                    "
                >
                    + Configure Persona
                </button>
            </div>

            <div v-if="!activePersona" class="persona-preview">
                <p
                    class="persona-preview-name"
                    style="color: var(--text-muted)"
                >
                    {{ t("chatConfig.noPersonaHint") }}
                </p>
            </div>

            <div v-if="activePersona" class="persona-preview">
                <div class="persona-preview-header">
                    <span class="persona-preview-name">{{
                        activePersona.name
                    }}</span>
                </div>
                <p class="persona-preview-prompt">
                    {{ activePersona.prompt }}
                </p>
            </div>
        </section>

        <!-- Section 3: Custom Error Message -->
        <section class="config-section">
            <h2 class="section-title">
                <span class="section-icon">💬</span>
                {{ t("chatConfig.customErrorMessage") }}
            </h2>
            <div class="form-field">
                <input
                    v-model="customErrorMessage"
                    type="text"
                    class="text-input"
                    :placeholder="
                        t(
                            'chatConfig.customErrorMessagePlaceholder',
                            'e.g. Sorry, something went wrong.',
                        )
                    "
                />
                <p class="input-hint">
                    {{
                        t(
                            "chatConfig.customErrorMessageHint",
                            "This message will be shown when an error occurs during generation",
                        )
                    }}
                </p>
            </div>
        </section>

        <!-- Section 4: Knowledge Base Selection -->
        <section class="config-section">
            <h2 class="section-title">
                <span class="section-icon">📚</span>
                {{ t("chatConfig.knowledgeBases") }}
            </h2>
            <p class="section-desc">
                {{
                    t(
                        "chatConfig.knowledgeBasesDesc",
                        "Select knowledge bases to enhance responses with relevant information",
                    )
                }}
            </p>

            <div v-if="knowledgeBases.length > 0" class="kb-actions">
                <button
                    class="btn btn-sm btn-outline"
                    @click="selectAllKb"
                    :disabled="allKbSelected"
                >
                    {{ t("chatConfig.selectAllKb") }}
                </button>
                <button
                    class="btn btn-sm btn-outline"
                    @click="clearAllKb"
                    :disabled="noKbSelected"
                >
                    {{ t("chatConfig.clearAllKb") }}
                </button>
            </div>

            <div v-if="knowledgeBases.length > 0" class="kb-grid">
                <div
                    v-for="kb in knowledgeBases"
                    :key="kb.id"
                    class="kb-card"
                    :class="{
                        'kb-card--selected': selectedKbIds.includes(kb.id),
                    }"
                    @click="toggleKbSelection(kb.id)"
                >
                    <div class="kb-card-header">
                        <span class="kb-card-name">{{ kb.name }}</span>
                        <span class="kb-card-check">
                            <svg
                                v-if="selectedKbIds.includes(kb.id)"
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="3"
                            >
                                <polyline points="20 6 9 17 4 12" />
                            </svg>
                        </span>
                    </div>
                    <p v-if="kb.description" class="kb-card-desc">
                        {{ kb.description }}
                    </p>
                    <div class="kb-card-meta">
                        <span>{{ kb.document_count }} docs</span>
                        <span>{{ kb.chunk_count }} chunks</span>
                    </div>
                </div>
            </div>

            <div v-else class="empty-state">
                <p>{{ t("chatConfig.noKnowledgeBases") }}</p>
                <router-link to="/knowledge-base" class="learn-more-link">
                    {{ t("chatConfig.goToKb") }}
                </router-link>
            </div>
        </section>

        <!-- Section: Skills -->
        <section class="config-section">
            <h2 class="section-title">
                <span class="section-icon">⚡</span>
                {{ t("chatConfig.skills") }}
            </h2>
            <p class="section-desc">
                {{
                    t(
                        "chatConfig.skillsDesc",
                        "Select skills to enable for this conversation",
                    )
                }}
            </p>

            <div v-if="skills.length > 0" class="kb-actions">
                <button
                    class="btn btn-sm btn-outline"
                    @click="selectAllSkills"
                    :disabled="allSkillsSelected"
                >
                    {{ t("chatConfig.selectAllSkills") }}
                </button>
                <button
                    class="btn btn-sm btn-outline"
                    @click="clearAllSkills"
                    :disabled="noSkillsSelected"
                >
                    {{ t("chatConfig.clearAllSkills") }}
                </button>
            </div>

            <div v-if="skills.length > 0" class="kb-grid">
                <div
                    v-for="skill in skills"
                    :key="skill.name"
                    class="kb-card"
                    :class="{
                        'kb-card--selected': selectedSkillNames.includes(
                            skill.name,
                        ),
                    }"
                    @click="toggleSkillSelection(skill.name)"
                >
                    <div class="kb-card-header">
                        <span class="kb-card-name">{{ skill.name }}</span>
                        <span class="kb-card-check">
                            <svg
                                v-if="selectedSkillNames.includes(skill.name)"
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="3"
                            >
                                <polyline points="20 6 9 17 4 12" />
                            </svg>
                        </span>
                    </div>
                    <p v-if="skill.description" class="kb-card-desc">
                        {{ skill.description }}
                    </p>
                    <div class="kb-card-meta">
                        <span>{{ skill.skill_type }}</span>
                    </div>
                </div>
            </div>

            <div v-else class="empty-state">
                <p>{{ t("chatConfig.noSkills") }}</p>
                <router-link to="/skills" class="learn-more-link">
                    {{ t("chatConfig.goToSkills") }}
                </router-link>
            </div>
        </section>

        <!-- Section 5: Proxy Configuration -->
        <section class="config-section">
            <h2 class="section-title">
                <span class="section-icon">🌐</span>
                {{ t("chatConfig.proxyConfig") }}
            </h2>
            <p class="section-desc">
                {{
                    t(
                        "chatConfig.proxyConfigDesc",
                        "Configure proxy settings for outgoing connections",
                    )
                }}
            </p>

            <!-- Enable Toggle -->
            <div class="toggle-row">
                <label class="toggle-container">
                    <input
                        type="checkbox"
                        v-model="proxyConfig.enabled"
                        class="toggle-input"
                    />
                    <span
                        class="toggle"
                        :class="{ 'toggle--on': proxyConfig.enabled }"
                    >
                        <span class="toggle-thumb"></span>
                    </span>
                </label>
                <div class="toggle-info">
                    <span class="toggle-text">{{
                        t("chatConfig.proxyEnabled")
                    }}</span>
                    <span class="toggle-description">{{
                        t(
                            "chatConfig.proxyEnabledDesc",
                            "Route outgoing requests through a proxy server",
                        )
                    }}</span>
                </div>
            </div>

            <!-- Proxy detail fields — self-contained scrollable area -->
            <Transition name="proxy-slide">
                <div v-if="proxyConfig.enabled" class="proxy-fields">
                    <!-- URL -->
                    <div class="form-field">
                        <label class="input-label">{{
                            t("chatConfig.proxyUrl")
                        }}</label>
                        <input
                            v-model="proxyConfig.url"
                            type="text"
                            class="text-input"
                            :placeholder="
                                t(
                                    'chatConfig.proxyUrlPlaceholder',
                                    'e.g. http://127.0.0.1:7890',
                                )
                            "
                        />
                    </div>

                    <!-- Mode -->
                    <div class="form-field">
                        <label class="input-label">{{
                            t("chatConfig.proxyMode")
                        }}</label>
                        <select v-model="proxyConfig.mode" class="select-input">
                            <option value="global">
                                {{ t("chatConfig.proxyModeGlobal") }}
                            </option>
                            <option value="rules">
                                {{ t("chatConfig.proxyModeRules") }}
                            </option>
                        </select>
                    </div>

                    <!-- Clash-style Rules Editor (only when mode=rules) -->
                    <div v-if="proxyConfig.mode === 'rules'" class="form-field">
                        <label class="input-label">{{
                            t("chatConfig.proxyRules")
                        }}</label>
                        <p class="input-hint" style="margin-bottom: 0.5rem">
                            {{
                                t(
                                    "chatConfig.proxyRulesDesc",
                                    "Clash-style rules: matching traffic goes through the proxy",
                                )
                            }}
                        </p>

                        <!-- Add rule row -->
                        <div class="rule-add-row">
                            <select
                                v-model="newRuleType"
                                class="rule-type-select"
                            >
                                <option
                                    v-for="opt in ruleTypeOptions"
                                    :key="opt.value"
                                    :value="opt.value"
                                >
                                    {{ opt.label }}
                                </option>
                            </select>
                            <input
                                v-if="newRuleType !== 'match'"
                                v-model="newRuleValue"
                                type="text"
                                class="rule-value-input"
                                :placeholder="
                                    t(
                                        'chatConfig.proxyRuleValuePlaceholder',
                                        'e.g. google.com',
                                    )
                                "
                                @keyup.enter="addRule"
                            />
                            <button
                                type="button"
                                class="rule-add-btn"
                                @click="addRule"
                                :disabled="
                                    newRuleType !== 'match' &&
                                    !newRuleValue.trim()
                                "
                            >
                                <svg
                                    width="14"
                                    height="14"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                >
                                    <line x1="12" y1="5" x2="12" y2="19" />
                                    <line x1="5" y1="12" x2="19" y2="12" />
                                </svg>
                            </button>
                        </div>

                        <!-- Rules list -->
                        <div
                            v-if="proxyConfig.rules.length > 0"
                            class="rules-list"
                        >
                            <div
                                v-for="(rule, index) in proxyConfig.rules"
                                :key="index"
                                class="rule-item"
                            >
                                <span
                                    class="rule-type-badge"
                                    :style="{
                                        color: getRuleTypeColor(rule.rule_type),
                                        borderColor: getRuleTypeColor(
                                            rule.rule_type,
                                        ),
                                        background: `${getRuleTypeColor(rule.rule_type)}15`,
                                    }"
                                >
                                    {{ ProxyRuleTypeLabels[rule.rule_type] }}
                                </span>
                                <span class="rule-value">{{
                                    rule.rule_type === "match"
                                        ? t(
                                              "chatConfig.proxyRuleMatchAll",
                                              "Match All",
                                          )
                                        : rule.value
                                }}</span>
                                <button
                                    type="button"
                                    class="rule-remove-btn"
                                    @click="removeRule(index)"
                                >
                                    ×
                                </button>
                            </div>
                        </div>
                        <p v-else class="empty-hint">
                            {{
                                t(
                                    "chatConfig.proxyRulesEmpty",
                                    "No rules configured — all traffic will go direct",
                                )
                            }}
                        </p>
                    </div>

                    <!-- Proxy Domains (tag input) -->
                    <template
                        v-if="
                            proxyConfig.mode === 'rules' &&
                            proxyConfig.rules.length === 0
                        "
                    >
                        <div class="form-field">
                            <label class="input-label">{{
                                t("chatConfig.proxyDomains")
                            }}</label>
                            <input
                                v-model="proxyDomainInput"
                                type="text"
                                class="text-input"
                                :placeholder="
                                    t(
                                        'chatConfig.proxyDomainsPlaceholder',
                                        'Type a domain and press Enter',
                                    )
                                "
                                @keyup.enter="addProxyDomain"
                            />
                            <div
                                v-if="proxyConfig.proxy_domains.length > 0"
                                class="tag-list"
                            >
                                <span
                                    v-for="(
                                        domain, index
                                    ) in proxyConfig.proxy_domains"
                                    :key="index"
                                    class="tag-item"
                                >
                                    {{ domain }}
                                    <button
                                        type="button"
                                        class="tag-remove-btn"
                                        @click="removeProxyDomain(index)"
                                    >
                                        ×
                                    </button>
                                </span>
                            </div>
                        </div>
                    </template>

                    <!-- Bypass Domains (tag input) -->
                    <div class="form-field">
                        <label class="input-label">{{
                            t("chatConfig.bypassDomains")
                        }}</label>
                        <input
                            v-model="bypassDomainInput"
                            type="text"
                            class="text-input"
                            :placeholder="
                                t(
                                    'chatConfig.bypassDomainsPlaceholder',
                                    'Type a domain and press Enter',
                                )
                            "
                            @keyup.enter="addBypassDomain"
                        />
                        <div
                            v-if="proxyConfig.bypass_domains.length > 0"
                            class="tag-list"
                        >
                            <span
                                v-for="(
                                    domain, index
                                ) in proxyConfig.bypass_domains"
                                :key="index"
                                class="tag-item"
                            >
                                {{ domain }}
                                <button
                                    type="button"
                                    class="tag-remove-btn"
                                    @click="removeBypassDomain(index)"
                                >
                                    ×
                                </button>
                            </span>
                        </div>
                    </div>

                    <!-- Username + Password -->
                    <div class="form-row-2col">
                        <div class="form-field">
                            <label class="input-label">{{
                                t("chatConfig.proxyUsername")
                            }}</label>
                            <input
                                v-model="proxyConfig.username"
                                type="text"
                                class="text-input"
                                :placeholder="
                                    t(
                                        'chatConfig.proxyUsernamePlaceholder',
                                        'Optional',
                                    )
                                "
                            />
                        </div>
                        <div class="form-field">
                            <label class="input-label">{{
                                t("chatConfig.proxyPassword")
                            }}</label>
                            <input
                                v-model="proxyConfig.password"
                                type="password"
                                class="text-input"
                                :placeholder="
                                    t(
                                        'chatConfig.proxyPasswordPlaceholder',
                                        'Optional',
                                    )
                                "
                            />
                        </div>
                    </div>

                    <!-- Bypass Localhost Toggle -->
                    <div class="toggle-row" style="margin-top: 0.5rem">
                        <label class="toggle-container">
                            <input
                                type="checkbox"
                                v-model="proxyConfig.bypass_localhost"
                                class="toggle-input"
                            />
                            <span
                                class="toggle"
                                :class="{
                                    'toggle--on': proxyConfig.bypass_localhost,
                                }"
                            >
                                <span class="toggle-thumb"></span>
                            </span>
                        </label>
                        <div class="toggle-info">
                            <span class="toggle-text">{{
                                t("chatConfig.bypassLocalhost")
                            }}</span>
                            <span class="toggle-description">{{
                                t(
                                    "chatConfig.bypassLocalhostDesc",
                                    "Skip proxy for localhost and 127.0.0.1",
                                )
                            }}</span>
                        </div>
                    </div>
                </div>
            </Transition>
        </section>
    </div>
</template>

<style scoped>
/* ══════════════════════════════════════════════════════
   Root page — normal document flow inside parent <main>.
   No fixed height, no nested scroll container.
   The parent <main overflow-y-auto> handles all scrolling,
   and overflow-anchor prevents scroll jumps when
   proxy fields appear/disappear.
   ══════════════════════════════════════════════════════ */
.chat-config-page {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
}

/* ── Page Header (fixed at top, does not scroll) ── */
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

/* ── Save Button ── */
.save-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1.25rem;
    border: none;
    border-radius: var(--radius-md, 0.5rem);
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(var(--primary) / 0.85) 100%
    );
    color: white;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    white-space: nowrap;
    flex-shrink: 0;
}

.save-btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.4);
}

.save-btn:active {
    transform: translateY(0);
}

/* ── Banners ── */
.success-banner {
    background-color: var(--color-accent-soft, rgba(134, 59, 255, 0.1));
    border: 1px solid rgba(134, 59, 255, 0.2);
    color: var(--color-accent-hover, #7c3aed);
    padding: 0.75rem 1rem;
    border-radius: var(--radius-md, 0.5rem);
    margin-bottom: 1rem;
    font-size: 0.875rem;
}

.error-banner {
    background-color: var(--color-danger-soft, rgba(239, 68, 68, 0.1));
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: var(--color-danger, #ef4444);
    padding: 0.75rem 1rem;
    border-radius: var(--radius-md, 0.5rem);
    margin-bottom: 1rem;
    font-size: 0.875rem;
}

/* ── Config Section ── */
.config-section {
    background-color: var(--color-bg-soft, hsl(var(--secondary) / 0.3));
    border: 1px solid var(--color-border, hsl(var(--border) / 0.4));
    border-radius: var(--radius-lg, 0.75rem);
    padding: 1.5rem;
    margin-bottom: 1.5rem;
}

.section-title {
    font-size: 1.0625rem;
    font-weight: 600;
    color: var(--color-text, hsl(var(--foreground)));
    margin: 0 0 0.25rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.section-icon {
    font-size: 1.125rem;
    line-height: 1;
}

.section-desc {
    color: var(--color-text-muted, hsl(var(--muted-foreground)));
    font-size: 0.8125rem;
    margin: 0.25rem 0 1rem;
}

/* ── Form Fields ── */
.form-field {
    margin-bottom: 1.25rem;
}

.form-field:last-child {
    margin-bottom: 0;
}

.input-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-weight: 500;
    color: var(--color-text, hsl(var(--foreground)));
    margin-bottom: 0.5rem;
    font-size: 0.875rem;
}

.label-value {
    font-weight: 600;
    color: var(--color-accent, hsl(var(--primary)));
    font-variant-numeric: tabular-nums;
}

.text-input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--color-border, hsl(var(--border) / 0.4));
    border-radius: var(--radius-md, 0.5rem);
    font-size: 0.9375rem;
    transition: all 0.2s;
    background-color: var(--color-bg, hsl(var(--background)));
    color: var(--color-text, hsl(var(--foreground)));
    box-sizing: border-box;
}

.text-input:focus {
    outline: none;
    border-color: var(--color-primary, hsl(var(--primary)));
    box-shadow: 0 0 0 3px var(--color-accent-soft, hsl(var(--primary) / 0.1));
}

.text-input::placeholder {
    color: var(--color-text-muted, hsl(var(--muted-foreground) / 0.6));
}

.select-input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--color-border, hsl(var(--border) / 0.4));
    border-radius: var(--radius-md, 0.5rem);
    font-size: 0.9375rem;
    transition: all 0.2s;
    background-color: var(--color-bg, hsl(var(--background)));
    color: var(--color-text, hsl(var(--foreground)));
    cursor: pointer;
    appearance: auto;
    box-sizing: border-box;
}

.select-input:focus {
    outline: none;
    border-color: var(--color-primary, hsl(var(--primary)));
    box-shadow: 0 0 0 3px var(--color-accent-soft, hsl(var(--primary) / 0.1));
}

.input-hint {
    margin-top: 0.375rem;
    font-size: 0.8125rem;
    color: var(--color-text-muted, hsl(var(--muted-foreground) / 0.8));
}

.empty-hint {
    font-size: 0.8125rem;
    color: var(--color-text-muted, hsl(var(--muted-foreground) / 0.6));
    font-style: italic;
    margin: 0.5rem 0 0;
}

/* ── Temperature Slider ── */
.slider-input {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: hsl(var(--secondary));
    outline: none;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    margin: 0.5rem 0;
}

.slider-input::-webkit-slider-thumb {
    appearance: none;
    -webkit-appearance: none;
    width: 1.25rem;
    height: 1.25rem;
    border-radius: 50%;
    background: var(--color-accent, hsl(var(--primary)));
    cursor: pointer;
    border: 2px solid var(--color-bg, hsl(var(--background)));
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.15);
    transition: transform 0.15s ease;
}

.slider-input::-webkit-slider-thumb:hover {
    transform: scale(1.15);
}

.slider-input::-moz-range-thumb {
    width: 1.25rem;
    height: 1.25rem;
    border-radius: 50%;
    background: var(--color-accent, hsl(var(--primary)));
    cursor: pointer;
    border: 2px solid var(--color-bg, hsl(var(--background)));
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.15);
}

.slider-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    color: var(--color-text-muted, hsl(var(--muted-foreground)));
    margin-top: 0.125rem;
}

/* ── Persona Preview ── */
.persona-preview {
    margin-top: 0.75rem;
    padding: 1rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.06) 0%,
        hsl(var(--primary) / 0.02) 100%
    );
    border: 1px solid hsl(var(--border) / 0.4);
    border-radius: var(--radius-md, 0.5rem);
}

.persona-preview-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
}

.persona-preview-name {
    font-weight: 600;
    color: var(--color-accent, hsl(var(--primary)));
    font-size: 0.9375rem;
}

.persona-preview-prompt {
    font-size: 0.8125rem;
    color: var(--color-text-muted, hsl(var(--muted-foreground)));
    line-height: 1.5;
    margin: 0;
    max-height: 6rem;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
}

/* ── Knowledge Base Grid ── */
.kb-actions {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
}

.kb-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.75rem;
}

.kb-card {
    padding: 1rem;
    border: 2px solid var(--color-border, hsl(var(--border) / 0.4));
    border-radius: var(--radius-lg, 0.75rem);
    background-color: var(--color-bg-soft, hsl(var(--secondary) / 0.3));
    cursor: pointer;
    transition: all 0.2s ease;
}

.kb-card:hover {
    border-color: var(--color-primary, hsl(var(--primary)));
    background-color: var(--color-bg-hover, hsl(var(--secondary) / 0.5));
}

.kb-card--selected {
    border-color: var(--color-accent, hsl(var(--primary)));
    background-color: var(--color-accent-soft, hsl(var(--primary) / 0.08));
}

.kb-card--selected:hover {
    border-color: var(--color-accent-hover, hsl(var(--primary) / 0.9));
}

.kb-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.25rem;
}

.kb-card-name {
    font-weight: 600;
    color: var(--color-text, hsl(var(--foreground)));
    font-size: 0.9375rem;
}

.kb-card-check {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    border: 2px solid var(--color-border, hsl(var(--border) / 0.4));
    border-radius: 0.25rem;
    color: var(--color-accent, hsl(var(--primary)));
    transition: all 0.2s;
}

.kb-card--selected .kb-card-check {
    background-color: var(--color-accent, hsl(var(--primary)));
    border-color: var(--color-accent, hsl(var(--primary)));
    color: var(--color-primary-foreground, #fff);
}

.kb-card-desc {
    font-size: 0.8125rem;
    color: var(--color-text-muted, hsl(var(--muted-foreground)));
    margin: 0.25rem 0 0.5rem;
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
}

.kb-card-meta {
    display: flex;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: var(--color-text-muted, hsl(var(--muted-foreground) / 0.7));
}

.empty-state {
    text-align: center;
    padding: 2rem;
    color: var(--color-text-muted, hsl(var(--muted-foreground)));
    font-size: 0.875rem;
}

.empty-state p {
    margin: 0 0 0.5rem;
}

/* ── Buttons ── */
.btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: var(--radius-md, 0.5rem);
    font-size: 0.9375rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
}

.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-sm {
    padding: 0.375rem 0.875rem;
    font-size: 0.8125rem;
    border-radius: var(--radius-md, 0.375rem);
}

.btn-outline {
    background: transparent;
    border: 1px solid var(--color-border, hsl(var(--border) / 0.4));
    color: var(--color-text, hsl(var(--foreground)));
}

.btn-outline:hover:not(:disabled) {
    border-color: var(--color-primary, hsl(var(--primary)));
    background-color: var(--color-bg-hover, hsl(var(--secondary) / 0.5));
}

/* ── Toggle ── */
.toggle-row {
    display: flex;
    align-items: center;
    gap: 1rem;
}

.toggle-container {
    display: inline-flex;
    align-items: center;
    cursor: pointer;
    flex-shrink: 0;
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
    background-color: var(--color-bg-soft, hsl(var(--secondary) / 0.5));
    border: 2px solid var(--color-border, hsl(var(--border) / 0.4));
    border-radius: 1.5rem;
    transition: all 0.2s ease;
    flex-shrink: 0;
}

.toggle:hover {
    border-color: var(--color-primary, hsl(var(--primary)));
}

.toggle--on {
    background-color: var(--color-accent, hsl(var(--primary)));
    border-color: var(--color-accent, hsl(var(--primary)));
}

.toggle--on:hover {
    border-color: var(--color-accent-hover, hsl(var(--primary) / 0.9));
    background-color: var(--color-accent-hover, hsl(var(--primary) / 0.9));
}

.toggle-thumb {
    position: absolute;
    top: 0.125rem;
    left: 0.125rem;
    width: 1rem;
    height: 1rem;
    background-color: var(--color-text, hsl(var(--foreground)));
    border-radius: 50%;
    transition: transform 0.2s ease;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.toggle--on .toggle-thumb {
    transform: translateX(1.25rem);
}

.toggle-info {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    min-width: 0;
}

.toggle-text {
    font-weight: 500;
    color: var(--color-text, hsl(var(--foreground)));
    font-size: 0.9375rem;
}

.toggle-description {
    font-size: 0.8125rem;
    color: var(--color-text-muted, hsl(var(--muted-foreground)));
}

.proxy-fields {
    margin-top: 1.25rem;
}

/* Vue Transition for proxy fields expand/collapse */
.proxy-slide-enter-active {
    transition: opacity 0.2s ease-out;
}

.proxy-slide-leave-active {
    transition: opacity 0.15s ease-in;
}

.proxy-slide-enter-from {
    opacity: 0;
}

.proxy-slide-leave-to {
    opacity: 0;
}

.form-row-2col {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
}

/* ── Clash-style Rule Editor ── */
.rule-add-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.75rem;
}

.rule-type-select {
    flex-shrink: 0;
    width: auto;
    min-width: 8.5rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.8125rem;
    font-family:
        ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
    font-weight: 600;
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--foreground));
    border: 1px solid hsl(var(--border) / 0.4);
    border-radius: 0.5rem;
    cursor: pointer;
    transition: all 0.2s;
    appearance: auto;
    outline: none;
}

.rule-type-select:focus {
    border-color: hsl(var(--primary) / 0.5);
    box-shadow: 0 0 0 2px hsl(var(--primary) / 0.1);
}

.rule-value-input {
    flex: 1;
    min-width: 0;
    padding: 0.5rem 0.75rem;
    font-size: 0.8125rem;
    background: hsl(var(--background) / 0.5);
    color: hsl(var(--foreground));
    border: 1px solid hsl(var(--border) / 0.4);
    border-radius: 0.5rem;
    transition: all 0.2s;
    outline: none;
}

.rule-value-input::placeholder {
    color: hsl(var(--muted-foreground) / 0.6);
}

.rule-value-input:focus {
    border-color: hsl(var(--primary) / 0.5);
    box-shadow: 0 0 0 2px hsl(var(--primary) / 0.1);
}

.rule-add-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    flex-shrink: 0;
    padding: 0;
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    border: none;
    border-radius: 0.5rem;
    cursor: pointer;
    transition: all 0.2s;
}

.rule-add-btn:hover:not(:disabled) {
    background: hsl(var(--primary) / 0.9);
    transform: translateY(-1px);
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.3);
}

.rule-add-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
}

.rules-list {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
    margin-top: 0.5rem;
}

.rule-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.625rem;
    background: hsl(var(--secondary) / 0.3);
    border: 1px solid hsl(var(--border) / 0.3);
    border-radius: 0.375rem;
    transition: all 0.2s;
}

.rule-item:hover {
    border-color: hsl(var(--border) / 0.6);
    background: hsl(var(--secondary) / 0.5);
}

.rule-type-badge {
    flex-shrink: 0;
    padding: 0.0625rem 0.375rem;
    font-size: 0.6875rem;
    font-weight: 700;
    font-family:
        ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
    letter-spacing: 0.05em;
    border: 1px solid;
    border-radius: 0.25rem;
    white-space: nowrap;
}

.rule-value {
    flex: 1;
    font-size: 0.8125rem;
    color: hsl(var(--foreground));
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family:
        ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
}

.rule-remove-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    flex-shrink: 0;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 50%;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
    transition:
        background-color 0.2s,
        color 0.2s;
}

.rule-remove-btn:hover {
    background-color: hsl(var(--destructive));
    color: hsl(var(--destructive-foreground));
}

/* ── Tag List ── */
.tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
    margin-top: 0.375rem;
}

.tag-item {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.1875rem 0.5rem;
    background-color: hsl(var(--secondary) / 0.6);
    border: 1px solid hsl(var(--border) / 0.3);
    border-radius: 0.25rem;
    font-size: 0.8125rem;
    color: hsl(var(--foreground));
}

.tag-remove-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1rem;
    height: 1rem;
    padding: 0;
    margin-left: 0.125rem;
    background: transparent;
    border: none;
    border-radius: 50%;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    font-size: 0.875rem;
    line-height: 1;
    transition:
        background-color 0.2s,
        color 0.2s;
}

.tag-remove-btn:hover {
    background-color: hsl(var(--destructive));
    color: hsl(var(--destructive-foreground));
}

/* ── Link ── */
.learn-more-link {
    color: var(--color-accent, hsl(var(--primary)));
    text-decoration: none;
    font-weight: 500;
}

.learn-more-link:hover {
    text-decoration: underline;
}

/* ── Responsive ── */
@media (max-width: 640px) {
    .chat-config-page {
        padding: 1rem;
    }

    .page-header {
        flex-direction: column;
        align-items: flex-start;
    }

    .kb-grid {
        grid-template-columns: 1fr;
    }

    .form-row-2col {
        grid-template-columns: 1fr;
    }
}
</style>

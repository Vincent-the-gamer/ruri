<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useAcpStore } from "../stores/acp";
import { useKnowledgeBaseStore } from "../stores/knowledgeBase";
import { useToast } from "../composables/useToast";
import type { ProxyConfig, ProxyRuleType } from "../types";
import { ProxyRuleTypeLabels } from "../types";

const { t } = useI18n();
const acpStore = useAcpStore();
const kbStore = useKnowledgeBaseStore();
const toast = useToast();

const selectedProviderId = ref<string | null>(null);
const selectedSkillNames = ref<string[]>([]);
const selectedKbIds = ref<string[]>([]);
const proxyConfig = ref<ProxyConfig>({
    enabled: false,
    url: "",
    mode: "global",
    proxy_domains: [],
    bypass_domains: [],
    username: null,
    password: null,
    bypass_localhost: true,
    rules: [],
});
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
        proxyConfig.value = normalizeProxy(acpStore.config.proxy_config);
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
            proxy_config: { ...proxyConfig.value },
        });
        toast.success(t("acpConfig.saveSuccess"));
        saveSuccess.value = true;
        setTimeout(() => {
            saveSuccess.value = false;
        }, 3000);
    } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : t("errors.unknown");
        saveError.value = msg;
        toast.error(msg);
    }
}

const providerTypeLabel = (type: string) => {
    switch (type) {
        case "openai":
            return "OpenAI";
        case "anthropic":
            return "Anthropic";
        case "gemini":
            return "Gemini";
        case "siliconflow":
            return "SiliconFlow";
        case "deepseek":
            return "DeepSeek";
        case "custom":
            return "Custom";
        default:
            return type;
    }
};

function normalizeProxy(pc: ProxyConfig | null | undefined): ProxyConfig {
    return {
        enabled: pc?.enabled ?? false,
        url: pc?.url ?? "",
        mode: pc?.mode ?? "global",
        proxy_domains: [...(pc?.proxy_domains ?? [])],
        bypass_domains: [...(pc?.bypass_domains ?? [])],
        username: pc?.username ?? null,
        password: pc?.password ?? null,
        bypass_localhost: pc?.bypass_localhost ?? true,
        rules: [...(pc?.rules ?? [])],
    };
}

const hasChanges = computed(() => {
    if (!acpStore.config) return false;
    const origProxy = normalizeProxy(acpStore.config.proxy_config);
    return (
        selectedProviderId.value !== acpStore.config.active_provider_id ||
        JSON.stringify(selectedSkillNames.value) !==
            JSON.stringify(acpStore.config.active_skill_names) ||
        JSON.stringify(selectedKbIds.value) !==
            JSON.stringify(acpStore.config.active_knowledge_base_ids || []) ||
        JSON.stringify(proxyConfig.value) !== JSON.stringify(origProxy)
    );
});

// ─── Proxy Config Helpers ──────────────────────────────────────────
const proxyDomainInput = ref("");
const bypassDomainInput = ref("");
const newRuleType = ref<ProxyRuleType>("domain-suffix");
const newRuleValue = ref("");
const ruleTypeOptions: { value: ProxyRuleType; label: string }[] = [
    { value: "domain", label: ProxyRuleTypeLabels["domain"] },
    { value: "domain-suffix", label: ProxyRuleTypeLabels["domain-suffix"] },
    { value: "domain-keyword", label: ProxyRuleTypeLabels["domain-keyword"] },
    { value: "ip-cidr", label: ProxyRuleTypeLabels["ip-cidr"] },
    { value: "geoip", label: ProxyRuleTypeLabels["geoip"] },
    { value: "match", label: ProxyRuleTypeLabels["match"] },
];

function addProxyDomain() {
    const domain = proxyDomainInput.value.trim();
    if (domain && !proxyConfig.value.proxy_domains.includes(domain)) {
        proxyConfig.value.proxy_domains.push(domain);
    }
    proxyDomainInput.value = "";
}

function removeProxyDomain(index: number) {
    proxyConfig.value.proxy_domains.splice(index, 1);
}

function addBypassDomain() {
    const domain = bypassDomainInput.value.trim();
    if (domain && !proxyConfig.value.bypass_domains.includes(domain)) {
        proxyConfig.value.bypass_domains.push(domain);
    }
    bypassDomainInput.value = "";
}

function removeBypassDomain(index: number) {
    proxyConfig.value.bypass_domains.splice(index, 1);
}

function addRule() {
    const value = newRuleValue.value.trim();
    if (newRuleType.value === "match") {
        proxyConfig.value.rules.push({ rule_type: "match", value: "" });
    } else if (value) {
        proxyConfig.value.rules.push({
            rule_type: newRuleType.value,
            value,
        });
    }
    newRuleValue.value = "";
}

function removeRule(index: number) {
    proxyConfig.value.rules.splice(index, 1);
}

function getRuleTypeColor(type: ProxyRuleType) {
    const colors: Record<
        string,
        { color: string; borderColor: string; background: string }
    > = {
        domain: {
            color: "#3b82f6",
            borderColor: "#3b82f640",
            background: "#3b82f615",
        },
        "domain-suffix": {
            color: "#8b5cf6",
            borderColor: "#8b5cf640",
            background: "#8b5cf615",
        },
        "domain-keyword": {
            color: "#f59e0b",
            borderColor: "#f59e0b40",
            background: "#f59e0b15",
        },
        "ip-cidr": {
            color: "#ef4444",
            borderColor: "#ef444440",
            background: "#ef444415",
        },
        geoip: {
            color: "#10b981",
            borderColor: "#10b98140",
            background: "#10b98115",
        },
        match: {
            color: "#6b7280",
            borderColor: "#6b728040",
            background: "#6b728015",
        },
    };
    return colors[type] || colors.match;
}
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
                    {{ t("acpConfig.knowledgeBases") }}
                </h2>
                <p class="section-desc">
                    {{ t("acpConfig.selectKnowledgeBases") }}
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

            <!-- Proxy Config Section -->
            <section class="config-section">
                <h2 class="section-title">
                    {{ t("acpConfig.proxyConfig") }}
                </h2>
                <p class="section-desc">
                    {{ t("acpConfig.proxyConfigDesc") }}
                </p>

                <div class="toggle-row-prox">
                    <div class="toggle-info">
                        <span class="toggle-text">{{
                            t("acpConfig.proxyEnabled")
                        }}</span>
                        <span class="toggle-description">{{
                            t("acpConfig.proxyEnabledDesc")
                        }}</span>
                    </div>
                    <button
                        class="prox-toggle-switch"
                        :class="{
                            'prox-toggle-switch-active': proxyConfig.enabled,
                        }"
                        @click="
                            proxyConfig.enabled = !proxyConfig.enabled;
                            clearMessages();
                        "
                        role="switch"
                        :aria-checked="proxyConfig.enabled"
                    >
                        <span
                            class="prox-toggle-thumb"
                            :class="{
                                'prox-toggle-thumb-active': proxyConfig.enabled,
                            }"
                        ></span>
                    </button>
                </div>

                <template v-if="proxyConfig.enabled">
                    <div class="prox-form-grid">
                        <div class="prox-form-group">
                            <label class="prox-form-label">{{
                                t("acpConfig.proxyUrl")
                            }}</label>
                            <input
                                v-model="proxyConfig.url"
                                type="text"
                                class="prox-form-input"
                                :placeholder="
                                    t('acpConfig.proxyUrlPlaceholder')
                                "
                                @input="clearMessages()"
                            />
                        </div>
                        <div class="prox-form-group">
                            <label class="prox-form-label">{{
                                t("acpConfig.proxyMode")
                            }}</label>
                            <select
                                v-model="proxyConfig.mode"
                                class="prox-form-select"
                                @change="clearMessages()"
                            >
                                <option value="global">
                                    {{ t("acpConfig.proxyModeGlobal") }}
                                </option>
                                <option value="rules">
                                    {{ t("acpConfig.proxyModeRules") }}
                                </option>
                            </select>
                        </div>
                    </div>

                    <!-- Clash-style rules -->
                    <div class="prox-rules-section">
                        <label class="prox-form-label">{{
                            t("acpConfig.proxyRules")
                        }}</label>
                        <p class="prox-form-hint">
                            {{ t("acpConfig.proxyRulesDesc") }}
                        </p>
                        <div class="prox-rule-add-row">
                            <select
                                v-model="newRuleType"
                                class="prox-rule-type-select"
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
                                v-model="newRuleValue"
                                type="text"
                                class="prox-rule-value-input"
                                :disabled="newRuleType === 'match'"
                                :placeholder="
                                    newRuleType === 'match'
                                        ? t('acpConfig.proxyRuleMatchAll')
                                        : t(
                                              'acpConfig.proxyRuleValuePlaceholder',
                                          )
                                "
                                @keydown.enter="addRule"
                            />
                            <button
                                class="prox-rule-add-btn"
                                :disabled="
                                    newRuleType !== 'match' &&
                                    !newRuleValue.trim()
                                "
                                @click="addRule"
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
                                    <line x1="12" y1="5" x2="12" y2="19" />
                                    <line x1="5" y1="12" x2="19" y2="12" />
                                </svg>
                            </button>
                        </div>
                        <div
                            v-if="proxyConfig.rules.length"
                            class="prox-rules-list"
                        >
                            <div
                                v-for="(rule, index) in proxyConfig.rules"
                                :key="index"
                                class="prox-rule-item"
                            >
                                <span
                                    class="prox-rule-type-badge"
                                    :style="{
                                        color: getRuleTypeColor(rule.rule_type)
                                            .color,
                                        borderColor: getRuleTypeColor(
                                            rule.rule_type,
                                        ).borderColor,
                                        background: getRuleTypeColor(
                                            rule.rule_type,
                                        ).background,
                                    }"
                                >
                                    {{ ProxyRuleTypeLabels[rule.rule_type] }}
                                </span>
                                <span
                                    v-if="rule.rule_type !== 'match'"
                                    class="prox-rule-value"
                                    >{{ rule.value }}</span
                                >
                                <button
                                    class="prox-rule-remove-btn"
                                    @click="removeRule(index)"
                                >
                                    ✕
                                </button>
                            </div>
                        </div>
                        <p v-else class="prox-form-hint prox-form-hint--empty">
                            {{ t("acpConfig.proxyRulesEmpty") }}
                        </p>
                    </div>

                    <!-- Legacy domain lists -->
                    <template
                        v-if="
                            proxyConfig.mode === 'rules' &&
                            !proxyConfig.rules.length
                        "
                    >
                        <div class="prox-form-group">
                            <label class="prox-form-label">{{
                                t("acpConfig.proxyDomains")
                            }}</label>
                            <input
                                v-model="proxyDomainInput"
                                type="text"
                                class="prox-form-input"
                                :placeholder="
                                    t('acpConfig.proxyDomainsPlaceholder')
                                "
                                @keydown.enter="addProxyDomain"
                            />
                            <div
                                v-if="proxyConfig.proxy_domains.length"
                                class="prox-tag-list"
                            >
                                <span
                                    v-for="(
                                        domain, idx
                                    ) in proxyConfig.proxy_domains"
                                    :key="idx"
                                    class="prox-tag-item"
                                >
                                    {{ domain }}
                                    <button
                                        class="prox-tag-remove-btn"
                                        @click="removeProxyDomain(idx)"
                                    >
                                        ✕
                                    </button>
                                </span>
                            </div>
                        </div>
                        <div class="prox-form-group">
                            <label class="prox-form-label">{{
                                t("acpConfig.bypassDomains")
                            }}</label>
                            <input
                                v-model="bypassDomainInput"
                                type="text"
                                class="prox-form-input"
                                :placeholder="
                                    t('acpConfig.bypassDomainsPlaceholder')
                                "
                                @keydown.enter="addBypassDomain"
                            />
                            <div
                                v-if="proxyConfig.bypass_domains.length"
                                class="prox-tag-list"
                            >
                                <span
                                    v-for="(
                                        domain, idx
                                    ) in proxyConfig.bypass_domains"
                                    :key="idx"
                                    class="prox-tag-item"
                                >
                                    {{ domain }}
                                    <button
                                        class="prox-tag-remove-btn"
                                        @click="removeBypassDomain(idx)"
                                    >
                                        ✕
                                    </button>
                                </span>
                            </div>
                        </div>
                    </template>

                    <!-- Auth & localhost -->
                    <div class="prox-form-grid">
                        <div class="prox-form-group">
                            <label class="prox-form-label">{{
                                t("acpConfig.proxyUsername")
                            }}</label>
                            <input
                                v-model="proxyConfig.username"
                                type="text"
                                class="prox-form-input"
                                :placeholder="
                                    t('acpConfig.proxyUsernamePlaceholder')
                                "
                                @input="clearMessages()"
                            />
                        </div>
                        <div class="prox-form-group">
                            <label class="prox-form-label">{{
                                t("acpConfig.proxyPassword")
                            }}</label>
                            <input
                                v-model="proxyConfig.password"
                                type="password"
                                class="prox-form-input"
                                :placeholder="
                                    t('acpConfig.proxyPasswordPlaceholder')
                                "
                                @input="clearMessages()"
                            />
                        </div>
                    </div>
                    <div class="toggle-row-prox">
                        <div class="toggle-info">
                            <span class="toggle-text">{{
                                t("acpConfig.bypassLocalhost")
                            }}</span>
                            <span class="toggle-description">{{
                                t("acpConfig.bypassLocalhostDesc")
                            }}</span>
                        </div>
                        <button
                            class="prox-toggle-switch"
                            :class="{
                                'prox-toggle-switch-active':
                                    proxyConfig.bypass_localhost,
                            }"
                            @click="
                                proxyConfig.bypass_localhost =
                                    !proxyConfig.bypass_localhost;
                                clearMessages();
                            "
                            role="switch"
                            :aria-checked="proxyConfig.bypass_localhost"
                        >
                            <span
                                class="prox-toggle-thumb"
                                :class="{
                                    'prox-toggle-thumb-active':
                                        proxyConfig.bypass_localhost,
                                }"
                            ></span>
                        </button>
                    </div>
                </template>
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
    background-color: var(--color-success-soft);
    border: 1px solid rgba(34, 197, 94, 0.2);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    color: var(--color-success);
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

.type-badge--siliconflow {
    background-color: rgba(6, 182, 212, 0.1);
    color: #0891b2;
}

.type-badge--deepseek {
    background-color: rgba(59, 130, 246, 0.1);
    color: #2563eb;
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

/* ─── Proxy Config Styles ─────────────────────────────────────── */
.toggle-row-prox {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 0;
}

.prox-toggle-switch {
    position: relative;
    display: inline-block;
    width: 2.5rem;
    height: 1.375rem;
    border-radius: 999px;
    background: var(--color-bg-mute);
    border: 1px solid var(--color-border);
    cursor: pointer;
    transition: all var(--transition-fast);
    flex-shrink: 0;
}

.prox-toggle-switch-active {
    background: var(--color-accent);
    border-color: var(--color-accent);
}

.prox-toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 1rem;
    height: 1rem;
    border-radius: 50%;
    background: white;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
    transition: transform var(--transition-fast);
}

.prox-toggle-thumb-active {
    transform: translateX(1.125rem);
}

.prox-form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-top: 0.75rem;
}

.prox-form-group {
    margin-bottom: 0.75rem;
}

.prox-form-label {
    display: block;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-text);
    margin-bottom: 0.25rem;
}

.prox-form-input,
.prox-form-select {
    width: 100%;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    color: var(--color-text);
    font-size: 0.8125rem;
    transition: border-color var(--transition-fast);
}

.prox-form-input:focus,
.prox-form-select:focus {
    outline: none;
    border-color: var(--color-accent);
    box-shadow: 0 0 0 2px rgba(134, 59, 255, 0.1);
}

.prox-form-select {
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%236b7280' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.75rem center;
    padding-right: 2rem;
}

.prox-form-hint {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    margin: 0.25rem 0 0.5rem;
    line-height: 1.5;
}

.prox-form-hint--empty {
    font-style: italic;
    opacity: 0.7;
}

.prox-rules-section {
    margin-top: 0.75rem;
}

.prox-rule-add-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-top: 0.5rem;
}

.prox-rule-type-select {
    width: 10rem;
    padding: 0.4rem 0.625rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    color: var(--color-text);
    font-size: 0.75rem;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='10' viewBox='0 0 24 24' fill='none' stroke='%236b7280' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.5rem center;
    padding-right: 1.75rem;
    flex-shrink: 0;
}

.prox-rule-type-select:focus {
    outline: none;
    border-color: var(--color-accent);
}

.prox-rule-value-input {
    flex: 1;
    padding: 0.4rem 0.625rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    color: var(--color-text);
    font-size: 0.75rem;
}

.prox-rule-value-input::placeholder {
    color: var(--color-text-muted);
    opacity: 0.6;
}

.prox-rule-value-input:focus {
    outline: none;
    border-color: var(--color-accent);
}

.prox-rule-add-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    color: var(--color-text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
    flex-shrink: 0;
}

.prox-rule-add-btn:hover:not(:disabled) {
    border-color: var(--color-accent);
    color: var(--color-accent);
    background: var(--color-bg-soft);
}

.prox-rule-add-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
}

.prox-rules-list {
    margin-top: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
}

.prox-rule-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.5rem;
    background: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: 0.75rem;
}

.prox-rule-item:hover {
    border-color: var(--color-bg-hover);
}

.prox-rule-type-badge {
    font-size: 0.6875rem;
    font-weight: 600;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    border: 1px solid;
    letter-spacing: 0.025em;
    flex-shrink: 0;
}

.prox-rule-value {
    flex: 1;
    color: var(--color-text);
    font-family: monospace;
}

.prox-rule-remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 0.625rem;
    transition: all var(--transition-fast);
    flex-shrink: 0;
}

.prox-rule-remove-btn:hover {
    background: var(--color-danger-soft);
    color: var(--color-danger);
}

.prox-tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
    margin-top: 0.375rem;
}

.prox-tag-item {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.1875rem 0.5rem;
    background: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: 999px;
    font-size: 0.75rem;
    color: var(--color-text);
}

.prox-tag-remove-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 0.875rem;
    height: 0.875rem;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 0.5625rem;
    transition: all var(--transition-fast);
}

.prox-tag-remove-btn:hover {
    background: var(--color-danger-soft);
    color: var(--color-danger);
}
</style>

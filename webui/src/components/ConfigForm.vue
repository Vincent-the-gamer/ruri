<script setup lang="ts">
import { computed, watch, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useProviderStore } from "../stores/provider";
import { usePersonaStore } from "../stores/persona";
import { useSkillStore } from "../stores/skill";
import { usePlatformStore } from "../stores/platform";
import type {
    ConfigProfile,
    CreateConfigProfileRequest,
    UpdateConfigProfileRequest,
    ProxyConfig,
    ProxyMode,
    ProxyRule,
    ProxyRuleType,
} from "../types";
import { ProxyRuleTypeLabels } from "../types";

interface Props {
    config?: ConfigProfile | null;
    saving?: boolean;
}

interface Emits {
    (
        e: "save",
        data: CreateConfigProfileRequest | UpdateConfigProfileRequest,
    ): void;
    (e: "cancel"): void;
}

const props = withDefaults(defineProps<Props>(), {
    config: null,
    saving: false,
});

const emit = defineEmits<Emits>();

const { t } = useI18n();
const providerStore = useProviderStore();
const personaStore = usePersonaStore();
const skillStore = useSkillStore();
const platformStore = usePlatformStore();

// Form data
const formData = ref({
    name: "",
    description: "",
    enable: true,
    provider_id: null as string | null,
    persona_id: null as string | null,
    command_prefix: "/" as string,
    web_search_enabled: false,
    computer_use_enabled: false,
    acp_enabled: false,
    active_skill_names: [] as string[],
    active_platform_ids: [] as string[],
    proxy_config: {
        enabled: false,
        url: "",
        mode: "global" as ProxyMode,
        proxy_domains: [] as string[],
        bypass_domains: [] as string[],
        username: null,
        password: null,
        bypass_localhost: true,
        rules: [] as ProxyRule[],
    } as ProxyConfig,
});

// Initialize form when config changes
watch(
    () => props.config,
    (config) => {
        if (config) {
            formData.value = {
                name: config.name,
                description: config.description,
                enable: config.enable,
                provider_id: config.provider_id,
                persona_id: config.persona_id,
                command_prefix: config.command_prefix || "/",
                web_search_enabled: config.web_search_enabled,
                computer_use_enabled: config.computer_use_enabled,
                acp_enabled: config.acp_enabled,
                active_skill_names: [...config.active_skill_names],
                active_platform_ids: [...config.active_platform_ids],
                proxy_config: {
                    enabled: config.proxy_config?.enabled ?? false,
                    url: config.proxy_config?.url || "",
                    mode: config.proxy_config?.mode || "global",
                    proxy_domains: config.proxy_config?.proxy_domains
                        ? [...config.proxy_config.proxy_domains]
                        : [],
                    bypass_domains: config.proxy_config?.bypass_domains
                        ? [...config.proxy_config.bypass_domains]
                        : [],
                    username: config.proxy_config?.username || null,
                    password: config.proxy_config?.password || null,
                    bypass_localhost:
                        config.proxy_config?.bypass_localhost ?? true,
                    rules: config.proxy_config?.rules
                        ? [...config.proxy_config.rules]
                        : [],
                },
            };
        } else {
            // Reset for new config
            formData.value = {
                name: "",
                description: "",
                enable: true,
                provider_id: null,
                persona_id: null,
                command_prefix: "/",
                web_search_enabled: false,
                computer_use_enabled: false,
                acp_enabled: false,
                active_skill_names: [],
                active_platform_ids: [],
                proxy_config: {
                    enabled: false,
                    url: "",
                    mode: "global",
                    proxy_domains: [],
                    bypass_domains: [],
                    username: null,
                    password: null,
                    bypass_localhost: true,
                    rules: [],
                },
            };
        }
    },
    { immediate: true },
);

const isEdit = computed(() => props.config !== null);

// Proxy domain inputs
const proxyDomainInput = ref("");
const bypassDomainInput = ref("");

function addProxyDomain() {
    const domain = proxyDomainInput.value.trim();
    if (domain && !formData.value.proxy_config.proxy_domains.includes(domain)) {
        formData.value.proxy_config.proxy_domains.push(domain);
        proxyDomainInput.value = "";
    }
}

function removeProxyDomain(index: number) {
    formData.value.proxy_config.proxy_domains.splice(index, 1);
}

function addBypassDomain() {
    const domain = bypassDomainInput.value.trim();
    if (
        domain &&
        !formData.value.proxy_config.bypass_domains.includes(domain)
    ) {
        formData.value.proxy_config.bypass_domains.push(domain);
        bypassDomainInput.value = "";
    }
}

function removeBypassDomain(index: number) {
    formData.value.proxy_config.bypass_domains.splice(index, 1);
}

// Clash-style rule editing
const newRuleType = ref<ProxyRuleType>("domain-suffix");
const newRuleValue = ref("");

const ruleTypeOptions = computed(() => {
    return (Object.keys(ProxyRuleTypeLabels) as ProxyRuleType[]).map(
        (type) => ({
            value: type,
            label: ProxyRuleTypeLabels[type],
        }),
    );
});

function addRule() {
    const value = newRuleValue.value.trim();
    if (newRuleType.value === "match") {
        // MATCH rule doesn't need a value
        formData.value.proxy_config.rules.push({
            rule_type: "match",
            value: "",
        });
        newRuleValue.value = "";
        return;
    }
    if (!value) return;
    formData.value.proxy_config.rules.push({
        rule_type: newRuleType.value,
        value,
    });
    newRuleValue.value = "";
}

function removeRule(index: number) {
    formData.value.proxy_config.rules.splice(index, 1);
}

function formatRule(rule: ProxyRule): string {
    if (rule.rule_type === "match") return "MATCH";
    return `${ProxyRuleTypeLabels[rule.rule_type]},${rule.value}`;
}

function getRuleTypeColor(type: ProxyRuleType): string {
    const colors: Record<ProxyRuleType, string> = {
        domain: "hsl(var(--primary))",
        "domain-suffix": "hsl(220 70% 55%)",
        "domain-keyword": "hsl(280 60% 55%)",
        "ip-cidr": "hsl(30 80% 50%)",
        geoip: "hsl(150 60% 40%)",
        match: "hsl(var(--destructive))",
    };
    return colors[type];
}

const formTitle = computed(() =>
    isEdit.value ? t("config.form.editTitle") : t("config.form.createTitle"),
);

// Skill toggle
function toggleSkill(skillName: string) {
    const index = formData.value.active_skill_names.indexOf(skillName);
    if (index === -1) {
        formData.value.active_skill_names.push(skillName);
    } else {
        formData.value.active_skill_names.splice(index, 1);
    }
}

function isSkillActive(skillName: string): boolean {
    return formData.value.active_skill_names.includes(skillName);
}

// Platform toggle
function togglePlatform(platformId: string) {
    const index = formData.value.active_platform_ids.indexOf(platformId);
    if (index === -1) {
        formData.value.active_platform_ids.push(platformId);
    } else {
        formData.value.active_platform_ids.splice(index, 1);
    }
}

function isPlatformActive(platformId: string): boolean {
    return formData.value.active_platform_ids.includes(platformId);
}

function getPlatformStatus(platformId: string): string {
    const instance = platformStore.instances.find((p) => p.id === platformId);
    return instance?.status || "unknown";
}

function isPlatformRunning(platformId: string): boolean {
    return getPlatformStatus(platformId) === "running";
}

const restartingPlatformId = ref<string | null>(null);

async function handleRestartPlatform(platformId: string) {
    restartingPlatformId.value = platformId;
    try {
        await platformStore.restartInstance(platformId);
    } catch (e: unknown) {
        console.error("Failed to restart platform:", e);
    } finally {
        restartingPlatformId.value = null;
    }
}

function handleSubmit() {
    emit("save", formData.value);
}
</script>

<template>
    <div class="config-form">
        <div class="form-header">
            <h2 class="form-title">{{ formTitle }}</h2>
        </div>

        <div class="form-body">
            <!-- Basic Info -->
            <div class="form-section">
                <h3 class="section-title">{{ t("config.form.basicInfo") }}</h3>

                <div class="form-group">
                    <label class="form-label"
                        >{{ t("config.form.name") }} *</label
                    >
                    <input
                        v-model="formData.name"
                        type="text"
                        class="form-input"
                        :placeholder="t('config.form.namePlaceholder')"
                        required
                    />
                </div>

                <div class="form-group">
                    <label class="form-label">{{
                        t("config.form.description")
                    }}</label>
                    <textarea
                        v-model="formData.description"
                        class="form-textarea"
                        :placeholder="t('config.form.descriptionPlaceholder')"
                        rows="2"
                    />
                </div>

                <div class="toggle-group">
                    <label class="toggle-label">
                        <input
                            v-model="formData.enable"
                            type="checkbox"
                            class="toggle-input"
                        />
                        <span class="toggle-text">{{
                            t("config.form.enableProfile")
                        }}</span>
                        <span class="toggle-description">{{
                            t("config.form.enableProfileDesc")
                        }}</span>
                    </label>
                </div>

                <div class="form-group">
                    <label class="form-label">{{
                        t("config.form.commandPrefix")
                    }}</label>
                    <input
                        v-model="formData.command_prefix"
                        type="text"
                        class="form-input"
                        style="max-width: 120px"
                        :placeholder="'/'"
                    />
                    <p class="form-hint">
                        {{ t("config.form.commandPrefixDesc") }}
                    </p>
                </div>
            </div>

            <!-- Model Provider -->
            <div class="form-section">
                <h3 class="section-title">
                    {{ t("config.form.modelProvider") }}
                </h3>
                <div class="form-group">
                    <label class="form-label">{{
                        t("config.form.selectProvider")
                    }}</label>
                    <select v-model="formData.provider_id" class="form-select">
                        <option :value="null">
                            {{ t("config.form.noProvider") }}
                        </option>
                        <option
                            v-for="provider in providerStore.providers"
                            :key="provider.id"
                            :value="provider.id"
                        >
                            {{ provider.name }} ({{ provider.provider_type }})
                        </option>
                    </select>
                </div>
            </div>

            <!-- Persona -->
            <div class="form-section">
                <h3 class="section-title">{{ t("config.form.persona") }}</h3>
                <div class="form-group">
                    <label class="form-label">{{
                        t("config.form.selectPersona")
                    }}</label>
                    <select v-model="formData.persona_id" class="form-select">
                        <option :value="null">
                            {{ t("config.form.noPersona") }}
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

            <!-- Capabilities -->
            <div class="form-section">
                <h3 class="section-title">
                    {{ t("config.form.capabilities") }}
                </h3>

                <div class="toggle-group">
                    <label class="toggle-label">
                        <input
                            v-model="formData.web_search_enabled"
                            type="checkbox"
                            class="toggle-input"
                        />
                        <span class="toggle-text">{{
                            t("config.form.webSearch")
                        }}</span>
                        <span class="toggle-description">{{
                            t("config.form.webSearchDesc")
                        }}</span>
                    </label>
                </div>

                <div class="toggle-group">
                    <label class="toggle-label">
                        <input
                            v-model="formData.computer_use_enabled"
                            type="checkbox"
                            class="toggle-input"
                        />
                        <span class="toggle-text">{{
                            t("config.form.computerUse")
                        }}</span>
                        <span class="toggle-description">{{
                            t("config.form.computerUseDesc")
                        }}</span>
                    </label>
                </div>

                <div class="toggle-group">
                    <label class="toggle-label">
                        <input
                            v-model="formData.acp_enabled"
                            type="checkbox"
                            class="toggle-input"
                        />
                        <span class="toggle-text">{{
                            t("config.form.acp")
                        }}</span>
                        <span class="toggle-description">{{
                            t("config.form.acpDesc")
                        }}</span>
                    </label>
                </div>
            </div>

            <!-- Skills -->
            <div class="form-section">
                <h3 class="section-title">{{ t("config.form.skills") }}</h3>
                <div v-if="skillStore.skills.length === 0" class="no-skills">
                    {{ t("config.form.noSkillsAvailable") }}
                </div>
                <div v-else class="skills-grid">
                    <div
                        v-for="skill in skillStore.skills"
                        :key="skill.name"
                        :class="[
                            'skill-item',
                            { active: isSkillActive(skill.name) },
                        ]"
                        @click="toggleSkill(skill.name)"
                    >
                        <span class="skill-name">{{ skill.name }}</span>
                        <span class="skill-checkbox">
                            <svg
                                v-if="isSkillActive(skill.name)"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <polyline points="20 6 9 17 4 12" />
                            </svg>
                        </span>
                    </div>
                </div>
            </div>

            <!-- Platforms -->
            <div class="form-section">
                <h3 class="section-title">{{ t("config.form.platforms") }}</h3>
                <div
                    v-if="platformStore.instances.length === 0"
                    class="no-skills"
                >
                    {{ t("config.form.noPlatformsAvailable") }}
                </div>
                <div v-else class="skills-grid">
                    <div
                        v-for="platform in platformStore.instances"
                        :key="platform.id"
                        :class="[
                            'skill-item',
                            { active: isPlatformActive(platform.id) },
                        ]"
                        @click="togglePlatform(platform.id)"
                    >
                        <span class="platform-info">
                            <span
                                :class="[
                                    'platform-status-dot',
                                    {
                                        'platform-status-dot--running':
                                            isPlatformRunning(platform.id),
                                        'platform-status-dot--stopped':
                                            !isPlatformRunning(platform.id),
                                    },
                                ]"
                                :title="
                                    isPlatformRunning(platform.id)
                                        ? t('common.active')
                                        : t('common.inactive')
                                "
                            ></span>
                            <span class="skill-name"
                                >{{ platform.id }} ({{
                                    platform.platform_type
                                }})</span
                            >
                        </span>
                        <span class="platform-actions">
                            <button
                                v-if="
                                    isPlatformActive(platform.id) &&
                                    isPlatformRunning(platform.id)
                                "
                                class="platform-restart-btn"
                                :disabled="restartingPlatformId === platform.id"
                                @click.stop="handleRestartPlatform(platform.id)"
                                :title="t('config.form.restartPlatform')"
                            >
                                <svg
                                    :class="{
                                        spinning:
                                            restartingPlatformId ===
                                            platform.id,
                                    }"
                                    width="14"
                                    height="14"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <polyline points="23 4 23 10 17 10" />
                                    <path
                                        d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"
                                    />
                                </svg>
                            </button>
                            <span class="skill-checkbox">
                                <svg
                                    v-if="isPlatformActive(platform.id)"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                >
                                    <polyline points="20 6 9 17 4 12" />
                                </svg>
                            </span>
                        </span>
                    </div>
                </div>
            </div>

            <!-- Proxy Configuration -->
            <div class="form-section">
                <h3 class="section-title">
                    {{ t("config.form.proxyConfig") }}
                </h3>

                <div class="toggle-group">
                    <label class="toggle-label">
                        <input
                            v-model="formData.proxy_config.enabled"
                            type="checkbox"
                            class="toggle-input"
                        />
                        <span class="toggle-text">{{
                            t("config.form.proxyEnabled")
                        }}</span>
                        <span class="toggle-description">{{
                            t("config.form.proxyEnabledDesc")
                        }}</span>
                    </label>
                </div>

                <template v-if="formData.proxy_config.enabled">
                    <div class="form-group">
                        <label class="form-label">{{
                            t("config.form.proxyUrl")
                        }}</label>
                        <input
                            v-model="formData.proxy_config.url"
                            type="text"
                            class="form-input"
                            :placeholder="t('config.form.proxyUrlPlaceholder')"
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label">{{
                            t("config.form.proxyMode")
                        }}</label>
                        <select
                            v-model="formData.proxy_config.mode"
                            class="form-select"
                        >
                            <option value="global">
                                {{ t("config.form.proxyModeGlobal") }}
                            </option>
                            <option value="rules">
                                {{ t("config.form.proxyModeRules") }}
                            </option>
                        </select>
                    </div>

                    <!-- Clash-style Rules Editor -->
                    <div
                        v-if="formData.proxy_config.mode === 'rules'"
                        class="form-group"
                    >
                        <label class="form-label">{{
                            t("config.form.proxyRules")
                        }}</label>
                        <p class="form-hint">
                            {{ t("config.form.proxyRulesDesc") }}
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
                                    t('config.form.proxyRuleValuePlaceholder')
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
                            v-if="formData.proxy_config.rules.length > 0"
                            class="rules-list"
                        >
                            <div
                                v-for="(rule, index) in formData.proxy_config
                                    .rules"
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
                                        ? t("config.form.proxyRuleMatchAll")
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
                        <p v-else class="form-hint form-hint--empty">
                            {{ t("config.form.proxyRulesEmpty") }}
                        </p>
                    </div>

                    <!-- Legacy domain lists (only show in rules mode when no rules exist, or in global mode) -->
                    <template
                        v-if="
                            formData.proxy_config.mode === 'rules' &&
                            formData.proxy_config.rules.length === 0
                        "
                    >
                        <div class="form-group">
                            <label class="form-label">{{
                                t("config.form.proxyDomains")
                            }}</label>
                            <input
                                v-model="proxyDomainInput"
                                type="text"
                                class="form-input"
                                :placeholder="
                                    t('config.form.proxyDomainsPlaceholder')
                                "
                                @keyup.enter="addProxyDomain"
                            />
                            <div
                                v-if="
                                    formData.proxy_config.proxy_domains.length >
                                    0
                                "
                                class="bypass-hosts-list"
                            >
                                <span
                                    v-for="(domain, index) in formData
                                        .proxy_config.proxy_domains"
                                    :key="index"
                                    class="bypass-host-tag"
                                >
                                    {{ domain }}
                                    <button
                                        type="button"
                                        class="remove-host-btn"
                                        @click="removeProxyDomain(index)"
                                    >
                                        ×
                                    </button>
                                </span>
                            </div>
                        </div>
                    </template>

                    <div class="form-group">
                        <label class="form-label">{{
                            t("config.form.bypassDomains")
                        }}</label>
                        <input
                            v-model="bypassDomainInput"
                            type="text"
                            class="form-input"
                            :placeholder="
                                t('config.form.bypassDomainsPlaceholder')
                            "
                            @keyup.enter="addBypassDomain"
                        />
                        <div
                            v-if="
                                formData.proxy_config.bypass_domains.length > 0
                            "
                            class="bypass-hosts-list"
                        >
                            <span
                                v-for="(domain, index) in formData.proxy_config
                                    .bypass_domains"
                                :key="index"
                                class="bypass-host-tag"
                            >
                                {{ domain }}
                                <button
                                    type="button"
                                    class="remove-host-btn"
                                    @click="removeBypassDomain(index)"
                                >
                                    ×
                                </button>
                            </span>
                        </div>
                    </div>

                    <div class="form-group">
                        <label class="form-label">{{
                            t("config.form.proxyUsername")
                        }}</label>
                        <input
                            v-model="formData.proxy_config.username"
                            type="text"
                            class="form-input"
                            :placeholder="
                                t('config.form.proxyUsernamePlaceholder')
                            "
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label">{{
                            t("config.form.proxyPassword")
                        }}</label>
                        <input
                            v-model="formData.proxy_config.password"
                            type="password"
                            class="form-input"
                            :placeholder="
                                t('config.form.proxyPasswordPlaceholder')
                            "
                        />
                    </div>

                    <div class="toggle-group">
                        <label class="toggle-label">
                            <input
                                v-model="formData.proxy_config.bypass_localhost"
                                type="checkbox"
                                class="toggle-input"
                            />
                            <span class="toggle-text">{{
                                t("config.form.bypassLocalhost")
                            }}</span>
                            <span class="toggle-description">{{
                                t("config.form.bypassLocalhostDesc")
                            }}</span>
                        </label>
                    </div>
                </template>
            </div>
        </div>

        <div class="form-footer">
            <button
                type="button"
                class="btn btn-ghost"
                @click="$emit('cancel')"
            >
                {{ t("common.cancel") }}
            </button>
            <button
                type="button"
                class="btn btn-accent"
                :disabled="!formData.name || saving"
                @click="handleSubmit"
            >
                {{ saving ? t("common.loading") : t("common.save") }}
            </button>
        </div>
    </div>
</template>

<style scoped>
.config-form {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 120px);
}

.form-header {
    padding: 1.5rem;
    border-bottom: 1px solid hsl(var(--border));
}

.form-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

.form-body {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
}

.form-section {
    margin-bottom: 2rem;
}

.form-section:last-child {
    margin-bottom: 0;
}

.section-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin-bottom: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.form-group {
    margin-bottom: 1.25rem;
}

.form-label {
    display: block;
    font-size: 0.875rem;
    font-weight: 500;
    color: hsl(var(--foreground));
    margin-bottom: 0.5rem;
}

.form-input,
.form-textarea,
.form-select {
    width: 100%;
    padding: 0.625rem 0.875rem;
    font-size: 0.875rem;
    line-height: 1.5;
    color: hsl(var(--foreground));
    background-color: hsl(var(--background));
    border: 1px solid hsl(var(--border));
    border-radius: 0.5rem;
    transition: all 0.2s;
    font-family: inherit;
}

.form-input:focus,
.form-textarea:focus,
.form-select:focus {
    outline: none;
    border-color: hsl(var(--primary));
    box-shadow: 0 0 0 3px hsl(var(--primary) / 0.1);
}

.form-textarea {
    resize: vertical;
    min-height: 80px;
}

.toggle-group {
    margin-bottom: 1rem;
    padding: 0.875rem;
    border: 1px solid hsl(var(--border));
    border-radius: 0.5rem;
    background: hsl(var(--background));
    transition: all 0.2s;
}

.toggle-group:hover {
    border-color: hsl(var(--primary) / 0.5);
}

.toggle-label {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    cursor: pointer;
}

/* Proxy Configuration Styles */

.form-hint {
    font-size: 0.8125rem;
    color: hsl(var(--muted-foreground));
    margin: 0.375rem 0 0.75rem;
    line-height: 1.4;
}

.form-hint--empty {
    font-style: italic;
    opacity: 0.7;
}

/* Clash-style Rule Editor */
.rule-add-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.75rem;
}

.rule-type-select {
    flex-shrink: 0;
    width: auto;
    min-width: 9rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.8125rem;
    font-family:
        ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
    font-weight: 600;
    background: hsl(var(--secondary));
    color: hsl(var(--foreground));
    border: 1px solid hsl(var(--border));
    border-radius: 0.5rem;
    cursor: pointer;
    transition: all 0.2s;
    appearance: auto;
}

.rule-type-select:focus {
    border-color: hsl(var(--primary));
    outline: none;
    box-shadow: 0 0 0 2px hsl(var(--primary) / 0.2);
}

.rule-value-input {
    flex: 1;
    min-width: 0;
    padding: 0.5rem 0.75rem;
    font-size: 0.8125rem;
    background: hsl(var(--background));
    color: hsl(var(--foreground));
    border: 1px solid hsl(var(--border));
    border-radius: 0.5rem;
    transition: all 0.2s;
}

.rule-value-input::placeholder {
    color: hsl(var(--muted-foreground) / 0.6);
}

.rule-value-input:focus {
    border-color: hsl(var(--primary));
    outline: none;
    box-shadow: 0 0 0 2px hsl(var(--primary) / 0.2);
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
    gap: 0.5rem;
    margin-top: 0.5rem;
}

.rule-item {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.5rem 0.75rem;
    background: hsl(var(--secondary) / 0.5);
    border: 1px solid hsl(var(--border) / 0.5);
    border-radius: 0.5rem;
    transition: all 0.2s;
}

.rule-item:hover {
    border-color: hsl(var(--border));
    background: hsl(var(--secondary));
}

.rule-type-badge {
    flex-shrink: 0;
    padding: 0.125rem 0.5rem;
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

.bypass-hosts-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.5rem;
}

.bypass-host-tag {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    background-color: hsl(var(--muted));
    border-radius: 0.25rem;
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
}

.remove-host-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    padding: 0;
    margin-left: 0.25rem;
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

.remove-host-btn:hover {
    background-color: hsl(var(--destructive));
    color: hsl(var(--destructive-foreground));
}

.toggle-input {
    margin-top: 0.25rem;
    width: 1rem;
    height: 1rem;
    accent-color: hsl(var(--primary));
    cursor: pointer;
}

.toggle-text {
    font-weight: 500;
    color: hsl(var(--foreground));
    flex-shrink: 0;
}

.toggle-description {
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
    margin-left: 0.25rem;
}

.no-skills {
    padding: 1rem;
    text-align: center;
    color: hsl(var(--muted-foreground));
    font-size: 0.875rem;
}

.skills-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 0.5rem;
}

.skill-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.625rem 0.875rem;
    font-size: 0.875rem;
    border: 1px solid hsl(var(--border));
    border-radius: 0.5rem;
    background: hsl(var(--background));
    cursor: pointer;
    transition: all 0.2s;
}

.skill-item:hover {
    border-color: hsl(var(--primary) / 0.5);
    background: hsl(var(--secondary));
}

.skill-item.active {
    border-color: hsl(var(--primary));
    background: hsl(var(--primary) / 0.1);
}

.skill-name {
    font-weight: 500;
    color: hsl(var(--foreground));
}

.skill-checkbox {
    width: 1.25rem;
    height: 1.25rem;
    color: hsl(var(--primary));
    flex-shrink: 0;
}

.platform-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    overflow: hidden;
}

.platform-status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
}

.platform-status-dot--running {
    background: hsl(142 71% 45%);
    box-shadow: 0 0 6px hsl(142 71% 45% / 0.5);
}

.platform-status-dot--stopped {
    background: hsl(var(--muted-foreground) / 0.4);
}

.platform-actions {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    flex-shrink: 0;
}

.platform-restart-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    padding: 0;
    border: none;
    border-radius: 0.25rem;
    background: transparent;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s;
}

.platform-restart-btn:hover:not(:disabled) {
    background: hsl(var(--secondary));
    color: hsl(var(--foreground));
}

.platform-restart-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.platform-restart-btn svg.spinning {
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

.form-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 1.5rem;
    border-top: 1px solid hsl(var(--border));
}

.btn {
    padding: 0.625rem 1.25rem;
    font-size: 0.875rem;
    font-weight: 500;
    border-radius: 0.5rem;
    border: none;
    cursor: pointer;
    transition: all 0.2s;
}

.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-ghost {
    background: transparent;
    color: hsl(var(--foreground));
    border: 1px solid hsl(var(--border));
}

.btn-ghost:hover:not(:disabled) {
    background: hsl(var(--secondary));
}

.btn-accent {
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
}

.btn-accent:hover:not(:disabled) {
    background: hsl(var(--primary) / 0.9);
}

/* Scrollbar */
.form-body::-webkit-scrollbar {
    width: 6px;
}

.form-body::-webkit-scrollbar-track {
    background: transparent;
}

.form-body::-webkit-scrollbar-thumb {
    background: hsl(var(--muted));
    border-radius: 3px;
}

.form-body::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--muted-foreground) / 0.5);
}
</style>

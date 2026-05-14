<script setup lang="ts">
import { ref, watch, computed, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { useProviderStore } from "../stores/provider";
import { usePersonaStore } from "../stores/persona";
import { useSkillStore } from "../stores/skill";
import { usePlatformStore } from "../stores/platform";
import { useKnowledgeBaseStore } from "../stores/knowledgeBase";
import { useConfigStore } from "../stores/config";
import { getBuiltinCommands } from "../api";
import {
    type ProxyRuleType,
    type ProxyMode,
    ProxyRuleTypeLabels,
    type ConfigProfile,
    type ProxyRule,
    type BuiltinCommand,
} from "../types";

interface Props {
    config?: ConfigProfile | null;
    saving?: boolean;
}

interface Emits {
    (e: "save", data: Partial<ConfigProfile>): void;
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
const configStore = useConfigStore();
const kbStore = useKnowledgeBaseStore();

const formData = ref<{
    name: string;
    description: string;
    enable: boolean;
    provider_id: string | null;
    persona_id: string | null;
    command_prefix: string;
    enabled_commands: string[];
    command_admin_required: Record<string, boolean>;
    custom_error_message: string;
    web_search_enabled: boolean;
    computer_use_enabled: boolean;
    active_skill_names: string[];
    active_knowledge_base_ids: string[];
    platform_ids: string[];
    proxy_config: {
        enabled: boolean;
        url: string;
        mode: ProxyMode;
        proxy_domains: string[];
        bypass_domains: string[];
        username: string | null;
        password: string | null;
        bypass_localhost: boolean;
        rules: ProxyRule[];
    };
}>({
    name: "",
    description: "",
    enable: true,
    provider_id: null,
    persona_id: null,
    command_prefix: "/",
    enabled_commands: [],
    command_admin_required: {},
    custom_error_message: "",
    web_search_enabled: false,
    computer_use_enabled: false,
    active_skill_names: [],
    active_knowledge_base_ids: [],
    platform_ids: [],
    proxy_config: {
        enabled: false,
        url: "",
        mode: "global",
        proxy_domains: [],
        bypass_domains: [],
        username: "",
        password: "",
        bypass_localhost: true,
        rules: [],
    },
});

watch(
    () => props.config,
    (newConfig) => {
        if (newConfig) {
            formData.value = {
                name: newConfig.name,
                description: newConfig.description || "",
                enable: newConfig.enable ?? true,
                provider_id: newConfig.provider_id || null,
                persona_id: newConfig.persona_id || null,
                command_prefix: newConfig.command_prefix || "/",
                enabled_commands: [...(newConfig.enabled_commands || [])],
                command_admin_required: {
                    ...(newConfig.command_admin_required || {}),
                },
                custom_error_message: newConfig.custom_error_message || "",
                web_search_enabled: newConfig.web_search_enabled ?? false,
                computer_use_enabled: newConfig.computer_use_enabled ?? false,
                active_skill_names: [...(newConfig.active_skill_names || [])],
                active_knowledge_base_ids: [
                    ...(newConfig.active_knowledge_base_ids || []),
                ],
                platform_ids: [...(newConfig.platform_ids || [])],
                proxy_config: {
                    enabled: newConfig.proxy_config?.enabled ?? false,
                    url: newConfig.proxy_config?.url || "",
                    mode: newConfig.proxy_config?.mode || "global",
                    proxy_domains: [
                        ...(newConfig.proxy_config?.proxy_domains || []),
                    ],
                    bypass_domains: [
                        ...(newConfig.proxy_config?.bypass_domains || []),
                    ],
                    username: newConfig.proxy_config?.username || "",
                    password: newConfig.proxy_config?.password || "",
                    bypass_localhost:
                        newConfig.proxy_config?.bypass_localhost ?? true,
                    rules: [...(newConfig.proxy_config?.rules || [])],
                },
            };
        } else {
            formData.value = {
                name: "",
                description: "",
                enable: true,
                provider_id: null,
                persona_id: null,
                command_prefix: "/",
                enabled_commands: [],
                command_admin_required: {},
                custom_error_message: "",
                web_search_enabled: false,
                computer_use_enabled: false,
                active_skill_names: [],
                active_knowledge_base_ids: [],
                platform_ids: [],
                proxy_config: {
                    enabled: false,
                    url: "",
                    mode: "global",
                    proxy_domains: [],
                    bypass_domains: [],
                    username: "",
                    password: "",
                    bypass_localhost: true,
                    rules: [],
                },
            };
        }
    },
    { immediate: true },
);

// Built-in commands for the command configuration section
const builtinCommands = ref<BuiltinCommand[]>([]);
const commandsLoading = ref(false);

onMounted(async () => {
    commandsLoading.value = true;
    try {
        builtinCommands.value = await getBuiltinCommands();
    } catch {
        // Silently fail - commands are not critical for form rendering
    } finally {
        commandsLoading.value = false;
    }
});

const visibleCommands = computed(() =>
    builtinCommands.value.filter((c) => !c.hidden),
);

function isCommandEnabled(name: string): boolean {
    return formData.value.enabled_commands.includes(name);
}

function toggleCommand(name: string) {
    const index = formData.value.enabled_commands.indexOf(name);
    if (index >= 0) {
        formData.value.enabled_commands.splice(index, 1);
    } else {
        formData.value.enabled_commands.push(name);
    }
}

const allCommandsSelected = computed(
    () =>
        visibleCommands.value.length > 0 &&
        formData.value.enabled_commands.length === visibleCommands.value.length,
);
const noCommandsSelected = computed(
    () => formData.value.enabled_commands.length === 0,
);

function selectAllCommands() {
    formData.value.enabled_commands = visibleCommands.value.map((c) => c.name);
}

function deselectAllCommands() {
    formData.value.enabled_commands = [];
}

function toggleCommandAdmin(cmd: BuiltinCommand) {
    if (formData.value.command_admin_required[cmd.name] === undefined) {
        // First click: set to opposite of default
        formData.value.command_admin_required[cmd.name] =
            !cmd.default_require_admin;
    } else {
        // Already overridden, toggle
        formData.value.command_admin_required[cmd.name] =
            !formData.value.command_admin_required[cmd.name];
    }
}

function getEffectiveAdminRequired(cmd: BuiltinCommand): boolean {
    return (
        formData.value.command_admin_required[cmd.name] ??
        cmd.default_require_admin
    );
}

const isEdit = computed(() => !!props.config);

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

const newRuleType = ref<ProxyRuleType>("domain");
const newRuleValue = ref("");

const ruleTypeOptions = computed(() => [
    { value: "domain", label: ProxyRuleTypeLabels.domain },
    { value: "domain-suffix", label: ProxyRuleTypeLabels["domain-suffix"] },
    {
        value: "domain-keyword",
        label: ProxyRuleTypeLabels["domain-keyword"],
    },
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
        formData.value.proxy_config.rules.push(rule);
        newRuleValue.value = "";
        if (newRuleType.value === "match") {
            newRuleType.value = "domain";
        }
    }
}

function removeRule(index: number) {
    formData.value.proxy_config.rules.splice(index, 1);
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

const formTitle = computed(() =>
    isEdit.value ? t("config.form.editTitle") : t("config.form.createTitle"),
);

function toggleSkill(name: string) {
    const index = formData.value.active_skill_names.indexOf(name);
    if (index >= 0) {
        formData.value.active_skill_names.splice(index, 1);
    } else {
        formData.value.active_skill_names.push(name);
    }
}

function isSkillActive(name: string) {
    return formData.value.active_skill_names.includes(name);
}

const allSkillsSelected = computed(
    () =>
        skillStore.skills.length > 0 &&
        formData.value.active_skill_names.length === skillStore.skills.length,
);
const noSkillsSelected = computed(
    () => formData.value.active_skill_names.length === 0,
);

function selectAllSkills() {
    formData.value.active_skill_names = skillStore.skills.map((s) => s.name);
}

function deselectAllSkills() {
    formData.value.active_skill_names = [];
}

function togglePlatform(id: string) {
    const index = formData.value.platform_ids.indexOf(id);
    if (index >= 0) {
        formData.value.platform_ids.splice(index, 1);
    } else {
        // Enforce single-platform-per-profile: when selecting a new platform,
        // clear any previously selected platform to prevent conflicts
        formData.value.platform_ids = [id];
    }
}

function isPlatformActive(id: string) {
    return formData.value.platform_ids.includes(id);
}

/**
 * Whether a platform should be disabled for selection.
 * A platform is disabled when:
 * 1. It is used by another config profile (exclusive ownership), OR
 * 2. A different platform is already selected in this profile (single-platform constraint)
 */
function isPlatformDisabled(id: string): boolean {
    // If this platform is already selected, it's never disabled
    if (isPlatformActive(id)) return false;
    // If a different platform is already selected, disable all others
    if (formData.value.platform_ids.length > 0) return true;
    // If used by another profile, disable
    return isPlatformUsedByOtherProfile(id);
}

function toggleKb(id: string) {
    const index = formData.value.active_knowledge_base_ids.indexOf(id);
    if (index >= 0) {
        formData.value.active_knowledge_base_ids.splice(index, 1);
    } else {
        formData.value.active_knowledge_base_ids.push(id);
    }
}

function isKbActive(id: string) {
    return formData.value.active_knowledge_base_ids.includes(id);
}

function getPlatformStatus(id: string) {
    const instance = platformStore.instances.find((i) => i.id === id);
    return instance?.status || "stopped";
}

function isPlatformRunning(id: string) {
    return getPlatformStatus(id) === "running";
}

function isPlatformUsedByOtherProfile(id: string): boolean {
    // Check if any OTHER profile uses this platform
    const profile = configStore.configProfiles.find(
        (p) =>
            (!props.config || p.id !== props.config.id) &&
            (p.platform_ids || []).includes(id),
    );
    return !!profile;
}

function getPlatformUsedByProfileName(id: string): string | null {
    if (isPlatformUsedByOtherProfile(id)) {
        // When creating new config, find any profile using this platform
        // When editing, exclude the current config from the search
        const profile = configStore.configProfiles.find(
            (p) =>
                (!props.config || p.id !== props.config.id) &&
                (p.platform_ids || []).includes(id),
        );
        return profile?.name || null;
    }
    return null;
}

const restartingPlatformId = ref<string | null>(null);

async function handleRestartPlatform(platformId: string) {
    restartingPlatformId.value = platformId;
    try {
        await platformStore.restartInstance(platformId);
    } finally {
        restartingPlatformId.value = null;
    }
}

function handleSubmit() {
    emit("save", { ...formData.value });
}
</script>

<template>
    <div class="config-form">
        <div class="form-header">
            <h2 class="form-title">{{ formTitle }}</h2>
            <button class="btn-close" @click="$emit('cancel')">
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

        <div class="form-body">
            <!-- Basic Info -->
            <div class="form-section">
                <h3 class="section-title">{{ t("config.form.basicInfo") }}</h3>

                <div class="form-grid">
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
                            t("config.form.customErrorMessage")
                        }}</label>
                        <input
                            v-model="formData.custom_error_message"
                            type="text"
                            class="form-input"
                            :placeholder="
                                t('config.form.customErrorMessagePlaceholder')
                            "
                        />
                        <p class="form-hint">
                            {{ t("config.form.customErrorMessageHint") }}
                        </p>
                    </div>
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

                <div class="toggle-row">
                    <div class="toggle-info">
                        <span class="toggle-text">{{
                            t("config.form.enableProfile")
                        }}</span>
                        <span class="toggle-description">{{
                            t("config.form.enableProfileDesc")
                        }}</span>
                    </div>
                    <button
                        type="button"
                        class="toggle-switch"
                        :class="{ 'toggle-switch-active': formData.enable }"
                        @click="formData.enable = !formData.enable"
                    >
                        <span
                            class="toggle-thumb"
                            :class="{
                                'toggle-thumb-active': formData.enable,
                            }"
                        ></span>
                    </button>
                </div>
            </div>

            <!-- Model Provider & Persona -->
            <div class="form-section">
                <h3 class="section-title">
                    {{ t("config.form.modelProvider") }} &
                    {{ t("config.form.persona") }}
                </h3>
                <div class="form-grid">
                    <div class="form-group">
                        <label class="form-label">{{
                            t("config.form.selectProvider")
                        }}</label>
                        <select
                            v-model="formData.provider_id"
                            class="form-select"
                        >
                            <option :value="null">
                                {{ t("config.form.noProvider") }}
                            </option>
                            <option
                                v-for="provider in providerStore.providers"
                                :key="provider.id"
                                :value="provider.id"
                            >
                                {{ provider.name }} ({{
                                    provider.provider_type
                                }})
                            </option>
                        </select>
                    </div>
                    <div class="form-group">
                        <label class="form-label">{{
                            t("config.form.selectPersona")
                        }}</label>
                        <select
                            v-model="formData.persona_id"
                            class="form-select"
                        >
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
            </div>

            <!-- Capabilities -->
            <div class="form-section">
                <h3 class="section-title">
                    {{ t("config.form.capabilities") }}
                </h3>
                <div class="toggles-grid">
                    <div class="toggle-row">
                        <div class="toggle-info">
                            <span class="toggle-text">{{
                                t("config.form.webSearch")
                            }}</span>
                            <span class="toggle-description">{{
                                t("config.form.webSearchDesc")
                            }}</span>
                        </div>
                        <button
                            type="button"
                            class="toggle-switch"
                            :class="{
                                'toggle-switch-active':
                                    formData.web_search_enabled,
                            }"
                            @click="
                                formData.web_search_enabled =
                                    !formData.web_search_enabled
                            "
                        >
                            <span
                                class="toggle-thumb"
                                :class="{
                                    'toggle-thumb-active':
                                        formData.web_search_enabled,
                                }"
                            ></span>
                        </button>
                    </div>

                    <div class="toggle-row">
                        <div class="toggle-info">
                            <span class="toggle-text">{{
                                t("config.form.computerUse")
                            }}</span>
                            <span class="toggle-description">{{
                                t("config.form.computerUseDesc")
                            }}</span>
                        </div>
                        <button
                            type="button"
                            class="toggle-switch"
                            :class="{
                                'toggle-switch-active':
                                    formData.computer_use_enabled,
                            }"
                            @click="
                                formData.computer_use_enabled =
                                    !formData.computer_use_enabled
                            "
                        >
                            <span
                                class="toggle-thumb"
                                :class="{
                                    'toggle-thumb-active':
                                        formData.computer_use_enabled,
                                }"
                            ></span>
                        </button>
                    </div>
                </div>
            </div>

            <!-- Commands -->
            <div class="form-section">
                <h3 class="section-title">
                    {{ t("config.form.commands") }}
                </h3>
                <div class="form-grid">
                    <div class="form-group">
                        <label class="form-label">{{
                            t("config.form.commandPrefix")
                        }}</label>
                        <input
                            v-model="formData.command_prefix"
                            type="text"
                            class="form-input"
                            :placeholder="'/'"
                        />
                        <p class="form-hint">
                            {{ t("config.form.commandPrefixHint") }}
                        </p>
                    </div>
                </div>
                <div
                    v-if="visibleCommands.length > 0"
                    class="kb-actions"
                    style="margin-top: 0.75rem"
                >
                    <button
                        class="btn btn-sm btn-outline"
                        @click="selectAllCommands"
                        :disabled="allCommandsSelected"
                    >
                        {{ t("config.form.selectAllCommands") }}
                    </button>
                    <button
                        class="btn btn-sm btn-outline"
                        @click="deselectAllCommands"
                        :disabled="noCommandsSelected"
                    >
                        {{ t("config.form.deselectAllCommands") }}
                    </button>
                </div>
                <div
                    v-if="commandsLoading"
                    class="no-items"
                    style="margin-top: 0.75rem"
                >
                    {{ t("config.form.loadingCommands") }}
                </div>
                <div
                    v-else-if="visibleCommands.length === 0"
                    class="no-items"
                    style="margin-top: 0.75rem"
                >
                    {{ t("config.form.noCommands") }}
                </div>
                <div v-else class="items-grid" style="margin-top: 0.75rem">
                    <div
                        v-for="cmd in visibleCommands"
                        :key="cmd.name"
                        class="item-card"
                        :class="{ active: isCommandEnabled(cmd.name) }"
                        @click="toggleCommand(cmd.name)"
                    >
                        <span class="item-name">
                            <span class="cmd-name"
                                >{{ formData.command_prefix || "/"
                                }}{{ cmd.name }}</span
                            >
                            <span class="cmd-desc">{{ cmd.description }}</span>
                        </span>
                        <span class="cmd-right">
                            <button
                                v-if="isCommandEnabled(cmd.name)"
                                class="admin-toggle-btn"
                                :class="{
                                    'admin-toggle-btn--admin':
                                        getEffectiveAdminRequired(cmd),
                                    'admin-toggle-btn--open':
                                        !getEffectiveAdminRequired(cmd),
                                }"
                                @click.stop="toggleCommandAdmin(cmd)"
                                :title="
                                    getEffectiveAdminRequired(cmd)
                                        ? t('config.form.requireAdmin')
                                        : t('config.form.openToAll')
                                "
                            >
                                {{
                                    getEffectiveAdminRequired(cmd) ? "🔒" : "🌐"
                                }}
                            </button>
                            <span class="item-checkbox">
                                <svg
                                    v-if="isCommandEnabled(cmd.name)"
                                    width="14"
                                    height="14"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="3"
                                >
                                    <polyline points="20 6 9 17 4 12" />
                                </svg>
                            </span>
                        </span>
                    </div>
                </div>
                <p class="form-hint" style="margin-top: 0.5rem">
                    {{ t("config.form.commandsHint") }}
                </p>
            </div>

            <!-- Skills -->
            <div class="form-section">
                <h3 class="section-title">{{ t("config.form.skills") }}</h3>
                <div v-if="skillStore.skills.length > 0" class="kb-actions">
                    <button
                        class="btn btn-sm btn-outline"
                        @click="selectAllSkills"
                        :disabled="allSkillsSelected"
                    >
                        {{ t("config.form.selectAllSkills") }}
                    </button>
                    <button
                        class="btn btn-sm btn-outline"
                        @click="deselectAllSkills"
                        :disabled="noSkillsSelected"
                    >
                        {{ t("config.form.deselectAllSkills") }}
                    </button>
                </div>
                <div v-if="skillStore.skills.length === 0" class="no-items">
                    {{ t("config.form.noSkillsAvailable") }}
                </div>
                <div v-else class="items-grid">
                    <div
                        v-for="skill in skillStore.skills"
                        :key="skill.name"
                        :class="[
                            'item-card',
                            { active: isSkillActive(skill.name) },
                        ]"
                        @click="toggleSkill(skill.name)"
                    >
                        <span class="item-name">{{ skill.name }}</span>
                        <span class="item-checkbox">
                            <svg
                                v-if="isSkillActive(skill.name)"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2.5"
                            >
                                <polyline points="20 6 9 17 4 12" />
                            </svg>
                        </span>
                    </div>
                </div>
            </div>

            <!-- Platforms -->
            <div class="form-section">
                <h3 class="section-title">
                    {{ t("config.form.platforms") }}
                </h3>
                <div
                    v-if="platformStore.instances.length === 0"
                    class="no-items"
                >
                    {{ t("config.form.noPlatformsAvailable") }}
                </div>
                <div v-else class="items-grid">
                    <div
                        v-for="platform in platformStore.instances"
                        :key="platform.id"
                        :class="[
                            'item-card',
                            { active: isPlatformActive(platform.id) },
                            {
                                disabled: isPlatformDisabled(platform.id),
                            },
                        ]"
                        @click="
                            !isPlatformDisabled(platform.id) &&
                            togglePlatform(platform.id)
                        "
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
                            <span class="item-name"
                                >{{ platform.id }} ({{
                                    platform.platform_type
                                }})</span
                            >
                            <span
                                v-if="
                                    isPlatformDisabled(platform.id) &&
                                    !isPlatformActive(platform.id)
                                "
                                class="platform-used-badge"
                                :title="
                                    getPlatformUsedByProfileName(platform.id)
                                        ? t('config.form.platformUsedBy') +
                                          ' ' +
                                          getPlatformUsedByProfileName(
                                              platform.id,
                                          )
                                        : t(
                                              'config.form.platformAlreadySelected',
                                          )
                                "
                            >
                                🔒
                            </span>
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
                            <span class="item-checkbox">
                                <svg
                                    v-if="isPlatformActive(platform.id)"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2.5"
                                >
                                    <polyline points="20 6 9 17 4 12" />
                                </svg>
                            </span>
                        </span>
                    </div>
                </div>
            </div>

            <!-- Knowledge Bases -->
            <div class="form-section">
                <h3 class="section-title">
                    {{ t("config.form.knowledgeBases", "Knowledge Bases") }}
                </h3>
                <div
                    v-if="kbStore.knowledgeBases.length === 0"
                    class="no-items"
                >
                    {{
                        t(
                            "config.form.noKbAvailable",
                            "No knowledge bases available. Create one in the Knowledge Base page.",
                        )
                    }}
                </div>
                <div v-else class="items-grid">
                    <div
                        v-for="kb in kbStore.knowledgeBases"
                        :key="kb.id"
                        :class="['item-card', { active: isKbActive(kb.id) }]"
                        @click="toggleKb(kb.id)"
                    >
                        <span class="item-name">{{ kb.name }}</span>
                        <span class="item-checkbox">
                            <svg
                                v-if="isKbActive(kb.id)"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2.5"
                            >
                                <polyline points="20 6 9 17 4 12" />
                            </svg>
                        </span>
                    </div>
                </div>
            </div>

            <!-- Proxy Configuration -->
            <div class="form-section">
                <h3 class="section-title">
                    {{ t("config.form.proxyConfig") }}
                </h3>

                <div class="toggle-row">
                    <div class="toggle-info">
                        <span class="toggle-text">{{
                            t("config.form.proxyEnabled")
                        }}</span>
                        <span class="toggle-description">{{
                            t("config.form.proxyEnabledDesc")
                        }}</span>
                    </div>
                    <button
                        type="button"
                        class="toggle-switch"
                        :class="{
                            'toggle-switch-active':
                                formData.proxy_config.enabled,
                        }"
                        @click="
                            formData.proxy_config.enabled =
                                !formData.proxy_config.enabled
                        "
                    >
                        <span
                            class="toggle-thumb"
                            :class="{
                                'toggle-thumb-active':
                                    formData.proxy_config.enabled,
                            }"
                        ></span>
                    </button>
                </div>

                <template v-if="formData.proxy_config.enabled">
                    <div class="form-grid" style="margin-top: 0.75rem">
                        <div class="form-group">
                            <label class="form-label">{{
                                t("config.form.proxyUrl")
                            }}</label>
                            <input
                                v-model="formData.proxy_config.url"
                                type="text"
                                class="form-input"
                                :placeholder="
                                    t('config.form.proxyUrlPlaceholder')
                                "
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
                                class="tag-list"
                            >
                                <span
                                    v-for="(domain, index) in formData
                                        .proxy_config.proxy_domains"
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
                            class="tag-list"
                        >
                            <span
                                v-for="(domain, index) in formData.proxy_config
                                    .bypass_domains"
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

                    <div class="form-grid">
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
                    </div>

                    <div class="toggle-row">
                        <div class="toggle-info">
                            <span class="toggle-text">{{
                                t("config.form.bypassLocalhost")
                            }}</span>
                            <span class="toggle-description">{{
                                t("config.form.bypassLocalhostDesc")
                            }}</span>
                        </div>
                        <button
                            type="button"
                            class="toggle-switch"
                            :class="{
                                'toggle-switch-active':
                                    formData.proxy_config.bypass_localhost,
                            }"
                            @click="
                                formData.proxy_config.bypass_localhost =
                                    !formData.proxy_config.bypass_localhost
                            "
                        >
                            <span
                                class="toggle-thumb"
                                :class="{
                                    'toggle-thumb-active':
                                        formData.proxy_config.bypass_localhost,
                                }"
                            ></span>
                        </button>
                    </div>
                </template>
            </div>
        </div>

        <div class="form-footer">
            <button
                type="button"
                class="btn btn-outline"
                @click="$emit('cancel')"
            >
                {{ t("common.cancel") }}
            </button>
            <button
                type="button"
                class="btn btn-primary"
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
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid hsl(var(--border) / 0.2);
}

.form-title {
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
    background: transparent;
    border: none;
    color: hsl(var(--muted-foreground));
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

.form-body {
    flex: 1;
    overflow-y: auto;
    padding: 1.25rem;
}

.form-section {
    margin-bottom: 1.75rem;
}

.form-section:last-child {
    margin-bottom: 0;
}

.section-title {
    font-size: 0.8rem;
    font-weight: 700;
    color: hsl(var(--muted-foreground));
    margin-bottom: 0.875rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
}

.form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.875rem;
}

.form-group {
    margin-bottom: 1rem;
}

.form-label {
    display: block;
    font-size: 0.8rem;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
    margin-bottom: 0.375rem;
}

.form-input,
.form-textarea,
.form-select {
    width: 100%;
    padding: 0.6rem 0.75rem;
    font-size: 0.875rem;
    line-height: 1.5;
    color: hsl(var(--foreground));
    background-color: hsl(var(--background) / 0.5);
    border: 1px solid hsl(var(--border) / 0.4);
    border-radius: 0.5rem;
    transition: all 0.2s ease;
    font-family: inherit;
    outline: none;
}

.form-input:focus,
.form-textarea:focus,
.form-select:focus {
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

.form-textarea {
    resize: vertical;
    min-height: 60px;
}

/* Toggle Row */
.toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.625rem 0;
}

.toggle-info {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    min-width: 0;
}

.toggle-text {
    font-size: 0.875rem;
    font-weight: 500;
    color: hsl(var(--foreground));
}

.toggle-description {
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
    line-height: 1.4;
}

/* Toggle Switch */
.toggle-switch {
    position: relative;
    flex-shrink: 0;
    width: 2.25rem;
    height: 1.25rem;
    border-radius: 9999px;
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border) / 0.5);
    cursor: pointer;
    transition: all 0.2s ease;
    padding: 0;
}

.toggle-switch-active {
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

/* Toggles grid for capabilities section */
.toggles-grid {
    display: flex;
    flex-direction: column;
    gap: 0;
    border: 1px solid hsl(var(--border) / 0.3);
    border-radius: 0.5rem;
    overflow: hidden;
}

.toggles-grid .toggle-row {
    padding: 0.75rem 0.875rem;
    border-bottom: 1px solid hsl(var(--border) / 0.15);
}

.toggles-grid .toggle-row:last-child {
    border-bottom: none;
}

.form-hint {
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground) / 0.8);
    margin: 0.25rem 0 0.5rem;
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

/* Tag list */
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

/* Items grid (skills, platforms, KB) */
.kb-actions {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
}

.no-items {
    padding: 1.5rem;
    text-align: center;
    color: hsl(var(--muted-foreground));
    font-size: 0.8125rem;
    background: hsl(var(--secondary) / 0.2);
    border-radius: 0.5rem;
    border: 1px dashed hsl(var(--border) / 0.5);
}

.items-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 0.5rem;
}

.item-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    font-size: 0.8125rem;
    border: 1px solid hsl(var(--border) / 0.4);
    border-radius: 0.5rem;
    background: hsl(var(--background) / 0.5);
    cursor: pointer;
    transition: all 0.2s;
}

.item-card:hover {
    border-color: hsl(var(--primary) / 0.4);
    background: hsl(var(--secondary) / 0.3);
}

.item-card.active {
    border-color: hsl(var(--primary) / 0.6);
    background: hsl(var(--primary) / 0.08);
}

.item-card.disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.item-card.disabled:hover {
    transform: none;
}

.platform-used-badge {
    font-size: 0.7rem;
    color: var(--text-tertiary, #888);
    margin-left: 0.25rem;
}

.item-name {
    font-weight: 500;
    color: hsl(var(--foreground));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    display: flex;
    flex-direction: column;
}

.item-checkbox {
    width: 1.125rem;
    height: 1.125rem;
    color: hsl(var(--primary));
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
}

.platform-info {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    min-width: 0;
    overflow: hidden;
}

.platform-status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
}

.platform-status-dot--running {
    background: hsl(142 71% 45%);
    box-shadow: 0 0 4px hsl(142 71% 45% / 0.5);
}

.platform-status-dot--stopped {
    background: hsl(var(--muted-foreground) / 0.3);
}

.platform-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex-shrink: 0;
}

.platform-restart-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    padding: 0;
    border: none;
    border-radius: 0.25rem;
    background: transparent;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s;
}

.platform-restart-btn:hover:not(:disabled) {
    background: hsl(var(--secondary) / 0.5);
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

/* Command card specific styles */
.cmd-right {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    flex-shrink: 0;
}

.cmd-name {
    font-family: monospace;
    font-weight: 700;
    font-size: 0.8125rem;
}

.cmd-desc {
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
    display: block;
    margin-top: 0.125rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.admin-toggle-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    border: none;
    border-radius: 50%;
    cursor: pointer;
    font-size: 0.75rem;
    transition: all 0.2s ease;
    padding: 0;
}

.admin-toggle-btn--admin {
    background: hsl(38 92% 50% / 0.15);
}

.admin-toggle-btn--admin:hover {
    background: hsl(38 92% 50% / 0.25);
}

.admin-toggle-btn--open {
    background: hsl(142 76% 36% / 0.12);
}

.admin-toggle-btn--open:hover {
    background: hsl(142 76% 36% / 0.2);
}

/* Footer with aligned buttons */
.form-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.625rem;
    padding: 1rem 1.25rem;
    border-top: 1px solid hsl(var(--border) / 0.2);
}

/* Consistent button styles - same height, same padding baseline */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    white-space: nowrap;
    border-radius: 0.5rem;
    font-size: 0.8125rem;
    font-weight: 500;
    transition: all 0.2s ease;
    cursor: pointer;
    outline: none;
    padding: 0.5rem 1.125rem;
    min-height: 2.125rem;
    line-height: 1.4;
}

.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
}

.btn-outline {
    background: transparent;
    color: hsl(var(--muted-foreground));
    border: 1px solid hsl(var(--border) / 0.5);
}

.btn-outline:hover:not(:disabled) {
    color: hsl(var(--foreground));
    border-color: hsl(var(--border));
    background: hsl(var(--secondary) / 0.5);
}

.btn-primary {
    background: linear-gradient(
        135deg,
        hsl(var(--primary)),
        hsl(var(--primary) / 0.9)
    );
    color: hsl(var(--primary-foreground));
    border: none;
    box-shadow: 0 1px 4px hsl(var(--primary) / 0.25);
}

.btn-primary:hover:not(:disabled) {
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.95),
        hsl(var(--primary) / 0.85)
    );
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.35);
    transform: translateY(-1px);
}

/* Scrollbar */
.form-body::-webkit-scrollbar {
    width: 5px;
}

.form-body::-webkit-scrollbar-track {
    background: transparent;
}

.form-body::-webkit-scrollbar-thumb {
    background: hsl(var(--muted) / 0.5);
    border-radius: 3px;
}

.form-body::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--muted-foreground) / 0.4);
}

/* Responsive */
@media (max-width: 640px) {
    .form-grid {
        grid-template-columns: 1fr;
    }
}
</style>

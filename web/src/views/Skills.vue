<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useSkillStore } from "../stores/skill";

const { t } = useI18n();
const skillStore = useSkillStore();
const fileInputRef = ref<HTMLInputElement | null>(null);

onMounted(() => {
    skillStore.fetchSkills();
});

async function handleRemove(name: string) {
    if (!confirm(t("skills.deleteConfirm"))) return;
    try {
        await skillStore.removeSkill(name);
    } catch {
        // error is in store
    }
}

async function handleToggle(name: string, isActive: boolean) {
    try {
        await skillStore.toggleSkill(name, isActive);
    } catch {
        // error is in store
    }
}

const skillIcon = (type: string) => {
    switch (type) {
        case "system_prompt":
            return "📝";
        case "memory":
            return "🧠";
        case "context_prefix":
            return "📋";
        default:
            return "⚡";
    }
};

function hasConfigContent(skill: {
    skill_type: string;
    config: Record<string, unknown>;
}): boolean {
    if (!skill.config) return false;

    switch (skill.skill_type) {
        case "system_prompt":
            return (
                !!skill.config.prompt &&
                String(skill.config.prompt).trim() !== ""
            );
        case "memory":
            return (
                skill.config.max_messages !== undefined &&
                skill.config.max_messages !== null
            );
        case "context_prefix":
            return (
                !!skill.config.prefix &&
                String(skill.config.prefix).trim() !== ""
            );
        default:
            return false;
    }
}

function triggerFileUpload() {
    fileInputRef.value?.click();
}

async function handleFileUpload(event: Event) {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;

    // Check if it's a ZIP file
    if (!file.name.endsWith(".zip")) {
        alert(t("skills.uploadZipOnly"));
        return;
    }

    try {
        await skillStore.uploadSkillPackage(file);
        alert(t("skills.uploadSuccess"));
    } catch (e) {
        console.error("Upload failed:", e);
        alert(
            t("skills.uploadFailed", {
                error: skillStore.error || t("errors.unknown"),
            }),
        );
    } finally {
        // Reset input
        if (target) {
            target.value = "";
        }
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
                        width="24"
                        height="24"
                        viewBox="0 0 24 24"
                        fill="none"
                        xmlns="http://www.w3.org/2000/svg"
                    >
                        <path
                            d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">{{ t("skills.title") }}</h1>
                    <p class="header-desc">{{ t("skills.subtitle") }}</p>
                </div>
            </div>
            <div class="header-actions">
                <button class="btn btn-outline" @click="triggerFileUpload">
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
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                        <polyline points="17 8 12 3 7 8" />
                        <line x1="12" y1="3" x2="12" y2="15" />
                    </svg>
                    {{ t("skills.uploadPackage") }}
                </button>
            </div>
            <!-- Hidden file input -->
            <input
                type="file"
                ref="fileInputRef"
                accept=".zip"
                style="display: none"
                @change="handleFileUpload"
            />
        </div>

        <!-- Error -->
        <div v-if="skillStore.error" class="error-banner">
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
                <circle cx="12" cy="12" r="10" />
                <line x1="15" y1="9" x2="9" y2="15" />
                <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
            {{ skillStore.error }}
        </div>

        <!-- Loading -->
        <div
            v-if="skillStore.loading && skillStore.skills.length === 0"
            class="loading-state"
        >
            <div class="loading-spinner"></div>
            <span class="loading-text">{{ t("common.loading") }}</span>
        </div>

        <!-- Empty State -->
        <div v-else-if="skillStore.skills.length === 0" class="empty-state">
            <div class="empty-illustration">
                <div class="empty-icon-wrapper">
                    <svg
                        width="48"
                        height="48"
                        viewBox="0 0 24 24"
                        fill="none"
                        xmlns="http://www.w3.org/2000/svg"
                    >
                        <path
                            d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                    </svg>
                </div>
                <div class="empty-decoration">
                    <span class="deco-dot deco-dot-1"></span>
                    <span class="deco-dot deco-dot-2"></span>
                    <span class="deco-dot deco-dot-3"></span>
                </div>
            </div>
            <h3 class="empty-title">{{ t("skills.noSkills") }}</h3>
            <p class="empty-desc">{{ t("skills.noSkillsDesc") }}</p>
            <button class="btn btn-accent" @click="triggerFileUpload">
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
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="17 8 12 3 7 8" />
                    <line x1="12" y1="3" x2="12" y2="15" />
                </svg>
                {{ t("skills.addFirstSkill") }}
            </button>
        </div>

        <!-- Skill Cards -->
        <div v-else class="card-list">
            <div
                v-for="(skill, index) in skillStore.skills"
                :key="skill.name"
                class="skill-card"
                :class="{ 'skill-card--active': skill.is_active }"
                :style="{ animationDelay: `${index * 50}ms` }"
            >
                <div
                    class="card-glow"
                    :class="{ 'card-glow--active': skill.is_active }"
                ></div>
                <div class="card-content">
                    <div class="card-info">
                        <div
                            class="card-icon"
                            :class="`card-icon--${skill.skill_type}`"
                        >
                            <!-- system_prompt: document icon -->
                            <svg
                                v-if="skill.skill_type === 'system_prompt'"
                                width="20"
                                height="20"
                                viewBox="0 0 24 24"
                                fill="none"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path
                                    d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8l-6-6z"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                                <path
                                    d="M14 2v6h6"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                                <path
                                    d="M16 13H8M16 17H8M10 9H8"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                            </svg>
                            <!-- memory: brain icon -->
                            <svg
                                v-else-if="skill.skill_type === 'memory'"
                                width="20"
                                height="20"
                                viewBox="0 0 24 24"
                                fill="none"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path
                                    d="M12 2a4 4 0 0 1 4 4c0 1.1-.45 2.1-1.17 2.83L12 12l-2.83-3.17A4 4 0 0 1 12 2z"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                                <path
                                    d="M8 6a4 4 0 0 0 0 8"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                                <path
                                    d="M16 6a4 4 0 0 1 0 8"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                                <path
                                    d="M8 14c-1.1 0-2 .9-2 2v2a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2v-2c0-1.1-.9-2-2-2"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                                <path
                                    d="M12 12v8"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                />
                            </svg>
                            <!-- context_prefix: tag icon -->
                            <svg
                                v-else-if="
                                    skill.skill_type === 'context_prefix'
                                "
                                width="20"
                                height="20"
                                viewBox="0 0 24 24"
                                fill="none"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path
                                    d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                                <circle
                                    cx="7"
                                    cy="7"
                                    r="1.5"
                                    fill="currentColor"
                                />
                            </svg>
                            <!-- default: lightning icon -->
                            <svg
                                v-else
                                width="20"
                                height="20"
                                viewBox="0 0 24 24"
                                fill="none"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path
                                    d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                            </svg>
                        </div>
                        <div class="card-details">
                            <div class="card-title-row">
                                <h3 class="card-title">{{ skill.name }}</h3>
                                <span
                                    class="status-badge"
                                    :class="
                                        skill.is_active
                                            ? 'status-badge--active'
                                            : 'status-badge--inactive'
                                    "
                                >
                                    <span class="status-dot"></span>
                                    {{
                                        skill.is_active
                                            ? t("skills.enabled")
                                            : t("skills.disabled")
                                    }}
                                </span>
                                <span class="type-badge">{{
                                    skill.skill_type
                                }}</span>
                            </div>
                            <div class="card-desc" v-if="skill.description">
                                {{ skill.description }}
                            </div>
                            <!-- Config details -->
                            <div
                                class="card-config"
                                v-if="hasConfigContent(skill)"
                            >
                                <div class="config-row">
                                    <span class="config-label">
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
                                            <circle cx="12" cy="12" r="10" />
                                            <line
                                                x1="12"
                                                y1="16"
                                                x2="12"
                                                y2="12"
                                            />
                                            <line
                                                x1="12"
                                                y1="8"
                                                x2="12.01"
                                                y2="8"
                                            />
                                        </svg>
                                    </span>
                                    <template
                                        v-if="
                                            skill.skill_type === 'system_prompt'
                                        "
                                    >
                                        {{ t("skills.prompt")
                                        }}<span class="card-config-value"
                                            >{{
                                                String(
                                                    skill.config.prompt || "",
                                                ).slice(0, 60)
                                            }}{{
                                                String(
                                                    skill.config.prompt || "",
                                                ).length > 60
                                                    ? "..."
                                                    : ""
                                            }}</span
                                        >
                                    </template>
                                    <template
                                        v-else-if="
                                            skill.skill_type === 'memory'
                                        "
                                    >
                                        {{ t("skills.maxMessages")
                                        }}<span class="card-config-value">{{
                                            skill.config.max_messages
                                        }}</span>
                                    </template>
                                    <template
                                        v-else-if="
                                            skill.skill_type ===
                                            'context_prefix'
                                        "
                                    >
                                        {{ t("skills.prefix")
                                        }}<span class="card-config-value"
                                            >{{
                                                String(
                                                    skill.config.prefix || "",
                                                ).slice(0, 60)
                                            }}{{
                                                String(
                                                    skill.config.prefix || "",
                                                ).length > 60
                                                    ? "..."
                                                    : ""
                                            }}</span
                                        >
                                    </template>
                                </div>
                            </div>
                        </div>
                    </div>

                    <div class="card-actions">
                        <!-- Toggle Switch -->
                        <button
                            class="toggle"
                            :class="{ 'toggle--on': skill.is_active }"
                            @click="handleToggle(skill.name, !skill.is_active)"
                            role="switch"
                            :aria-checked="skill.is_active"
                            :title="
                                skill.is_active
                                    ? t('skills.deactivate')
                                    : t('skills.activate')
                            "
                        >
                            <span class="toggle-thumb"></span>
                        </button>
                        <button
                            class="btn btn-ghost btn-sm btn-danger-ghost"
                            @click="handleRemove(skill.name)"
                            :title="t('skills.remove')"
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
                                <polyline points="3 6 5 6 21 6" />
                                <path
                                    d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                                />
                            </svg>
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
/* ═══════════════════════════════════════════════════════════════
 * Skills Page - Enhanced frosted glass design with animations
 * ═══════════════════════════════════════════════════════════════ */
.page {
    padding: 1.5rem;
    max-width: 56rem;
    margin: 0 auto;
    animation: fadeIn var(--transition-normal) cubic-bezier(0.25, 0.1, 0.25, 1);
}

/* Header - Enhanced with icon and glass effect */
.page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.5rem;
    padding: 1.25rem 1.5rem;
    background: hsl(var(--card));
    border-radius: var(--radius-xl);
    border: 1px solid hsl(var(--border));
    box-shadow: var(--shadow-sm);
}

.header-content {
    display: flex;
    align-items: center;
    gap: 1rem;
}

.header-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 3rem;
    height: 3rem;
    border-radius: var(--radius);
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.2);
    flex-shrink: 0;
}

.header-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-text);
    letter-spacing: -0.02em;
    background: linear-gradient(
        135deg,
        var(--color-text) 0%,
        var(--color-accent) 100%
    );
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.header-desc {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    margin-top: 0.25rem;
}

.header-actions {
    display: flex;
    gap: 0.75rem;
    align-items: center;
}

/* Buttons - Clear and vibrant */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.625rem 1.25rem;
    font-size: 0.875rem;
    font-weight: 600;
    border-radius: 0.5rem;
    border: 2px solid hsl(var(--border));
    background-color: hsl(var(--secondary));
    color: hsl(var(--secondary-foreground));
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    white-space: nowrap;
    position: relative;
    overflow: hidden;
}

.btn::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.2), transparent);
    opacity: 0;
    transition: opacity 0.2s ease;
}

.btn:hover::before {
    opacity: 1;
}

.btn:hover {
    border-color: hsl(var(--primary) / 0.5);
    background-color: hsl(var(--accent));
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.btn-sm {
    padding: 0.375rem 0.875rem;
    font-size: 0.8125rem;
}

.btn-accent {
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(207 70% 55%));
    color: white;
    border: none;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.3);
}

.btn-accent:hover {
    background: linear-gradient(135deg, hsl(207 70% 55%), hsl(var(--primary)));
    box-shadow: 0 4px 16px hsl(var(--primary) / 0.4);
    transform: translateY(-2px);
}

.btn-ghost {
    background-color: transparent;
    color: hsl(var(--muted-foreground));
    border: 1px solid transparent;
}

.btn-ghost:hover {
    background-color: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
    border-color: hsl(var(--primary) / 0.3);
}

.btn-danger-ghost {
    color: hsl(var(--destructive));
    border: 1px solid transparent;
}

.btn-danger-ghost:hover {
    background-color: hsl(var(--destructive) / 0.1);
    border-color: hsl(var(--destructive) / 0.5);
    color: hsl(var(--destructive));
}

.btn-outline {
    background-color: transparent;
    color: hsl(var(--primary));
    border: 2px solid hsl(var(--primary) / 0.5);
}

.btn-outline:hover {
    background-color: hsl(var(--primary) / 0.1);
    border-color: hsl(var(--primary));
    box-shadow: 0 0 0 3px hsl(var(--primary) / 0.1);
}

/* Status Badge with animated dot */
.status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.625rem;
    font-size: 0.6875rem;
    font-weight: 500;
    border-radius: 9999px;
    transition: all var(--transition-fast);
}

.status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    animation: pulse-dot 2s ease-in-out infinite;
}

.status-badge--active {
    background: linear-gradient(
        135deg,
        rgba(236, 72, 153, 0.1) 0%,
        rgba(139, 92, 246, 0.08) 100%
    );
    color: var(--color-accent);
    border: 1px solid rgba(236, 72, 153, 0.2);
}

.status-badge--active .status-dot {
    background: var(--color-accent);
    box-shadow: 0 0 8px rgba(236, 72, 153, 0.6);
}

.status-badge--inactive {
    background: var(--color-bg-mute);
    color: var(--color-text-muted);
    border: 1px solid var(--color-border);
}

.status-badge--inactive .status-dot {
    background: var(--color-text-muted);
}

/* Type Badge */
.type-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.1875rem 0.5rem;
    font-size: 0.625rem;
    font-weight: 500;
    border-radius: var(--radius-sm);
    background: var(--color-bg-mute);
    color: var(--color-text-muted);
    border: 1px solid var(--color-border);
    font-family: "SF Mono", "Fira Code", monospace;
    letter-spacing: 0.02em;
}

/* Error Banner with icon */
.error-banner {
    margin-bottom: 1rem;
    padding: 0.875rem 1.25rem;
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    background: linear-gradient(
        135deg,
        rgba(239, 68, 68, 0.08) 0%,
        rgba(239, 68, 68, 0.05) 100%
    );
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    color: var(--color-danger);
    box-shadow: 0 2px 8px rgba(239, 68, 68, 0.08);
}

.error-banner svg {
    flex-shrink: 0;
    margin-top: 0.125rem;
}

/* Loading State with spinner */
.loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 4rem 0;
    color: var(--color-text-muted);
    font-size: 0.875rem;
}

.loading-spinner {
    width: 2rem;
    height: 2rem;
    border: 3px solid rgba(139, 92, 246, 0.15);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
}

.loading-text {
    color: var(--color-text-muted);
}

/* Empty State with decorations */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    background: hsl(var(--card));
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border-radius: var(--radius-xl);
    border: 1px dashed hsl(var(--primary) / 0.3);
    position: relative;
    overflow: hidden;
}

.empty-illustration {
    position: relative;
    margin-bottom: 1.5rem;
}

.empty-icon-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 5rem;
    height: 5rem;
    border-radius: var(--radius-lg);
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
    box-shadow: var(--shadow-md);
    color: hsl(var(--primary));
}

.empty-decoration {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
}

.deco-dot {
    position: absolute;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    animation: float-dot 3s ease-in-out infinite;
}

.deco-dot-1 {
    top: 0.5rem;
    right: 0.5rem;
    background: var(--color-accent);
    opacity: 0.4;
    animation-delay: 0s;
}

.deco-dot-2 {
    bottom: 0.5rem;
    left: 0.5rem;
    background: var(--color-primary);
    opacity: 0.3;
    animation-delay: 1s;
}

.deco-dot-3 {
    top: 50%;
    right: 0;
    background: var(--color-info);
    opacity: 0.35;
    animation-delay: 2s;
}

.empty-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--color-text-secondary);
    margin-bottom: 0.5rem;
}

.empty-desc {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    margin-bottom: 1.75rem;
    text-align: center;
    max-width: 28rem;
    line-height: 1.5;
}

/* Card List - Staggered animation */
.card-list {
    display: grid;
    gap: 1rem;
}

/* Skill Card - Enhanced with glow effect */
.skill-card {
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: var(--radius-xl);
    padding: 1.25rem;
    transition: all var(--transition-fast);
    box-shadow: var(--shadow-sm);
    position: relative;
    overflow: hidden;
    animation: slideUp 0.4s cubic-bezier(0.25, 0.1, 0.25, 1) both;
}

/* Card glow effect */
.card-glow {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    border-radius: var(--radius-xl);
    opacity: 0;
    transition: opacity var(--transition-fast);
    pointer-events: none;
    background:
        radial-gradient(
            ellipse at top left,
            rgba(139, 92, 246, 0.08) 0%,
            transparent 50%
        ),
        radial-gradient(
            ellipse at bottom right,
            rgba(236, 72, 153, 0.06) 0%,
            transparent 50%
        );
}

.card-glow--active {
    opacity: 1;
    background:
        radial-gradient(
            ellipse at top left,
            rgba(139, 92, 246, 0.12) 0%,
            transparent 50%
        ),
        radial-gradient(
            ellipse at bottom right,
            rgba(236, 72, 153, 0.1) 0%,
            transparent 50%
        );
}

.skill-card:hover {
    border-color: rgba(192, 132, 252, 0.4);
    transform: translateY(-3px);
    box-shadow:
        0 8px 24px rgba(139, 92, 246, 0.12),
        0 4px 12px rgba(236, 72, 153, 0.08);
}

.skill-card:hover .card-glow {
    opacity: 0.6;
}

.skill-card--active {
    background: hsl(var(--primary) / 0.05);
    border: 1px solid hsl(var(--primary) / 0.3);
    box-shadow:
        0 4px 16px hsl(var(--primary) / 0.1),
        var(--shadow-sm);
}

.skill-card--active:hover {
    border-color: rgba(236, 72, 153, 0.4);
    transform: translateY(-4px);
    box-shadow:
        0 12px 32px rgba(236, 72, 153, 0.15),
        0 6px 16px rgba(139, 92, 246, 0.12);
}

.card-content {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    position: relative;
    z-index: 1;
}

.card-info {
    display: flex;
    align-items: flex-start;
    gap: 0.875rem;
    min-width: 0;
    flex: 1;
}

/* Card icon container - with type-specific colors */
.card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: var(--radius-md);
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
    color: hsl(var(--muted-foreground));
    flex-shrink: 0;
    transition: all var(--transition-fast);
}

.card-icon--system_prompt {
    background: linear-gradient(
        135deg,
        rgba(59, 130, 246, 0.1) 0%,
        rgba(99, 102, 241, 0.08) 100%
    );
    border-color: rgba(59, 130, 246, 0.25);
    color: rgb(59, 130, 246);
}

.card-icon--memory {
    background: linear-gradient(
        135deg,
        rgba(168, 85, 247, 0.1) 0%,
        rgba(139, 92, 246, 0.08) 100%
    );
    border-color: rgba(168, 85, 247, 0.25);
    color: rgb(168, 85, 247);
}

.card-icon--context_prefix {
    background: linear-gradient(
        135deg,
        rgba(34, 197, 94, 0.1) 0%,
        rgba(22, 163, 74, 0.08) 100%
    );
    border-color: rgba(34, 197, 94, 0.25);
    color: rgb(34, 197, 94);
}

.skill-card--active .card-icon {
    box-shadow: 0 2px 12px rgba(139, 92, 246, 0.15);
}

/* Card details */
.card-details {
    min-width: 0;
    flex: 1;
}

.card-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
}

.card-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text);
    letter-spacing: -0.01em;
}

.card-desc {
    font-size: 0.8125rem;
    color: var(--color-text-secondary);
    margin-top: 0.375rem;
    line-height: 1.5;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
}

/* Config section */
.card-config {
    margin-top: 0.75rem;
}

.config-row {
    display: flex;
    align-items: flex-start;
    gap: 0.375rem;
    padding: 0.5rem 0.75rem;
    background: hsl(var(--secondary));
    border-radius: var(--radius-md);
    border: 1px solid hsl(var(--border));
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
    line-height: 1.5;
}

.config-label {
    display: flex;
    align-items: center;
    color: var(--color-text-secondary);
    flex-shrink: 0;
    margin-top: 0.125rem;
}

.card-config-value {
    color: var(--color-text-secondary);
    font-weight: 500;
}

/* Card actions */
.card-actions {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    flex-shrink: 0;
}

/* Toggle Switch - Enhanced */
.toggle {
    position: relative;
    width: 2.5rem;
    height: 1.375rem;
    border-radius: 9999px;
    border: 1px solid rgba(192, 132, 252, 0.3);
    background: linear-gradient(
        135deg,
        rgba(216, 180, 254, 0.25) 0%,
        rgba(192, 132, 252, 0.15) 100%
    );
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    cursor: pointer;
    transition: all var(--transition-fast);
    flex-shrink: 0;
    padding: 0;
    box-shadow: 0 2px 6px rgba(139, 92, 246, 0.08);
}

.toggle:hover {
    background: linear-gradient(
        135deg,
        rgba(216, 180, 254, 0.35) 0%,
        rgba(192, 132, 252, 0.25) 100%
    );
    box-shadow: 0 2px 8px rgba(139, 92, 246, 0.12);
}

.toggle--on {
    background: linear-gradient(
        135deg,
        var(--color-accent) 0%,
        var(--color-primary) 100%
    );
    border-color: rgba(236, 72, 153, 0.4);
    box-shadow:
        0 4px 12px rgba(236, 72, 153, 0.25),
        0 0 16px rgba(139, 92, 246, 0.15);
}

.toggle--on:hover {
    background: linear-gradient(
        135deg,
        var(--color-accent-hover) 0%,
        var(--color-primary-hover) 100%
    );
    box-shadow:
        0 4px 16px rgba(236, 72, 153, 0.3),
        0 0 20px rgba(139, 92, 246, 0.2);
}

.toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: calc(1.375rem - 4px);
    height: calc(1.375rem - 4px);
    border-radius: 50%;
    background: hsl(var(--background));
    border: 1px solid hsl(var(--border));
    box-shadow: var(--shadow-sm);
    transition: all var(--transition-spring);
    display: block;
}

.toggle--on .toggle-thumb {
    transform: translateX(1.125rem);
}

/* Animations */
@keyframes slideUp {
    from {
        opacity: 0;
        transform: translateY(12px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

@keyframes pulse-dot {
    0%,
    100% {
        opacity: 1;
    }
    50% {
        opacity: 0.5;
    }
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

@keyframes float-dot {
    0%,
    100% {
        transform: translateY(0) scale(1);
    }
    50% {
        transform: translateY(-8px) scale(1.2);
    }
}

/* Responsive */
@media (max-width: 640px) {
    .page {
        padding: 1rem;
    }

    .page-header {
        flex-direction: column;
        gap: 1rem;
        align-items: stretch;
    }

    .header-content {
        flex-direction: row;
    }

    .header-actions {
        justify-content: stretch;
    }

    .header-actions .btn {
        flex: 1;
    }

    .card-content {
        flex-direction: column;
    }

    .card-actions {
        align-self: flex-end;
        width: 100%;
        justify-content: flex-end;
    }
}
</style>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useSkillStore } from "../stores/skill";
import SkillForm from "../components/SkillForm.vue";
import type { CreateSkillRequest } from "../types";

const skillStore = useSkillStore();
const showForm = ref(false);
const fileInputRef = ref<HTMLInputElement | null>(null);

onMounted(() => {
    skillStore.fetchSkills();
});

async function handleSave(data: CreateSkillRequest) {
    try {
        await skillStore.addSkill(data);
        showForm.value = false;
    } catch {
        // error is in store
    }
}

async function handleRemove(name: string) {
    if (!confirm(`确定移除技能 "${name}" 吗？`)) return;
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

function triggerFileUpload() {
    fileInputRef.value?.click();
}

async function handleFileUpload(event: Event) {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;

    // Check if it's a ZIP file
    if (!file.name.endsWith(".zip")) {
        alert("请上传 ZIP 格式的技能包文件");
        return;
    }

    try {
        await skillStore.uploadSkillPackage(file);
        alert("技能包上传成功！");
    } catch (e) {
        console.error("Upload failed:", e);
        alert(`上传失败：${skillStore.error || "未知错误"}`);
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
            <div class="header-info">
                <h1 class="header-title">技能</h1>
                <p class="header-desc">管理智能体的能力和行为</p>
            </div>
            <div class="header-actions">
                <button class="btn btn-outline" @click="triggerFileUpload">
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
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                        <polyline points="17 8 12 3 7 8" />
                        <line x1="12" y1="3" x2="12" y2="15" />
                    </svg>
                    上传技能包
                </button>
                <button class="btn btn-accent" @click="showForm = true">
                    <svg
                        width="14"
                        height="14"
                        viewBox="0 0 14 14"
                        fill="none"
                        xmlns="http://www.w3.org/2000/svg"
                    >
                        <path
                            d="M7 1v12M1 7h12"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                        />
                    </svg>
                    添加技能
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
            {{ skillStore.error }}
        </div>

        <!-- Loading -->
        <div
            v-if="skillStore.loading && skillStore.skills.length === 0"
            class="loading-state"
        >
            加载中...
        </div>

        <!-- Empty State -->
        <div v-else-if="skillStore.skills.length === 0" class="empty-state">
            <div class="empty-icon">
                <svg
                    width="40"
                    height="40"
                    viewBox="0 0 24 24"
                    fill="none"
                    xmlns="http://www.w3.org/2000/svg"
                >
                    <path
                        d="M12 2L2 7l10 5 10-5-10-5z"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    />
                    <path
                        d="M2 17l10 5 10-5"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    />
                    <path
                        d="M2 12l10 5 10-5"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    />
                </svg>
            </div>
            <h3 class="empty-title">暂无技能</h3>
            <p class="empty-desc">技能可以增强智能体的行为和能力</p>
            <button class="btn btn-accent" @click="showForm = true">
                添加第一个技能
            </button>
        </div>

        <!-- Skill Cards -->
        <div v-else class="card-list">
            <div
                v-for="skill in skillStore.skills"
                :key="skill.name"
                class="skill-card"
                :class="{ 'skill-card--active': skill.is_active }"
            >
                <div class="card-content">
                    <div class="card-info">
                        <div class="card-icon">
                            <!-- system_prompt: document icon -->
                            <svg
                                v-if="skill.skill_type === 'system_prompt'"
                                width="16"
                                height="16"
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
                                width="16"
                                height="16"
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
                                width="16"
                                height="16"
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
                                width="16"
                                height="16"
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
                                    class="badge"
                                    :class="
                                        skill.is_active
                                            ? 'badge-accent'
                                            : 'badge-muted'
                                    "
                                >
                                    {{ skill.is_active ? "已启用" : "已停用" }}
                                </span>
                                <span class="badge badge-type">{{
                                    skill.skill_type
                                }}</span>
                            </div>
                            <div class="card-desc">{{ skill.description }}</div>
                            <!-- Config details -->
                            <div class="card-config">
                                <template
                                    v-if="skill.skill_type === 'system_prompt'"
                                >
                                    提示词：<span class="card-config-value"
                                        >{{
                                            String(
                                                skill.config.prompt || "",
                                            ).slice(0, 80)
                                        }}{{
                                            String(skill.config.prompt || "")
                                                .length > 80
                                                ? "..."
                                                : ""
                                        }}</span
                                    >
                                </template>
                                <template
                                    v-else-if="skill.skill_type === 'memory'"
                                >
                                    最大消息数：<span
                                        class="card-config-value"
                                        >{{ skill.config.max_messages }}</span
                                    >
                                </template>
                                <template
                                    v-else-if="
                                        skill.skill_type === 'context_prefix'
                                    "
                                >
                                    前缀：<span class="card-config-value"
                                        >{{
                                            String(
                                                skill.config.prefix || "",
                                            ).slice(0, 80)
                                        }}{{
                                            String(skill.config.prefix || "")
                                                .length > 80
                                                ? "..."
                                                : ""
                                        }}</span
                                    >
                                </template>
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
                        >
                            <span class="toggle-thumb"></span>
                        </button>
                        <button
                            class="btn btn-ghost btn-sm btn-danger-ghost"
                            @click="handleRemove(skill.name)"
                        >
                            移除
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Form Modal -->
        <SkillForm
            v-if="showForm"
            @save="handleSave"
            @cancel="showForm = false"
        />
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

.header-actions {
    display: flex;
    gap: 0.75rem;
    align-items: center;
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

.btn-sm {
    padding: 0.25rem 0.625rem;
    font-size: 0.75rem;
}

.btn-accent {
    background-color: var(--color-accent);
    color: white;
    border-color: transparent;
}

.btn-accent:hover {
    background-color: var(--color-accent-hover);
    border-color: transparent;
}

.btn-ghost {
    background-color: transparent;
    border-color: transparent;
    color: var(--color-text-secondary);
}

.btn-ghost:hover {
    background-color: var(--color-bg-mute);
    border-color: var(--color-border);
    color: var(--color-text);
}

.btn-danger-ghost {
    color: var(--color-danger);
}

.btn-outline {
    background-color: transparent;
    border-color: var(--color-border);
    color: var(--color-text-secondary);
}

.btn-outline:hover {
    background-color: var(--color-bg-mute);
    border-color: var(--color-border-hover);
    color: var(--color-text);
}

.btn-danger-ghost:hover {
    background-color: var(--color-danger-soft);
    border-color: transparent;
    color: var(--color-danger);
}

/* Badge */
.badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.125rem 0.5rem;
    font-size: 0.6875rem;
    font-weight: 500;
    border-radius: var(--radius-sm);
    background-color: var(--color-bg-mute);
    color: var(--color-text-secondary);
}

.badge-accent {
    background-color: var(--color-accent-soft);
    color: var(--color-accent-hover);
}

.badge-muted {
    background-color: var(--color-bg-mute);
    color: var(--color-text-muted);
}

.badge-type {
    background-color: var(--color-bg-mute);
    color: var(--color-text-muted);
    border: 1px solid var(--color-border);
    font-family: monospace;
    font-size: 0.625rem;
}

/* Error */
.error-banner {
    margin-bottom: 1rem;
    padding: 0.75rem 1rem;
    background-color: var(--color-danger-soft);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    color: var(--color-danger);
}

/* Loading */
.loading-state {
    text-align: center;
    padding: 3rem 0;
    color: var(--color-text-muted);
    font-size: 0.875rem;
}

/* Empty State */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 5rem 0;
}

.empty-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 4rem;
    height: 4rem;
    border-radius: var(--radius-lg);
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    color: var(--color-text-muted);
    margin-bottom: 1rem;
}

.empty-title {
    font-size: 1.125rem;
    font-weight: 500;
    color: var(--color-text-secondary);
    margin-bottom: 0.375rem;
}

.empty-desc {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    margin-bottom: 1.5rem;
}

/* Card List */
.card-list {
    display: grid;
    gap: 0.75rem;
}

/* Skill Card */
.skill-card {
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1.25rem;
    transition:
        border-color var(--transition-fast),
        background-color var(--transition-fast);
}

.skill-card:hover {
    border-color: var(--color-border-hover);
}

.skill-card--active {
    background-color: var(--color-accent-soft);
    border-color: rgba(134, 59, 255, 0.25);
    border-left: 3px solid var(--color-accent);
}

.skill-card--active:hover {
    border-color: rgba(134, 59, 255, 0.4);
    border-left: 3px solid var(--color-accent);
}

.card-content {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
}

.card-info {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    min-width: 0;
    flex: 1;
}

/* Card icon container */
.card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border-radius: var(--radius-sm);
    background-color: var(--color-bg-mute);
    color: var(--color-text-secondary);
    margin-top: 0.125rem;
    flex-shrink: 0;
}

.skill-card--active .card-icon {
    background-color: var(--color-accent-soft);
    color: var(--color-accent-hover);
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
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--color-text);
}

.card-desc {
    font-size: 0.8125rem;
    color: var(--color-text-secondary);
    margin-top: 0.25rem;
}

.card-config {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    margin-top: 0.5rem;
}

.card-config-value {
    color: var(--color-text-secondary);
}

/* Card actions */
.card-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
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

/* Responsive */
@media (max-width: 640px) {
    .page {
        padding: 1rem;
    }

    .card-content {
        flex-direction: column;
    }

    .card-actions {
        align-self: flex-end;
    }
}
</style>

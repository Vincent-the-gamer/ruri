<script setup lang="ts">
import { onMounted, ref, reactive } from "vue";
import { useI18n } from "vue-i18n";
import { usePersonaStore } from "../stores/persona";
import type {
    Persona,
    CreatePersonaRequest,
    UpdatePersonaRequest,
} from "../types";

const { t } = useI18n();
const personaStore = usePersonaStore();

const showForm = ref(false);
const editingPersona = ref<Persona | null>(null);
const formData = reactive({
    name: "",
    description: "",
    prompt: "",
});

onMounted(() => {
    personaStore.fetchPersonas();
});

function openCreate() {
    editingPersona.value = null;
    formData.name = "";
    formData.description = "";
    formData.prompt = "";
    showForm.value = true;
}

function openEdit(persona: Persona) {
    editingPersona.value = persona;
    formData.name = persona.name;
    formData.description = persona.description;
    formData.prompt = persona.prompt;
    showForm.value = true;
}

async function handleSave() {
    try {
        if (editingPersona.value) {
            await personaStore.updatePersona(editingPersona.value.id, {
                name: formData.name,
                description: formData.description,
                prompt: formData.prompt,
            } as UpdatePersonaRequest);
        } else {
            await personaStore.createPersona({
                name: formData.name,
                description: formData.description,
                prompt: formData.prompt,
            } as CreatePersonaRequest);
        }
        showForm.value = false;
        editingPersona.value = null;
    } catch {
        // error is in store
    }
}

function handleCancel() {
    showForm.value = false;
    editingPersona.value = null;
}

async function handleDelete(id: string) {
    if (!confirm(t("personas.deleteConfirm"))) return;
    try {
        await personaStore.deletePersona(id);
        await personaStore.fetchPersonas();
    } catch {
        // error is in store
    }
}

async function handleActivate(id: string) {
    if (!confirm(t("personas.activateConfirm"))) return;
    try {
        await personaStore.activatePersona(id);
        await personaStore.fetchPersonas();
    } catch {
        // error is in store
    }
}

async function handleDeactivate(id: string) {
    if (!confirm(t("personas.deactivateConfirm"))) return;
    try {
        await personaStore.updatePersona(id, { is_active: false });
        await personaStore.fetchPersonas();
    } catch {
        // error is in store
    }
}

function truncatePrompt(prompt: string, maxLen: number = 100): string {
    if (!prompt) return "";
    return prompt.length > maxLen ? prompt.slice(0, maxLen) + "..." : prompt;
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
                            d="M2 3h6a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H2z"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <path
                            d="M22 3h-6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h6z"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                    </svg>
                </div>
                <div class="header-text">
                    <h1 class="header-title">{{ t("personas.title") }}</h1>
                    <p class="header-desc">{{ t("personas.subtitle") }}</p>
                </div>
            </div>
            <button class="btn btn-accent" @click="openCreate">
                <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    xmlns="http://www.w3.org/2000/svg"
                >
                    <path
                        d="M12 5v14M5 12h14"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                    />
                </svg>
                {{ t("personas.addPersona") }}
            </button>
        </div>

        <!-- Error -->
        <div v-if="personaStore.error" class="error-banner">
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
            {{ personaStore.error }}
        </div>

        <!-- Loading -->
        <div
            v-if="personaStore.loading && personaStore.personas.length === 0"
            class="loading-state"
        >
            <div class="loading-spinner"></div>
            <span class="loading-text">{{ t("common.loading") }}</span>
        </div>

        <!-- Empty State -->
        <div v-else-if="personaStore.personas.length === 0" class="empty-state">
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
                            d="M2 3h6a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H2z"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                        <path
                            d="M22 3h-6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h6z"
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
            <h3 class="empty-title">{{ t("personas.noPersonas") }}</h3>
            <p class="empty-desc">{{ t("personas.noPersonasDesc") }}</p>
            <button class="btn btn-accent" @click="openCreate">
                <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                >
                    <path d="M12 5v14M5 12h14" />
                </svg>
                {{ t("personas.addFirstPersona") }}
            </button>
        </div>

        <!-- Persona Cards -->
        <div v-else class="card-list">
            <div
                v-for="(persona, index) in personaStore.personas"
                :key="persona.id"
                class="persona-card"
                :class="{ 'persona-card--active': persona.is_active }"
                :style="{ animationDelay: `${index * 50}ms` }"
            >
                <div
                    class="card-glow"
                    :class="{ 'card-glow--active': persona.is_active }"
                ></div>
                <div class="card-content">
                    <div class="card-info">
                        <div class="card-icon">
                            <span class="icon-emoji">🎭</span>
                        </div>
                        <div class="card-details">
                            <div class="card-title-row">
                                <h3 class="card-title">{{ persona.name }}</h3>
                                <span
                                    v-if="persona.is_active"
                                    class="status-badge status-badge--active"
                                >
                                    <span class="status-dot"></span>
                                    {{ t("personas.active") }}
                                </span>
                                <span
                                    v-else
                                    class="status-badge status-badge--inactive"
                                >
                                    <span class="status-dot"></span>
                                    {{ t("personas.inactive") }}
                                </span>
                            </div>
                            <div class="card-desc" v-if="persona.description">
                                {{ persona.description }}
                            </div>
                            <div class="card-prompt" v-if="persona.prompt">
                                <span class="prompt-label">💬</span>
                                <span class="prompt-text">
                                    {{ truncatePrompt(persona.prompt) }}
                                </span>
                            </div>
                        </div>
                    </div>

                    <div class="card-actions">
                        <button
                            v-if="!persona.is_active"
                            class="btn btn-ghost btn-sm"
                            @click="handleActivate(persona.id)"
                            :title="t('personas.activate')"
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
                                <path d="M22 11.08V12a10 10 0 1 1 -5.93-9.14" />
                                <polyline points="22 4 12 14.01 9 11.01" />
                            </svg>
                            {{ t("personas.activate") }}
                        </button>
                        <button
                            v-if="persona.is_active"
                            class="btn btn-ghost btn-sm"
                            @click="handleDeactivate(persona.id)"
                            :title="t('personas.deactivate')"
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
                                <circle cx="12" cy="12" r="10" />
                                <line x1="4.93" y1="12" x2="19.07" y2="12" />
                            </svg>
                            {{ t("personas.deactivate") }}
                        </button>
                        <button
                            class="btn btn-ghost btn-sm"
                            @click="openEdit(persona)"
                            :title="t('personas.edit')"
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
                                    d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"
                                />
                                <path
                                    d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"
                                />
                            </svg>
                            {{ t("personas.edit") }}
                        </button>
                        <button
                            class="btn btn-ghost btn-sm btn-danger-ghost"
                            @click="handleDelete(persona.id)"
                            :title="t('personas.delete')"
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

        <!-- Form Modal -->
        <div v-if="showForm" class="modal-overlay">
            <div class="modal-content glass" @click.stop>
                <div class="modal-header">
                    <h2 class="modal-title">
                        {{
                            editingPersona
                                ? t("personas.editPersona")
                                : t("personas.createPersona")
                        }}
                    </h2>
                    <button class="modal-close" @click="handleCancel">
                        <svg
                            width="18"
                            height="18"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                        >
                            <path d="M18 6L6 18M6 6l12 12" />
                        </svg>
                    </button>
                </div>
                <div class="modal-body">
                    <div class="form-group">
                        <label class="form-label">{{
                            t("personas.name")
                        }}</label>
                        <input
                            v-model="formData.name"
                            type="text"
                            class="form-input"
                            :placeholder="t('personas.namePlaceholder')"
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label">
                            {{ t("personas.description") }}
                        </label>
                        <input
                            v-model="formData.description"
                            type="text"
                            class="form-input"
                            :placeholder="t('personas.descriptionPlaceholder')"
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label">{{
                            t("personas.prompt")
                        }}</label>
                        <textarea
                            v-model="formData.prompt"
                            class="form-textarea"
                            rows="6"
                            :placeholder="t('personas.promptPlaceholder')"
                        ></textarea>
                    </div>
                </div>
                <div class="modal-footer">
                    <button class="btn btn-ghost" @click="handleCancel">
                        {{ t("personas.cancel") }}
                    </button>
                    <button
                        class="btn btn-accent"
                        @click="handleSave"
                        :disabled="!formData.name.trim()"
                    >
                        {{ t("personas.save") }}
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.page {
    padding: 1.5rem;
    max-width: 960px;
    margin: 0 auto;
    animation: fadeIn 0.4s ease-out;
}

@keyframes fadeIn {
    from {
        opacity: 0;
        transform: translateY(8px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

/* Page Header */
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
    border: 1px solid hsl(var(--primary) / 0.2);
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

/* Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: 0.5rem;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    border: 1px solid transparent;
}

.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-accent {
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(var(--primary) / 0.9) 100%
    );
    color: hsl(var(--primary-foreground));
    border-color: hsl(var(--primary) / 0.3);
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.2);
}

.btn-accent:hover:not(:disabled) {
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.3);
    transform: translateY(-1px);
}

.btn-ghost {
    background: transparent;
    color: hsl(var(--muted-foreground));
    border-color: transparent;
}

.btn-ghost:hover {
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--foreground));
}

.btn-danger-ghost:hover {
    background: hsl(var(--destructive) / 0.1);
    color: hsl(var(--destructive));
}

.btn-sm {
    padding: 0.35rem 0.65rem;
    font-size: 0.8rem;
}

/* Status Badge */
.status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.6rem;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 600;
}

.status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
}

.status-badge--active {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
    border: 1px solid hsl(var(--primary) / 0.2);
}

.status-badge--active .status-dot {
    background: hsl(var(--primary));
    box-shadow: 0 0 6px hsl(var(--primary) / 0.5);
}

.status-badge--inactive {
    background: hsl(var(--muted) / 0.3);
    color: hsl(var(--muted-foreground));
    border: 1px solid hsl(var(--border) / 0.3);
}

.status-badge--inactive .status-dot {
    background: hsl(var(--muted-foreground));
}

/* Error Banner */
.error-banner {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: hsl(var(--destructive) / 0.1);
    border: 1px solid hsl(var(--destructive) / 0.2);
    border-radius: 0.5rem;
    color: hsl(var(--destructive));
    font-size: 0.875rem;
    margin-bottom: 1rem;
}

/* Loading State */
.loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 3rem;
    color: hsl(var(--muted-foreground));
}

.loading-spinner {
    width: 2rem;
    height: 2rem;
    border: 3px solid hsl(var(--primary) / 0.2);
    border-top-color: hsl(var(--primary));
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

.loading-text {
    font-size: 0.875rem;
}

/* Empty State */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 3rem 1rem;
    text-align: center;
}

.empty-illustration {
    position: relative;
    margin-bottom: 0.5rem;
}

.empty-icon-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 5rem;
    height: 5rem;
    border-radius: 1rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.15) 0%,
        hsl(var(--primary) / 0.05) 100%
    );
    color: hsl(var(--primary));
    border: 1px solid hsl(var(--primary) / 0.15);
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
    background: hsl(var(--primary) / 0.4);
    animation: float 3s ease-in-out infinite;
}

.deco-dot-1 {
    top: 10%;
    right: 15%;
    animation-delay: 0s;
}

.deco-dot-2 {
    bottom: 15%;
    left: 10%;
    animation-delay: 1s;
}

.deco-dot-3 {
    top: 50%;
    right: 5%;
    animation-delay: 2s;
}

@keyframes float {
    0%,
    100% {
        transform: translateY(0);
        opacity: 0.6;
    }
    50% {
        transform: translateY(-8px);
        opacity: 1;
    }
}

.empty-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

.empty-desc {
    font-size: 0.875rem;
    color: hsl(var(--muted-foreground));
    margin: 0;
    max-width: 24rem;
}

/* Card List */
.card-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}

.persona-card {
    position: relative;
    border-radius: 0.75rem;
    overflow: hidden;
    animation: slideUp 0.4s ease-out both;
}

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

.card-glow {
    position: absolute;
    inset: 0;
    border-radius: 0.75rem;
    padding: 1px;
    background: linear-gradient(
        135deg,
        hsl(var(--border) / 0.3) 0%,
        transparent 50%,
        hsl(var(--border) / 0.2) 100%
    );
    -webkit-mask:
        linear-gradient(#fff 0 0) content-box,
        linear-gradient(#fff 0 0);
    mask:
        linear-gradient(#fff 0 0) content-box,
        linear-gradient(#fff 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
}

.card-glow--active {
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.4) 0%,
        transparent 50%,
        hsl(var(--primary) / 0.3) 100%
    );
}

.persona-card:hover {
    transform: translateY(-2px);
    transition: transform 0.2s ease;
}

.persona-card--active {
    box-shadow: 0 4px 16px hsl(var(--primary) / 0.1);
}

.card-content {
    position: relative;
    background: linear-gradient(
        180deg,
        hsl(var(--card) / 0.95) 0%,
        hsl(var(--card) / 0.85) 100%
    );
    backdrop-filter: blur(12px);
    padding: 1rem;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
}

.card-info {
    display: flex;
    gap: 0.75rem;
    flex: 1;
    min-width: 0;
}

.card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 0.5rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.15) 0%,
        hsl(var(--primary) / 0.08) 100%
    );
    flex-shrink: 0;
}

.icon-emoji {
    font-size: 1.25rem;
}

.card-details {
    flex: 1;
    min-width: 0;
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
    color: hsl(var(--foreground));
    margin: 0;
}

.card-desc {
    font-size: 0.8rem;
    color: hsl(var(--muted-foreground));
    margin-top: 0.25rem;
}

.card-prompt {
    display: flex;
    align-items: flex-start;
    gap: 0.35rem;
    margin-top: 0.5rem;
    padding: 0.4rem 0.6rem;
    background: hsl(var(--muted) / 0.2);
    border-radius: 0.35rem;
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
}

.prompt-label {
    flex-shrink: 0;
}

.prompt-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* Card Actions */
.card-actions {
    display: flex;
    gap: 0.35rem;
    flex-shrink: 0;
}

/* Modal */
.modal-overlay {
    position: fixed;
    inset: 0;
    background: hsl(var(--background) / 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 1rem;
    animation: fadeIn 0.2s ease-out;
}

.modal-content {
    width: 100%;
    max-width: 480px;
    background: linear-gradient(
        180deg,
        hsl(var(--card) / 0.98) 0%,
        hsl(var(--card) / 0.92) 100%
    );
    backdrop-filter: blur(20px);
    border: 1px solid hsl(var(--border) / 0.3);
    border-radius: 1rem;
    box-shadow: 0 8px 32px hsl(var(--foreground) / 0.1);
    animation: modalSlide 0.3s ease-out;
}

@keyframes modalSlide {
    from {
        opacity: 0;
        transform: scale(0.95) translateY(10px);
    }
    to {
        opacity: 1;
        transform: scale(1) translateY(0);
    }
}

.modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid hsl(var(--border) / 0.2);
}

.modal-title {
    font-size: 1.1rem;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0;
}

.modal-close {
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

.modal-close:hover {
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--foreground));
}

.modal-body {
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
}

.form-group {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
}

.form-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
}

.form-input,
.form-textarea {
    padding: 0.6rem 0.75rem;
    border-radius: 0.5rem;
    border: 1px solid hsl(var(--border) / 0.4);
    background: hsl(var(--background) / 0.5);
    color: hsl(var(--foreground));
    font-size: 0.875rem;
    outline: none;
    transition: all 0.2s ease;
}

.form-input:focus,
.form-textarea:focus {
    border-color: hsl(var(--primary) / 0.5);
    box-shadow: 0 0 0 2px hsl(var(--primary) / 0.1);
}

.form-textarea {
    resize: vertical;
    min-height: 120px;
    font-family: inherit;
}

.modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 1rem 1.25rem;
    border-top: 1px solid hsl(var(--border) / 0.2);
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

    .card-content {
        flex-direction: column;
    }

    .card-actions {
        width: 100%;
        justify-content: flex-end;
    }
}
</style>

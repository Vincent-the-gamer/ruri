<script setup lang="ts">
import { computed, watch, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useProviderStore } from '../stores/provider'
import { usePersonaStore } from '../stores/persona'
import { useSkillStore } from '../stores/skill'
import type { ConfigProfile, CreateConfigProfileRequest, UpdateConfigProfileRequest } from '../types'

interface Props {
  config?: ConfigProfile | null
  saving?: boolean
}

interface Emits {
  (e: 'save', data: CreateConfigProfileRequest | UpdateConfigProfileRequest): void
  (e: 'cancel'): void
}

const props = withDefaults(defineProps<Props>(), {
  config: null,
  saving: false,
})

const emit = defineEmits<Emits>()

const { t } = useI18n()
const providerStore = useProviderStore()
const personaStore = usePersonaStore()
const skillStore = useSkillStore()

// Form data
const formData = ref({
  name: '',
  description: '',
  provider_id: null as string | null,
  persona_id: null as string | null,
  web_search_enabled: false,
  computer_use_enabled: false,
  acp_enabled: false,
  active_skill_names: [] as string[],
})

// Initialize form when config changes
watch(() => props.config, (config) => {
  if (config) {
    formData.value = {
      name: config.name,
      description: config.description,
      provider_id: config.provider_id,
      persona_id: config.persona_id,
      web_search_enabled: config.web_search_enabled,
      computer_use_enabled: config.computer_use_enabled,
      acp_enabled: config.acp_enabled,
      active_skill_names: [...config.active_skill_names],
    }
  } else {
    // Reset for new config
    formData.value = {
      name: '',
      description: '',
      provider_id: null,
      persona_id: null,
      web_search_enabled: false,
      computer_use_enabled: false,
      acp_enabled: false,
      active_skill_names: [],
    }
  }
}, { immediate: true })

const isEdit = computed(() => props.config !== null)
const formTitle = computed(() => isEdit.value ? t('config.form.editTitle') : t('config.form.createTitle'))

// Skill toggle
function toggleSkill(skillName: string) {
  const index = formData.value.active_skill_names.indexOf(skillName)
  if (index === -1) {
    formData.value.active_skill_names.push(skillName)
  } else {
    formData.value.active_skill_names.splice(index, 1)
  }
}

function isSkillActive(skillName: string): boolean {
  return formData.value.active_skill_names.includes(skillName)
}

function handleSubmit() {
  emit('save', formData.value)
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
        <h3 class="section-title">{{ t('config.form.basicInfo') }}</h3>

        <div class="form-group">
          <label class="form-label">{{ t('config.form.name') }} *</label>
          <input
            v-model="formData.name"
            type="text"
            class="form-input"
            :placeholder="t('config.form.namePlaceholder')"
            required
          />
        </div>

        <div class="form-group">
          <label class="form-label">{{ t('config.form.description') }}</label>
          <textarea
            v-model="formData.description"
            class="form-textarea"
            :placeholder="t('config.form.descriptionPlaceholder')"
            rows="2"
          />
        </div>
      </div>

      <!-- Model Provider -->
      <div class="form-section">
        <h3 class="section-title">{{ t('config.form.modelProvider') }}</h3>
        <div class="form-group">
          <label class="form-label">{{ t('config.form.selectProvider') }}</label>
          <select v-model="formData.provider_id" class="form-select">
            <option :value="null">{{ t('config.form.noProvider') }}</option>
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
        <h3 class="section-title">{{ t('config.form.persona') }}</h3>
        <div class="form-group">
          <label class="form-label">{{ t('config.form.selectPersona') }}</label>
          <select v-model="formData.persona_id" class="form-select">
            <option :value="null">{{ t('config.form.noPersona') }}</option>
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
        <h3 class="section-title">{{ t('config.form.capabilities') }}</h3>

        <div class="toggle-group">
          <label class="toggle-label">
            <input
              v-model="formData.web_search_enabled"
              type="checkbox"
              class="toggle-input"
            />
            <span class="toggle-text">{{ t('config.form.webSearch') }}</span>
            <span class="toggle-description">{{ t('config.form.webSearchDesc') }}</span>
          </label>
        </div>

        <div class="toggle-group">
          <label class="toggle-label">
            <input
              v-model="formData.computer_use_enabled"
              type="checkbox"
              class="toggle-input"
            />
            <span class="toggle-text">{{ t('config.form.computerUse') }}</span>
            <span class="toggle-description">{{ t('config.form.computerUseDesc') }}</span>
          </label>
        </div>

        <div class="toggle-group">
          <label class="toggle-label">
            <input
              v-model="formData.acp_enabled"
              type="checkbox"
              class="toggle-input"
            />
            <span class="toggle-text">{{ t('config.form.acp') }}</span>
            <span class="toggle-description">{{ t('config.form.acpDesc') }}</span>
          </label>
        </div>
      </div>

      <!-- Skills -->
      <div class="form-section">
        <h3 class="section-title">{{ t('config.form.skills') }}</h3>
        <div v-if="skillStore.skills.length === 0" class="no-skills">
          {{ t('config.form.noSkillsAvailable') }}
        </div>
        <div v-else class="skills-grid">
          <div
            v-for="skill in skillStore.skills"
            :key="skill.name"
            :class="['skill-item', { active: isSkillActive(skill.name) }]"
            @click="toggleSkill(skill.name)"
          >
            <span class="skill-name">{{ skill.name }}</span>
            <span class="skill-checkbox">
              <svg v-if="isSkillActive(skill.name)" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="20 6 9 17 4 12" />
              </svg>
            </span>
          </div>
        </div>
      </div>
    </div>

    <div class="form-footer">
      <button
        type="button"
        class="btn btn-ghost"
        @click="$emit('cancel')"
      >
        {{ t('common.cancel') }}
      </button>
      <button
        type="button"
        class="btn btn-accent"
        :disabled="!formData.name || saving"
        @click="handleSubmit"
      >
        {{ saving ? t('common.loading') : t('common.save') }}
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

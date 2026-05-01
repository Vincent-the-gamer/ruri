<script setup lang="ts">
import { ref, reactive } from "vue";

const emit = defineEmits<{
    save: [data: { skill_type: string; config: Record<string, unknown> }];
    cancel: [];
}>();

const skillType = ref("system_prompt");

const systemPromptConfig = reactive({ prompt: "" });
const memoryConfig = reactive({ max_messages: 50 });
const contextPrefixConfig = reactive({ prefix: "" });

function handleSave() {
    let config: Record<string, unknown>;
    switch (skillType.value) {
        case "system_prompt":
            config = { prompt: systemPromptConfig.prompt };
            break;
        case "memory":
            config = { max_messages: memoryConfig.max_messages };
            break;
        case "context_prefix":
            config = { prefix: contextPrefixConfig.prefix };
            break;
        default:
            config = {};
    }
    emit("save", { skill_type: skillType.value, config });
}

const skillTypes = [
    {
        value: "system_prompt",
        label: "系统提示词",
        desc: "注入系统提示词来引导模型",
    },
    {
        value: "memory",
        label: "记忆",
        desc: "管理对话记忆，限制消息数量",
    },
    {
        value: "context_prefix",
        label: "上下文前缀",
        desc: "为用户消息添加额外上下文前缀",
    },
];
</script>

<template>
    <div class="modal-backdrop" @click.self="emit('cancel')">
        <div class="modal-card">
            <!-- Header -->
            <div class="modal-header">
                <h2 class="modal-title">添加技能</h2>
                <button @click="emit('cancel')" class="btn-close">
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

            <!-- Body -->
            <div class="modal-body">
                <!-- Skill Type Selector -->
                <div class="form-group">
                    <label class="form-label">技能类型</label>
                    <div class="type-selector">
                        <button
                            v-for="st in skillTypes"
                            :key="st.value"
                            @click="skillType = st.value"
                            class="type-btn"
                            :class="{
                                'type-btn-active': skillType === st.value,
                            }"
                        >
                            <div class="type-btn-content">
                                <div class="type-btn-top">
                                    <!-- System Prompt icon -->
                                    <svg
                                        v-if="st.value === 'system_prompt'"
                                        class="type-icon"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="1.5"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <path
                                            d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"
                                        />
                                    </svg>
                                    <!-- Memory icon -->
                                    <svg
                                        v-if="st.value === 'memory'"
                                        class="type-icon"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="1.5"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <path
                                            d="M12 2a8 8 0 0 0-8 8c0 3.4 2.1 6.3 5 7.5V20h6v-2.5c2.9-1.2 5-4.1 5-7.5a8 8 0 0 0-8-8z"
                                        />
                                        <line x1="10" y1="22" x2="14" y2="22" />
                                    </svg>
                                    <!-- Context Prefix icon -->
                                    <svg
                                        v-if="st.value === 'context_prefix'"
                                        class="type-icon"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="1.5"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <path
                                            d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                                        />
                                        <polyline points="14 2 14 8 20 8" />
                                        <line x1="16" y1="13" x2="8" y2="13" />
                                        <line x1="16" y1="17" x2="8" y2="17" />
                                        <polyline points="10 9 9 9 8 9" />
                                    </svg>
                                    <span class="type-btn-label">{{
                                        st.label
                                    }}</span>
                                </div>
                                <div class="type-btn-desc">{{ st.desc }}</div>
                            </div>
                        </button>
                    </div>
                </div>

                <!-- System Prompt Config -->
                <template v-if="skillType === 'system_prompt'">
                    <div class="form-group">
                        <label class="form-label">系统提示词</label>
                        <textarea
                            v-model="systemPromptConfig.prompt"
                            rows="4"
                            placeholder="你是一个有帮助的 AI 助手..."
                            class="form-textarea"
                        ></textarea>
                    </div>
                </template>

                <!-- Memory Config -->
                <template v-if="skillType === 'memory'">
                    <div class="form-group">
                        <label class="form-label">最大消息数</label>
                        <input
                            v-model.number="memoryConfig.max_messages"
                            type="number"
                            min="1"
                            max="1000"
                            class="form-input"
                        />
                        <p class="form-hint">历史记录中保留的最大消息数</p>
                    </div>
                </template>

                <!-- Context Prefix Config -->
                <template v-if="skillType === 'context_prefix'">
                    <div class="form-group">
                        <label class="form-label">前缀文本</label>
                        <textarea
                            v-model="contextPrefixConfig.prefix"
                            rows="3"
                            placeholder="添加到用户消息前的额外上下文..."
                            class="form-textarea"
                        ></textarea>
                    </div>
                </template>
            </div>

            <!-- Footer -->
            <div class="modal-footer">
                <button @click="emit('cancel')" class="btn-ghost">取消</button>
                <button @click="handleSave" class="btn-accent">添加技能</button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
}

.modal-card {
    background: var(--color-bg-soft);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xl);
    width: 100%;
    max-width: 32rem;
}

.modal-header {
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--color-border);
    display: flex;
    align-items: center;
    justify-content: space-between;
}

.modal-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text);
}

.btn-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.75rem;
    height: 1.75rem;
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: all 0.15s ease;
}

.btn-close:hover {
    color: var(--color-text);
    background: var(--color-bg-hover);
}

.btn-close svg {
    width: 1.125rem;
    height: 1.125rem;
}

.modal-body {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
}

.modal-footer {
    padding: 1rem 1.5rem;
    border-top: 1px solid var(--color-border);
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
}

/* Form elements */
.form-group {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
}

.form-label {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-text-secondary);
}

.form-input,
.form-textarea {
    width: 100%;
    background: var(--color-bg-mute);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 0.5rem 0.75rem;
    font-size: 0.8125rem;
    color: var(--color-text);
    transition: border-color 0.15s ease;
    outline: none;
}

.form-input::placeholder,
.form-textarea::placeholder {
    color: var(--color-text-dim);
}

.form-input:focus,
.form-textarea:focus {
    border-color: var(--color-accent);
}

.form-textarea {
    resize: vertical;
    min-height: 2.5rem;
}

.form-hint {
    font-size: 0.6875rem;
    color: var(--color-text-dim);
    margin: 0;
}

/* Type selector */
.type-selector {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.type-btn {
    width: 100%;
    text-align: left;
    padding: 0.75rem 1rem;
    border-radius: var(--radius-lg);
    border: 1px solid var(--color-border);
    background: var(--color-bg-mute);
    cursor: pointer;
    transition: all 0.15s ease;
}

.type-btn:hover {
    border-color: var(--color-border-hover);
}

.type-btn-active {
    background: var(--color-accent-soft);
    border-color: transparent;
}

.type-btn-content {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
}

.type-btn-top {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.type-icon {
    width: 1rem;
    height: 1rem;
    flex-shrink: 0;
    color: var(--color-text-dim);
}

.type-btn-active .type-icon {
    color: var(--color-accent);
}

.type-btn-label {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-text-muted);
}

.type-btn-active .type-btn-label {
    color: var(--color-accent);
}

.type-btn-desc {
    font-size: 0.6875rem;
    color: var(--color-text-dim);
    padding-left: 1.5rem;
}

.type-btn-active .type-btn-desc {
    color: var(--color-text-muted);
}

/* Footer buttons */
.btn-ghost {
    padding: 0.5rem 1.25rem;
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    background: transparent;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.15s ease;
}

.btn-ghost:hover {
    color: var(--color-text);
    border-color: var(--color-border-hover);
    background: var(--color-bg-hover);
}

.btn-accent {
    padding: 0.5rem 1.25rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: #fff;
    background: var(--color-accent);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: background 0.15s ease;
}

.btn-accent:hover {
    background: var(--color-accent-hover);
}
</style>

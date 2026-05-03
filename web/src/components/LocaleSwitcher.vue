<script setup lang="ts">
import { ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { onClickOutside } from "@vueuse/core";
import { setLocale, getAvailableLocales } from "../locales";

const { t, locale } = useI18n();

const availableLocales = getAvailableLocales();
const isOpen = ref(false);
const localeSwitcherRef = ref<HTMLElement | null>(null);

// 使用 VueUse 的 onClickOutside 监听点击外部关闭下拉菜单
onClickOutside(localeSwitcherRef, () => {
    if (isOpen.value) {
        isOpen.value = false;
    }
});

const currentLocale = computed(() => {
    const current = availableLocales.find((l) => l.code === locale.value);
    return current || availableLocales[0];
});

function changeLocale(code: string) {
    // 先更新 locale，触发响应式更新
    locale.value = code;
    // 保存到 localStorage
    setLocale(code);
    isOpen.value = false;
}

function toggleDropdown() {
    isOpen.value = !isOpen.value;
}
</script>

<template>
    <div class="locale-switcher" ref="localeSwitcherRef">
        <button
            class="locale-btn"
            @click="toggleDropdown"
            :class="{ active: isOpen }"
            :title="t('settings.language')"
        >
            <span class="locale-icon">🌐</span>
            <span class="locale-text">{{ currentLocale.name }}</span>
            <svg
                class="dropdown-arrow"
                :class="{ rotated: isOpen }"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            >
                <polyline points="6 9 12 15 18 9" />
            </svg>
        </button>

        <Transition name="dropdown">
            <div v-if="isOpen" class="locale-dropdown">
                <button
                    v-for="loc in availableLocales"
                    :key="loc.code"
                    class="locale-option"
                    :class="{ active: loc.code === locale }"
                    @click="changeLocale(loc.code)"
                >
                    <span class="option-check" v-if="loc.code === locale"
                        >✓</span
                    >
                    <span class="option-name">{{ loc.name }}</span>
                </button>
            </div>
        </Transition>
    </div>
</template>

<style scoped>
.locale-switcher {
    position: relative;
}

.locale-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-radius: 0.5rem;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border) / 0.4);
    color: hsl(var(--foreground));
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
}

.locale-btn:hover {
    background: hsl(var(--accent));
    border-color: hsl(var(--accent));
}

.locale-btn.active {
    background: hsl(var(--accent));
    border-color: hsl(var(--accent));
}

.locale-icon {
    font-size: 1rem;
}

.locale-text {
    font-size: 0.875rem;
}

.dropdown-arrow {
    transition: transform 0.2s ease;
}

.dropdown-arrow.rotated {
    transform: rotate(180deg);
}

.locale-dropdown {
    position: absolute;
    top: calc(100% + 0.5rem);
    right: 0;
    min-width: 140px;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border) / 0.4);
    border-radius: 0.5rem;
    box-shadow: 0 10px 40px hsl(var(--foreground) / 0.1);
    overflow: hidden;
    z-index: 50;
}

.locale-option {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.75rem 1rem;
    background: transparent;
    border: none;
    color: hsl(var(--foreground));
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
}

.locale-option:hover {
    background: hsl(var(--accent));
}

.locale-option.active {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
}

.option-check {
    width: 1rem;
    color: hsl(var(--primary));
    font-weight: bold;
}

.option-name {
    flex: 1;
}

/* Dropdown transition */
.dropdown-enter-active,
.dropdown-leave-active {
    transition: all 0.2s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
    opacity: 0;
    transform: translateY(-10px);
}
</style>

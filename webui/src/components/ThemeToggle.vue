<script setup lang="ts">
import { computed } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { useDarkMode } from "../composables/useDarkMode";

const { t } = useI18n();
const { colorMode, toggleDarkMode } = useDarkMode();

const themeIcon = computed(() => {
    switch (colorMode.value) {
        case "light":
            return "lucide:sun";
        case "dark":
            return "lucide:moon";
        case "auto":
            return "lucide:monitor";
    }
});

const themeLabel = computed(() => {
    switch (colorMode.value) {
        case "light":
            return t("settings.themeLight");
        case "dark":
            return t("settings.themeDark");
        case "auto":
            return t("settings.themeAuto");
    }
});
</script>

<template>
    <button
        class="h-9 w-9 flex items-center justify-center rounded-lg bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors duration-200"
        @click="toggleDarkMode"
        :title="themeLabel"
    >
        <Icon :icon="themeIcon" class="text-lg" />
    </button>
</template>

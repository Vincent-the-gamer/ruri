<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { Icon } from "@iconify/vue";
import { restartSystem } from "../api";

const { t } = useI18n();

const showRestartDialog = ref(false);
const restarting = ref(false);
const restartError = ref<string | null>(null);
const restartSuccess = ref(false);

const confirmRestart = async () => {
    restarting.value = true;
    restartError.value = null;
    restartSuccess.value = false;

    try {
        await restartSystem();
        restartSuccess.value = true;
        // The backend will restart, so the connection will drop shortly.
        // Show a message and then try to reconnect.
        setTimeout(() => {
            window.location.reload();
        }, 3000);
    } catch (e: any) {
        // If the error is a network error, the server is likely already restarting
        if (
            e?.code === "ERR_NETWORK" ||
            e?.message?.includes("Network Error") ||
            e?.message?.includes("Failed to fetch")
        ) {
            restartSuccess.value = true;
            setTimeout(() => {
                window.location.reload();
            }, 3000);
        } else {
            restartError.value =
                e?.response?.data?.error ||
                e?.message ||
                t("system.restartFailed");
        }
    } finally {
        restarting.value = false;
    }
};
</script>

<template>
    <div class="system-view p-6 max-w-3xl mx-auto">
        <!-- Page Header -->
        <div class="mb-8">
            <h1 class="text-2xl font-bold text-foreground">
                {{ t("system.title") }}
            </h1>
            <p class="text-muted-foreground mt-1">{{ t("system.subtitle") }}</p>
        </div>

        <!-- Restart Backend Card -->
        <div
            class="rounded-xl border border-border/50 bg-card/50 backdrop-blur-sm p-6 space-y-4"
        >
            <div class="flex items-start gap-4">
                <div
                    class="flex-shrink-0 w-12 h-12 rounded-lg bg-amber-500/10 flex items-center justify-center"
                >
                    <Icon
                        icon="lucide:rotate-cw"
                        class="text-xl text-amber-500"
                    />
                </div>
                <div class="flex-1 min-w-0">
                    <h2 class="text-lg font-semibold text-foreground">
                        {{ t("system.restartBackend") }}
                    </h2>
                    <p class="text-sm text-muted-foreground mt-1">
                        {{ t("system.restartBackendDesc") }}
                    </p>
                </div>
            </div>

            <!-- Restart Status Messages -->
            <div
                v-if="restartSuccess"
                class="rounded-lg bg-emerald-500/10 border border-emerald-500/20 p-4"
            >
                <div class="flex items-center gap-2">
                    <Icon icon="lucide:check-circle" class="text-emerald-500" />
                    <span
                        class="text-sm text-emerald-600 dark:text-emerald-400"
                    >
                        {{ t("system.restarting") }}
                    </span>
                </div>
            </div>

            <div
                v-if="restartError"
                class="rounded-lg bg-destructive/10 border border-destructive/20 p-4"
            >
                <div class="flex items-center gap-2">
                    <Icon icon="lucide:alert-circle" class="text-destructive" />
                    <span class="text-sm text-destructive">{{
                        restartError
                    }}</span>
                </div>
            </div>

            <!-- Action Button -->
            <div class="flex justify-end pt-2">
                <button
                    class="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all duration-200"
                    :class="[
                        restarting
                            ? 'bg-muted text-muted-foreground cursor-not-allowed'
                            : 'bg-amber-500/10 text-amber-600 dark:text-amber-400 hover:bg-amber-500/20 border border-amber-500/30',
                    ]"
                    :disabled="restarting"
                    @click="showRestartDialog = true"
                >
                    <Icon
                        :icon="
                            restarting ? 'lucide:loader-2' : 'lucide:rotate-cw'
                        "
                        :class="{ 'animate-spin': restarting }"
                        class="text-base"
                    />
                    {{
                        restarting
                            ? t("system.restarting")
                            : t("system.restartBackend")
                    }}
                </button>
            </div>
        </div>

        <!-- Confirmation Dialog -->
        <Teleport to="body">
            <Transition name="fade">
                <div
                    v-if="showRestartDialog"
                    class="fixed inset-0 z-50 flex items-center justify-center"
                >
                    <!-- Backdrop -->
                    <div
                        class="absolute inset-0 bg-black/50 backdrop-blur-sm"
                        @click="showRestartDialog = false"
                    />

                    <!-- Dialog -->
                    <div
                        class="relative bg-card border border-border rounded-xl p-6 max-w-md w-full mx-4 shadow-2xl"
                    >
                        <div class="flex items-start gap-4">
                            <div
                                class="flex-shrink-0 w-10 h-10 rounded-full bg-amber-500/10 flex items-center justify-center"
                            >
                                <Icon
                                    icon="lucide:alert-triangle"
                                    class="text-lg text-amber-500"
                                />
                            </div>
                            <div class="flex-1">
                                <h3
                                    class="text-lg font-semibold text-foreground"
                                >
                                    {{ t("system.restartConfirmTitle") }}
                                </h3>
                                <p class="text-sm text-muted-foreground mt-2">
                                    {{ t("system.restartConfirmDesc") }}
                                </p>
                            </div>
                        </div>

                        <div class="flex justify-end gap-3 mt-6">
                            <button
                                class="px-4 py-2 rounded-lg text-sm font-medium bg-secondary text-secondary-foreground hover:bg-secondary/80 transition-colors"
                                @click="showRestartDialog = false"
                            >
                                {{ t("common.cancel") }}
                            </button>
                            <button
                                class="px-4 py-2 rounded-lg text-sm font-medium bg-amber-500 text-white hover:bg-amber-600 transition-colors"
                                @click="
                                    showRestartDialog = false;
                                    confirmRestart();
                                "
                            >
                                {{ t("system.restartBackend") }}
                            </button>
                        </div>
                    </div>
                </div>
            </Transition>
        </Teleport>
    </div>
</template>

<style scoped>
/* Fade transition for dialog */
.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}
</style>

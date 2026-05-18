<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { Icon } from "@iconify/vue";
import {
    restartSystem,
    getShellCommandBlacklist,
    updateShellCommandBlacklist,
} from "../api";

const { t } = useI18n();

// ─── Restart ───
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

// ─── Shell Command Blacklist ───
const shellCommandBlacklist = ref<string[]>([]);
const newBlacklistEntry = ref("");
const blacklistLoading = ref(false);
const blacklistSaving = ref(false);
const blacklistSaveSuccess = ref(false);
const blacklistSaveError = ref<string | null>(null);
const blacklistLoadError = ref<string | null>(null);

onMounted(async () => {
    await loadBlacklist();
});

async function loadBlacklist() {
    blacklistLoading.value = true;
    blacklistLoadError.value = null;
    try {
        const data = await getShellCommandBlacklist();
        shellCommandBlacklist.value = data.blacklist ?? [];
    } catch (e: any) {
        blacklistLoadError.value =
            e?.response?.data?.error ||
            e?.message ||
            t("system.loadBlacklistFailed");
    } finally {
        blacklistLoading.value = false;
    }
}

function addBlacklistEntry() {
    if (
        newBlacklistEntry.value.trim() &&
        !shellCommandBlacklist.value.includes(newBlacklistEntry.value.trim())
    ) {
        shellCommandBlacklist.value.push(newBlacklistEntry.value.trim());
        newBlacklistEntry.value = "";
        clearBlacklistMessages();
    }
}

function removeBlacklistEntry(entry: string) {
    const idx = shellCommandBlacklist.value.indexOf(entry);
    if (idx !== -1) {
        shellCommandBlacklist.value.splice(idx, 1);
        clearBlacklistMessages();
    }
}

function clearBlacklistMessages() {
    blacklistSaveSuccess.value = false;
    blacklistSaveError.value = null;
}

async function saveBlacklist() {
    blacklistSaving.value = true;
    blacklistSaveError.value = null;
    blacklistSaveSuccess.value = false;
    try {
        await updateShellCommandBlacklist(shellCommandBlacklist.value);
        blacklistSaveSuccess.value = true;
        setTimeout(() => {
            blacklistSaveSuccess.value = false;
        }, 3000);
    } catch (e: any) {
        blacklistSaveError.value =
            e?.response?.data?.error ||
            e?.message ||
            t("system.saveBlacklistFailed");
    } finally {
        blacklistSaving.value = false;
    }
}
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

        <!-- Shell Command Blacklist Card -->
        <div
            class="rounded-xl border border-border/50 bg-card/50 backdrop-blur-sm p-6 space-y-4 mt-6"
        >
            <div class="flex items-start gap-4">
                <div
                    class="flex-shrink-0 w-12 h-12 rounded-lg bg-destructive/10 flex items-center justify-center"
                >
                    <Icon
                        icon="lucide:shield-off"
                        class="text-xl text-destructive"
                    />
                </div>
                <div class="flex-1 min-w-0">
                    <h2 class="text-lg font-semibold text-foreground">
                        {{ t("system.shellCommandBlacklist") }}
                    </h2>
                    <p class="text-sm text-muted-foreground mt-1">
                        {{ t("system.shellCommandBlacklistDesc") }}
                    </p>
                </div>
            </div>

            <!-- Loading State -->
            <div
                v-if="blacklistLoading"
                class="flex items-center gap-2 text-sm text-muted-foreground"
            >
                <Icon icon="lucide:loader-2" class="animate-spin text-base" />
                {{ t("common.loading") }}
            </div>

            <!-- Load Error -->
            <div
                v-if="blacklistLoadError"
                class="rounded-lg bg-destructive/10 border border-destructive/20 p-4"
            >
                <div class="flex items-center gap-2">
                    <Icon icon="lucide:alert-circle" class="text-destructive" />
                    <span class="text-sm text-destructive">{{
                        blacklistLoadError
                    }}</span>
                </div>
            </div>

            <!-- Blacklist Editor -->
            <template v-if="!blacklistLoading">
                <div class="space-y-2">
                    <div class="flex gap-2">
                        <input
                            v-model="newBlacklistEntry"
                            type="text"
                            :placeholder="
                                t('system.shellCommandBlacklistPlaceholder')
                            "
                            class="flex-1 px-3 py-2 rounded-lg border border-border/50 bg-background/50 text-sm text-foreground placeholder:text-muted-foreground/50 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all"
                            @keyup.enter="addBlacklistEntry"
                        />
                        <button
                            class="px-3 py-2 rounded-lg text-sm font-medium bg-secondary text-secondary-foreground hover:bg-secondary/80 transition-colors"
                            @click="addBlacklistEntry"
                        >
                            {{ t("common.add") }}
                        </button>
                    </div>

                    <!-- Blacklist Items -->
                    <div
                        v-if="shellCommandBlacklist.length > 0"
                        class="rounded-lg border border-border/30 bg-background/30 divide-y divide-border/20"
                    >
                        <div
                            v-for="entry in shellCommandBlacklist"
                            :key="entry"
                            class="flex items-center justify-between px-3 py-2"
                        >
                            <code
                                class="text-sm font-mono text-destructive bg-destructive/5 px-2 py-0.5 rounded"
                            >
                                {{ entry }}
                            </code>
                            <button
                                @click="removeBlacklistEntry(entry)"
                                class="flex-shrink-0 w-6 h-6 rounded flex items-center justify-center text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
                                :title="t('common.delete')"
                            >
                                <Icon icon="lucide:x" class="text-sm" />
                            </button>
                        </div>
                    </div>
                    <div
                        v-else
                        class="text-sm text-muted-foreground py-4 text-center"
                    >
                        {{ t("system.shellCommandBlacklistPlaceholder") }}
                    </div>
                </div>

                <!-- Save Status Messages -->
                <div
                    v-if="blacklistSaveSuccess"
                    class="rounded-lg bg-emerald-500/10 border border-emerald-500/20 p-4"
                >
                    <div class="flex items-center gap-2">
                        <Icon
                            icon="lucide:check-circle"
                            class="text-emerald-500"
                        />
                        <span
                            class="text-sm text-emerald-600 dark:text-emerald-400"
                        >
                            {{ t("system.saveBlacklistSuccess") }}
                        </span>
                    </div>
                </div>

                <div
                    v-if="blacklistSaveError"
                    class="rounded-lg bg-destructive/10 border border-destructive/20 p-4"
                >
                    <div class="flex items-center gap-2">
                        <Icon
                            icon="lucide:alert-circle"
                            class="text-destructive"
                        />
                        <span class="text-sm text-destructive">{{
                            blacklistSaveError
                        }}</span>
                    </div>
                </div>

                <!-- Save Button -->
                <div class="flex justify-end pt-2">
                    <button
                        class="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all duration-200"
                        :class="[
                            blacklistSaving
                                ? 'bg-muted text-muted-foreground cursor-not-allowed'
                                : 'bg-primary/10 text-primary hover:bg-primary/20 border border-primary/30',
                        ]"
                        :disabled="blacklistSaving"
                        @click="saveBlacklist"
                    >
                        <Icon
                            :icon="
                                blacklistSaving
                                    ? 'lucide:loader-2'
                                    : 'lucide:save'
                            "
                            :class="{ 'animate-spin': blacklistSaving }"
                            class="text-base"
                        />
                        {{
                            blacklistSaving
                                ? t("common.saving")
                                : t("system.saveBlacklist")
                        }}
                    </button>
                </div>
            </template>
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

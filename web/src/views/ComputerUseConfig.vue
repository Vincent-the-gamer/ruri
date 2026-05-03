<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useComputerUseStore } from "../stores/computerUse";

const { t } = useI18n();
const computerUseStore = useComputerUseStore();

const selectedRuntime = ref<string>("none");
const requireAdmin = ref(true);
const adminIds = ref<string[]>([]);
const allowedPaths = ref<string[]>([]);
const newAdminId = ref("");
const newAllowedPath = ref("");

const saveSuccess = ref(false);
const saveError = ref<string | null>(null);

onMounted(async () => {
    await computerUseStore.fetchConfig();
    syncFromStore();
});

function syncFromStore() {
    if (computerUseStore.config) {
        selectedRuntime.value = computerUseStore.config.runtime;
        requireAdmin.value = computerUseStore.config.require_admin;
        adminIds.value = [...computerUseStore.config.admin_ids];
        allowedPaths.value = [...computerUseStore.config.allowed_paths];
    }
}

function clearMessages() {
    saveSuccess.value = false;
    saveError.value = null;
}

function addAdminId() {
    if (newAdminId.value.trim() && !adminIds.value.includes(newAdminId.value.trim())) {
        adminIds.value.push(newAdminId.value.trim());
        newAdminId.value = "";
        clearMessages();
    }
}

function removeAdminId(id: string) {
    const idx = adminIds.value.indexOf(id);
    if (idx !== -1) {
        adminIds.value.splice(idx, 1);
        clearMessages();
    }
}

function addAllowedPath() {
    if (newAllowedPath.value.trim() && !allowedPaths.value.includes(newAllowedPath.value.trim())) {
        allowedPaths.value.push(newAllowedPath.value.trim());
        newAllowedPath.value = "";
        clearMessages();
    }
}

function removeAllowedPath(path: string) {
    const idx = allowedPaths.value.indexOf(path);
    if (idx !== -1) {
        allowedPaths.value.splice(idx, 1);
        clearMessages();
    }
}

async function handleSave() {
    clearMessages();
    try {
        await computerUseStore.updateConfig({
            runtime: selectedRuntime.value as "none" | "local" | "sandbox",
            require_admin: requireAdmin.value,
            admin_ids: adminIds.value,
            allowed_paths: allowedPaths.value,
        });
        saveSuccess.value = true;
        setTimeout(() => {
            saveSuccess.value = false;
        }, 3000);
    } catch (e: unknown) {
        saveError.value = e instanceof Error ? e.message : t("errors.unknown");
    }
}

const hasChanges = computed(() => {
    if (!computerUseStore.config) return false;
    return (
        selectedRuntime.value !== computerUseStore.config.runtime ||
        requireAdmin.value !== computerUseStore.config.require_admin ||
        JSON.stringify(adminIds.value) !== JSON.stringify(computerUseStore.config.admin_ids) ||
        JSON.stringify(allowedPaths.value) !== JSON.stringify(computerUseStore.config.allowed_paths)
    );
});

const runtimeLabel = (runtime: string) => {
    switch (runtime) {
        case "none":
            return "Disabled";
        case "local":
            return "Local";
        case "sandbox":
            return "Sandbox";
        default:
            return runtime;
    }
};
</script>

<template>
    <div class="p-6 max-w-4xl mx-auto">
        <h1 class="text-2xl font-bold mb-6">Computer Use Configuration</h1>

        <!-- Loading and Error States -->
        <div v-if="computerUseStore.loading" class="text-center py-4">
            Loading...
        </div>

        <div v-if="computerUseStore.error" class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
            {{ computerUseStore.error }}
        </div>

        <!-- Success Message -->
        <div v-if="saveSuccess" class="bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded mb-4">
            Configuration saved successfully!
        </div>

        <!-- Save Error Message -->
        <div v-if="saveError" class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
            {{ saveError }}
        </div>

        <!-- Configuration Form -->
        <div v-if="computerUseStore.config" class="space-y-6">
            <!-- Runtime Selection -->
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">
                    Runtime Mode
                </label>
                <select
                    v-model="selectedRuntime"
                    class="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm rounded-md"
                    @change="clearMessages"
                >
                    <option value="none">Disabled</option>
                    <option value="local">Local</option>
                    <option value="sandbox">Sandbox (Coming Soon)</option>
                </select>
                <p class="mt-2 text-sm text-gray-500">
                    <span v-if="selectedRuntime === 'none'">Computer use is disabled.</span>
                    <span v-else-if="selectedRuntime === 'local'">Run in local environment. Tools will execute on this machine.</span>
                    <span v-else-if="selectedRuntime === 'sandbox'">Run in isolated sandbox (not yet implemented).</span>
                </p>
            </div>

            <!-- Require Admin -->
            <div>
                <label class="flex items-center">
                    <input
                        type="checkbox"
                        v-model="requireAdmin"
                        class="rounded border-gray-300 text-indigo-600 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
                        @change="clearMessages"
                    />
                    <span class="ml-2 text-sm text-gray-700">
                        Require Admin Privileges for Shell/Python
                    </span>
                </label>
                <p class="mt-2 text-sm text-gray-500">
                    When enabled, only admin users can execute shell commands and Python code.
                </p>
            </div>

            <!-- Admin IDs -->
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">
                    Admin User IDs
                </label>
                <div class="flex gap-2 mb-2">
                    <input
                        v-model="newAdminId"
                        type="text"
                        placeholder="Enter user ID"
                        class="flex-1 rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
                        @keyup.enter="addAdminId"
                    />
                    <button
                        @click="addAdminId"
                        class="px-4 py-2 bg-indigo-600 text-white rounded-md hover:bg-indigo-700"
                    >
                        Add
                    </button>
                </div>
                <div class="flex flex-wrap gap-2">
                    <span
                        v-for="id in adminIds"
                        :key="id"
                        class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-indigo-100 text-indigo-800"
                    >
                        {{ id }}
                        <button
                            @click="removeAdminId(id)"
                            class="ml-2 text-indigo-600 hover:text-indigo-800"
                        >
                            ×
                        </button>
                    </span>
                </div>
            </div>

            <!-- Allowed Paths -->
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">
                    Allowed Paths (for non-admin users)
                </label>
                <div class="flex gap-2 mb-2">
                    <input
                        v-model="newAllowedPath"
                        type="text"
                        placeholder="Enter path"
                        class="flex-1 rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
                        @keyup.enter="addAllowedPath"
                    />
                    <button
                        @click="addAllowedPath"
                        class="px-4 py-2 bg-indigo-600 text-white rounded-md hover:bg-indigo-700"
                    >
                        Add
                    </button>
                </div>
                <div class="flex flex-wrap gap-2">
                    <span
                        v-for="path in allowedPaths"
                        :key="path"
                        class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-gray-100 text-gray-800"
                    >
                        {{ path }}
                        <button
                            @click="removeAllowedPath(path)"
                            class="ml-2 text-gray-600 hover:text-gray-800"
                        >
                            ×
                        </button>
                    </span>
                </div>
            </div>

            <!-- Save Button -->
            <div class="flex justify-end">
                <button
                    @click="handleSave"
                    :disabled="!hasChanges || computerUseStore.loading"
                    class="px-6 py-2 bg-indigo-600 text-white rounded-md hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                    Save Configuration
                </button>
            </div>
        </div>

        <!-- Info Box -->
        <div class="mt-8 bg-blue-50 border border-blue-200 rounded-lg p-4">
            <h3 class="text-sm font-medium text-blue-800 mb-2">Learn More</h3>
            <p class="text-sm text-blue-700">
                Computer Use allows the AI agent to execute commands, run code, and access files.
                <a
                    href="https://github.com/yourusername/ruri/blob/main/COMPUTER_USE.md"
                    target="_blank"
                    class="underline"
                >
                    View Documentation
                </a>
            </p>
        </div>
    </div>
</template>

<style scoped>
/* Add any additional styles here */
</style>

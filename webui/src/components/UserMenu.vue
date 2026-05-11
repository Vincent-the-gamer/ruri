<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { useAuthStore } from "../stores/auth";
import { Icon } from "@iconify/vue";

const router = useRouter();
const { t } = useI18n();
const authStore = useAuthStore();

const showMenu = ref(false);

const username = computed(() => authStore.username || "User");
const mustChangePassword = computed(() => authStore.mustChangePassword);

function toggleMenu() {
    showMenu.value = !showMenu.value;
}

function closeMenu() {
    showMenu.value = false;
}

async function handleLogout() {
    closeMenu();
    await authStore.logout();
    router.push("/login");
}

function goToChangePassword() {
    closeMenu();
    router.push("/change-password");
}

// Close menu when clicking outside
function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (!target.closest(".user-menu")) {
        closeMenu();
    }
}

// Register click outside listener when menu opens
watch(showMenu, (newValue) => {
    if (newValue) {
        document.addEventListener("click", handleClickOutside);
    } else {
        document.removeEventListener("click", handleClickOutside);
    }
});

onBeforeUnmount(() => {
    document.removeEventListener("click", handleClickOutside);
});

// Get initial letter of username for avatar fallback
const initialLetter = computed(() => username.value.charAt(0).toUpperCase());
</script>

<template>
    <div class="user-menu" @click.stop>
        <!-- Toggle Button -->
        <button
            class="user-menu-toggle"
            :class="{ 'is-open': showMenu }"
            @click="toggleMenu"
        >
            <!-- Avatar with initial -->
            <div class="user-avatar">
                {{ initialLetter }}
            </div>

            <!-- Username and status indicator -->
            <div class="user-info">
                <span class="user-name">{{ username }}</span>
                <span
                    v-if="mustChangePassword"
                    class="password-warning"
                    :title="t('userMenu.passwordChangeRequired')"
                >
                    <Icon icon="lucide:alert-triangle" class="warning-icon" />
                </span>
            </div>

            <!-- Chevron indicator -->
            <Icon
                :icon="showMenu ? 'lucide:chevron-up' : 'lucide:chevron-down'"
                class="menu-chevron"
            />
        </button>

        <!-- Dropdown Menu -->
        <Transition name="dropdown">
            <div v-if="showMenu" class="user-menu-dropdown">
                <!-- User info header -->
                <div class="dropdown-header">
                    <div class="dropdown-avatar">{{ initialLetter }}</div>
                    <div class="dropdown-user-info">
                        <div class="dropdown-username">{{ username }}</div>
                        <div v-if="mustChangePassword" class="dropdown-warning">
                            <Icon
                                icon="lucide:shield-alert"
                                class="warning-icon-small"
                            />
                            <span>{{
                                t("userMenu.passwordChangeRequired")
                            }}</span>
                        </div>
                    </div>
                </div>

                <div class="dropdown-divider"></div>

                <!-- Menu Items -->
                <div class="dropdown-items">
                    <!-- Change Password -->
                    <button class="dropdown-item" @click="goToChangePassword">
                        <Icon icon="lucide:key-round" class="dropdown-icon" />
                        <span>{{ t("userMenu.changePassword") }}</span>
                        <Icon
                            v-if="mustChangePassword"
                            icon="lucide:arrow-right"
                            class="dropdown-arrow"
                        />
                    </button>

                    <!-- Logout -->
                    <button
                        class="dropdown-item logout-item"
                        @click="handleLogout"
                    >
                        <Icon icon="lucide:log-out" class="dropdown-icon" />
                        <span>{{ t("userMenu.logout") }}</span>
                        <Icon
                            icon="lucide:arrow-right"
                            class="dropdown-arrow"
                        />
                    </button>
                </div>
            </div>
        </Transition>
    </div>
</template>

<style scoped>
/* User Menu Container */
.user-menu {
    position: relative;
}

/* Toggle Button */
.user-menu-toggle {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 12px 6px 6px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.2s ease;
    outline: none;
}

.user-menu-toggle:hover,
.user-menu-toggle.is-open {
    background: hsl(var(--secondary) / 0.5);
    border-color: hsl(var(--border) / 0.3);
}

/* Avatar */
.user-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(280 70% 60%));
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    font-weight: 700;
    color: hsl(var(--primary-foreground));
    flex-shrink: 0;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.2);
}

/* User Info */
.user-info {
    display: flex;
    align-items: center;
    gap: 6px;
}

.user-name {
    font-size: 14px;
    font-weight: 600;
    color: hsl(var(--foreground));
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.password-warning {
    display: flex;
    align-items: center;
    color: hsl(38 90% 50%);
}

.warning-icon {
    font-size: 14px;
}

/* Chevron */
.menu-chevron {
    font-size: 14px;
    color: hsl(var(--muted-foreground));
    transition: transform 0.2s ease;
}

/* Dropdown */
.user-menu-dropdown {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    width: 280px;
    background: hsl(var(--background) / 0.95);
    backdrop-filter: blur(20px) saturate(1.5);
    -webkit-backdrop-filter: blur(20px) saturate(1.5);
    border: 1px solid hsl(var(--border) / 0.3);
    border-radius: 12px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.15);
    overflow: hidden;
    z-index: 100;
}

/* Dropdown Header */
.dropdown-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    background: hsl(var(--secondary) / 0.2);
}

.dropdown-avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(280 70% 60%));
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    font-weight: 700;
    color: hsl(var(--primary-foreground));
    flex-shrink: 0;
}

.dropdown-user-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
}

.dropdown-username {
    font-size: 14px;
    font-weight: 600;
    color: hsl(var(--foreground));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.dropdown-warning {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: hsl(38 90% 50%);
}

.warning-icon-small {
    font-size: 12px;
    flex-shrink: 0;
}

/* Divider */
.dropdown-divider {
    height: 1px;
    background: hsl(var(--border) / 0.3);
    margin: 4px 0;
}

/* Dropdown Items */
.dropdown-items {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.dropdown-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    background: transparent;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: left;
    width: 100%;
    font-size: 14px;
    color: hsl(var(--foreground));
    outline: none;
}

.dropdown-item:hover {
    background: hsl(var(--secondary) / 0.5);
}

.dropdown-item:active {
    background: hsl(var(--secondary) / 0.8);
}

.dropdown-icon {
    font-size: 16px;
    color: hsl(var(--muted-foreground));
    flex-shrink: 0;
}

.dropdown-item:hover .dropdown-icon {
    color: hsl(var(--foreground));
}

.dropdown-arrow {
    font-size: 14px;
    color: hsl(var(--muted-foreground) / 0.5);
    margin-left: auto;
    opacity: 0;
    transition: all 0.15s ease;
}

.dropdown-item:hover .dropdown-arrow {
    opacity: 1;
    color: hsl(var(--foreground));
}

/* Logout item special styling */
.logout-item:hover {
    background: hsl(0 70% 50% / 0.1);
    color: hsl(0 70% 50%);
}

.logout-item:hover .dropdown-icon {
    color: hsl(0 70% 50%);
}

/* Dropdown Transition */
.dropdown-enter-active,
.dropdown-leave-active {
    transition: all 0.2s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
    opacity: 0;
    transform: translateY(-8px) scale(0.95);
}

.dropdown-enter-to,
.dropdown-leave-from {
    opacity: 1;
    transform: translateY(0) scale(1);
}

/* Responsive */
@media (max-width: 768px) {
    .user-name {
        display: none;
    }

    .user-menu-toggle {
        padding: 6px;
    }

    .user-menu-dropdown {
        width: 240px;
        right: -8px;
    }
}
</style>

<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { useAuthStore } from "../stores/auth";
import { Icon } from "@iconify/vue";

const router = useRouter();
const { t } = useI18n();
const authStore = useAuthStore();

const oldPassword = ref("");
const newPassword = ref("");
const confirmPassword = ref("");
const showOldPassword = ref(false);
const showNewPassword = ref(false);
const showConfirmPassword = ref(false);
const errorMessage = ref("");
const successMessage = ref("");
const usernameMessage = ref("");
const usernameSuccessMessage = ref("");
const newUsername = ref("");
const isUpdatingUsername = ref(false);

const avatarInput = ref<HTMLInputElement | null>(null);
const avatarUploading = ref(false);
const avatarMessage = ref("");
const avatarSuccessMessage = ref("");

const MAX_AVATAR_SIZE = 5 * 1024 * 1024; // 5MB

const avatarUrl = computed(() => authStore.avatarUrl);

const isChanging = computed(() => authStore.loading);

function validateNewPassword(): boolean {
    if (newPassword.value.length < 4) {
        errorMessage.value = t("changePassword.passwordTooShort");
        return false;
    }
    if (newPassword.value !== confirmPassword.value) {
        errorMessage.value = t("changePassword.passwordsDoNotMatch");
        return false;
    }
    if (newPassword.value === oldPassword.value) {
        errorMessage.value = t("changePassword.newPasswordSameAsOld");
        return false;
    }
    return true;
}

async function handleChangePassword() {
    errorMessage.value = "";
    successMessage.value = "";

    if (!oldPassword.value || !newPassword.value || !confirmPassword.value) {
        errorMessage.value = t("changePassword.fillAllFields");
        return;
    }

    if (!validateNewPassword()) {
        return;
    }

    try {
        await authStore.changePassword({
            old_password: oldPassword.value,
            new_password: newPassword.value,
        });

        successMessage.value = t("changePassword.passwordChangedSuccess");
        // Redirect to home after a short delay
        setTimeout(() => {
            router.push("/");
        }, 1500);
    } catch (e: unknown) {
        errorMessage.value =
            authStore.error || t("changePassword.failedToChangePassword");
    }
}

function handleKeyPress(e: KeyboardEvent) {
    if (e.key === "Enter") {
        handleChangePassword();
    }
}

async function handleUpdateUsername() {
    usernameMessage.value = "";
    usernameSuccessMessage.value = "";

    if (!newUsername.value.trim()) {
        usernameMessage.value = t("changePassword.usernameCannotBeEmpty");
        return;
    }

    isUpdatingUsername.value = true;

    try {
        await authStore.updateUsername({
            new_username: newUsername.value.trim(),
        });
        usernameSuccessMessage.value = t(
            "changePassword.usernameUpdatedSuccess",
        );
    } catch (e: unknown) {
        usernameMessage.value =
            authStore.error || t("changePassword.failedToUpdateUsername");
    } finally {
        isUpdatingUsername.value = false;
    }
}

function handleUsernameKeyPress(e: KeyboardEvent) {
    if (e.key === "Enter") {
        handleUpdateUsername();
    }
}

function triggerAvatarUpload() {
    avatarInput.value?.click();
}

async function handleAvatarChange(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    avatarMessage.value = "";
    avatarSuccessMessage.value = "";

    if (!file) return;

    // Validate file size (max 5MB)
    if (file.size > MAX_AVATAR_SIZE) {
        avatarMessage.value = t("changePassword.avatarTooLarge");
        return;
    }

    // Validate file type
    const allowedTypes = ["image/jpeg", "image/png", "image/gif", "image/webp"];
    if (!allowedTypes.includes(file.type)) {
        avatarMessage.value = t("changePassword.avatarInvalidType");
        return;
    }

    avatarUploading.value = true;
    try {
        await authStore.uploadAvatar(file);
        avatarSuccessMessage.value = t("changePassword.avatarUpdatedSuccess");
    } catch (e: unknown) {
        avatarMessage.value =
            authStore.error || t("changePassword.failedToUpdateAvatar");
    } finally {
        avatarUploading.value = false;
        // Reset the input so the same file can be selected again
        if (input) input.value = "";
    }
}
</script>

<template>
    <div class="change-password-container">
        <!-- Floating orbs background animation -->
        <div class="bg-orbs">
            <div class="orb orb-1"></div>
            <div class="orb orb-2"></div>
            <div class="orb orb-3"></div>
        </div>

        <!-- Change Password Card -->
        <div class="change-password-card">
            <!-- Header -->
            <div class="card-header">
                <div class="header-icon">
                    <Icon icon="lucide:shield-check" class="shield-icon" />
                </div>
                <h1 class="card-title">{{ t("changePassword.title") }}</h1>
                <p class="card-subtitle">{{ t("changePassword.subtitle") }}</p>
            </div>

            <!-- Avatar Section -->
            <div class="avatar-section">
                <h3 class="section-title">{{ t("changePassword.avatar") }}</h3>
                <div class="avatar-upload">
                    <div class="avatar-preview" @click="triggerAvatarUpload">
                        <img
                            v-if="avatarUrl"
                            :src="avatarUrl"
                            alt="Avatar"
                            class="avatar-image"
                        />
                        <div v-else class="avatar-placeholder">
                            {{
                                authStore.username?.charAt(0).toUpperCase() ||
                                "U"
                            }}
                        </div>
                        <div class="avatar-overlay">
                            <Icon icon="lucide:camera" class="overlay-icon" />
                        </div>
                    </div>
                    <div class="avatar-info">
                        <button
                            type="button"
                            class="avatar-upload-button"
                            :disabled="avatarUploading"
                            @click="triggerAvatarUpload"
                        >
                            <Icon
                                v-if="avatarUploading"
                                icon="lucide:loader-2"
                                class="loading-icon"
                            />
                            <Icon v-else icon="lucide:upload" />
                            <span>{{
                                avatarUploading
                                    ? t("changePassword.updating")
                                    : t("changePassword.changeAvatar")
                            }}</span>
                        </button>
                        <p class="avatar-hint">
                            {{ t("changePassword.avatarHint") }}
                        </p>
                    </div>
                </div>
                <input
                    ref="avatarInput"
                    type="file"
                    accept="image/jpeg,image/png,image/gif,image/webp"
                    style="display: none"
                    @change="handleAvatarChange"
                />
                <!-- Avatar Error Message -->
                <div v-if="avatarMessage" class="message error-message">
                    <Icon icon="lucide:alert-circle" class="message-icon" />
                    <span>{{ avatarMessage }}</span>
                </div>
                <!-- Avatar Success Message -->
                <div
                    v-if="avatarSuccessMessage"
                    class="message success-message"
                >
                    <Icon icon="lucide:check-circle" class="message-icon" />
                    <span>{{ avatarSuccessMessage }}</span>
                </div>
            </div>

            <!-- Form -->
            <form
                @submit.prevent="handleChangePassword"
                class="change-password-form"
            >
                <!-- Old Password -->
                <div class="form-group">
                    <label for="old-password" class="form-label">{{
                        t("changePassword.currentPassword")
                    }}</label>
                    <div class="input-wrapper">
                        <Icon icon="lucide:key-round" class="input-icon" />
                        <input
                            id="old-password"
                            v-model="oldPassword"
                            :type="showOldPassword ? 'text' : 'password'"
                            class="form-input"
                            :placeholder="
                                t('changePassword.currentPasswordPlaceholder')
                            "
                            autocomplete="current-password"
                            @keydown="handleKeyPress"
                        />
                        <button
                            type="button"
                            class="toggle-password"
                            @click="showOldPassword = !showOldPassword"
                        >
                            <Icon
                                :icon="
                                    showOldPassword
                                        ? 'lucide:eye-off'
                                        : 'lucide:eye'
                                "
                            />
                        </button>
                    </div>
                </div>

                <!-- New Password -->
                <div class="form-group">
                    <label for="new-password" class="form-label">{{
                        t("changePassword.newPassword")
                    }}</label>
                    <div class="input-wrapper">
                        <Icon icon="lucide:lock" class="input-icon" />
                        <input
                            id="new-password"
                            v-model="newPassword"
                            :type="showNewPassword ? 'text' : 'password'"
                            class="form-input"
                            :placeholder="
                                t('changePassword.newPasswordPlaceholder')
                            "
                            autocomplete="new-password"
                            @keydown="handleKeyPress"
                        />
                        <button
                            type="button"
                            class="toggle-password"
                            @click="showNewPassword = !showNewPassword"
                        >
                            <Icon
                                :icon="
                                    showNewPassword
                                        ? 'lucide:eye-off'
                                        : 'lucide:eye'
                                "
                            />
                        </button>
                    </div>
                </div>

                <!-- Confirm Password -->
                <div class="form-group">
                    <label for="confirm-password" class="form-label">{{
                        t("changePassword.confirmPassword")
                    }}</label>
                    <div class="input-wrapper">
                        <Icon icon="lucide:lock-keyhole" class="input-icon" />
                        <input
                            id="confirm-password"
                            v-model="confirmPassword"
                            :type="showConfirmPassword ? 'text' : 'password'"
                            class="form-input"
                            :placeholder="
                                t('changePassword.confirmPasswordPlaceholder')
                            "
                            autocomplete="new-password"
                            @keydown="handleKeyPress"
                        />
                        <button
                            type="button"
                            class="toggle-password"
                            @click="showConfirmPassword = !showConfirmPassword"
                        >
                            <Icon
                                :icon="
                                    showConfirmPassword
                                        ? 'lucide:eye-off'
                                        : 'lucide:eye'
                                "
                            />
                        </button>
                    </div>
                </div>

                <!-- Error Message -->
                <div v-if="errorMessage" class="message error-message">
                    <Icon icon="lucide:alert-circle" class="message-icon" />
                    <span>{{ errorMessage }}</span>
                </div>

                <!-- Success Message -->
                <div v-if="successMessage" class="message success-message">
                    <Icon icon="lucide:check-circle" class="message-icon" />
                    <span>{{ successMessage }}</span>
                </div>

                <!-- Submit Button -->
                <button
                    type="submit"
                    class="submit-button"
                    :disabled="isChanging"
                >
                    <Icon
                        v-if="isChanging"
                        icon="lucide:loader-2"
                        class="loading-icon"
                    />
                    <span>{{
                        isChanging
                            ? t("changePassword.updating")
                            : t("changePassword.changePassword")
                    }}</span>
                </button>
            </form>

            <!-- Footer -->
            <!-- Username Update Section -->
            <div class="username-section">
                <h3 class="section-title">
                    {{ t("changePassword.updateUsername") }}
                </h3>
                <div class="form-group">
                    <label for="new-username" class="form-label">{{
                        t("changePassword.username")
                    }}</label>
                    <div class="input-wrapper">
                        <Icon icon="lucide:user" class="input-icon" />
                        <input
                            id="new-username"
                            v-model="newUsername"
                            type="text"
                            class="form-input username-input"
                            :placeholder="
                                t('changePassword.usernamePlaceholder')
                            "
                            autocomplete="username"
                            @keydown="handleUsernameKeyPress"
                        />
                    </div>
                </div>

                <!-- Username Error Message -->
                <div v-if="usernameMessage" class="message error-message">
                    <Icon icon="lucide:alert-circle" class="message-icon" />
                    <span>{{ usernameMessage }}</span>
                </div>

                <!-- Username Success Message -->
                <div
                    v-if="usernameSuccessMessage"
                    class="message success-message"
                >
                    <Icon icon="lucide:check-circle" class="message-icon" />
                    <span>{{ usernameSuccessMessage }}</span>
                </div>

                <!-- Update Username Button -->
                <button
                    type="button"
                    class="update-username-button"
                    :disabled="isUpdatingUsername"
                    @click="handleUpdateUsername"
                    mt-3
                >
                    <Icon
                        v-if="isUpdatingUsername"
                        icon="lucide:loader-2"
                        class="loading-icon"
                    />
                    <span>{{
                        isUpdatingUsername
                            ? t("changePassword.updating")
                            : t("changePassword.updateUsername")
                    }}</span>
                </button>
            </div>

            <!-- Footer -->
            <div class="card-footer">
                <p>{{ t("changePassword.passwordStoredSecurely") }}</p>
            </div>
        </div>
    </div>
</template>

<style scoped>
/* Container */
.change-password-container {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    position: relative;
    overflow: hidden;
    background: hsl(var(--background));
}

/* Floating orbs background animation */
.bg-orbs {
    position: fixed;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 0;
    overflow: hidden;
}

.orb {
    position: absolute;
    border-radius: 50%;
    filter: blur(80px);
    opacity: 0.3;
    animation: orb-float 25s ease-in-out infinite;
}

.orb-1 {
    width: 500px;
    height: 500px;
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(280 70% 60%));
    top: -100px;
    right: -100px;
    animation-delay: 0s;
}

.orb-2 {
    width: 400px;
    height: 400px;
    background: linear-gradient(135deg, hsl(320 70% 60%), hsl(var(--primary)));
    bottom: -80px;
    left: -80px;
    animation-delay: -8s;
}

.orb-3 {
    width: 350px;
    height: 350px;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.7),
        hsl(200 70% 70%)
    );
    top: 40%;
    left: 20%;
    animation-delay: -16s;
}

@keyframes orb-float {
    0%,
    100% {
        transform: translate(0, 0) scale(1);
    }
    33% {
        transform: translate(40px, -40px) scale(1.05);
    }
    66% {
        transform: translate(-30px, 30px) scale(0.95);
    }
}

/* Card */
.change-password-card {
    width: 100%;
    max-width: 460px;
    padding: 24px;
    background: hsl(var(--background) / 0.8);
    backdrop-filter: blur(20px) saturate(1.5);
    -webkit-backdrop-filter: blur(20px) saturate(1.5);
    border-radius: 16px;
    border: 1px solid hsl(var(--border) / 0.3);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.1);
    position: relative;
    z-index: 1;
}

/* Card Header */
.card-header {
    text-align: center;
    margin-bottom: 20px;
}

.header-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.15),
        hsl(280 70% 60% / 0.1)
    );
    margin-bottom: 12px;
}

.shield-icon {
    font-size: 24px;
    color: hsl(var(--primary));
}

.card-title {
    font-size: 20px;
    font-weight: 700;
    color: hsl(var(--foreground));
    margin: 0 0 6px 0;
}

.card-subtitle {
    font-size: 14px;
    color: hsl(var(--muted-foreground));
    margin: 0;
}

/* Avatar Section */
.avatar-section {
    margin-bottom: 1.5rem;
    padding-bottom: 1.5rem;
    border-bottom: 1px solid hsl(var(--border) / 0.3);
}

.avatar-upload {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-top: 0.75rem;
}

.avatar-preview {
    position: relative;
    width: 80px;
    height: 80px;
    border-radius: 50%;
    overflow: hidden;
    cursor: pointer;
    flex-shrink: 0;
    border: 3px solid hsl(var(--primary) / 0.5);
    transition: all 0.2s ease;
}

.avatar-preview:hover {
    border-color: hsl(var(--primary));
    transform: scale(1.05);
}

.avatar-image {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.avatar-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(280 70% 60%));
    color: hsl(var(--primary-foreground));
    font-size: 28px;
    font-weight: 700;
}

.avatar-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.2s ease;
}

.avatar-preview:hover .avatar-overlay {
    opacity: 1;
}

.overlay-icon {
    font-size: 24px;
    color: white;
}

.avatar-info {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.avatar-upload-button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border) / 0.3);
    border-radius: 8px;
    color: hsl(var(--foreground));
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s ease;
}

.avatar-upload-button:hover:not(:disabled) {
    background: hsl(var(--secondary) / 0.8);
    border-color: hsl(var(--primary) / 0.5);
}

.avatar-upload-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}

.avatar-upload-button .loading-icon {
    animation: spin 1s linear infinite;
}

.avatar-hint {
    font-size: 12px;
    color: hsl(var(--muted-foreground));
    margin: 0;
}

/* Form */
.change-password-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
}

/* Form Group */
.form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.form-label {
    font-size: 13px;
    font-weight: 600;
    color: hsl(var(--foreground));
}

/* Input Wrapper */
.input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
}

.input-icon {
    position: absolute;
    left: 12px;
    font-size: 18px;
    color: hsl(var(--muted-foreground));
    pointer-events: none;
}

.form-input {
    width: 100%;
    padding: 10px 44px 10px 36px;
    font-size: 14px;
    color: hsl(var(--foreground));
    background: hsl(var(--input) / 0.5);
    border: 1px solid hsl(var(--border) / 0.5);
    border-radius: 8px;
    outline: none;
    transition: all 0.2s ease;
}

.form-input.username-input {
    padding: 10px 10px 10px 36px;
}

.form-input::placeholder {
    color: hsl(var(--muted-foreground) / 0.6);
}

.form-input:focus {
    border-color: hsl(var(--primary) / 0.5);
    box-shadow: 0 0 0 3px hsl(var(--primary) / 0.1);
    background: hsl(var(--input) / 0.8);
}

/* Toggle Password Button */
.toggle-password {
    position: absolute;
    right: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s ease;
}

.toggle-password:hover {
    background: hsl(var(--secondary) / 0.5);
    color: hsl(var(--foreground));
}

.toggle-password :deep(svg) {
    width: 18px;
    height: 18px;
}

/* Messages */
.message {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 12px;
    animation: fadeIn 0.3s ease;
}

.message-icon {
    flex-shrink: 0;
    font-size: 16px;
}

.error-message {
    background: hsl(0 70% 50% / 0.08);
    border: 1px solid hsl(0 70% 50% / 0.2);
    color: hsl(0 70% 50%);
}

.success-message {
    background: hsl(142 70% 45% / 0.08);
    border: 1px solid hsl(142 70% 45% / 0.2);
    color: hsl(142 70% 45%);
}

@keyframes fadeIn {
    from {
        opacity: 0;
        transform: translateY(-5px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

/* Submit Button */
.submit-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    width: 100%;
    padding: 10px;
    font-size: 14px;
    font-weight: 600;
    color: hsl(var(--primary-foreground));
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(280 70% 60%));
    border: none;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.2);
}

.submit-button:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 6px 16px hsl(var(--primary) / 0.3);
}

.submit-button:active:not(:disabled) {
    transform: translateY(0);
}

.submit-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}

.loading-icon {
    animation: spin 1s linear infinite;
}

@keyframes spin {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

/* Card Footer */
.card-footer {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid hsl(var(--border) / 0.2);
    text-align: center;
    font-size: 12px;
    color: hsl(var(--muted-foreground));
}

.card-footer p {
    margin: 0;
}

/* Username Section */
.username-section {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid hsl(var(--border) / 0.2);
}

.section-title {
    font-size: 14px;
    font-weight: 600;
    color: hsl(var(--foreground));
    margin: 0 0 10px 0;
    text-align: center;
}

.update-username-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    width: 100%;
    padding: 10px;
    font-size: 14px;
    font-weight: 600;
    color: hsl(var(--primary-foreground));
    background: linear-gradient(
        135deg,
        hsl(var(--primary) / 0.8),
        hsl(280 70% 60% / 0.8)
    );
    border: 1px solid hsl(var(--primary) / 0.3);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;
}

.update-username-button:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.2);
}

.update-username-button:active:not(:disabled) {
    transform: translateY(0);
}

.update-username-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}

/* Responsive */
@media (max-width: 480px) {
    .change-password-card {
        padding: 24px;
    }

    .card-title {
        font-size: 20px;
    }

    .section-title {
        font-size: 14px;
    }
}
</style>

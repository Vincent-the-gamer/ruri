<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { useAuthStore } from "../stores/auth";
import { Icon } from "@iconify/vue";
import ruriAvatar from "../../assets/ruri-avatar.png";

const router = useRouter();
const { t } = useI18n();
const authStore = useAuthStore();

const username = ref("");
const password = ref("");
const showPassword = ref(false);
const errorMessage = ref("");

const isLoggingIn = computed(() => authStore.loading);

async function handleLogin() {
    errorMessage.value = "";

    if (!username.value || !password.value) {
        errorMessage.value = t("login.pleaseEnterCredentials");
        return;
    }

    try {
        await authStore.login({
            username: username.value,
            password: password.value,
        });

        // Redirect based on whether password change is required
        if (authStore.mustChangePassword) {
            router.push("/change-password");
        } else {
            router.push("/");
        }
    } catch (e: unknown) {
        errorMessage.value = authStore.error || t("login.loginFailed");
    }
}

function handleKeyPress(e: KeyboardEvent) {
    if (e.key === "Enter") {
        handleLogin();
    }
}
</script>

<template>
    <div class="login-container">
        <!-- Floating orbs background animation -->
        <div class="bg-orbs">
            <div class="orb orb-1"></div>
            <div class="orb orb-2"></div>
            <div class="orb orb-3"></div>
        </div>

        <!-- Login Card -->
        <div class="login-card">
            <!-- Logo and Title -->
            <div class="login-header">
                <img :src="ruriAvatar" alt="Ruri" class="login-avatar" />
                <h1 class="login-title">{{ t("login.title") }}</h1>
                <p class="login-subtitle">{{ t("login.subtitle") }}</p>
            </div>

            <!-- Default Credentials Notice -->
            <div class="default-credentials">
                <Icon icon="lucide:info" class="info-icon" />
                <span
                    >{{
                        t("login.defaultCredentialsHint", {
                            username: "ruri",
                            password: "ruri",
                        })
                    }}
                </span>
            </div>

            <!-- Login Form -->
            <form @submit.prevent="handleLogin" class="login-form">
                <!-- Username Input -->
                <div class="form-group">
                    <label for="username" class="form-label">{{
                        t("login.username")
                    }}</label>
                    <div class="input-wrapper">
                        <Icon icon="lucide:user" class="input-icon" />
                        <input
                            id="username"
                            v-model="username"
                            type="text"
                            class="form-input"
                            :placeholder="t('login.usernamePlaceholder')"
                            autocomplete="username"
                            @keydown="handleKeyPress"
                        />
                    </div>
                </div>

                <!-- Password Input -->
                <div class="form-group">
                    <label for="password" class="form-label">{{
                        t("login.password")
                    }}</label>
                    <div class="input-wrapper">
                        <Icon icon="lucide:lock" class="input-icon" />
                        <input
                            id="password"
                            v-model="password"
                            :type="showPassword ? 'text' : 'password'"
                            class="form-input"
                            :placeholder="t('login.passwordPlaceholder')"
                            autocomplete="current-password"
                            @keydown="handleKeyPress"
                        />
                        <button
                            type="button"
                            class="toggle-password"
                            @click="showPassword = !showPassword"
                        >
                            <Icon
                                :icon="
                                    showPassword
                                        ? 'lucide:eye-off'
                                        : 'lucide:eye'
                                "
                            />
                        </button>
                    </div>
                </div>

                <!-- Error Message -->
                <div v-if="errorMessage" class="error-message">
                    <Icon icon="lucide:alert-circle" class="error-icon" />
                    <span>{{ errorMessage }}</span>
                </div>

                <!-- Submit Button -->
                <button
                    type="submit"
                    class="login-button"
                    :disabled="isLoggingIn"
                >
                    <Icon
                        v-if="isLoggingIn"
                        icon="lucide:loader-2"
                        class="loading-icon"
                    />
                    <span>{{
                        isLoggingIn ? t("login.signingIn") : t("login.signIn")
                    }}</span>
                </button>
            </form>

            <!-- Footer -->
            <div class="login-footer">
                <p>
                    {{ t("login.onlyOneDefaultUser") }}
                </p>
            </div>
        </div>
    </div>
</template>

<style scoped>
/* Login Container */
.login-container {
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

/* Login Card */
.login-card {
    width: 100%;
    max-width: 420px;
    padding: 40px;
    background: hsl(var(--background) / 0.8);
    backdrop-filter: blur(20px) saturate(1.5);
    -webkit-backdrop-filter: blur(20px) saturate(1.5);
    border-radius: 16px;
    border: 1px solid hsl(var(--border) / 0.3);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.1);
    position: relative;
    z-index: 1;
}

/* Login Header */
.login-header {
    text-align: center;
    margin-bottom: 24px;
}

.login-avatar {
    display: block;
    width: 64px;
    height: 64px;
    border-radius: 50%;
    object-fit: cover;
    border: 3px solid hsl(var(--primary) / 0.5);
    margin: 0 auto 16px;
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.2);
}

.login-title {
    font-size: 28px;
    font-weight: 700;
    color: hsl(var(--foreground));
    margin: 0 0 8px 0;
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(280 70% 60%));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.login-subtitle {
    font-size: 14px;
    color: hsl(var(--muted-foreground));
    margin: 0;
}

/* Default Credentials Notice */
.default-credentials {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: hsl(var(--primary) / 0.08);
    border: 1px solid hsl(var(--primary) / 0.2);
    border-radius: 8px;
    margin-bottom: 24px;
    font-size: 13px;
    color: hsl(var(--primary));
}

.info-icon {
    flex-shrink: 0;
    font-size: 16px;
}

.default-credentials strong {
    font-weight: 600;
    color: hsl(var(--primary));
}

/* Login Form */
.login-form {
    display: flex;
    flex-direction: column;
    gap: 20px;
}

/* Form Group */
.form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
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
    padding: 12px 12px 12px 40px;
    font-size: 14px;
    color: hsl(var(--foreground));
    background: hsl(var(--input) / 0.5);
    border: 1px solid hsl(var(--border) / 0.5);
    border-radius: 8px;
    outline: none;
    transition: all 0.2s ease;
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

/* Error Message */
.error-message {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: hsl(0 70% 50% / 0.08);
    border: 1px solid hsl(0 70% 50% / 0.2);
    border-radius: 8px;
    font-size: 13px;
    color: hsl(0 70% 50%);
    animation: shake 0.5s ease-in-out;
}

.error-icon {
    flex-shrink: 0;
    font-size: 16px;
}

@keyframes shake {
    0%,
    100% {
        transform: translateX(0);
    }
    20% {
        transform: translateX(-5px);
    }
    40% {
        transform: translateX(5px);
    }
    60% {
        transform: translateX(-3px);
    }
    80% {
        transform: translateX(3px);
    }
}

/* Login Button */
.login-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 12px;
    font-size: 15px;
    font-weight: 600;
    color: hsl(var(--primary-foreground));
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(280 70% 60%));
    border: none;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.2);
}

.login-button:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 6px 16px hsl(var(--primary) / 0.3);
}

.login-button:active:not(:disabled) {
    transform: translateY(0);
}

.login-button:disabled {
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

/* Login Footer */
.login-footer {
    margin-top: 24px;
    padding-top: 16px;
    border-top: 1px solid hsl(var(--border) / 0.2);
    text-align: center;
    font-size: 12px;
    color: hsl(var(--muted-foreground));
}

.login-footer p {
    margin: 0;
}

/* Responsive */
@media (max-width: 480px) {
    .login-card {
        padding: 24px;
    }

    .login-title {
        font-size: 24px;
    }

    .login-avatar {
        width: 56px;
        height: 56px;
    }
}
</style>

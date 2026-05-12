<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import ruriImage from "../../assets/ruri.png";

const router = useRouter();
const { t } = useI18n();

// Mouse position for parallax effect
const mousePos = ref({ x: 0, y: 0 });
const imageOffset = ref({ x: 0, y: 0 });
const shadow1Offset = ref({ x: 8, y: 8 });
const shadow2Offset = ref({ x: -8, y: -8 });
const introOffset = ref({ x: 0, y: 0 });
const introRotation = ref(0);

// Animation frame for smooth movement
let animationFrame: number;

const handleMouseMove = (e: MouseEvent) => {
    mousePos.value = { x: e.clientX, y: e.clientY };
};

const updateImagePosition = () => {
    // Calculate offset from center
    const centerX = window.innerWidth / 2;
    const centerY = window.innerHeight / 2;

    // Normalized values (-1 to 1)
    const normalizedX = (mousePos.value.x - centerX) / centerX;
    const normalizedY = (mousePos.value.y - centerY) / centerY;

    // Reverse movement with smooth interpolation
    const targetX = -normalizedX * 15;
    const targetY = -normalizedY * 15;

    // Smooth interpolation
    imageOffset.value.x += (targetX - imageOffset.value.x) * 0.08;
    imageOffset.value.y += (targetY - imageOffset.value.y) * 0.08;

    // Intro section sway - follows character with softer, delayed motion
    const introTargetX = -normalizedX * 6;
    const introTargetY = -normalizedY * 4;
    const introTargetRotation = normalizedX * 1.2;
    introOffset.value.x += (introTargetX - introOffset.value.x) * 0.05;
    introOffset.value.y += (introTargetY - introOffset.value.y) * 0.05;
    introRotation.value += (introTargetRotation - introRotation.value) * 0.05;

    // Shadow offsets - different directions for 3D effect
    shadow1Offset.value = {
        x: 12 + normalizedX * 8,
        y: 12 + normalizedY * 8,
    };
    shadow2Offset.value = {
        x: -8 - normalizedX * 6,
        y: -8 - normalizedY * 6,
    };

    animationFrame = requestAnimationFrame(updateImagePosition);
};

onMounted(() => {
    window.addEventListener("mousemove", handleMouseMove);
    animationFrame = requestAnimationFrame(updateImagePosition);
});

onUnmounted(() => {
    window.removeEventListener("mousemove", handleMouseMove);
    if (animationFrame) {
        cancelAnimationFrame(animationFrame);
    }
});

const goToChat = () => {
    router.push("/chat");
};

const goToDashboard = () => {
    router.push("/dashboard");
};
</script>

<template>
    <div class="home-container">
        <!-- Main content -->
        <div class="home-content">
            <!-- Character section -->
            <div class="character-section">
                <div class="character-wrapper">
                    <!-- Shadow layers -->
                    <div
                        class="character-shadow shadow-layer-1"
                        :style="{
                            transform: `translate(${shadow1Offset.x}px, ${shadow1Offset.y}px) scale(1.02)`,
                        }"
                    >
                        <img
                            :src="ruriImage"
                            alt="Ruri Shadow 1"
                            class="shadow-img"
                        />
                    </div>
                    <div
                        class="character-shadow shadow-layer-2"
                        :style="{
                            transform: `translate(${shadow2Offset.x}px, ${shadow2Offset.y}px) scale(0.98)`,
                        }"
                    >
                        <img
                            :src="ruriImage"
                            alt="Ruri Shadow 2"
                            class="shadow-img"
                        />
                    </div>

                    <!-- Main character image -->
                    <div
                        class="character-main"
                        :style="{
                            transform: `translate(${imageOffset.x}px, ${imageOffset.y}px)`,
                        }"
                    >
                        <img :src="ruriImage" alt="Ruri" class="main-img" />
                        <div class="character-glow"></div>
                    </div>
                </div>
            </div>

            <!-- Text section -->
            <div
                class="intro-section"
                :style="{
                    transform: `translate(${introOffset.x}px, ${introOffset.y}px) rotate(${introRotation}deg)`,
                }"
            >
                <div class="intro-content">
                    <h1 class="title">
                        <span class="title-gradient">{{
                            t("home.welcome")
                        }}</span>
                    </h1>
                    <h2 class="subtitle">
                        <span class="character-name">Ruri</span>
                        <span class="divider">·</span>
                        <span class="tagline">{{ t("home.tagline") }}</span>
                    </h2>

                    <p class="description">
                        {{ t("home.description") }}
                    </p>

                    <!-- Feature highlights -->
                    <div class="features">
                        <div class="feature-item">
                            <svg
                                class="feature-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <circle cx="12" cy="12" r="3" />
                                <path d="M12 1v6m0 6v10M1 12h6m6 0h10" />
                            </svg>
                            <span>{{ t("home.features.multiProvider") }}</span>
                        </div>
                        <div class="feature-item">
                            <svg
                                class="feature-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <path
                                    d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"
                                />
                            </svg>
                            <span>{{ t("home.features.toolFramework") }}</span>
                        </div>
                        <div class="feature-item">
                            <svg
                                class="feature-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <polygon
                                    points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
                                />
                            </svg>
                            <span>{{ t("home.features.skillSystem") }}</span>
                        </div>
                        <div class="feature-item">
                            <svg
                                class="feature-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
                                <path
                                    d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"
                                />
                            </svg>
                            <span>{{ t("home.features.acpSupport") }}</span>
                        </div>
                    </div>

                    <!-- Action buttons -->
                    <div class="actions">
                        <button class="btn btn-start" @click="goToChat">
                            <svg
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <path
                                    d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"
                                />
                            </svg>
                            {{ t("home.startChat") }}
                        </button>
                        <button class="btn btn-explore" @click="goToDashboard">
                            <svg
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <rect x="3" y="3" width="7" height="7" />
                                <rect x="14" y="3" width="7" height="7" />
                                <rect x="14" y="14" width="7" height="7" />
                                <rect x="3" y="14" width="7" height="7" />
                            </svg>
                            {{ t("home.exploreFeatures") }}
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Bottom decoration removed -->
    </div>
</template>

<style scoped>
.home-container {
    min-height: 100vh;
    width: 100%;
    display: flex;
    flex-direction: column;
}

/* Main content */
.home-content {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    position: relative;
    z-index: 1;
    gap: 4rem;
}

/* Character section */
.character-section {
    flex-shrink: 0;
}

.character-wrapper {
    position: relative;
    width: 380px;
    height: 500px;
}

.character-shadow {
    position: absolute;
    inset: 0;
    transition: transform 0.1s ease-out;
}

.shadow-layer-1 {
    filter: blur(2px);
}

/* Shadow layer 1 - 可爱的浅粉色 */
.shadow-layer-1 .shadow-img {
    filter: brightness(0) saturate(100%) hue-rotate(320deg) brightness(1)
        opacity(0.6);
    opacity: 0.6;
}

.shadow-layer-2 {
    filter: blur(3px);
}

/* Shadow layer 2 - 可爱的浅蓝色 */
.shadow-layer-2 .shadow-img {
    filter: brightness(0) saturate(100%) hue-rotate(200deg) brightness(1.1)
        opacity(0.5);
    opacity: 0.5;
}

.shadow-img {
    width: 100%;
    height: 100%;
    object-fit: contain;
}

.character-main {
    position: relative;
    z-index: 10;
    transition: transform 0.15s ease-out;
    will-change: transform;
}

.main-img {
    width: 100%;
    height: 100%;
    object-fit: contain;
}

.character-glow {
    position: absolute;
    inset: -20px;
    background: radial-gradient(
        circle at center,
        hsl(var(--primary) / 0.2) 0%,
        transparent 70%
    );
    z-index: -1;
    animation: pulse-glow 3s ease-in-out infinite;
}

@keyframes pulse-glow {
    0%,
    100% {
        opacity: 0.5;
        transform: scale(1);
    }
    50% {
        opacity: 0.8;
        transform: scale(1.05);
    }
}

/* Intro section */
.intro-section {
    max-width: 540px;
    transition: transform 0.15s ease-out;
    will-change: transform;
}

.intro-content {
    animation: slideIn 0.8s ease-out;
}

@keyframes slideIn {
    from {
        opacity: 0;
        transform: translateX(30px);
    }
    to {
        opacity: 1;
        transform: translateX(0);
    }
}

.title {
    font-size: 3.5rem;
    font-weight: 800;
    line-height: 1.1;
    margin-bottom: 0.5rem;
}

.title-gradient {
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(280 70% 65%) 50%,
        hsl(var(--primary)) 100%
    );
    background-size: 200% auto;
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
    animation: gradient-shift 4s ease infinite;
}

@keyframes gradient-shift {
    0%,
    100% {
        background-position: 0% center;
    }
    50% {
        background-position: 100% center;
    }
}

.subtitle {
    font-size: 1.75rem;
    font-weight: 600;
    color: hsl(var(--muted-foreground));
    margin-bottom: 1.5rem;
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.character-name {
    color: hsl(var(--primary));
    font-weight: 700;
}

.divider {
    color: hsl(var(--border));
    display: inline-flex;
    align-items: center;
    line-height: 1;
}

.tagline {
    font-weight: 500;
}

.description {
    font-size: 1.125rem;
    line-height: 1.7;
    color: hsl(var(--muted-foreground));
    margin-bottom: 2rem;
}

/* Features */
.features {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
    margin-bottom: 2.5rem;
}

.feature-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.875rem 1rem;
    background: hsl(var(--card) / 0.5);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid hsl(var(--border) / 0.4);
    border-radius: 12px;
    font-size: 0.875rem;
    font-weight: 500;
    color: hsl(var(--foreground));
    transition: all 0.2s ease;
}

.feature-item:hover {
    border-color: hsl(var(--primary) / 0.5);
    background: hsl(var(--primary) / 0.05);
    transform: translateX(4px);
}

.feature-icon {
    width: 20px;
    height: 20px;
    color: hsl(var(--primary));
    flex-shrink: 0;
}

/* Actions */
.actions {
    display: flex;
    gap: 1rem;
}

.btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.875rem 1.5rem;
    font-size: 1rem;
    font-weight: 600;
    border-radius: 12px;
    cursor: pointer;
    transition: all 0.2s ease;
    border: none;
}

.btn svg {
    width: 20px;
    height: 20px;
}

.btn-start {
    background: linear-gradient(135deg, hsl(var(--primary)), hsl(207 70% 50%));
    color: white;
    box-shadow: 0 4px 20px hsl(var(--primary) / 0.4);
}

.btn-start:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 30px hsl(var(--primary) / 0.5);
}

.btn-explore {
    background: hsl(var(--card));
    color: hsl(var(--foreground));
    border: 2px solid hsl(var(--border));
}

.btn-explore:hover {
    border-color: hsl(var(--primary) / 0.5);
    background: hsl(var(--primary) / 0.05);
}

/* Bottom decoration */
.bottom-decoration {
    position: absolute;
    bottom: 2rem;
    left: 50%;
    transform: translateX(-50%);
    z-index: 10;
}

.scroll-hint {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    color: hsl(var(--muted-foreground));
    font-size: 0.875rem;
    animation: bounce 2s ease-in-out infinite;
}

.scroll-hint svg {
    width: 24px;
    height: 24px;
}

@keyframes bounce {
    0%,
    100% {
        transform: translateY(0);
    }
    50% {
        transform: translateY(8px);
    }
}

/* Responsive */
@media (max-width: 1024px) {
    .home-content {
        flex-direction: column-reverse;
        text-align: center;
        gap: 2rem;
    }

    .character-wrapper {
        width: 280px;
        height: 380px;
    }

    .title {
        font-size: 2.5rem;
    }

    .subtitle {
        justify-content: center;
        font-size: 1.25rem;
    }

    .features {
        grid-template-columns: 1fr;
    }

    .actions {
        justify-content: center;
    }

    .intro-section {
        max-width: 100%;
    }
}

@media (max-width: 640px) {
    .home-content {
        padding: 1rem;
    }

    .character-wrapper {
        width: 220px;
        height: 300px;
    }

    .title {
        font-size: 2rem;
    }

    .subtitle {
        font-size: 1rem;
        flex-wrap: wrap;
    }

    .description {
        font-size: 1rem;
    }

    .features {
        gap: 0.75rem;
    }

    .feature-item {
        padding: 0.75rem;
        font-size: 0.8125rem;
    }

    .actions {
        flex-direction: column;
        width: 100%;
    }

    .btn {
        width: 100%;
        justify-content: center;
    }

    .bottom-decoration {
        display: none;
    }
}
</style>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue';

interface Particle {
  id: number;
  x: number;
  y: number;
  size: number;
  delay: number;
  duration: number;
  type: 'sparkle' | 'heart' | 'star';
}

const props = withDefaults(defineProps<{
  size?: 'small' | 'medium' | 'large';
  mood?: 'happy' | 'sad' | 'thinking' | 'excited';
  showParticles?: boolean;
  interactive?: boolean;
}>(), {
  size: 'medium',
  mood: 'happy',
  showParticles: true,
  interactive: true
});

const isHovered = ref(false);
const particles = ref<Particle[]>([]);
const containerRef = ref<HTMLDivElement>();

const sizeMap = {
  small: { width: 120, height: 160 },
  medium: { width: 180, height: 240 },
  large: { width: 240, height: 320 }
};

const moodEmojis = {
  happy: '😊',
  sad: '😢',
  thinking: '🤔',
  excited: '🥰'
};

function generateParticles(): Particle[] {
  const newParticles: Particle[] = [];
  const types: Particle['type'][] = ['sparkle', 'heart', 'star'];
  const { width, height } = sizeMap[props.size];

  for (let i = 0; i < 12; i++) {
    const angle = (i / 12) * Math.PI * 2;
    const radius = 80 + Math.random() * 40;
    newParticles.push({
      id: i,
      x: Math.cos(angle) * radius,
      y: Math.sin(angle) * radius,
      size: 4 + Math.random() * 8,
      delay: Math.random() * 3,
      duration: 2 + Math.random() * 2,
      type: types[Math.floor(Math.random() * types.length)]
    });
  }

  return newParticles;
}

function handleHover() {
  if (!props.interactive) return;
  isHovered.value = true;
}

function handleLeave() {
  if (!props.interactive) return;
  isHovered.value = false;
}

function handleClick() {
  if (!props.interactive) return;
  // Burst of particles on click
  const burstCount = 8;
  for (let i = 0; i < burstCount; i++) {
    const angle = (i / burstCount) * Math.PI * 2;
    particles.value.push({
      id: Date.now() + i,
      x: Math.cos(angle) * 60,
      y: Math.sin(angle) * 60,
      size: 6 + Math.random() * 6,
      delay: 0,
      duration: 1,
      type: 'heart'
    });
  }
  setTimeout(() => {
    particles.value = generateParticles();
  }, 1000);
}

onMounted(() => {
  particles.value = generateParticles();
});
</script>

<template>
  <div
    ref="containerRef"
    class="anime-girl-container"
    :class="`anime-girl-${size}`"
    @mouseenter="handleHover"
    @mouseleave="handleLeave"
    @click="handleClick"
  >
    <!-- Particle effects -->
    <div v-if="showParticles" class="particle-field">
      <div
        v-for="particle in particles"
        :key="particle.id"
        class="particle"
        :class="`particle-${particle.type}`"
        :style="{
          '--particle-x': `${particle.x}px`,
          '--particle-y': `${particle.y}px`,
          '--particle-size': `${particle.size}px`,
          'animation-delay': `${particle.delay}s`,
          'animation-duration': `${particle.duration}s`
        }"
      >
        <template v-if="particle.type === 'sparkle'">
          ✨
        </template>
        <template v-else-if="particle.type === 'heart'">
          💕
        </template>
        <template v-else>
          ⭐
        </template>
      </div>
    </div>

    <!-- Anime Girl Character (SVG) -->
    <div class="character-wrapper">
      <svg
        class="anime-character"
        viewBox="0 0 200 280"
        xmlns="http://www.w3.org/2000/svg"
      >
        <defs>
          <!-- Hair gradient -->
          <linearGradient id="hairGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#c084fc" />
            <stop offset="50%" stop-color="#a855f7" />
            <stop offset="100%" stop-color="#9333ea" />
          </linearGradient>

          <!-- Skin gradient -->
          <linearGradient id="skinGradient" x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stop-color="#fef3c7" />
            <stop offset="100%" stop-color="#fde68a" />
          </linearGradient>

          <!-- Eye gradient -->
          <radialGradient id="eyeGradient" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stop-color="#ec4899" />
            <stop offset="60%" stop-color="#a855f7" />
            <stop offset="100%" stop-color="#7c3aed" />
          </radialGradient>

          <!-- Blush gradient -->
          <radialGradient id="blushGradient" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stop-color="rgba(236, 72, 153, 0.4)" />
            <stop offset="100%" stop-color="rgba(236, 72, 153, 0)" />
          </radialGradient>

          <!-- Glow filter -->
          <filter id="glow">
            <feGaussianBlur stdDeviation="3" result="coloredBlur" />
            <feMerge>
              <feMergeNode in="coloredBlur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>

          <!-- Hair shine -->
          <linearGradient id="hairShine" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="rgba(255,255,255,0.4)" />
            <stop offset="100%" stop-color="rgba(255,255,255,0)" />
          </linearGradient>
        </defs>

        <!-- Background glow -->
        <ellipse
          cx="100"
          cy="140"
          rx="70"
          ry="90"
          fill="rgba(236, 72, 153, 0.08)"
          class="bg-glow"
        />

        <!-- Body / Outfit -->
        <g class="body-group">
          <!-- Neck -->
          <rect x="88" y="120" width="24" height="20" rx="5" fill="url(#skinGradient)" />

          <!-- School uniform top -->
          <path
            d="M60 140 L40 280 L160 280 L140 140 Z"
            fill="#f8fafc"
            stroke="#e2e8f0"
            stroke-width="1"
          />

          <!-- Collar -->
          <path
            d="M75 140 L100 170 L125 140"
            fill="#1e293b"
            stroke="#0f172a"
            stroke-width="0.5"
          />

          <!-- Ribbon -->
          <path
            d="M92 155 L100 165 L108 155 L100 160 Z"
            fill="#ec4899"
          />
          <path
            d="M100 160 L85 175 L95 165 Z"
            fill="#ec4899"
          />
          <path
            d="M100 160 L115 175 L105 165 Z"
            fill="#ec4899"
          />

          <!-- Skirt -->
          <path
            d="M55 220 L35 280 L165 280 L145 220 Z"
            fill="#1e293b"
            stroke="#0f172a"
            stroke-width="0.5"
          />

          <!-- Skirt pleats -->
          <line x1="75" y1="220" x2="65" y2="280" stroke="#334155" stroke-width="0.5" />
          <line x1="100" y1="220" x2="100" y2="280" stroke="#334155" stroke-width="0.5" />
          <line x1="125" y1="220" x2="135" y2="280" stroke="#334155" stroke-width="0.5" />
        </g>

        <!-- Head -->
        <g class="head-group">
          <!-- Back hair -->
          <ellipse cx="100" cy="85" rx="65" ry="75" fill="url(#hairGradient)" />

          <!-- Left side hair strands -->
          <path
            d="M40 80 Q30 120 35 160 Q37 170 42 165 Q45 140 50 100 Z"
            fill="url(#hairGradient)"
            class="hair-strand-left"
          />
          <path
            d="M55 75 Q45 110 48 150 Q50 160 55 155 Q58 130 60 95 Z"
            fill="url(#hairGradient)"
            class="hair-strand-left-2"
          />

          <!-- Right side hair strands -->
          <path
            d="M160 80 Q170 120 165 160 Q163 170 158 165 Q155 140 150 100 Z"
            fill="url(#hairGradient)"
            class="hair-strand-right"
          />
          <path
            d="M145 75 Q155 110 152 150 Q150 160 145 155 Q142 130 140 95 Z"
            fill="url(#hairGradient)"
            class="hair-strand-right-2"
          />

          <!-- Face -->
          <ellipse cx="100" cy="90" rx="50" ry="55" fill="url(#skinGradient)" />

          <!-- Ears -->
          <ellipse cx="52" cy="95" rx="8" ry="12" fill="url(#skinGradient)" />
          <ellipse cx="148" cy="95" rx="8" ry="12" fill="url(#skinGradient)" />

          <!-- Ear accessories -->
          <circle cx="52" cy="98" r="4" fill="#ec4899" />
          <circle cx="148" cy="98" r="4" fill="#ec4899" />

          <!-- Eyes -->
          <g class="eyes-group">
            <!-- Left eye -->
            <g class="eye left-eye">
              <ellipse cx="78" cy="90" rx="14" ry="16" fill="white" />
              <ellipse cx="78" cy="92" rx="10" ry="12" fill="url(#eyeGradient)" />
              <ellipse cx="78" cy="92" rx="5" ry="6" fill="#1e1b4b" />
              <circle cx="74" cy="88" r="3" fill="white" class="eye-highlight" />
              <circle cx="82" cy="94" r="1.5" fill="white" class="eye-highlight-small" />
              <!-- Eyelashes -->
              <path
                d="M64 85 Q70 78 78 76 Q86 78 92 85"
                stroke="#1e1b4b"
                stroke-width="2"
                fill="none"
                stroke-linecap="round"
              />
            </g>

            <!-- Right eye -->
            <g class="eye right-eye">
              <ellipse cx="122" cy="90" rx="14" ry="16" fill="white" />
              <ellipse cx="122" cy="92" rx="10" ry="12" fill="url(#eyeGradient)" />
              <ellipse cx="122" cy="92" rx="5" ry="6" fill="#1e1b4b" />
              <circle cx="118" cy="88" r="3" fill="white" class="eye-highlight" />
              <circle cx="126" cy="94" r="1.5" fill="white" class="eye-highlight-small" />
              <!-- Eyelashes -->
              <path
                d="M108 85 Q114 78 122 76 Q130 78 136 85"
                stroke="#1e1b4b"
                stroke-width="2"
                fill="none"
                stroke-linecap="round"
              />
            </g>
          </g>

          <!-- Eyebrows -->
          <path
            d="M65 75 Q78 70 90 75"
            stroke="#7c3aed"
            stroke-width="2"
            fill="none"
            stroke-linecap="round"
            class="eyebrow-left"
          />
          <path
            d="M110 75 Q122 70 135 75"
            stroke="#7c3aed"
            stroke-width="2"
            fill="none"
            stroke-linecap="round"
            class="eyebrow-right"
          />

          <!-- Nose -->
          <path
            d="M98 105 Q100 108 102 105"
            stroke="#d4a574"
            stroke-width="1.5"
            fill="none"
            stroke-linecap="round"
          />

          <!-- Mouth -->
          <g class="mouth-group">
            <!-- Happy mouth -->
            <path
              v-if="mood === 'happy'"
              d="M90 115 Q100 125 110 115"
              stroke="#c084fc"
              stroke-width="2"
              fill="none"
              stroke-linecap="round"
            />
            <!-- Sad mouth -->
            <path
              v-else-if="mood === 'sad'"
              d="M90 120 Q100 115 110 120"
              stroke="#c084fc"
              stroke-width="2"
              fill="none"
              stroke-linecap="round"
            />
            <!-- Thinking mouth -->
            <line
              v-else-if="mood === 'thinking'"
              x1="95"
              y1="118"
              x2="105"
              y2="118"
              stroke="#c084fc"
              stroke-width="2"
              stroke-linecap="round"
            />
            <!-- Excited mouth (open) -->
            <path
              v-else
              d="M90 115 Q100 130 110 115 Z"
              fill="#fda4af"
              stroke="#c084fc"
              stroke-width="1.5"
            />
          </g>

          <!-- Blush -->
          <ellipse cx="65" cy="105" rx="12" ry="6" fill="url(#blushGradient)" />
          <ellipse cx="135" cy="105" rx="12" ry="6" fill="url(#blushGradient)" />

          <!-- Front hair / Bangs -->
          <g class="bangs-group">
            <path
              d="M50 80 Q55 50 70 45 Q80 42 90 48 Q95 52 90 60 Q85 55 75 58 Q65 62 60 75 Z"
              fill="url(#hairGradient)"
            />
            <path
              d="M70 45 Q80 35 100 32 Q115 30 125 38 Q135 45 130 55 Q125 50 115 48 Q105 46 95 48 Q85 52 80 55 Q75 58 70 55 Z"
              fill="url(#hairGradient)"
            />
            <path
              d="M125 38 Q140 42 150 55 Q155 62 150 75 Q145 65 135 60 Q125 55 120 50 Z"
              fill="url(#hairGradient)"
            />
            <!-- Hair shine -->
            <path
              d="M85 40 Q95 38 105 42 Q110 45 105 48 Q95 46 88 44 Z"
              fill="url(#hairShine)"
            />
          </g>

          <!-- Hair accessories -->
          <g class="hair-accessories">
            <!-- Left hair clip -->
            <circle cx="55" cy="65" r="5" fill="#ec4899" filter="url(#glow)" />
            <circle cx="55" cy="65" r="2" fill="white" />
            <!-- Right hair ribbon -->
            <path
              d="M145 55 L155 50 L152 60 L160 58 L150 65 Z"
              fill="#ec4899"
              filter="url(#glow)"
            />
          </g>
        </g>

        <!-- Mood indicator emoji -->
        <text
          v-if="mood !== 'happy'"
          class="mood-emoji"
          x="150"
          y="40"
          font-size="20"
          text-anchor="middle"
        >
          {{ moodEmojis[mood] }}
        </text>
      </svg>

      <!-- Name tag -->
      <div class="name-tag">
        <span class="name-text">琉璃</span>
        <span class="name-subtitle">Ruri</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.anime-girl-container {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: transform 0.3s ease;
}

.anime-girl-container:hover {
  transform: scale(1.05);
}

.anime-girl-small {
  width: 120px;
  height: 160px;
}

.anime-girl-medium {
  width: 180px;
  height: 240px;
}

.anime-girl-large {
  width: 240px;
  height: 320px;
}

/* Particle field */
.particle-field {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 100%;
  height: 100%;
  transform: translate(-50%, -50%);
  pointer-events: none;
}

.particle {
  position: absolute;
  top: 50%;
  left: 50%;
  font-size: var(--particle-size);
  animation: particleFloat 3s ease-in-out infinite;
  opacity: 0.7;
  filter: drop-shadow(0 0 4px rgba(236, 72, 153, 0.5));
}

.particle-sparkle {
  animation: particleFloat 3s ease-in-out infinite, sparklePulse 1.5s ease-in-out infinite;
}

.particle-heart {
  animation: particleFloat 3s ease-in-out infinite, heartBeat 2s ease-in-out infinite;
}

.particle-star {
  animation: particleFloat 3s ease-in-out infinite, starTwinkle 2s ease-in-out infinite;
}

@keyframes particleFloat {
  0%, 100% {
    transform: translate(var(--particle-x), var(--particle-y)) rotate(0deg);
  }
  25% {
    transform: translate(calc(var(--particle-x) * 1.1), calc(var(--particle-y) * 0.9)) rotate(15deg);
  }
  50% {
    transform: translate(calc(var(--particle-x) * 0.9), calc(var(--particle-y) * 1.1)) rotate(-15deg);
  }
  75% {
    transform: translate(calc(var(--particle-x) * 1.05), calc(var(--particle-y) * 0.95)) rotate(10deg);
  }
}

@keyframes sparklePulse {
  0%, 100% {
    opacity: 0.5;
    transform: scale(1);
  }
  50% {
    opacity: 1;
    transform: scale(1.3);
  }
}

@keyframes heartBeat {
  0%, 100% {
    transform: scale(1);
  }
  15% {
    transform: scale(1.2);
  }
  30% {
    transform: scale(1);
  }
  45% {
    transform: scale(1.15);
  }
}

@keyframes starTwinkle {
  0%, 100% {
    opacity: 0.3;
    transform: rotate(0deg);
  }
  50% {
    opacity: 1;
    transform: rotate(45deg);
  }
}

/* Character wrapper */
.character-wrapper {
  position: relative;
  width: 100%;
  height: 100%;
  animation: breathe 4s ease-in-out infinite;
}

@keyframes breathe {
  0%, 100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-4px);
  }
}

/* Anime character SVG */
.anime-character {
  width: 100%;
  height: 100%;
  filter: drop-shadow(0 4px 12px rgba(168, 85, 247, 0.2));
}

/* Background glow pulse */
.bg-glow {
  animation: bgGlow 3s ease-in-out infinite;
}

@keyframes bgGlow {
  0%, 100% {
    opacity: 0.5;
    transform: scale(1);
  }
  50% {
    opacity: 0.8;
    transform: scale(1.05);
  }
}

/* Hair sway animation */
.hair-strand-left,
.hair-strand-left-2,
.hair-strand-right,
.hair-strand-right-2 {
  animation: hairSway 5s ease-in-out infinite;
  transform-origin: top center;
}

.hair-strand-left,
.hair-strand-left-2 {
  animation-delay: 0s;
}

.hair-strand-right,
.hair-strand-right-2 {
  animation-delay: 0.5s;
}

@keyframes hairSway {
  0%, 100% {
    transform: rotate(0deg);
  }
  25% {
    transform: rotate(2deg);
  }
  75% {
    transform: rotate(-2deg);
  }
}

/* Bangs subtle movement */
.bangs-group {
  animation: bangsSway 6s ease-in-out infinite;
  transform-origin: center top;
}

@keyframes bangsSway {
  0%, 100% {
    transform: rotate(0deg);
  }
  50% {
    transform: rotate(1deg);
  }
}

/* Eye blink animation */
.eye {
  animation: blink 4s ease-in-out infinite;
}

.left-eye {
  animation-delay: 0s;
}

.right-eye {
  animation-delay: 0.05s;
}

@keyframes blink {
  0%, 45%, 55%, 100% {
    transform: scaleY(1);
  }
  50% {
    transform: scaleY(0.1);
  }
}

/* Eye highlight shimmer */
.eye-highlight {
  animation: highlightShimmer 3s ease-in-out infinite;
}

.eye-highlight-small {
  animation: highlightShimmer 3s ease-in-out infinite;
  animation-delay: 0.5s;
}

@keyframes highlightShimmer {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.6;
  }
}

/* Eyebrow expression */
.eyebrow-left,
.eyebrow-right {
  animation: eyebrowMove 5s ease-in-out infinite;
}

.eyebrow-left {
  animation-delay: 0s;
}

.eyebrow-right {
  animation-delay: 0.2s;
}

@keyframes eyebrowMove {
  0%, 100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-2px);
  }
}

/* Mouth subtle animation */
.mouth-group {
  animation: mouthPulse 3s ease-in-out infinite;
}

@keyframes mouthPulse {
  0%, 100% {
    transform: scale(1);
  }
  50% {
    transform: scale(1.05);
  }
}

/* Hair accessories glow */
.hair-accessories {
  animation: accessoryGlow 2s ease-in-out infinite;
}

@keyframes accessoryGlow {
  0%, 100% {
    filter: drop-shadow(0 0 2px rgba(236, 72, 153, 0.3));
  }
  50% {
    filter: drop-shadow(0 0 8px rgba(236, 72, 153, 0.6));
  }
}

/* Mood emoji float */
.mood-emoji {
  animation: moodFloat 2s ease-in-out infinite;
}

@keyframes moodFloat {
  0%, 100% {
    transform: translateY(0);
    opacity: 0.8;
  }
  50% {
    transform: translateY(-5px);
    opacity: 1;
  }
}

/* Name tag */
.name-tag {
  position: absolute;
  bottom: 0;
  left: 50%;
  transform: translateX(-50%);
  text-align: center;
  padding: 4px 16px;
  background: linear-gradient(135deg, rgba(236, 72, 153, 0.2), rgba(168, 85, 247, 0.2));
  border: 1px solid rgba(236, 72, 153, 0.3);
  border-radius: 12px;
  backdrop-filter: blur(8px);
  white-space: nowrap;
}

.name-text {
  font-size: 14px;
  font-weight: 700;
  background: linear-gradient(135deg, #ec4899, #a855f7);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.name-subtitle {
  display: block;
  font-size: 10px;
  color: rgba(168, 85, 247, 0.7);
  letter-spacing: 2px;
  text-transform: uppercase;
}

/* Hover effects */
.anime-girl-container:hover .anime-character {
  filter: drop-shadow(0 4px 20px rgba(236, 72, 153, 0.4));
}

.anime-girl-container:hover .bg-glow {
  opacity: 1;
  transform: scale(1.1);
}

.anime-girl-container:hover .name-tag {
  background: linear-gradient(135deg, rgba(236, 72, 153, 0.3), rgba(168, 85, 247, 0.3));
  border-color: rgba(236, 72, 153, 0.5);
}

/* Interactive click burst */
.particle:active {
  animation: burst 0.5s ease-out forwards;
}

@keyframes burst {
  0% {
    transform: scale(1);
    opacity: 1;
  }
  100% {
    transform: scale(2);
    opacity: 0;
  }
}

/* Responsive adjustments */
@media (max-width: 768px) {
  .anime-girl-small {
    width: 80px;
    height: 120px;
  }

  .anime-girl-medium {
    width: 120px;
    height: 160px;
  }

  .anime-girl-large {
    width: 160px;
    height: 220px;
  }
}

@media (max-width: 480px) {
  .particle {
    font-size: calc(var(--particle-size) * 0.7) !important;
  }
}
</style>
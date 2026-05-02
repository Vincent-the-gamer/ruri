<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue';

interface Sparkle {
  id: number;
  x: number;
  y: number;
  size: number;
  delay: number;
  duration: number;
  opacity: number;
  color: string;
  twinkleSpeed: number;
}

const props = withDefaults(defineProps<{
  count?: number;
  colors?: string[];
  maxSize?: number;
  minSize?: number;
}>(), {
  count: 50,
  maxSize: 20,
  minSize: 4,
  colors: () => ['#fff', '#ffd700', '#ffb7c5', '#e0b0ff', '#87ceeb']
});

const containerRef = ref<HTMLDivElement>();
const sparkles = ref<Sparkle[]>([]);
let animationFrameId: number | null = null;

function generateSparkles(): Sparkle[] {
  const newSparkles: Sparkle[] = [];

  for (let i = 0; i < props.count; i++) {
    newSparkles.push({
      id: i,
      x: Math.random() * 100,
      y: Math.random() * 100,
      size: props.minSize + Math.random() * (props.maxSize - props.minSize),
      delay: Math.random() * 5,
      duration: 2 + Math.random() * 4,
      opacity: 0.3 + Math.random() * 0.7,
      color: props.colors[Math.floor(Math.random() * props.colors.length)],
      twinkleSpeed: 1 + Math.random() * 3
    });
  }

  return newSparkles;
}

function createStarPath(points: number): string {
  const outerRadius = 50;
  const innerRadius = 20;
  let path = '';

  for (let i = 0; i < points * 2; i++) {
    const radius = i % 2 === 0 ? outerRadius : innerRadius;
    const angle = (Math.PI * i) / points - Math.PI / 2;
    const x = 50 + radius * Math.cos(angle);
    const y = 50 + radius * Math.sin(angle);
    path += (i === 0 ? 'M' : 'L') + `${x} ${y}`;
  }

  path += 'Z';
  return path;
}

onMounted(() => {
  sparkles.value = generateSparkles();
});

onBeforeUnmount(() => {
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId);
  }
});
</script>

<template>
  <div
    ref="containerRef"
    class="sparkle-container"
  >
    <div
      v-for="sparkle in sparkles"
      :key="sparkle.id"
      class="sparkle"
      :style="{
        left: `${sparkle.x}%`,
        top: `${sparkle.y}%`,
        width: `${sparkle.size}px`,
        height: `${sparkle.size}px`,
        animationDelay: `${sparkle.delay}s`,
        animationDuration: `${sparkle.duration}s`,
        '--twinkle-speed': `${sparkle.twinkleSpeed}s`,
        '--sparkle-opacity': sparkle.opacity
      }"
    >
      <!-- 4-point star -->
      <svg
        viewBox="0 0 100 100"
        class="sparkle-star"
        :fill="sparkle.color"
      >
        <path :d="createStarPath(4)" />
      </svg>
      <!-- Glow effect -->
      <div class="sparkle-glow" :style="{ backgroundColor: sparkle.color }"></div>
    </div>
  </div>
</template>

<style scoped>
.sparkle-container {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 0;
  overflow: hidden;
}

.sparkle {
  position: absolute;
  animation:
    float 6s ease-in-out infinite,
    twinkle var(--twinkle-speed, 2s) ease-in-out infinite;
  transform-origin: center center;
}

.sparkle-star {
  width: 100%;
  height: 100%;
  filter: drop-shadow(0 0 4px currentColor);
  animation: rotate 8s linear infinite;
}

.sparkle-glow {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 200%;
  height: 200%;
  transform: translate(-50%, -50%);
  border-radius: 50%;
  opacity: 0.3;
  filter: blur(4px);
  animation: pulse-glow 2s ease-in-out infinite;
}

@keyframes float {
  0%, 100% {
    transform: translateY(0) translateX(0);
  }
  25% {
    transform: translateY(-10px) translateX(5px);
  }
  50% {
    transform: translateY(-5px) translateX(-5px);
  }
  75% {
    transform: translateY(-15px) translateX(3px);
  }
}

@keyframes twinkle {
  0%, 100% {
    opacity: var(--sparkle-opacity, 0.8);
    transform: scale(1);
  }
  50% {
    opacity: 0.2;
    transform: scale(0.6);
  }
}

@keyframes rotate {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}

@keyframes pulse-glow {
  0%, 100% {
    opacity: 0.2;
    transform: translate(-50%, -50%) scale(1);
  }
  50% {
    opacity: 0.5;
    transform: translate(-50%, -50%) scale(1.2);
  }
}

/* Different animation delays for variety */
.sparkle:nth-child(3n) {
  animation-delay: 1s, 0.5s;
}

.sparkle:nth-child(3n + 1) {
  animation-delay: 2s, 1s;
}

.sparkle:nth-child(5n) {
  animation-delay: 0.5s, 0.3s;
}

/* Responsive adjustments */
@media (max-width: 768px) {
  .sparkle {
    width: 70% !important;
    height: 70% !important;
  }
}

@media (max-width: 480px) {
  .sparkle {
    width: 50% !important;
    height: 50% !important;
  }
}
</style>
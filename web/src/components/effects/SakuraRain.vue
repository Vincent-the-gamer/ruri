<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue';

interface Petal {
  id: number;
  left: number;
  delay: number;
  duration: number;
  size: number;
  opacity: number;
  rotation: number;
  swayAmount: number;
}

const props = withDefaults(defineProps<{
  count?: number;
  speed?: 'slow' | 'normal' | 'fast';
  colors?: string[];
  interactive?: boolean;
}>(), {
  count: 30,
  speed: 'normal',
  interactive: true,
  colors: () => ['#ffb7c5', '#ff91a4', '#ffd6e0', '#fff0f5', '#ffc0cb']
});

const containerRef = ref<HTMLDivElement>();
const petals = ref<Petal[]>([]);
const mousePosition = ref({ x: 0, y: 0 });
let animationFrameId: number | null = null;

const speedMap = {
  slow: { min: 8, max: 15 },
  normal: { min: 5, max: 10 },
  fast: { min: 3, max: 7 }
};

function generatePetals(): Petal[] {
  const newPetals: Petal[] = [];
  const { min, max } = speedMap[props.speed];

  for (let i = 0; i < props.count; i++) {
    newPetals.push({
      id: i,
      left: Math.random() * 100,
      delay: Math.random() * 10,
      duration: min + Math.random() * (max - min),
      size: 8 + Math.random() * 16,
      opacity: 0.4 + Math.random() * 0.6,
      rotation: Math.random() * 360,
      swayAmount: 30 + Math.random() * 50
    });
  }

  return newPetals;
}

function handleMouseMove(e: MouseEvent) {
  if (!props.interactive) return;
  mousePosition.value = {
    x: (e.clientX / window.innerWidth) * 100,
    y: (e.clientY / window.innerHeight) * 100
  };
}

function getWindEffect(): string {
  const baseWind = Math.sin(Date.now() / 3000) * 20;
  const mouseInfluence = props.interactive
    ? (mousePosition.value.x - 50) * 0.3
    : 0;
  return `${baseWind + mouseInfluence}px`;
}

onMounted(() => {
  petals.value = generatePetals();

  if (props.interactive) {
    window.addEventListener('mousemove', handleMouseMove);
  }

  // Update wind effect periodically
  const updateWind = () => {
    if (containerRef.value) {
      containerRef.value.style.setProperty('--wind-effect', getWindEffect());
      animationFrameId = requestAnimationFrame(updateWind);
    }
  };
  updateWind();
});

onBeforeUnmount(() => {
  if (props.interactive) {
    window.removeEventListener('mousemove', handleMouseMove);
  }
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId);
  }
});
</script>

<template>
  <div
    ref="containerRef"
    class="sakura-container"
    :class="`sakura-${speed}`"
  >
    <div
      v-for="petal in petals"
      :key="petal.id"
      class="sakura-petal"
      :style="{
        left: `${petal.left}%`,
        animationDelay: `${petal.delay}s`,
        animationDuration: `${petal.duration}s`,
        width: `${petal.size}px`,
        height: `${petal.size * 0.8}px`,
        opacity: petal.opacity,
        '--sway-amount': `${petal.swayAmount}px`,
        '--rotation': `${petal.rotation}deg`,
        '--wind-effect': '0px'
      }"
    >
      <svg
        viewBox="0 0 100 80"
        class="petal-svg"
        :fill="colors[petal.id % colors.length]"
      >
        <path d="M50 5 C60 5, 80 20, 85 40 C90 60, 70 75, 50 75 C30 75, 10 60, 15 40 C20 20, 40 5, 50 5 Z" />
        <path
          d="M50 10 L50 70"
          stroke="rgba(255,255,255,0.3)"
          stroke-width="2"
          fill="none"
        />
      </svg>
    </div>
  </div>
</template>

<style scoped>
.sakura-container {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 0;
  overflow: hidden;
}

.sakura-petal {
  position: absolute;
  top: -10%;
  animation:
    fall linear infinite,
    sway ease-in-out infinite,
    rotate linear infinite;
  transform-origin: center center;
  filter: drop-shadow(0 2px 4px rgba(255, 183, 197, 0.3));
}

.petal-svg {
  width: 100%;
  height: 100%;
  animation: pulse 2s ease-in-out infinite;
}

@keyframes fall {
  0% {
    top: -10%;
    transform: translateX(0);
  }
  100% {
    top: 110%;
    transform: translateX(var(--wind-effect, 0px));
  }
}

@keyframes sway {
  0%, 100% {
    transform: translateX(calc(var(--sway-amount, 40px) * -1));
  }
  50% {
    transform: translateX(var(--sway-amount, 40px));
  }
}

@keyframes rotate {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(var(--rotation, 360deg));
  }
}

@keyframes pulse {
  0%, 100% {
    transform: scale(1);
    opacity: 1;
  }
  50% {
    transform: scale(0.95);
    opacity: 0.8;
  }
}

/* Speed variations */
.sakura-slow .sakura-petal {
  animation-duration: 12s, 4s, 8s;
}

.sakura-normal .sakura-petal {
  animation-duration: 8s, 3s, 6s;
}

.sakura-fast .sakura-petal {
  animation-duration: 5s, 2s, 4s;
}

/* Responsive adjustments */
@media (max-width: 768px) {
  .sakura-petal {
    width: 60% !important;
    height: auto !important;
  }
}

@media (max-width: 480px) {
  .sakura-petal {
    width: 50% !important;
    height: auto !important;
  }
}
</style>
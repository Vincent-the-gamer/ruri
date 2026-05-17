<script setup lang="ts">
import { useToast } from "../../composables/useToast";

const { toasts, remove } = useToast();
</script>

<template>
  <Teleport to="body">
    <div class="toast-container">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="toast-item"
          :class="`toast-item--${toast.type}`"
          @click="remove(toast.id)"
        >
          <span class="toast-icon">
            <svg
              v-if="toast.type === 'success'"
              xmlns="http://www.w3.org/2000/svg"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
              <polyline points="22 4 12 14.01 9 11.01" />
            </svg>
            <svg
              v-else-if="toast.type === 'error'"
              xmlns="http://www.w3.org/2000/svg"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="10" />
              <line x1="15" y1="9" x2="9" y2="15" />
              <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
            <svg
              v-else-if="toast.type === 'warning'"
              xmlns="http://www.w3.org/2000/svg"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"
              />
              <line x1="12" y1="9" x2="12" y2="13" />
              <line x1="12" y1="17" x2="12.01" y2="17" />
            </svg>
            <svg
              v-else
              xmlns="http://www.w3.org/2000/svg"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="16" x2="12" y2="12" />
              <line x1="12" y1="8" x2="12.01" y2="8" />
            </svg>
          </span>
          <span class="toast-message">{{ toast.message }}</span>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-container {
  position: fixed;
  top: 1.5rem;
  right: 1.5rem;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  pointer-events: none;
}

.toast-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.875rem 1.25rem;
  border-radius: 12px;
  font-size: 0.9375rem;
  font-weight: 500;
  pointer-events: auto;
  cursor: pointer;
  backdrop-filter: blur(20px) saturate(1.5);
  -webkit-backdrop-filter: blur(20px) saturate(1.5);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.12),
    0 2px 8px rgba(0, 0, 0, 0.08);
  border: 1px solid;
  min-width: 280px;
  max-width: 440px;
}

.toast-item--success {
  background: linear-gradient(
    135deg,
    rgba(34, 197, 94, 0.15),
    rgba(34, 197, 94, 0.08)
  );
  border-color: rgba(34, 197, 94, 0.3);
  color: #16a34a;
}

:global(.dark) .toast-item--success {
  background: linear-gradient(
    135deg,
    rgba(34, 197, 94, 0.2),
    rgba(34, 197, 94, 0.1)
  );
  border-color: rgba(34, 197, 94, 0.4);
  color: #4ade80;
}

.toast-item--error {
  background: linear-gradient(
    135deg,
    rgba(239, 68, 68, 0.15),
    rgba(239, 68, 68, 0.08)
  );
  border-color: rgba(239, 68, 68, 0.3);
  color: #dc2626;
}

:global(.dark) .toast-item--error {
  background: linear-gradient(
    135deg,
    rgba(239, 68, 68, 0.2),
    rgba(239, 68, 68, 0.1)
  );
  border-color: rgba(239, 68, 68, 0.4);
  color: #f87171;
}

.toast-item--warning {
  background: linear-gradient(
    135deg,
    rgba(245, 158, 11, 0.15),
    rgba(245, 158, 11, 0.08)
  );
  border-color: rgba(245, 158, 11, 0.3);
  color: #d97706;
}

:global(.dark) .toast-item--warning {
  background: linear-gradient(
    135deg,
    rgba(245, 158, 11, 0.2),
    rgba(245, 158, 11, 0.1)
  );
  border-color: rgba(245, 158, 11, 0.4);
  color: #fbbf24;
}

.toast-item--info {
  background: linear-gradient(
    135deg,
    rgba(59, 130, 246, 0.15),
    rgba(59, 130, 246, 0.08)
  );
  border-color: rgba(59, 130, 246, 0.3);
  color: #2563eb;
}

:global(.dark) .toast-item--info {
  background: linear-gradient(
    135deg,
    rgba(59, 130, 246, 0.2),
    rgba(59, 130, 246, 0.1)
  );
  border-color: rgba(59, 130, 246, 0.4);
  color: #60a5fa;
}

.toast-icon {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.toast-message {
  flex: 1;
  line-height: 1.4;
}

/* Transition */
.toast-enter-active {
  transition: all 0.35s cubic-bezier(0.21, 1.02, 0.73, 1);
}

.toast-leave-active {
  transition: all 0.25s cubic-bezier(0.06, 0.71, 0.55, 1);
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(100px) scale(0.9);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(40px) scale(0.95);
}

.toast-move {
  transition: transform 0.3s ease;
}

@media (max-width: 640px) {
  .toast-container {
    top: 1rem;
    right: 1rem;
    left: 1rem;
  }

  .toast-item {
    min-width: unset;
    max-width: unset;
  }
}
</style>

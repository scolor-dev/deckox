<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: "text" | "block" | "circle";
    width?: string | null;
    height?: string | null;
  }>(),
  {
    variant: "text",
    width: null,
    height: null,
  },
);
</script>

<template>
  <span
    :class="['ds-skeleton', `ds-skeleton--${variant}`]"
    :style="{ width: width ?? undefined, height: height ?? undefined }"
    aria-hidden="true"
  />
</template>

<style scoped>
.ds-skeleton {
  display: block;
  background: linear-gradient(90deg, var(--surface-muted) 25%, var(--surface-hover) 50%, var(--surface-muted) 75%);
  background-size: 200% 100%;
  animation: ds-skeleton-shimmer calc(var(--duration-spin) * 2) linear infinite;
}
.ds-skeleton--text { width: 100%; height: var(--space-3); border-radius: var(--radius-sm); }
.ds-skeleton--block { width: 100%; height: var(--space-10); border-radius: var(--radius-md); }
.ds-skeleton--circle { width: var(--space-8); height: var(--space-8); border-radius: 50%; }

@keyframes ds-skeleton-shimmer {
  from { background-position: 200% 0; }
  to { background-position: -200% 0; }
}

@media (prefers-reduced-motion: reduce) {
  .ds-skeleton { animation: none; }
}
</style>

<script setup lang="ts">
withDefaults(
  defineProps<{
    label?: string;
  }>(),
  {
    label: "Loading",
  },
);
</script>

<template>
  <span
    class="ds-spinner"
    role="status"
    :aria-label="label"
  />
</template>

<style scoped>
.ds-spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid var(--spinner-track);
  border-radius: 50%;
  border-top-color: var(--brand-primary);
  animation: ds-spinner-spin var(--duration-spin) linear infinite;
}

@keyframes ds-spinner-spin {
  to { transform: rotate(360deg); }
}

@media (prefers-reduced-motion: reduce) {
  .ds-spinner { animation-duration: calc(var(--duration-spin) * 3); }
}
</style>

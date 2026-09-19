<script setup lang="ts">
// Based on the restart page's existing spinner (style.css's
// .restart-indicator), generalized into its own component — every other
// loading state in the app is currently just swapped-in text ("Loading…"),
// not a spinner graphic.
</script>

<template>
  <span
    class="ds-spinner"
    role="status"
    aria-label="Loading"
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

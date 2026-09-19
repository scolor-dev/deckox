<script setup lang="ts">
</script>

<template>
  <div class="ds-toast-region">
    <slot />
  </div>
</template>

<style scoped>
.ds-toast-region {
  position: fixed;
  z-index: var(--z-toast);
  top: var(--space-4);
  right: var(--space-4);
  display: grid;
  width: min(360px, calc(100vw - var(--space-8)));
  gap: var(--space-2);
}
</style>

<script setup lang="ts">
// Fixed top-right stack that AppToast items render into. Holds no toast
// state itself — the queue (and auto-dismiss timing) belongs to whatever
// app-level module pushes notifications, as with notifications.ts today.
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

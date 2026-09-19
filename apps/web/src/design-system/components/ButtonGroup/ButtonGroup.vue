<script setup lang="ts">
defineProps<{
  label?: string | null;
}>();
</script>

<template>
  <div
    class="ds-button-group"
    role="group"
    :aria-label="label ?? undefined"
  >
    <slot />
  </div>
</template>

<style scoped>
.ds-button-group { display: inline-flex; }
.ds-button-group :deep(.ds-button) { position: relative; border-radius: 0; }
.ds-button-group :deep(.ds-button:first-child) { border-radius: var(--radius-sm) 0 0 var(--radius-sm); }
.ds-button-group :deep(.ds-button:last-child) { border-radius: 0 var(--radius-sm) var(--radius-sm) 0; }
.ds-button-group :deep(.ds-button:only-child) { border-radius: var(--radius-sm); }
.ds-button-group :deep(.ds-button + .ds-button) { margin-left: -1px; }
.ds-button-group :deep(.ds-button:hover:not(:disabled)),
.ds-button-group :deep(.ds-button:focus-visible) { z-index: 1; }
</style>

<script setup lang="ts">
// Joins AppButtons into one segmented control (view switchers, ranges).
// It restyles the buttons' edges from outside, so it is coupled to
// AppButton's root class on purpose — put AppButtons in it, nothing else.
defineProps<{
  /** Accessible name for the group, e.g. "Update interval". */
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

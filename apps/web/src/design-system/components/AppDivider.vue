<script setup lang="ts">
withDefaults(
  defineProps<{
    spacing?: "sm" | "md" | "lg";
  }>(),
  {
    spacing: "md",
  },
);
</script>

<template>
  <hr :class="['ds-divider', `ds-divider--${spacing}`]">
</template>

<style scoped>
.ds-divider { border: 0; border-top: 1px solid var(--border-default); }
.ds-divider--sm { margin: var(--space-3) 0; }
.ds-divider--md { margin: var(--space-5) 0; }
.ds-divider--lg { margin: var(--space-8) 0; }
</style>

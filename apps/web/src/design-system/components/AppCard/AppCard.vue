<script setup lang="ts">
withDefaults(
  defineProps<{
    padding?: "sm" | "md";
  }>(),
  {
    padding: "md",
  },
);
</script>

<template>
  <div :class="['ds-card', `ds-card--${padding}`]">
    <slot />
  </div>
</template>

<style scoped>
.ds-card {
  min-width: 0;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--surface-elevated);
}
.ds-card--md { padding: var(--space-4) var(--space-5); }
.ds-card--sm { padding: var(--space-3) var(--space-4); }
</style>

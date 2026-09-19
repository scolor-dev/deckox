<script setup lang="ts">
// The bare bordered-panel shell shared by metric cards, diagnostics cards,
// and summary panels — TablePanel is this same shell plus a toolbar/table
// scroll area; use that instead when the content is a table.
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

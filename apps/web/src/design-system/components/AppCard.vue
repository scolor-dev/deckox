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
  border-radius: 6px;
  background: var(--surface-elevated);
}
.ds-card--md { padding: 17px 18px; }
.ds-card--sm { padding: 12px 14px; }
</style>

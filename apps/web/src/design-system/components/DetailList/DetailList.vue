<script setup lang="ts">
</script>

<template>
  <dl class="ds-detail-list">
    <slot />
  </dl>
</template>

<style scoped>
.ds-detail-list {
  display: grid;
  grid-template-columns: 110px minmax(0, 1fr) 110px minmax(0, 1fr);
  margin: 0;
  overflow: hidden;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--surface-elevated);
}
.ds-detail-list :deep(dt),
.ds-detail-list :deep(dd) {
  padding: var(--space-3) var(--space-5);
  border-top: 1px solid var(--border-faint);
}
.ds-detail-list :deep(dt:nth-child(-n+2)),
.ds-detail-list :deep(dd:nth-child(-n+2)) { border-top: 0; }
.ds-detail-list :deep(dt) { margin: 0; color: var(--text-muted); font-size: var(--font-sm); }
.ds-detail-list :deep(dd) {
  min-width: 0;
  margin: 0;
  overflow: hidden;
  color: var(--text-strong);
  font-size: var(--font-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 700px) {
  .ds-detail-list { grid-template-columns: 110px minmax(0, 1fr); }
}
</style>

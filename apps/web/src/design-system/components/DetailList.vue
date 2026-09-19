<script setup lang="ts">
// Two-column label/value grid (Overview's server summary, Diagnostics,
// Storage's usage summary) — pairs with DetailRow for each dt/dd. Owns its
// own card chrome, matching the real app's `.detail-panel`.
//
// DetailRow renders a bare `<dt>`/`<dd>` pair (no wrapping element, so the
// markup stays a valid `<dl>`), so the border-removal trick here targets
// the first two children directly rather than "the last row" — the count
// of pairs isn't known up front and doesn't need to be for this to work.
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
  padding: 13px 18px;
  border-top: 1px solid var(--border-faint);
}
.ds-detail-list :deep(dt:nth-child(-n+2)),
.ds-detail-list :deep(dd:nth-child(-n+2)) { border-top: 0; }
.ds-detail-list :deep(dt) { margin: 0; color: var(--text-muted); font-size: 12px; }
.ds-detail-list :deep(dd) {
  min-width: 0;
  margin: 0;
  overflow: hidden;
  color: var(--text-strong);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 700px) {
  .ds-detail-list { grid-template-columns: 110px minmax(0, 1fr); }
}
</style>

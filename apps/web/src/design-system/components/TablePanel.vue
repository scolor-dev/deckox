<script setup lang="ts">
// The card + scroll shell around a table — not a data-grid abstraction.
// The `<table>` itself stays consumer-authored (via the default slot),
// since column shape varies too much across screens (action buttons here,
// badges there, nested definition lists elsewhere) for a generic
// `:columns`/`:rows` API to pull its weight over plain template slots.
// This shell owns exactly the parts that repeat identically everywhere:
// the card chrome, the optional toolbar, horizontal scroll, and (on
// narrow screens) a sticky first column so the row's identity stays
// visible while scrolling.
defineProps<{
  loading?: boolean;
  empty?: boolean;
  emptyMessage?: string;
}>();
</script>

<template>
  <section class="ds-table-panel">
    <div
      v-if="$slots.toolbar"
      class="ds-table-toolbar"
    >
      <slot name="toolbar" />
    </div>
    <div
      v-if="$slots.note"
      class="ds-table-note"
    >
      <slot name="note" />
    </div>
    <div class="ds-table-scroll">
      <slot v-if="!loading && !empty" />
      <p
        v-else
        class="ds-table-empty"
      >
        {{ loading ? undefined : emptyMessage }}
        <slot
          v-if="loading"
          name="loading"
        />
      </p>
    </div>
  </section>
</template>

<style scoped>
.ds-table-panel {
  margin-top: 18px;
  overflow: hidden;
  border: 1px solid var(--border-default);
  border-radius: 6px;
  background: var(--surface-elevated);
}
.ds-table-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px 14px;
  padding: 12px 14px;
  border-bottom: 1px solid var(--border-subtle);
  background: var(--surface-subtle);
}
.ds-table-note { padding: 10px 14px 0; }
.ds-table-scroll { max-width: 100%; overflow-x: auto; }
.ds-table-empty { padding: 42px 18px; color: var(--text-faint); text-align: center; }

.ds-table-scroll :deep(table) { width: 100%; border-collapse: collapse; }
.ds-table-scroll :deep(th),
.ds-table-scroll :deep(td) {
  padding: 12px 14px;
  border-bottom: 1px solid var(--border-faint);
  text-align: left;
  vertical-align: middle;
}
.ds-table-scroll :deep(tr:last-child td) { border-bottom: 0; }
.ds-table-scroll :deep(th) {
  color: var(--text-label);
  background: var(--surface-subtle);
  font-size: 11px;
  font-weight: 600;
  white-space: nowrap;
}
.ds-table-scroll :deep(td) { color: var(--text-secondary); font-size: 12px; }

@media (max-width: 700px) {
  .ds-table-panel { border-radius: 4px; }
  .ds-table-scroll :deep(th),
  .ds-table-scroll :deep(td) { padding: 11px 12px; }
  .ds-table-scroll :deep(th:first-child),
  .ds-table-scroll :deep(td:first-child) {
    position: sticky;
    left: 0;
    z-index: 1;
    background: var(--surface-elevated);
  }
  .ds-table-scroll :deep(th:first-child) { background: var(--surface-subtle); }
}
</style>

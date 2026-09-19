<script setup lang="ts">
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
    <div
      v-if="$slots.footer"
      class="ds-table-footer"
    >
      <slot name="footer" />
    </div>
  </section>
</template>

<style scoped>
.ds-table-panel {
  overflow: hidden;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--surface-elevated);
}
.ds-table-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3) var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border-subtle);
  background: var(--surface-subtle);
}
.ds-table-note { padding: var(--space-3) var(--space-4) 0; }
.ds-table-footer { padding: var(--space-3) var(--space-4); border-top: 1px solid var(--border-subtle); }
.ds-table-scroll { max-width: 100%; overflow-x: auto; }
.ds-table-empty { padding: var(--space-10) var(--space-5); color: var(--text-faint); text-align: center; }

.ds-table-scroll :deep(table) { width: 100%; border-collapse: collapse; }
.ds-table-scroll :deep(th),
.ds-table-scroll :deep(td) {
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border-faint);
  text-align: left;
  vertical-align: middle;
}
.ds-table-scroll :deep(tr:last-child td) { border-bottom: 0; }
.ds-table-scroll :deep(th) {
  color: var(--text-label);
  background: var(--surface-subtle);
  font-size: var(--font-xs);
  font-weight: 600;
  white-space: nowrap;
}
.ds-table-scroll :deep(td) { color: var(--text-secondary); font-size: var(--font-sm); }

@media (max-width: 700px) {
  .ds-table-panel { border-radius: var(--radius-sm); }
  .ds-table-scroll :deep(th),
  .ds-table-scroll :deep(td) { padding: var(--space-3) var(--space-3); }
  .ds-table-scroll :deep(th:first-child),
  .ds-table-scroll :deep(td:first-child) {
    position: sticky;
    left: 0;
    z-index: var(--z-sticky-column);
    background: var(--surface-elevated);
  }
  .ds-table-scroll :deep(th:first-child) { background: var(--surface-subtle); }
}
</style>

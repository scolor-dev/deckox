<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

defineProps<{
  loading?: boolean;
  empty?: boolean;
  emptyMessage?: string;
}>();

const scrollRef = ref<HTMLElement | null>(null);
let observer: MutationObserver | undefined;

function labelCells() {
  for (const table of scrollRef.value?.querySelectorAll("table") ?? []) {
    table.setAttribute("role", "table");
    const headers = [...table.querySelectorAll("thead th")].map((th) => th.textContent.trim());
    for (const group of table.querySelectorAll("thead, tbody")) group.setAttribute("role", "rowgroup");
    for (const row of table.querySelectorAll("tr")) {
      row.setAttribute("role", "row");
      [...row.children].forEach((cell, index) => {
        cell.setAttribute("role", cell.tagName === "TH" ? "columnheader" : "cell");
        if (cell.tagName === "TD" && headers[index]) cell.setAttribute("data-label", headers[index]);
      });
    }
  }
}

onMounted(() => {
  labelCells();
  if (scrollRef.value) {
    observer = new MutationObserver(labelCells);
    observer.observe(scrollRef.value, { childList: true, subtree: true });
  }
});

onBeforeUnmount(() => {
  observer?.disconnect();
});
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
    <div
      ref="scrollRef"
      class="ds-table-scroll"
    >
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
  .ds-table-scroll { overflow-x: visible; }
  .ds-table-scroll :deep(table),
  .ds-table-scroll :deep(tbody) { display: block; min-width: 0; }
  .ds-table-scroll :deep(thead) {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .ds-table-scroll :deep(tr) {
    display: block;
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--border-faint);
  }
  .ds-table-scroll :deep(tr:last-child) { border-bottom: 0; }
  .ds-table-scroll :deep(td) {
    display: grid;
    grid-template-columns: minmax(var(--space-16), 34%) minmax(0, 1fr);
    align-items: start;
    gap: var(--space-3);
    padding: var(--space-1) 0;
    border: 0;
    overflow-wrap: anywhere;
  }
  .ds-table-scroll :deep(td > *) { grid-column: 2; min-width: 0; }
  .ds-table-scroll :deep(td[data-label])::before {
    content: attr(data-label);
    grid-column: 1;
    color: var(--text-label);
    font-size: var(--font-xs);
    font-weight: 600;
  }
}
</style>

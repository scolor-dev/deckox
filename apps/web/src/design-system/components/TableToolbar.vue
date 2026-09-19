<script setup lang="ts">
// The inner flex arrangement for a TablePanel's #toolbar slot: an optional
// search box (grows to fill space), optional filter controls (e.g. a
// TagToggleGroup), and a result count on the trailing edge. Renders no box
// chrome of its own — TablePanel's toolbar slot already supplies the
// background/border/padding, so nesting this inside it doesn't double up.
defineProps<{
  count?: string | null;
}>();
</script>

<template>
  <div class="ds-toolbar-row">
    <div
      v-if="$slots.search"
      class="ds-toolbar-search"
    >
      <slot name="search" />
    </div>
    <div
      v-if="$slots.filters"
      class="ds-toolbar-filters"
    >
      <slot name="filters" />
    </div>
    <span
      v-if="count"
      class="ds-toolbar-count"
    >{{ count }}</span>
  </div>
</template>

<style scoped>
.ds-toolbar-row {
  display: flex;
  flex: 1;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px 14px;
}
.ds-toolbar-search { flex: 1; }
.ds-toolbar-search :deep(input) {
  width: min(380px, 100%);
  padding: 7px 10px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  outline: none;
  color: var(--text-primary);
  background: var(--surface-elevated);
  font-size: 12px;
}
.ds-toolbar-search :deep(input:focus) { border-color: var(--brand-focus); box-shadow: 0 0 0 2px var(--brand-focus-ring); }
.ds-toolbar-filters { display: flex; flex-wrap: wrap; gap: 8px; }
.ds-toolbar-count { color: var(--text-faint); font-size: 11px; }
</style>

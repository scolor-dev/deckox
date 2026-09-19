<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  page: number;
  pageCount: number;
  label: string;
  prevLabel: string;
  nextLabel: string;
}>();

const emit = defineEmits<{
  "update:page": [page: number];
}>();

const items = computed<(number | string)[]>(() => {
  const total = props.pageCount;
  if (total <= 7) return Array.from({ length: total }, (_, i) => i + 1);
  const shown = [...new Set([1, total, props.page - 1, props.page, props.page + 1])]
    .filter((p) => p >= 1 && p <= total)
    .sort((a, b) => a - b);
  const out: (number | string)[] = [];
  shown.forEach((p, i) => {
    const previous = shown[i - 1];
    if (i > 0 && p - previous > 1) out.push(p - previous === 2 ? previous + 1 : `gap-${String(i)}`);
    out.push(p);
  });
  return out;
});

function go(target: number) {
  const next = Math.min(props.pageCount, Math.max(1, target));
  if (next !== props.page) emit("update:page", next);
}
</script>

<template>
  <nav
    class="ds-pagination"
    :aria-label="label"
  >
    <button
      type="button"
      class="ds-pagination-step"
      :disabled="page <= 1"
      :aria-label="prevLabel"
      @click="go(page - 1)"
    >
      ‹
    </button>
    <template
      v-for="item in items"
      :key="item"
    >
      <span
        v-if="typeof item === 'string'"
        class="ds-pagination-gap"
        aria-hidden="true"
      >…</span>
      <button
        v-else
        type="button"
        :class="['ds-pagination-page', { 'ds-pagination-page--current': item === page }]"
        :aria-current="item === page ? 'page' : undefined"
        @click="go(item)"
      >
        {{ item }}
      </button>
    </template>
    <button
      type="button"
      class="ds-pagination-step"
      :disabled="page >= pageCount"
      :aria-label="nextLabel"
      @click="go(page + 1)"
    >
      ›
    </button>
  </nav>
</template>

<style scoped>
.ds-pagination { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-1); margin: 0; }
.ds-pagination-page,
.ds-pagination-step {
  min-width: var(--space-8);
  padding: var(--space-1) var(--space-2);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  color: var(--text-strong);
  background: var(--surface-elevated);
  cursor: pointer;
  font: inherit;
  font-size: var(--font-sm);
}
.ds-pagination-page:hover:not(:disabled):not(.ds-pagination-page--current),
.ds-pagination-step:hover:not(:disabled) { border-color: var(--border-hover); background: var(--surface-hover); }
.ds-pagination-page--current {
  border-color: var(--brand-primary-border);
  color: var(--text-inverse);
  background: var(--brand-primary);
  font-weight: 600;
  cursor: default;
}
.ds-pagination-step:disabled { opacity: .48; cursor: not-allowed; }
.ds-pagination-gap { min-width: var(--space-6); color: var(--text-faint); text-align: center; }
</style>

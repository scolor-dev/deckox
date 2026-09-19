<script setup lang="ts">
import { computed } from "vue";
import AppIcon from "../AppIcon/AppIcon.vue";
import { sortRows, type DataColumn, type DataRow, type SortDirection } from "./dataTable";

const props = withDefaults(
  defineProps<{
    caption: string;
    columns: DataColumn[];
    rows: DataRow[];
    rowKey: string;
    sortKey?: string | null;
    sortDirection?: SortDirection;
    manual?: boolean;
    selectable?: boolean;
    selected?: string[];
    selectAllLabel?: string;
    selectRowLabel?: string;
    rowLabelKey?: string | null;
  }>(),
  {
    sortKey: null,
    sortDirection: "asc",
    manual: false,
    selectable: false,
    selected: () => [],
    selectAllLabel: "Select all rows",
    selectRowLabel: "Select row",
    rowLabelKey: null,
  },
);

const emit = defineEmits<{
  sort: [key: string, direction: SortDirection];
  "update:selected": [keys: string[]];
}>();

const visibleRows = computed(() => (props.manual ? props.rows : sortRows(props.rows, props.sortKey, props.sortDirection)));
const keyOf = (row: DataRow) => String(row[props.rowKey]);
const allKeys = computed(() => props.rows.map(keyOf));
const allSelected = computed(() => allKeys.value.length > 0 && allKeys.value.every((key) => props.selected.includes(key)));
const someSelected = computed(() => !allSelected.value && allKeys.value.some((key) => props.selected.includes(key)));
const labelKey = computed(() => props.rowLabelKey ?? props.columns[0].key);

function ariaSort(column: DataColumn) {
  if (!column.sortable) return undefined;
  if (props.sortKey !== column.key) return "none";
  return props.sortDirection === "asc" ? "ascending" : "descending";
}

function requestSort(column: DataColumn) {
  const next: SortDirection = props.sortKey === column.key && props.sortDirection === "asc" ? "desc" : "asc";
  emit("sort", column.key, next);
}

function toggleAll() {
  emit("update:selected", allSelected.value ? [] : allKeys.value);
}

function toggleRow(row: DataRow) {
  const key = keyOf(row);
  emit("update:selected", props.selected.includes(key) ? props.selected.filter((item) => item !== key) : [...props.selected, key]);
}
</script>

<template>
  <table class="ds-data-table">
    <caption class="ds-data-table-caption">
      {{ caption }}
    </caption>
    <thead>
      <tr>
        <th
          v-if="selectable"
          class="ds-data-table-select"
        >
          <input
            type="checkbox"
            :checked="allSelected"
            :indeterminate="someSelected"
            :aria-label="selectAllLabel"
            @change="toggleAll"
          >
        </th>
        <th
          v-for="column in columns"
          :key="column.key"
          :class="{ 'ds-data-table-end': column.align === 'end' }"
          :aria-sort="ariaSort(column)"
        >
          <button
            v-if="column.sortable"
            type="button"
            class="ds-data-table-sort"
            @click="requestSort(column)"
          >
            {{ column.label }}
            <AppIcon
              v-if="sortKey === column.key"
              :name="sortDirection === 'asc' ? 'chevron-up' : 'chevron-down'"
              size="sm"
            />
          </button>
          <template v-else>
            {{ column.label }}
          </template>
        </th>
      </tr>
    </thead>
    <tbody>
      <tr
        v-for="row in visibleRows"
        :key="keyOf(row)"
        :class="{ 'ds-data-table-row--selected': selected.includes(keyOf(row)) }"
      >
        <td
          v-if="selectable"
          class="ds-data-table-select"
        >
          <input
            type="checkbox"
            :checked="selected.includes(keyOf(row))"
            :aria-label="`${selectRowLabel} ${String(row[labelKey])}`"
            @change="toggleRow(row)"
          >
        </td>
        <td
          v-for="column in columns"
          :key="column.key"
          :class="{ 'ds-data-table-end': column.align === 'end' }"
        >
          <slot
            :name="`cell-${column.key}`"
            :row="row"
            :value="row[column.key]"
          >
            {{ row[column.key] }}
          </slot>
        </td>
      </tr>
    </tbody>
  </table>
</template>

<style scoped>
.ds-data-table-caption {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
.ds-data-table-sort {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  padding: 0;
  border: 0;
  color: inherit;
  background: transparent;
  cursor: pointer;
  font: inherit;
}
.ds-data-table-sort:hover { color: var(--text-primary); }
.ds-data-table-sort:focus-visible { outline: 2px solid var(--brand-primary); outline-offset: 2px; }
.ds-data-table-select { width: var(--space-10); }
.ds-data-table-select input { width: 16px; height: 16px; margin: 0; accent-color: var(--brand-primary); }
.ds-data-table-select input:focus-visible { outline: 2px solid var(--brand-primary); outline-offset: 2px; }
.ds-data-table th.ds-data-table-end,
.ds-data-table td.ds-data-table-end { text-align: right; }
.ds-data-table-row--selected td { background: var(--brand-focus-ring); }
</style>

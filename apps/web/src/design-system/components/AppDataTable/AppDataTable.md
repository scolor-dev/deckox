---
name: AppDataTable
components: [AppDataTable]
category: data
replaces: [hand-written sortable tables, selection checkboxes in tables]
related: [TablePanel, AppEmptyState, StateBadge, AppCheckbox]
---

# AppDataTable

Table with a screen-reader caption, sortable column headers and optional row selection. It renders the `table` element; put it inside TablePanel for the chrome.

## When to use

- A list of uniform rows where users sort by a column or pick rows for a bulk action.

## When not to use

- Rows with wildly different shapes, or a layout that only looks like a table: write markup by hand inside TablePanel.
- Very large data sets: it sorts all rows in memory and renders them all; there is no paging or virtualization.

## API

### AppDataTable

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `caption` | `string` | required | Accessible name of the table. Rendered but visually hidden. |
| `columns` | `DataColumn[]` | required | `{ key, label, sortable?, align? }` per column; `align` is `"start"` or `"end"`. |
| `rows` | `DataRow[]` | required | Rows as `Record<string, string \| number \| boolean \| null>`. |
| `rowKey` | `string` | required | Column key whose value uniquely identifies a row. Also the value stored in `selected`. |
| `sortKey` | `string \| null` | `null` | Key of the column the rows are currently sorted by. |
| `sortDirection` | `SortDirection` | `"asc"` | `"asc"` or `"desc"`. |
| `manual` | `boolean` | `false` | Do not sort locally; show `rows` in the given order (server-side sorting). |
| `selectable` | `boolean` | `false` | Add a leading checkbox column with a select-all header checkbox. |
| `selected` | `string[]` | `() => []` | `rowKey` values of the selected rows. |
| `selectAllLabel` | `string` | `"Select all rows"` | Accessible name of the header checkbox. |
| `selectRowLabel` | `string` | `"Select row"` | Prefix of each row checkbox name; the row's label value is appended. |
| `rowLabelKey` | `string \| null` | `null` | Column whose value names a row for assistive tech. Defaults to the first column. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `sort` | `[key: string, direction: SortDirection]` | A sortable header was activated. The direction toggles when the same column is activated again. |
| `update:selected` | `[keys: string[]]` | The selection changed. |

**Slots**

| Name | Description |
|---|---|
| `cell-{key}` | Custom cell content for the column with that `key`, with slot props `row` and `value`. Falls back to the plain value. |

## Examples

```vue
<TablePanel>
  <AppDataTable
    v-model:selected="selected"
    caption="Services"
    row-key="name"
    :columns="[
      { key: 'name', label: 'Name', sortable: true },
      { key: 'memory', label: 'Memory (MiB)', sortable: true, align: 'end' },
      { key: 'state', label: 'State' },
    ]"
    :rows="services"
    :sort-key="sortKey"
    :sort-direction="sortDirection"
    selectable
    @sort="(key, direction) => { sortKey = key; sortDirection = direction; }"
  >
    <template #cell-state="{ value }">
      <StateBadge :state="value === 'running' ? 'active' : 'inactive'">{{ value }}</StateBadge>
    </template>
  </AppDataTable>
</TablePanel>
```

## Accessibility

- The `caption` gives the table its name; it is visually hidden but announced.
- Sortable headers contain a button, and the `th` carries `aria-sort` (`none`, `ascending` or `descending`).
- Row checkboxes are named `selectRowLabel` plus the row label; the header checkbox shows the indeterminate state for a partial selection.

## Tokens

`--brand-focus-ring`, `--brand-primary`, `--space-1`, `--space-10`, `--text-primary`

## Gotchas

- Sort and selection state live in the parent: the component only emits `sort` and `update:selected`. Without handlers, header clicks change nothing.
- Local sorting compares numbers numerically and everything else with natural, case-insensitive string order; `null` always sorts last.
- Padding, borders and header colors come from TablePanel. Outside a TablePanel the table is unstyled apart from sorting and selection.
- Cell content is text unless you fill a `cell-{key}` slot; HTML in a value is escaped.

## Migration

Before:

```vue
<table>
  <thead>
    <tr><th>Name</th><th>Memory</th></tr>
  </thead>
  <tbody>
    <tr v-for="service in services" :key="service.name">
      <td>{{ service.name }}</td>
      <td>{{ service.memory }}</td>
    </tr>
  </tbody>
</table>
```

After:

```vue
<AppDataTable
  caption="Services"
  row-key="name"
  :columns="[{ key: 'name', label: 'Name', sortable: true }, { key: 'memory', label: 'Memory', sortable: true, align: 'end' }]"
  :rows="services"
/>
```

- Keep the surrounding TablePanel; only the inner `table` is replaced.
- Sorting a column needs `sort-key`, `sort-direction` and a `@sort` handler that stores the values.

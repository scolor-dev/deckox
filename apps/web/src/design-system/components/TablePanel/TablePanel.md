---
name: TablePanel
components: [TablePanel, TableToolbar]
category: data
replaces: [.table-panel, .table-scroll, .table-toolbar]
related: [AppEmptyState, TagToggle, StateBadge]
---

# TablePanel

Card and scroll shell around a table. TableToolbar arranges the search box, filters and count inside it.

## When to use

- Any table screen. You write the `table` markup; the panel supplies chrome, scrolling and table styles.

## When not to use

- A data-grid with sorting or paging: this shell has none.

## API

### TablePanel

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `loading` | `boolean` | — | Replace the table with the `loading` slot content. |
| `empty` | `boolean` | — | Replace the table with `emptyMessage`. |
| `emptyMessage` | `string` | — | Text shown when `empty` is true. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `toolbar` | Toolbar row above the table. The bar renders only when provided. |
| `note` | Extra content between the toolbar and the table. |
| `default` | The `table` element. |
| `loading` | Content shown while `loading` is true. |
| `footer` | Row below the table, for example a "load more" button. The bar renders only when provided. |

### TableToolbar

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `count` | `string \| null` | — | Result count text on the trailing edge, for example "12 items". |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `search` | Search input. It grows to fill the row. |
| `filters` | Filter controls, for example a TagToggleGroup. |

## Examples

```vue
<TablePanel :loading="loading" :empty="rows.length === 0" empty-message="No matching services.">
  <template #toolbar>
    <TableToolbar :count="`${rows.length} items`">
      <template #search><input v-model="query" type="search" placeholder="Search"></template>
    </TableToolbar>
  </template>
  <table>
    <thead><tr><th>Service</th><th>State</th></tr></thead>
    <tbody><tr v-for="row in rows" :key="row.id"><td>{{ row.id }}</td><td>{{ row.state }}</td></tr></tbody>
  </table>
</TablePanel>
```

## Accessibility

- Semantics come from the `table` you write; give it a caption or an accessible name if the screen has several tables.
- The panel sets explicit `role` attributes (`table`, `rowgroup`, `row`, `columnheader`, `cell`) because the narrow-screen card layout changes the display type of the table parts.

## Tokens

`--border-default`, `--border-faint`, `--border-strong`, `--border-subtle`, `--brand-focus`, `--font-sm`, `--font-xs`, `--radius-md`, `--radius-sm`, `--shadow-focus`, `--space-1`, `--space-10`, `--space-16`, `--space-2`, `--space-3`, `--space-4`, `--space-5`, `--surface-elevated`, `--surface-subtle`, `--text-faint`, `--text-label`, `--text-primary`, `--text-secondary`

## Gotchas

- `th` and `td` styles are applied through deep selectors, so they affect every table inside the panel.
- On narrow screens (700px and below) each row becomes a card: the header row is hidden and every cell shows its column header as a label, read from the `thead` text. Cells are labelled by position, so give every `td` a matching `th` and avoid `colspan` cells inside `tbody`. The panel adds `role` attributes to the table parts so assistive tech keeps the table semantics after the layout change.
- Put TableToolbar only inside the `toolbar` slot; it draws no box of its own.

## Migration

Before:

```vue
<section class="table-panel">
  <div class="table-toolbar">...</div>
  <div class="table-scroll"><table>...</table></div>
</section>
```

After:

```vue
<TablePanel>
  <template #toolbar><TableToolbar>...</TableToolbar></template>
  <table>...</table>
</TablePanel>
```

- The old loading and empty rows (`<tr><td class="empty">`) become `loading`/`empty` props on the panel.

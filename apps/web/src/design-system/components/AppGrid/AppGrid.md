---
name: AppGrid
components: [AppGrid]
category: layout
replaces: [.metric-grid, one-off repeat(auto-fill) grids]
related: [AppStack, MetricCard, AppCard]
---

# AppGrid

Responsive grid that fits as many equal columns as the width allows.

## When to use

- A set of cards or metrics that should flow into 1, 2, 3 or more columns depending on the width.

## When not to use

- A fixed layout with named regions or unequal columns: write the grid where it is used.
- A single row or column of items: AppStack.

## API

### AppGrid

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `as` | `string` | `"div"` | Element to render, for example `ul` or `section`. |
| `min` | `"sm" \| "md" \| "lg"` | `"md"` | Minimum column width: 160px, 240px or 320px. Columns never exceed the container on narrow screens. |
| `gap` | `"2" \| "3" \| "4" \| "5" \| "6" \| "8"` | `"4"` | Space between cells, mapped to `--space-N`. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | The grid cells. |

## Examples

```vue
<AppGrid min="md" gap="3">
  <MetricCard label="CPU" value="18%" />
  <MetricCard label="Memory" value="41%" />
  <MetricCard label="Disk" value="62%" />
</AppGrid>
```

## Accessibility

- It adds no roles or landmarks.

## Tokens

`--space-2`, `--space-3`, `--space-4`, `--space-5`, `--space-6`, `--space-8`

## Gotchas

- Columns are fluid, so the column count changes with the container width, not the viewport width; a grid inside a narrow sidebar collapses to one column.
- The last row may have empty cells; the grid does not stretch them.

## Migration

Before:

```vue
<div class="metric-grid">
  <MetricCard label="CPU" value="18%" />
  <MetricCard label="Memory" value="41%" />
  <MetricCard label="Disk" value="62%" />
</div>
```

After:

```vue
<AppGrid min="md" gap="3">
  <MetricCard label="CPU" value="18%" />
  <MetricCard label="Memory" value="41%" />
  <MetricCard label="Disk" value="62%" />
</AppGrid>
```

- The old `.metric-grid` was a fixed three columns; the grid reflows to fewer columns when narrow.

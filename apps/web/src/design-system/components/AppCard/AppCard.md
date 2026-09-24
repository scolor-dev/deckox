---
name: AppCard
components: [AppCard]
category: layout
replaces: [.diagnostics-card, .storage-summary]
related: [TablePanel, MetricCard, DetailList]
---

# AppCard

Bare bordered panel for arbitrary content.

## When to use

- Grouping related content that is neither a table, a metric nor a detail list.
- A titled panel, with its actions in the title row (`title` and the `actions` slot).

## When not to use

- A table: TablePanel.
- A single metric: MetricCard.
- Label/value pairs: DetailList.

## API

### AppCard

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `padding` | `"sm" \| "md"` | `"md"` | `md` (16px x 20px) or `sm` (12px x 16px). |
| `title` | `string` | `""` | Heading shown above the content, on one line with an ellipsis. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | Card content. |
| `actions` | Buttons shown at the right of the title row. |

## Examples

```vue
<AppCard padding="sm"><p>Restart runs at 03:00 every night.</p></AppCard>
<AppCard title="Backups"><template #actions><AppButton>Refresh</AppButton></template>Three copies are kept.</AppCard>
```

## Accessibility

- A plain `div`; it is not a landmark or region.

## Tokens

`--border-default`, `--radius-md`, `--space-2`, `--space-3`, `--space-4`, `--space-5`, `--surface-elevated`

## Gotchas

- It sets no margins; space cards with a flex or grid parent using `gap`.
- The title row appears only when `title` or `actions` is given; without them the card is unchanged.

## Migration

Before:

```vue
<section class="diagnostics-card">...</section>
```

After:

```vue
<AppCard>...</AppCard>
```

- Layout-specific rules on the old classes (for example `.diagnostics-card.host-card` spanning columns) stay on the parent grid.

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

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | Card content. |

## Examples

```vue
<AppCard padding="sm"><p>Restart runs at 03:00 every night.</p></AppCard>
```

## Accessibility

- A plain `div`; it is not a landmark or region.

## Tokens

`--border-default`, `--radius-md`, `--space-3`, `--space-4`, `--space-5`, `--surface-elevated`

## Gotchas

- It sets no margins; space cards with a flex or grid parent using `gap`.

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

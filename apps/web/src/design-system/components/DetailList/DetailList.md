---
name: DetailList
components: [DetailList, DetailRow]
category: data
replaces: [.details]
related: [AppCard, MetricCard]
---

# DetailList

Two-column label/value grid. DetailRow is one pair.

## When to use

- Host information, diagnostics values, storage facts.

## When not to use

- Tabular data with many columns: TablePanel.

## API

### DetailList

**Props**

None.

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | DetailRow elements. |

### DetailRow

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `term` | `string` | required | The label. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | The value. |

## Examples

```vue
<DetailList>
  <DetailRow term="Hostname">deckox-pi</DetailRow>
  <DetailRow term="OS">Debian 12</DetailRow>
</DetailList>
```

## Accessibility

- A real `dl` with bare `dt` and `dd` children, so screen readers announce the pairs.

## Tokens

`--border-default`, `--border-faint`, `--font-sm`, `--radius-md`, `--space-3`, `--space-5`, `--surface-elevated`, `--text-muted`, `--text-strong`

## Gotchas

- Two pairs sit side by side; on narrow screens one pair per row.
- Values do not wrap: they truncate with an ellipsis. Wrap the value in an element with a `title` when the full text matters.
- DetailRow renders two root nodes (`dt`, `dd`) with no wrapper element, so it must be a direct child of DetailList.

## Migration

Before:

```vue
<dl class="details">
  <div>
    <dt>Hostname</dt><dd :title="system?.hostname">{{ system?.hostname }}</dd>
  </div>
</dl>
```

After:

```vue
<DetailList>
  <DetailRow term="Hostname"><span :title="system?.hostname">{{ system?.hostname }}</span></DetailRow>
</DetailList>
```

- The old markup wrapped each `dt`/`dd` in a `div`; DetailRow does not, and the `title` moves onto an inner element.

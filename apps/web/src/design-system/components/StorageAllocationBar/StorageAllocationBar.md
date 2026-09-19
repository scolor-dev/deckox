---
name: StorageAllocationBar
components: [StorageAllocationBar]
category: data
replaces: [.storage-allocation-bar, .storage-allocation-legend]
related: [ProgressBar]
---

# StorageAllocationBar

Stacked multi-segment bar with a legend, for several shares of one whole.

## When to use

- Disk usage split across mounts.

## When not to use

- One value against a track: ProgressBar.

## API

### StorageAllocationBar

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `segments` | `{ key: string; label: string; percent: number; color: string; value: string }[]` | required | Shares to draw, in order. `percent` sets the width; `color` is any CSS color; `value` is the legend text. |
| `label` | `string` | required | Screen-reader summary of the whole bar. |

**Events**

None.

**Slots**

None.

## Examples

```vue
<StorageAllocationBar label="Disk usage by mount" :segments="[
  { key: 'root', label: '/', percent: 46, color: '#2a78d6', value: '46%' },
  { key: 'free', label: 'Free', percent: 54, color: 'var(--border-strong)', value: '54%' },
]" />
```

## Accessibility

- The bar is `role="img"` named by `label`; the legend is visible text.

## Tokens

`--border-track`, `--font-xs`, `--radius-sm`, `--space-0-5`, `--space-2`, `--space-3`, `--space-4`, `--text-faint-alt`, `--text-strong`

## Gotchas

- The caller chooses each segment's color, so a mount keeps its color across refreshes.
- Segments have no hover tooltip; the previous markup gave each one a `title` with the size and percentage.

## Migration

Before:

```vue
<div class="storage-allocation-bar" role="img" :aria-label="t('storage.allocation')">
  <span v-for="s in segments" :key="s.key" :style="{ width: `${s.percent}%`, background: s.color }" :title="s.label" />
</div>
<ul class="storage-allocation-legend">
  <li v-for="s in segments" :key="s.key">
    <span class="swatch" :style="{ background: s.color }" />
    <span class="storage-allocation-label">{{ s.label }}</span>
    <span class="storage-allocation-value">{{ s.percent.toFixed(0) }}%</span>
  </li>
</ul>
```

After:

```vue
<StorageAllocationBar :label="t('storage.allocation')" :segments="segments" />
```

- Map each old segment to `{ key, label, percent, color, value }`, formatting `value` first.

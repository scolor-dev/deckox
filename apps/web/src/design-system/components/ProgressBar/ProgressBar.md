---
name: ProgressBar
components: [ProgressBar]
category: feedback
replaces: [.progress]
related: [StorageAllocationBar, MetricCard]
---

# ProgressBar

Single value against a track.

## When to use

- One percentage: memory in use, disk usage of a single mount.

## When not to use

- Several shares of one whole: StorageAllocationBar.
- Unknown progress: AppSpinner.

## API

### ProgressBar

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `value` | `number` | required | 0 to 100. Out-of-range values are clamped. |
| `critical` | `boolean` | `false` | Colors the fill as danger, for example at or above 90%. |
| `label` | `string \| null` | `null` | Accessible name of the bar. |

**Events**

None.

**Slots**

None.

## Examples

```vue
<ProgressBar :value="usagePercent" :critical="usagePercent >= 90" label="Disk usage" />
```

## Accessibility

- `role="progressbar"` with `aria-valuenow`, `aria-valuemin` and `aria-valuemax`.
- Without `label` the bar has no accessible name; pass one.

## Tokens

`--border-track`, `--brand-focus`, `--danger-accent`, `--duration-slow`, `--ease-standard`

## Gotchas

- The bar is 5px high and expects to be placed inside a card; it sets no margins.

## Migration

Before:

```vue
<div class="progress">
  <span :class="{ critical: percent >= 90 }" :style="{ width: `${percent}%` }" />
</div>
```

After:

```vue
<ProgressBar :value="percent" :critical="percent >= 90" label="Disk usage" />
```

- Extra layout classes on the old wrapper (`disk-usage-progress`, `storage-summary-bar`) supplied margins; add spacing on a parent instead.

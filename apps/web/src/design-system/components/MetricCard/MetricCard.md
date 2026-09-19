---
name: MetricCard
components: [MetricCard]
category: data
replaces: [.metric-card]
related: [ProgressBar, AppCard]
---

# MetricCard

Tile shell for a single metric: header, value, optional chart or bar, footer and warning.

## When to use

- Overview metrics (CPU, memory, swap, load).

## When not to use

- Generic content: AppCard.

## API

### MetricCard

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `label` | `string` | required | Metric name in the header. |
| `meta` | `string \| null` | `null` | Small right-aligned header text, for example "4 cores". |
| `value` | `string` | required | The headline value, already formatted. |
| `footer` | `string \| null` | `null` | Small text under the content. |
| `warning` | `boolean` | `false` | Warm border and background, and shows `warningText`. |
| `warningText` | `string \| null` | `null` | Warning line, shown only while `warning` is true. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | Chart or ProgressBar between the value and the footer. |

## Examples

```vue
<MetricCard label="Memory" meta="7.6 GiB total" value="41%" footer="3.1 GiB used">
  <ProgressBar :value="41" label="Memory in use" />
</MetricCard>
```

## Accessibility

- A plain `div`; give the chart or bar inside it its own accessible name.

## Tokens

`--border-default`, `--font-2xl`, `--font-2xs`, `--font-xs`, `--metric-warning-border`, `--metric-warning-text`, `--radius-md`, `--space-2`, `--space-4`, `--space-5`, `--surface-elevated`, `--text-emphasis`, `--text-faint-alt`, `--text-label`, `--warning-bg`

## Gotchas

- It does not draw charts. Sparkline drawing is per-metric logic (scale, path), so pass it in the default slot.
- The value is a string; format numbers before passing them.

## Migration

Before:

```vue
<article class="metric-card">
  <div class="metric-head"><span>Memory</span><small>7.6 GiB total</small></div>
  <strong>41%</strong>
  <MetricChart :values="memoryHistory" :maximum="100" />
  <small class="metric-foot">41.0%</small>
</article>
```

After:

```vue
<MetricCard label="Memory" meta="7.6 GiB total" value="41%" footer="41.0%">
  <MetricChart :values="memoryHistory" :maximum="100" />
</MetricCard>
```

- The root element becomes a `div` instead of an `article`.

---
name: AppSpinner
components: [AppSpinner]
category: feedback
replaces: [.restart-indicator]
related: [AppSkeleton, ProgressBar]
---

# AppSpinner

Small spinning loading indicator.

## When to use

- Waiting on an action of unknown duration, for example next to a button label.

## When not to use

- Loading content with a known shape: AppSkeleton.
- Progress with a known percentage: ProgressBar.

## API

### AppSpinner

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `label` | `string` | `"Loading"` | Accessible name announced to screen readers. |

**Events**

None.

**Slots**

None.

## Examples

```vue
<AppButton :disabled="loading"><AppSpinner v-if="loading" label="Refreshing" /> Refresh</AppButton>
```

## Accessibility

- `role="status"` with `aria-label` from `label`. Pass a translated label.

## Tokens

`--brand-primary`, `--duration-spin`, `--spinner-track`

## Gotchas

- With `prefers-reduced-motion`, the spin slows to a third of its speed instead of stopping, because a stopped ring conveys nothing.

## Migration

Before:

```vue
<span class="restart-indicator" />
```

After:

```vue
<AppSpinner label="Restarting" />
```

- The old indicator was 12px with a fixed color pair; the spinner is 14px and uses `--spinner-track` and `--brand-primary`.
- The old `.ready` (finished) state has no counterpart here; render a check mark or StateBadge for it.

---
name: AppSkeleton
components: [AppSkeleton]
category: feedback
replaces: []
related: [AppSpinner]
---

# AppSkeleton

Loading placeholder shaped like the content that is coming.

## When to use

- Reserving space for text, blocks or avatars while data loads, so the layout does not jump.

## When not to use

- Short waits on an action: AppSpinner.

## API

### AppSkeleton

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `variant` | `"text" \| "block" \| "circle"` | `"text"` | `text` (one line), `block` (card-sized) or `circle` (avatar-sized). |
| `width` | `string \| null` | `null` | CSS width override, for example `60%`. |
| `height` | `string \| null` | `null` | CSS height override, for example `4rem`. |

**Events**

None.

**Slots**

None.

## Examples

```vue
<div aria-busy="true">
  <AppSkeleton width="60%" />
  <AppSkeleton variant="block" />
</div>
```

## Accessibility

- Purely visual (`aria-hidden`). Mark the loading region itself with `aria-busy="true"`.

## Tokens

`--duration-spin`, `--radius-md`, `--radius-sm`, `--space-10`, `--space-3`, `--space-8`, `--surface-hover`, `--surface-muted`

## Gotchas

- Blocks are `display: block`; space them with a flex or grid parent using `gap`.
- With `prefers-reduced-motion` the shimmer stops and a static placeholder remains.

## Migration

No existing equivalent in the app.

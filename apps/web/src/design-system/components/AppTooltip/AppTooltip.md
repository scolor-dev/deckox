---
name: AppTooltip
components: [AppTooltip]
category: overlay
replaces: []
related: [AppPopover]
---

# AppTooltip

One-line label that appears on hover or keyboard focus.

## When to use

- Explaining an icon or a terse button.

## When not to use

- Interactive content or long text: AppPopover.
- Information a touch-screen user needs: it only appears on hover and focus.

## API

### AppTooltip

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `text` | `string` | required | Tooltip text. |
| `placement` | `"top" \| "bottom"` | `"top"` | Show above or below the trigger. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | The trigger element. |

## Examples

```vue
<AppTooltip text="Restart this service">
  <AppButton variant="action">Restart</AppButton>
</AppTooltip>
```

## Accessibility

- The bubble has `role="tooltip"` and shows on `:hover` and `:focus-within`, so the trigger must be focusable.
- The bubble is not linked to the trigger with `aria-describedby`.

## Tokens

`--duration-base`, `--ease-standard`, `--font-xs`, `--radius-sm`, `--space-1`, `--space-2`, `--surface-elevated`, `--text-heading-strong`, `--z-popover`

## Gotchas

- Pure CSS: no timers or listeners, so there is no show delay.
- The bubble never wraps (`white-space: nowrap`); keep the text short.

## Migration

No existing equivalent in the app.

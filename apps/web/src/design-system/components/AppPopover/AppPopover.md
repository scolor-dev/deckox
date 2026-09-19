---
name: AppPopover
components: [AppPopover, AppPopoverItem]
category: overlay
replaces: []
related: [AppTooltip, AppButton]
---

# AppPopover

Floating panel anchored under a trigger, used as a dropdown menu. AppPopoverItem is one row of it.

## When to use

- A short list of actions behind one button.

## When not to use

- A single line of hint text: AppTooltip.
- A blocking decision: ConfirmDialog.

## API

### AppPopover

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `open` | `boolean` | required | Whether the panel is shown. |
| `align` | `"start" \| "end"` | `"start"` | Which edge of the trigger the panel lines up with. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `close` | `[]` | A click outside the popover or Escape. The caller must set `open` to false. |

**Slots**

| Name | Description |
|---|---|
| `trigger` | The element that opens the popover. Its click handler must toggle `open`. |
| `default` | Panel content, normally AppPopoverItem elements. |

### AppPopoverItem

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `danger` | `boolean` | `false` | Red text and red hover for destructive items. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `click` | `[]` | The item was clicked. |

**Slots**

| Name | Description |
|---|---|
| `default` | Item label. |

## Examples

```vue
<AppPopover :open="open" align="end" @close="open = false">
  <template #trigger>
    <AppButton @click="open = !open">Actions</AppButton>
  </template>
  <AppPopoverItem @click="restart">Restart</AppPopoverItem>
  <AppPopoverItem danger @click="remove">Remove</AppPopoverItem>
</AppPopover>
```

## Accessibility

- The panel has `role="menu"` and items have `role="menuitem"`.
- There is no arrow-key navigation between items; Tab moves between them.

## Tokens

`--border-strong`, `--danger-bg`, `--danger-strong-border`, `--duration-fast`, `--ease-standard`, `--font-sm`, `--radius-md`, `--radius-sm`, `--shadow-overlay`, `--space-2`, `--space-3`, `--surface-elevated`, `--surface-hover`, `--text-strong`, `--z-popover`

## Gotchas

- The component does not toggle itself. Clicks on the trigger count as inside, so they never emit `close`; toggle `open` in the trigger's own handler.
- Choosing an item does not close the panel; set `open = false` in the item's click handler.
- The panel is removed from the DOM when closed, not hidden.

## Migration

No existing equivalent in the app.

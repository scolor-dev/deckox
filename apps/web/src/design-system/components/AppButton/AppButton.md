---
name: AppButton
components: [AppButton]
category: actions
replaces: [.button, .action-button, .primary-button, .menu-button, .logout-button]
related: [AppIconButton, ButtonGroup, ConfirmDialog]
---

# AppButton

Text button. The variant is chosen by where the button sits, not by how important it feels.

## When to use

- `default`: page-level actions such as Refresh, Close, Cancel.
- `action`: small inline actions inside a table row (Start, Restart, Logs).
- `primary`: the single confirming button in a dialog.
- `menu`: the mobile hamburger toggle only.
- `logout`: the bare text link-style button in the sidebar footer only.
- Add `danger` for destructive actions: outlined red on `action`, filled red on `primary`.

## When not to use

- Icon-only buttons: use AppIconButton (this component needs a text label).
- Several joined buttons acting as one control: wrap them in ButtonGroup.
- Navigation to another page: use a link.

## API

### AppButton

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `variant` | `"default" \| "primary" \| "action" \| "menu" \| "logout"` | `"default"` | Visual role. See When to use. |
| `danger` | `boolean` | `false` | Destructive styling. No effect on `menu` and `logout`. |
| `disabled` | `boolean` | `false` | Disables the native button and dims it. |
| `type` | `"button" \| "submit"` | `"button"` | Native button type. Use `submit` only for the submit button of a form. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | Button label. |

## Examples

```vue
<AppButton :disabled="loading" @click="refresh">Refresh</AppButton>
```

```vue
<AppButton variant="action" danger @click="runAction(service, 'stop')">Stop</AppButton>
```

```vue
<AppButton variant="primary" danger type="submit">Restart computer</AppButton>
```

## Accessibility

- Renders a native `button`; keyboard and focus behavior come from the browser.
- Always pass visible text. For an icon-only control use AppIconButton, which requires an accessible label.

## Tokens

`--border-hover`, `--border-strong`, `--brand-primary`, `--brand-primary-border`, `--brand-primary-hover`, `--danger-bg`, `--danger-border`, `--danger-strong`, `--danger-strong-border`, `--danger-strong-hover`, `--font-md`, `--font-sm`, `--font-xs`, `--link`, `--radius-sm`, `--space-1`, `--space-2`, `--space-4`, `--surface-elevated`, `--surface-hover`, `--text-inverse`, `--text-label`, `--text-strong`

## Gotchas

- Confirming a destructive dialog uses `primary` + `danger` (filled). `action` + `danger` is the outlined row-level form.
- The danger hover changes the border color as well as the background on purpose: the background tint alone is nearly invisible in light mode.
- `click` is the native event; no custom event is declared.

## Migration

Before:

```vue
<button class="button" type="button" :disabled="loading" @click="refresh">Refresh</button>
<button class="action-button danger" type="button" @click="stop">Stop</button>
<button class="primary-button danger-button" type="submit">Restart computer</button>
```

After:

```vue
<AppButton :disabled="loading" @click="refresh">Refresh</AppButton>
<AppButton variant="action" danger @click="stop">Stop</AppButton>
<AppButton variant="primary" danger type="submit">Restart computer</AppButton>
```

- `.primary-button` carried a `margin-top`; the component does not. Space it from the parent.
- Horizontal padding is 16px instead of 14px because spacing now snaps to the scale.

---
name: AppIconButton
components: [AppIconButton]
category: actions
replaces: []
related: [AppButton, AppModal, AppChip, AppToast]
---

# AppIconButton

Small icon-only button. Used inside AppModal (close), AppChip (remove) and AppToast (dismiss).

## When to use

- A compact control whose glyph (for example ×) is understood without text.

## When not to use

- Anything that needs a visible text label: use AppButton.

## API

### AppIconButton

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `label` | `string` | required | Accessible name. Required because there is no visible text. |
| `size` | `"sm" \| "md"` | `"md"` | `md` is 24px square; `sm` is a 14px circle for use inside pills. |
| `danger` | `boolean` | `false` | Hover turns red instead of neutral. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `click` | `[]` | Emitted on click, no payload. |

**Slots**

| Name | Description |
|---|---|
| `default` | The glyph, for example `×`. |

## Examples

```vue
<AppIconButton label="Close" @click="open = false">×</AppIconButton>
```

```vue
<AppIconButton size="sm" danger label="Remove docker.io" @click="remove">×</AppIconButton>
```

## Accessibility

- `label` becomes `aria-label`; write it in the user's language.

## Tokens

`--danger-accent`, `--danger-bg`, `--font-2xl`, `--font-sm`, `--radius-sm`, `--surface-hover`, `--text-faint`, `--text-muted`, `--text-primary`

## Gotchas

- Because the component declares `click`, listen with `@click` as usual; no native event is forwarded twice.

## Migration

No existing equivalent in the app.

---
name: AppSwitch
components: [AppSwitch]
category: forms
replaces: []
related: [AppCheckbox]
---

# AppSwitch

On/off switch for settings that take effect immediately.

## When to use

- A setting applied the moment it is flipped, such as enabling a feature.

## When not to use

- An option submitted later with a form: AppCheckbox.

## API

### AppSwitch

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `modelValue` | `boolean` | required | On/off state. Use with `v-model`. |
| `label` | `string \| null` | `null` | Visible label text. Effectively required: it is the switch's accessible name. |
| `disabled` | `boolean` | `false` | Disables the switch and dims it. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `update:modelValue` | `[value: boolean]` | New on/off state. |

**Slots**

None.

## Examples

```vue
<AppSwitch v-model="allowReboot" label="Allow host restart" />
```

## Accessibility

- The input has `role="switch"` and `aria-checked`.
- Without `label` the switch has no accessible name; always pass it.

## Tokens

`--border-strong`, `--brand-primary`, `--duration-base`, `--ease-standard`, `--font-sm`, `--shadow-thumb`, `--space-2`, `--surface-elevated`, `--text-strong`

## Gotchas

- Not used anywhere in the app yet; existing on/off settings use AppCheckbox.
- The native input is visually hidden, so focus is shown on the track (2px outline).

## Migration

No existing equivalent in the app.

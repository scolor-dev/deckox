---
name: SelectField
components: [SelectField]
category: forms
replaces: []
related: [RadioGroup, TextField]
---

# SelectField

Labeled dropdown for choosing one option from a list.

## When to use

- Four or more options, or options that would take too much room as radio buttons.

## When not to use

- Two or three options that should all be visible: RadioGroup.
- Free text: TextField.

## API

### SelectField

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `modelValue` | `string` | required | Selected option value. Use with `v-model`. |
| `label` | `string` | required | Visible label text. |
| `id` | `string` | required | Required. Links the label to the select; must be unique on the page. |
| `options` | `{ value: string; label: string }[]` | required | Options as `{ value, label }` pairs, in display order. |
| `help` | `string \| null` | `null` | Small helper text under the select. |
| `disabled` | `boolean` | `false` | Disables the select. |
| `labelHidden` | `boolean` | `false` | Hide the label visually but keep it for screen readers, for example on a toolbar filter. |
| `compact` | `boolean` | `false` | A smaller select that is as wide as its content, for toolbars and dense bars. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `update:modelValue` | `[value: string]` | Newly selected option value. |

**Slots**

None.

## Examples

```vue
<SelectField id="theme" v-model="theme" label="Color theme" :options="[{ value: 'auto', label: 'Use device setting' }, { value: 'dark', label: 'Dark' }]" />
```

## Accessibility

- The label is a real `label` bound by `id`.

## Tokens

`--border-strong`, `--brand-focus`, `--font-sm`, `--font-xs`, `--radius-sm`, `--shadow-focus`, `--space-0-5`, `--space-1`, `--space-2`, `--space-3`, `--surface-elevated`, `--surface-muted`, `--text-base`, `--text-faint`, `--text-muted`, `--text-strong`

## Gotchas

- There is no placeholder or empty option; include one in `options` if "nothing selected" is valid.
- `modelValue` must match one option's `value`, otherwise the native select shows the first option.

## Migration

No existing equivalent in the app.

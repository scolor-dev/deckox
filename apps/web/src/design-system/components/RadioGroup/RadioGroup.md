---
name: RadioGroup
components: [RadioGroup]
category: forms
replaces: []
related: [SelectField, AppCheckbox]
---

# RadioGroup

Fieldset of radio buttons for choosing exactly one option.

## When to use

- Two to five mutually exclusive options that should all be visible.

## When not to use

- Long lists: SelectField.
- Several independent options: AppCheckbox.

## API

### RadioGroup

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `modelValue` | `string` | required | Selected option value. Use with `v-model`. |
| `label` | `string` | required | Group legend text. |
| `name` | `string` | required | Native radio group name. Must be unique per group on the page. |
| `options` | `{ value: string; label: string }[]` | required | Options as `{ value, label }` pairs, in display order. |
| `help` | `string \| null` | `null` | Small helper text under the options. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `update:modelValue` | `[value: string]` | Newly selected option value. |

**Slots**

None.

## Examples

```vue
<RadioGroup v-model="theme" name="theme" label="Color theme" :options="[{ value: 'auto', label: 'Use device setting' }, { value: 'dark', label: 'Dark' }]" />
```

## Accessibility

- Renders a `fieldset` with a `legend`; arrow-key selection comes from native radios sharing `name`.

## Tokens

`--brand-primary`, `--font-md`, `--font-sm`, `--font-xs`, `--space-0-5`, `--space-2`, `--text-muted`, `--text-secondary`, `--text-strong`

## Gotchas

- Two groups with the same `name` on one page fight each other's selection.

## Migration

No existing equivalent in the app.

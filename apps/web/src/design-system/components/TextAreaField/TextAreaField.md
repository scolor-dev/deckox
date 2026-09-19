---
name: TextAreaField
components: [TextAreaField]
category: forms
replaces: []
related: [TextField]
---

# TextAreaField

Labeled multi-line text input.

## When to use

- Free text longer than a line: notes, descriptions.

## When not to use

- Single-line values: TextField.

## API

### TextAreaField

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `modelValue` | `string` | required | Current value. Use with `v-model`. |
| `label` | `string` | required | Visible label text. |
| `id` | `string` | required | Required. Links the label to the textarea; must be unique on the page. |
| `rows` | `number` | `4` | Visible line count. |
| `placeholder` | `string \| null` | `null` | Placeholder text. |
| `help` | `string \| null` | `null` | Small helper text under the field. |
| `disabled` | `boolean` | `false` | Disables the field and its resize handle. |
| `required` | `boolean` | `false` | Marks the field as required for form validation. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `update:modelValue` | `[value: string]` | New string value on every input event. |

**Slots**

None.

## Examples

```vue
<TextAreaField id="note" v-model="note" label="Note" :rows="3" />
```

## Accessibility

- The label is a real `label` bound by `id`.

## Tokens

`--border-strong`, `--brand-focus`, `--font-sm`, `--font-xs`, `--radius-sm`, `--shadow-focus`, `--space-1`, `--space-2`, `--space-3`, `--surface-elevated`, `--text-base`, `--text-muted`, `--text-strong`

## Gotchas

- Resizing is vertical only.

## Migration

No existing equivalent in the app.

---
name: TextField
components: [TextField]
category: forms
replaces: [.settings-form]
related: [TextAreaField, SelectField, FileField]
---

# TextField

Labeled single-line text input with optional help text.

## When to use

- Any single-line value: names, package names, URLs, passwords.

## When not to use

- Multi-line text: TextAreaField.
- A choice from a list: SelectField or RadioGroup.

## API

### TextField

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `modelValue` | `string` | required | Current value. Use with `v-model`. |
| `label` | `string` | required | Visible label text. |
| `id` | `string` | required | Required. Links the label to the input and must be unique on the page. |
| `type` | `"text" \| "search" \| "password" \| "email" \| "url"` | `"text"` | Native input type. |
| `placeholder` | `string \| null` | `null` | Placeholder text. |
| `help` | `string \| null` | `null` | Small helper text under the input. |
| `disabled` | `boolean` | `false` | Disables the input. |
| `required` | `boolean` | `false` | Marks the input as required for form validation. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `update:modelValue` | `[value: string]` | New string value on every input event. |

**Slots**

None.

## Examples

```vue
<TextField id="package-name" v-model="name" label="Package name" help="Checked against this host's repositories." />
```

## Accessibility

- The label is a real `label` bound by `id`. Do not omit `label`.

## Tokens

`--border-strong`, `--brand-focus`, `--font-sm`, `--font-xs`, `--radius-sm`, `--shadow-focus`, `--space-1`, `--space-2`, `--space-3`, `--surface-elevated`, `--text-base`, `--text-muted`, `--text-strong`

## Gotchas

- Give every field a stable, unique `id`; two fields sharing one break label association.

## Migration

Before:

```vue
<div class="settings-form">
  <label for="language">Language</label>
  <input id="language" v-model="value">
</div>
```

After:

```vue
<TextField id="language" v-model="value" label="Language" />
```

- The old markup relied on the `.settings-form` parent for styling. The component carries its own styles, so the parent class is no longer needed for the field.

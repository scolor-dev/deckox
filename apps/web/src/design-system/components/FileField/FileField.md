---
name: FileField
components: [FileField]
category: forms
replaces: []
related: [TextField]
---

# FileField

Labeled native file picker.

## When to use

- Choosing one file to upload or import.

## When not to use

- Choosing several files: this component reports only the first.

## API

### FileField

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `label` | `string` | required | Visible label text. |
| `id` | `string` | required | Required. Links the label to the input; must be unique on the page. |
| `accept` | `string \| null` | `null` | Native `accept` filter, for example `.toml`. |
| `help` | `string \| null` | `null` | Small helper text under the input. |
| `disabled` | `boolean` | `false` | Disables the input. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `select` | `[file: File \| null]` | The chosen `File`, or `null` when the selection is cleared. |

**Slots**

None.

## Examples

```vue
<FileField id="import" label="Import config" accept=".toml" @select="file = $event" />
```

## Accessibility

- The label is a real `label` bound by `id`.

## Tokens

`--border-hover`, `--border-strong`, `--brand-focus`, `--font-sm`, `--font-xs`, `--radius-sm`, `--shadow-focus`, `--space-1`, `--space-2`, `--space-3`, `--surface-elevated`, `--surface-hover`, `--surface-subtle`, `--text-muted`, `--text-secondary`, `--text-strong`

## Gotchas

- A file input's value cannot be set from script, so there is no `v-model`. Store the emitted `File` yourself.

## Migration

No existing equivalent in the app.

---
name: AppCheckbox
components: [AppCheckbox]
category: forms
replaces: [.checkbox-field]
related: [AppSwitch, TagToggle]
---

# AppCheckbox

Labeled checkbox with optional help text.

## When to use

- An independent on/off option inside a form that is saved with the rest of the form.

## When not to use

- A setting that applies immediately when flipped: AppSwitch.
- Filtering a list by category: TagToggle.

## API

### AppCheckbox

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `modelValue` | `boolean` | required | Checked state. Use with `v-model`. |
| `label` | `string` | required | Visible label text. |
| `help` | `string \| null` | `null` | Small helper text under the label. |
| `disabled` | `boolean` | `false` | Disables the checkbox. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `update:modelValue` | `[value: boolean]` | New checked state. |

**Slots**

None.

## Examples

```vue
<AppCheckbox v-model="realtime" label="Live updates" help="Connects only while this page is visible." />
```

## Accessibility

- The input sits inside its `label`, so clicking the text toggles it.

## Tokens

`--brand-primary`, `--font-sm`, `--font-xs`, `--space-1`, `--space-2`, `--space-6`, `--text-muted`, `--text-strong`

## Gotchas

- There is no `id` prop; association is by nesting.

## Migration

Before:

```vue
<label class="checkbox-field">
  <input v-model="preferences.realtimeEnabled" type="checkbox">
  <span>Live updates</span>
</label>
```

After:

```vue
<AppCheckbox v-model="preferences.realtimeEnabled" label="Live updates" />
```

- The old rule was scoped to `.settings-form`; the component works anywhere.

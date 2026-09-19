---
name: ConfirmDialog
components: [ConfirmDialog]
category: overlay
replaces: [.confirm-dialog, .dialog-backdrop]
related: [AppModal, AppButton]
---

# ConfirmDialog

Small fixed-width dialog that asks one yes/no question.

## When to use

- Confirming a destructive or costly action, usually with a password field or a short explanation.

## When not to use

- Content-heavy dialogs (logs, forms with many fields): AppModal.

## API

### ConfirmDialog

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `open` | `boolean` | required | Whether the dialog is shown. |
| `title` | `string` | required | Heading text. Also the accessible name unless `ariaLabel` is set. |
| `ariaLabel` | `string \| null` | `null` | Accessible name override. Leave unset unless the heading is a poor name. |
| `closeOnBackdrop` | `boolean` | `true` | Emit `close` when the dimmed backdrop is clicked. |
| `closeOnEscape` | `boolean` | `true` | Emit `close` when Escape is pressed. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `close` | `[]` | The user asked to close: Escape or backdrop click. The caller must set `open` to false. |

**Slots**

| Name | Description |
|---|---|
| `default` | Dialog body. |
| `actions` | Footer buttons. The footer renders only when this slot is provided. |

## Examples

```vue
<ConfirmDialog :open="armed !== null" title="Remove docker.io?" @close="armed = null">
  <p>Configuration and data are kept.</p>
  <template #actions>
    <AppButton @click="armed = null">Close</AppButton>
    <AppButton variant="primary" danger @click="remove">Remove</AppButton>
  </template>
</ConfirmDialog>
```

## Accessibility

- `role="dialog"` and `aria-modal="true"`, named by `title`.
- Escape closes it (through `close`).
- Focus moves to the dialog on open (or to a descendant marked `data-autofocus`), Tab and Shift+Tab wrap inside it, and focus returns to the previously focused element on close. With several overlays open only the topmost traps focus.

## Tokens

`--border-strong`, `--font-lg`, `--overlay-color`, `--radius-md`, `--shadow-overlay`, `--space-1`, `--space-2`, `--space-3`, `--space-5`, `--space-6`, `--surface-elevated`, `--text-heading`, `--z-overlay`

## Gotchas

- Put `data-autofocus` on the least destructive action when the dialog confirms something irreversible; otherwise the dialog itself receives focus.
- It is rendered in place, not teleported. Do not place it inside an ancestor with a CSS `transform` or `filter`, or `position: fixed` will anchor to that ancestor.
- It has no close button; provide one in `actions`.

## Migration

Before:

```vue
<div v-if="armed" class="dialog-backdrop" @click.self="close">
  <section class="confirm-dialog" role="dialog" aria-modal="true" :aria-label="title">
    <h2>{{ title }}</h2>
    ...
    <div class="dialog-actions">...</div>
  </section>
</div>
```

After:

```vue
<ConfirmDialog :open="armed !== null" :title="title" @close="close">
  ...
  <template #actions>...</template>
</ConfirmDialog>
```

- `PasswordConfirmDialog.vue` in the app is built on the same old classes and would be rebuilt on this component.

---
name: AppModal
components: [AppModal]
category: overlay
replaces: [.log-dialog]
related: [ConfirmDialog, AppIconButton]
---

# AppModal

General dialog with a header, scrolling body and optional footer.

## When to use

- Content that needs room: a service log viewer, a form with several fields.

## When not to use

- A yes/no question: ConfirmDialog.

## API

### AppModal

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `open` | `boolean` | required | Whether the modal is shown. |
| `title` | `string` | required | Heading text. Also the accessible name unless `ariaLabel` is set. |
| `ariaLabel` | `string \| null` | `null` | Accessible name override. Leave unset unless the heading is a poor name. |
| `closeLabel` | `string` | `"Close"` | Accessible name of the × button. Pass a translated string. |
| `size` | `"small" \| "medium" \| "large"` | `"medium"` | Width: `small` 420px, `medium` 600px, `large` 900px (all capped to the viewport). |
| `closeOnBackdrop` | `boolean` | `true` | Emit `close` when the dimmed backdrop is clicked. |
| `closeOnEscape` | `boolean` | `true` | Emit `close` when Escape is pressed. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `close` | `[]` | The × button, Escape or backdrop asked to close. The caller must set `open` to false. |

**Slots**

| Name | Description |
|---|---|
| `header-actions` | Extra buttons to the left of the × in the header. |
| `default` | Scrolling body. |
| `footer` | Footer buttons. The footer renders only when this slot is provided. |

## Examples

```vue
<AppModal :open="logService !== null" title="Service log: docker.service" size="large" close-label="Close" @close="logService = null">
  <LogList>...</LogList>
  <template #footer>
    <AppButton @click="logService = null">Close</AppButton>
  </template>
</AppModal>
```

## Accessibility

- `role="dialog"` and `aria-modal="true"`, named by `title`.
- Escape closes it (through `close`).
- Focus moves to the dialog on open (or to a descendant marked `data-autofocus`), Tab and Shift+Tab wrap inside it, and focus returns to the previously focused element on close. With several overlays open only the topmost traps focus.

## Tokens

`--border-default`, `--border-strong`, `--font-lg`, `--overlay-color`, `--radius-md`, `--shadow-overlay`, `--space-2`, `--space-3`, `--space-4`, `--space-5`, `--space-6`, `--surface-elevated`, `--text-heading`, `--z-overlay`

## Gotchas

- Put `data-autofocus` on the control that should receive focus first (for example the main input); otherwise the dialog itself receives focus.
- It is rendered in place, not teleported. Avoid ancestors with a CSS `transform` or `filter`.
- Height is capped at 760px or the viewport height minus 48px; the body scrolls, the header and footer do not.

## Migration

Before:

```vue
<div v-if="logService" class="dialog-backdrop" @click.self="closeLogs">
  <section class="log-dialog" role="dialog" aria-modal="true" :aria-label="title">
    <header class="log-dialog-header">...</header>
    ...
  </section>
</div>
```

After:

```vue
<AppModal :open="logService !== null" :title="title" size="large" @close="closeLogs">
  ...
</AppModal>
```

- The old header carried Download and Close buttons; put Download in `header-actions`. The × replaces the Close button.

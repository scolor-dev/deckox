---
name: AppToast
components: [AppToast, ToastRegion]
category: feedback
replaces: [.notification-item, .notification-region]
related: [NoticeBanner, AppIconButton]
---

# AppToast

Transient message that floats over the page. AppToast is one message; ToastRegion is the fixed stack that holds them.

## When to use

- Confirming a completed action ("nginx.service restarted") or reporting a background failure.

## When not to use

- A message that must stay next to a specific form or table: NoticeBanner.

## API

### AppToast

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `tone` | `"success" \| "warning" \| "error"` | required | `success`, `warning` or `error`; shown as a colored left edge. |
| `dismissLabel` | `string` | required | Accessible name of the dismiss button. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `dismiss` | `[]` | The dismiss button was clicked. |

**Slots**

| Name | Description |
|---|---|
| `default` | Message text. |

### ToastRegion

**Props**

None.

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | AppToast elements. |

## Examples

```vue
<ToastRegion>
  <AppToast v-for="item in items" :key="item.id" :tone="item.tone" dismiss-label="Dismiss" @dismiss="remove(item.id)">
    {{ item.message }}
  </AppToast>
</ToastRegion>
```

## Accessibility

- `error` toasts use `role="alert"`; the others use `role="status"`.

## Tokens

`--border-strong`, `--danger-accent`, `--font-sm`, `--radius-sm`, `--shadow-raised`, `--space-2`, `--space-3`, `--space-4`, `--space-8`, `--success-strong`, `--surface-elevated`, `--text-strong`, `--warning-strong`, `--z-toast`

## Gotchas

- ToastRegion holds no state. The queue and auto-dismiss timing belong to the caller (today: `notifications.ts`).
- Toasts sit below dialogs and modals (`--z-toast` 50 is lower than `--z-overlay` 60), so an open dialog covers them.
- The old markup used a text button ("Close"); this uses an × icon button, which is a visible change.

## Migration

Before:

```vue
<div class="notification-region" aria-live="polite" aria-atomic="false">
  <div v-for="item in items" :key="item.id" :class="['notification-item', item.kind]" role="status">
    <span>{{ item.message }}</span>
    <button type="button" :aria-label="t('common.dismiss')" @click="dismiss(item.id)">{{ t("common.close") }}</button>
  </div>
</div>
```

After:

```vue
<ToastRegion>
  <AppToast v-for="item in items" :key="item.id" :tone="item.kind" :dismiss-label="t('common.dismiss')" @dismiss="dismiss(item.id)">
    {{ item.message }}
  </AppToast>
</ToastRegion>
```

- The old region set `aria-live="polite"` on the container; each toast now carries its own role instead.

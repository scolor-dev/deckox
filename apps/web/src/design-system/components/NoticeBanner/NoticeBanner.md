---
name: NoticeBanner
components: [NoticeBanner]
category: feedback
replaces: [.notice]
related: [InfoNote, AppToast]
---

# NoticeBanner

Inline banner reporting the result of something that just happened.

## When to use

- An error, warning or success message shown in place, next to the thing it is about.

## When not to use

- Permanent helper text: InfoNote.
- A message that should float over the page and disappear: AppToast.

## API

### NoticeBanner

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `tone` | `"error" \| "warning" \| "success"` | required | `error`, `warning` or `success`. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | Message text. |

## Examples

```vue
<NoticeBanner v-if="error" tone="error">{{ error }}</NoticeBanner>
```

## Accessibility

- `error` renders `role="alert"` (announced immediately); `warning` and `success` render `role="status"` (announced politely).

## Tokens

`--danger-bg`, `--danger-border`, `--danger-text`, `--font-md`, `--radius-sm`, `--space-3`, `--success-bg`, `--success-border`, `--success-text`, `--warning-bg`, `--warning-border`, `--warning-text`

## Gotchas

- It sets no outer margin; space banners with a flex or grid parent using `gap` (for example AppStack).

## Migration

Before:

```vue
<div v-if="error" class="notice error" role="alert">{{ error }}</div>
```

After:

```vue
<NoticeBanner v-if="error" tone="error">{{ error }}</NoticeBanner>
```

- The role is set by the component; do not add it yourself.

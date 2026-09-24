---
name: AppText
components: [AppText]
category: data
replaces: [.service-name, .storage-path]
related: [AppHeading, StateBadge, TagBadge]
---

# AppText

Inline or block text with a tone, a size and an optional monospace or bold face.

## When to use

- Supporting text next to a value: a description, a unit, a timestamp (`muted` or `faint`).
- An identifier the user may copy: a unit name, a path, a package name (`mono`, usually with `strong`).

## When not to use

- A heading: AppHeading.
- A condition or a category: StateBadge or TagBadge.
- Longer explanations under a form or table: InfoNote.

## API

### AppText

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `as` | `string` | `"span"` | Element to render, for example `small` or `strong`. |
| `tone` | `"default" \| "muted" \| "faint"` | `"default"` | Text color: primary, muted or faint. |
| `size` | `"xs" \| "sm" \| "md"` | `"md"` | Font size: 11px, 12px or 13px. |
| `mono` | `boolean` | `false` | Monospace face, for identifiers. |
| `strong` | `boolean` | `false` | Semi-bold weight. |
| `block` | `boolean` | `false` | Renders as a block so it sits on its own line. |
| `preserve` | `boolean` | `false` | Keeps the line breaks and spaces of user-typed text. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | The text. |

## Examples

```vue
<AppText as="strong" mono strong size="xs">nginx.service</AppText>
<AppText as="small" tone="muted" size="xs" block>Web server</AppText>
```

## Accessibility

- `muted` and `faint` keep the contrast enforced by `contrast.test.ts`; do not use `faint` for text that must be read.
- The element you pick with `as` carries the meaning; use `strong` or `small` where they are true, not for looks.

## Tokens

`--font-md`, `--font-sm`, `--font-xs`, `--text-faint`, `--text-muted`, `--text-primary`

## Gotchas

- Inside a table cell, a `small` is also caught by the app's global `td small` rule; prefer `span` there when you want only this component's style.
- It wraps long words anywhere; add `block` when the line must break before it.
- Margins are reset, so `as="p"` needs no extra CSS.

## Migration

Before:

```vue
<strong class="service-name">nginx.service</strong>
```

After:

```vue
<AppText as="strong" mono strong size="xs">nginx.service</AppText>
```

Before:

```vue
<small>Web server</small>
```

After:

```vue
<AppText as="small" tone="muted" size="xs">Web server</AppText>
```

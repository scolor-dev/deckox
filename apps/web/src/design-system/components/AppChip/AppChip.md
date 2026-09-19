---
name: AppChip
components: [AppChip]
category: data
replaces: []
related: [TagBadge, TagToggle, AppIconButton]
---

# AppChip

Pill for a discrete value, optionally removable.

## When to use

- A typed filter or a chosen value the user can remove.

## When not to use

- Classifying a resource: TagBadge.
- Filtering by category: TagToggle.

## API

### AppChip

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `removable` | `boolean` | `false` | Show the × button. |
| `removeLabel` | `string` | `"Remove"` | Accessible name of the × button. Pass a translated string. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `remove` | `[]` | The × button was clicked. |

**Slots**

| Name | Description |
|---|---|
| `default` | Chip content. |

## Examples

```vue
<AppChip v-for="name in packages" :key="name" removable remove-label="Remove" @remove="drop(name)">{{ name }}</AppChip>
```

## Accessibility

- The × is a button named by `removeLabel`; include the item in the label when several chips are shown.

## Tokens

`--border-strong`, `--font-xs`, `--radius-pill`, `--space-1`, `--space-3`, `--surface-elevated`, `--text-secondary`

## Gotchas

- `removeLabel` defaults to the English word "Remove"; pass a translated string in Japanese screens.

## Migration

No existing equivalent in the app.

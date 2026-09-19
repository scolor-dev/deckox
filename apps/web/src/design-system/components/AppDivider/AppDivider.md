---
name: AppDivider
components: [AppDivider]
category: layout
replaces: []
related: [AppCard]
---

# AppDivider

Horizontal rule between blocks of content.

## When to use

- Separating two blocks inside one region.

## When not to use

- Separating cards or panels: give the parent a `gap` instead.

## API

### AppDivider

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `spacing` | `"sm" \| "md" \| "lg"` | `"md"` | Vertical margin: `sm` 12px, `md` 20px, `lg` 32px. |

**Events**

None.

**Slots**

None.

## Examples

```vue
<AppDivider spacing="lg" />
```

## Accessibility

- Renders a native `hr`, which is exposed as a separator.

## Tokens

`--border-default`, `--space-3`, `--space-5`, `--space-8`

## Gotchas

- Margins apply above and below; do not add extra spacing around it.

## Migration

No existing equivalent in the app.

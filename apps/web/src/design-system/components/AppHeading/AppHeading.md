---
name: AppHeading
components: [AppHeading]
category: layout
replaces: [.widget-frame-title, .heading-block, .text-block-title]
related: [PageHeader, AppCard, AppText]
---

# AppHeading

Section heading inside a screen, panel or widget.

## When to use

- Titling a section, panel or widget below the screen title.
- A heading that must stay on one line inside a toolbar (`truncate`).

## When not to use

- The screen title: PageHeader.
- A titled AppCard: give the card a `title` instead.
- Small supporting text: AppText.

## API

### AppHeading

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `level` | `"2" \| "3" \| "4"` | `"2"` | Which `h2`, `h3` or `h4` element to render. Follow the outline of the page, not the look. |
| `size` | `"sm" \| "md" \| "lg" \| "xl"` | `"xl"` | Font size: 12px, 13px, 15px or 17px. Independent of `level`. |
| `truncate` | `boolean` | `false` | Keeps the text on one line with an ellipsis, and lets it take the free width of a flex row. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | Heading text. |

## Examples

```vue
<AppHeading level="3" size="lg">Backups</AppHeading>
```

## Accessibility

- Renders a native heading element, so it joins the page outline; pick `level` for the outline and `size` for the look.

## Tokens

`--font-lg`, `--font-md`, `--font-sm`, `--font-xl`, `--text-primary`

## Gotchas

- It sets no margins; space it with a stack or grid `gap`.
- `truncate` grows the heading to fill a flex row, so place it beside other items only in a flex parent.

## Migration

Before:

```vue
<h2>Backups</h2>
```

After:

```vue
<AppHeading>Backups</AppHeading>
```

Before:

```vue
<h2 class="widget-frame-title">Backups</h2>
```

After:

```vue
<AppHeading size="md">Backups</AppHeading>
```

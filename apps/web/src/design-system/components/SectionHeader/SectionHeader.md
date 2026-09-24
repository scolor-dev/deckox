---
name: SectionHeader
components: [SectionHeader]
category: layout
replaces: [.widget-header]
related: [PageHeader, AppHeading, AppCard]
---

# SectionHeader

Title row of a section or widget: a heading on the left, its actions on the right.

## When to use

- Above a table or a group of controls that make up one section of a screen or one widget.

## When not to use

- The screen title: PageHeader.
- A bordered panel with a title: AppCard with `title` and `actions`.

## API

### SectionHeader

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `title` | `string` | required | Heading text. |
| `level` | `"2" \| "3" \| "4"` | `"2"` | Which `h2`, `h3` or `h4` element to render. |
| `size` | `"sm" \| "md" \| "lg" \| "xl"` | `"lg"` | Font size of the heading. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `actions` | Buttons, counts or status text shown at the right of the row. |

## Examples

```vue
<SectionHeader title="Services">
  <template #actions>
    <AppText as="small" tone="muted" size="xs">12 units</AppText>
    <AppButton>Refresh</AppButton>
  </template>
</SectionHeader>
```

## Accessibility

- The title is a native heading, so it joins the page outline.

## Tokens

`--space-2`

## Gotchas

- It sets no margins; space it from what follows with a stack `gap`.
- Actions wrap under the title on narrow widths.

## Migration

Before:

```vue
<WidgetHeader :title="t('services.title')">
  <AppButton>Refresh</AppButton>
</WidgetHeader>
```

After:

```vue
<SectionHeader :title="t('services.title')">
  <template #actions>
    <AppButton>Refresh</AppButton>
  </template>
</SectionHeader>
```

---
name: AppPagination
components: [AppPagination]
category: navigation
replaces: []
related: [TabBar, AppBreadcrumb]
---

# AppPagination

Page number strip with previous and next buttons.

## When to use

- Moving through a long list split into numbered pages.

## When not to use

- "Load more" lists: a single AppButton.

## API

### AppPagination

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `page` | `number` | required | Current page, 1-based. Use with `v-model:page`. |
| `pageCount` | `number` | required | Total number of pages. |
| `label` | `string` | required | Accessible name of the navigation landmark. |
| `prevLabel` | `string` | required | Accessible name of the previous button. |
| `nextLabel` | `string` | required | Accessible name of the next button. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `update:page` | `[page: number]` | The requested page. Never emitted for the page that is already current. |

**Slots**

None.

## Examples

```vue
<AppPagination v-model:page="page" :page-count="12" label="Audit log pages" prev-label="Previous page" next-label="Next page" />
```

## Accessibility

- A `nav` landmark; the current page has `aria-current="page"`.

## Tokens

`--border-hover`, `--border-strong`, `--brand-primary`, `--brand-primary-border`, `--font-sm`, `--radius-sm`, `--space-1`, `--space-2`, `--space-6`, `--space-8`, `--surface-elevated`, `--surface-hover`, `--text-faint`, `--text-inverse`, `--text-strong`

## Gotchas

- With seven or fewer pages every page is shown. With more, it shows first, last, current and its neighbours; a gap that would hide exactly one page shows that page instead of an ellipsis.
- The component resets the app's global `nav` styles (grid layout and 32px top margin) on itself.

## Migration

No existing equivalent in the app.

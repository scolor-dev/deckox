---
name: AppBreadcrumb
components: [AppBreadcrumb]
category: navigation
replaces: []
related: [AppPagination, TabBar]
---

# AppBreadcrumb

Where the user is in a hierarchy.

## When to use

- Screens nested more than one level deep.

## When not to use

- Flat navigation: the sidebar already covers it. Nothing in the app uses this today.

## API

### AppBreadcrumb

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `items` | `{ label: string; href?: string }[]` | required | Trail from the top level down. Every item except the last may carry an `href`. |
| `label` | `string` | required | Accessible name of the navigation landmark, for example "Breadcrumb". |

**Events**

None.

**Slots**

None.

## Examples

```vue
<AppBreadcrumb label="Breadcrumb" :items="[{ label: 'Settings', href: '/settings' }, { label: 'Security' }]" />
```

## Accessibility

- A `nav` landmark containing an ordered list; the last item has `aria-current="page"`.

## Tokens

`--brand-primary`, `--font-sm`, `--link`, `--space-2`, `--text-faint`, `--text-primary`

## Gotchas

- The last item is never a link, even if it has an `href`.
- Links are plain anchors, not `router-link`, so they reload the page.
- The component resets the app's global `nav` and `nav a` styles on itself.

## Migration

No existing equivalent in the app.

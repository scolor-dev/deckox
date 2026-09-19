---
name: PageHeader
components: [PageHeader]
category: layout
replaces: [.view-header]
related: [AppButton]
---

# PageHeader

The title row every screen opens with: heading and subtitle left, screen-level actions right.

## When to use

- The top of a screen, once.

## When not to use

- Section headings inside a screen: use a plain heading in the section.

## API

### PageHeader

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `title` | `string` | required | Screen title, rendered as the page's `h1`. |
| `subtitle` | `string \| null` | — | Short line under the title. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `actions` | Screen-level controls such as Refresh. The right-hand area renders only when provided. |

## Examples

```vue
<PageHeader title="Services" subtitle="3 of 5 services running">
  <template #actions>
    <AppButton :disabled="loading" @click="refresh">Refresh</AppButton>
  </template>
</PageHeader>
```

## Accessibility

- Renders the only `h1` of a screen; use it once per page.

## Tokens

`--font-2xl`, `--font-3xl`, `--font-sm`, `--space-1`, `--space-2`, `--space-3`, `--space-5`, `--text-heading`, `--text-muted`

## Gotchas

- On narrow screens (700px and below) the actions wrap under the title instead of sharing its row.
- Below the mobile breakpoint the heading shrinks to 19px and the row aligns to the top.
- It sets no outer margin; space it from the content below with a flex or grid parent using `gap`.

## Migration

Before:

```vue
<header class="view-header">
  <div>
    <h1>{{ t("storage.title") }}</h1>
    <p class="subtitle">{{ t("storage.summary", { count: mounts.length }) }}</p>
  </div>
  <button class="button" type="button" :disabled="loading" @click="refresh">Refresh</button>
</header>
```

After:

```vue
<PageHeader :title="t('storage.title')" :subtitle="t('storage.summary', { count: mounts.length })">
  <template #actions>
    <AppButton :disabled="loading" @click="refresh">Refresh</AppButton>
  </template>
</PageHeader>
```

- Mobile heading size is 19px here versus 22px before, because 22px is not on the font scale.

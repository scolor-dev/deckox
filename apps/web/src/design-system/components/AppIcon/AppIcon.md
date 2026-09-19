---
name: AppIcon
components: [AppIcon]
category: actions
replaces: [× ‹ › text glyphs]
related: [AppIconButton, AppButton]
---

# AppIcon

Inline SVG icon that inherits the surrounding text color.

## When to use

- The glyph inside AppIconButton, or next to a label in a button, chip, toast or breadcrumb.
- Any place a text character (×, ‹, ›, /) was standing in for an icon.

## When not to use

- A pressable icon on its own: wrap it in AppIconButton, which supplies the button semantics and label.
- Illustrations or logos: AppIcon only draws the built-in stroke set.

## API

### AppIcon

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `name` | `IconName` | required | One of `close`, `chevron-left`, `chevron-right`, `chevron-down`, `chevron-up`, `check`, `plus`, `minus`, `alert`, `info`, `refresh`, `search`, `trash`, `external`. |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | 12px, 16px or 20px. |
| `label` | `string \| null` | `null` | Accessible name. Leave `null` when the icon is decorative. |

**Events**

None.

**Slots**

None.

## Examples

```vue
<AppIconButton label="Close"><AppIcon name="close" /></AppIconButton>
<AppButton><AppIcon name="refresh" size="sm" /> Refresh</AppButton>
<AppIcon name="check" label="Done" />
```

## Accessibility

- Without `label` the SVG is `aria-hidden="true"` and not focusable, so the surrounding control must carry the name.
- With `label` it becomes `role="img"` with that `aria-label`.

## Tokens

`--space-3`, `--space-4`, `--space-5`

## Gotchas

- Icons draw with `stroke="currentColor"`, so color comes from the parent's `color`. Do not try to color them with `fill`.
- Adding an icon means adding its path data to `icons.ts` and its name to the `name` row above; the icons test fails until both agree.
- The component is a 24-unit stroke icon scaled by CSS; there is no way to pass a custom size in pixels.

## Migration

Before:

```vue
<button type="button" aria-label="Close" @click="close">×</button>
```

After:

```vue
<AppIconButton label="Close" @click="close"><AppIcon name="close" /></AppIconButton>
```

Before:

```vue
<button type="button" :disabled="page <= 1">‹</button>
```

After:

```vue
<AppPagination :page="page" :page-count="pageCount" />
```

- Text glyphs sat on the font baseline and changed weight with the font; icons are fixed 2px strokes.

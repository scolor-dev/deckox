---
name: TagBadge
components: [TagBadge]
category: data
replaces: [.tag-badge]
related: [TagToggle, AppChip, StateBadge]
---

# TagBadge

Small static pill classifying a resource by kind.

## When to use

- Marking a service as Standard, Deckox or a known product next to its name.

## When not to use

- Reporting a condition: StateBadge.
- An interactive filter: TagToggle.
- A removable value: AppChip.

## API

### TagBadge

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `category` | `"standard" \| "deckox" \| "product"` | required | `standard`, `deckox` or `product`; each has fixed colors. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | Label text. |

## Examples

```vue
<TagBadge category="product">Docker</TagBadge>
```

## Accessibility

- Static text; color is not the only carrier because the label is always shown.

## Tokens

`--font-2xs`, `--radius-badge`, `--space-0-5`, `--space-2`, `--tag-deckox-bg`, `--tag-deckox-text`, `--tag-product-bg`, `--tag-product-text`, `--tag-standard-bg`, `--tag-standard-text`

## Gotchas

- The categories are a closed set. A new category needs new `--tag-*` tokens in both themes.
- Text is 10px, the smallest size on the font scale.

## Migration

Before:

```vue
<span :class="['tag-badge', tagClass(tag)]">{{ tagLabel(tag) }}</span>
```

After:

```vue
<TagBadge :category="tagCategory(tag)">{{ tagLabel(tag) }}</TagBadge>
```

- The old code used the class `other` for untagged items in filters; badges only ever use `standard`, `deckox` or `product`.
- 9px text becomes 10px.

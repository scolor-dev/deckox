---
name: TagToggle
components: [TagToggle, TagToggleGroup]
category: data
replaces: [.tag-toggles, .tag-toggle]
related: [TagBadge, AppCheckbox, AppChip]
---

# TagToggle

Checkbox pill for showing or hiding a category in a list. TagToggleGroup is the fieldset that holds a row of them.

## When to use

- Filtering a table by tag, usually inside TableToolbar's `filters` slot.

## When not to use

- A form option: AppCheckbox.
- A display-only label: TagBadge.

## API

### TagToggle

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `category` | `"standard" \| "deckox" \| "product" \| "other"` | required | `standard`, `deckox` or `product` take the tag colors when checked; `other` stays neutral. |
| `checked` | `boolean` | required | Whether the category is shown. Use with `v-model:checked`. |

**Events**

| Name | Payload | Description |
|---|---|---|
| `update:checked` | `[checked: boolean]` | New checked state. |

**Slots**

| Name | Description |
|---|---|
| `default` | Label text. |

### TagToggleGroup

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `label` | `string` | required | Legend text, visually hidden but read by screen readers. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | TagToggle elements. |

## Examples

```vue
<TagToggleGroup label="Visible tags">
  <TagToggle category="standard" v-model:checked="show.standard">Standard</TagToggle>
  <TagToggle category="deckox" v-model:checked="show.deckox">Deckox</TagToggle>
</TagToggleGroup>
```

## Accessibility

- Each pill is a real checkbox inside a label; the group is a `fieldset` with a `legend`.

## Tokens

`--border-hover`, `--brand-primary`, `--font-xs`, `--radius-pill`, `--space-1`, `--space-2`, `--surface-elevated`, `--surface-hover`, `--tag-deckox-border`, `--tag-deckox-text`, `--tag-off-bg`, `--tag-off-text`, `--tag-product-border`, `--tag-product-text`, `--tag-standard-border`, `--tag-toggle-border`, `--text-secondary`

## Gotchas

- An unchecked pill also gets a border color change on hover, because its own background is too close to the hover tint to notice.

## Migration

Before:

```vue
<fieldset class="tag-toggles">
  <legend class="sr-only">Visible tags</legend>
  <label :class="['tag-toggle', tag, { off: isTagHidden(tag) }]">
    <input type="checkbox" :checked="!isTagHidden(tag)" @change="toggleTag(tag)">
    {{ tagLabel(tag) }}
  </label>
</fieldset>
```

After:

```vue
<TagToggleGroup label="Visible tags">
  <TagToggle :category="tagCategory(tag)" :checked="!isTagHidden(tag)" @update:checked="toggleTag(tag)">{{ tagLabel(tag) }}</TagToggle>
</TagToggleGroup>
```

- The component takes `checked` (shown) where the old markup toggled an `off` class (hidden); invert accordingly.

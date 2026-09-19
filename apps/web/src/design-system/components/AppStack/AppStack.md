---
name: AppStack
components: [AppStack]
category: layout
replaces: [one-off flex containers with gap]
related: [AppGrid, ButtonGroup, PageHeader]
---

# AppStack

Flex container that lays children out in a row or column with a scale-based gap.

## When to use

- Spacing siblings apart: form fields, a row of buttons, label plus value.

## When not to use

- Equal-width cards that wrap into columns: AppGrid.
- Segmented controls that must touch: ButtonGroup.

## API

### AppStack

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `as` | `string` | `"div"` | Element to render, for example `ul`, `section` or `nav`. |
| `direction` | `"column" \| "row"` | `"column"` | Main axis. |
| `gap` | `"1" \| "2" \| "3" \| "4" \| "5" \| "6" \| "8"` | `"3"` | Space between children, mapped to `--space-N`. |
| `align` | `"start" \| "center" \| "end" \| "stretch"` | `"stretch"` | Cross-axis alignment. |
| `justify` | `"start" \| "center" \| "end" \| "between"` | `"start"` | Main-axis distribution. |
| `wrap` | `boolean` | `false` | Allow children to wrap onto new lines. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | The children to lay out. |

## Examples

```vue
<AppStack direction="row" gap="2" align="center" wrap>
  <AppButton variant="primary">Save</AppButton>
  <AppButton>Cancel</AppButton>
</AppStack>
```

## Accessibility

- It adds no roles. With `as="ul"` you still need `li` children.

## Tokens

`--space-1`, `--space-2`, `--space-3`, `--space-4`, `--space-5`, `--space-6`, `--space-8`

## Gotchas

- `gap` accepts only the listed scale steps; add a step to the scale first if you need another.
- Children with long text need `min-width: 0` of their own to shrink in a row; the stack itself already has it.

## Migration

Before:

```vue
<div class="actions" style="display: flex; gap: 8px; align-items: center">
  <AppButton>Cancel</AppButton>
  <AppButton variant="primary">Save</AppButton>
</div>
```

After:

```vue
<AppStack direction="row" gap="2" align="center">
  <AppButton>Cancel</AppButton>
  <AppButton variant="primary">Save</AppButton>
</AppStack>
```

- Pick `gap` from the scale: 8px is `"2"`, 12px is `"3"`, 16px is `"4"`.

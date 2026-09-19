---
name: ButtonGroup
components: [ButtonGroup]
category: actions
replaces: []
related: [AppButton]
---

# ButtonGroup

Joins AppButtons into one segmented control.

## When to use

- Two to four related choices shown together, such as a range or a view switcher.

## When not to use

- Unrelated actions side by side: place separate AppButtons with normal spacing.
- Content other than AppButton: the group restyles button edges from outside.

## API

### ButtonGroup

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `label` | `string \| null` | — | Accessible name of the group, for example the setting it controls. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | AppButton elements only. |

## Examples

```vue
<ButtonGroup label="Range">
  <AppButton>Day</AppButton>
  <AppButton>Week</AppButton>
  <AppButton>Month</AppButton>
</ButtonGroup>
```

## Accessibility

- Renders `role="group"` with `aria-label` from `label`.
- It does not track a selected item. If one is selected, express that state yourself (for example `aria-pressed` on the buttons).

## Tokens

`--radius-sm`

## Gotchas

- It targets AppButton's root class `.ds-button` through a deep selector. Renaming that class breaks the group.

## Migration

No existing equivalent in the app.

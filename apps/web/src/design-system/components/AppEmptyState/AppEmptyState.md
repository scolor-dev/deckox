---
name: AppEmptyState
components: [AppEmptyState]
category: feedback
replaces: [.empty]
related: [TablePanel]
---

# AppEmptyState

Centered "nothing here yet" message, with room for a follow-up action.

## When to use

- An empty card or region that is not a table.

## When not to use

- An empty table: TablePanel renders its own empty state through `empty` and `emptyMessage`.

## API

### AppEmptyState

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `message` | `string` | required | The message text. |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | Optional content under the message, such as a button. |

## Examples

```vue
<AppEmptyState message="No schedules configured.">
  <AppButton @click="openForm">Add schedule</AppButton>
</AppEmptyState>
```

## Accessibility

- Static text; no role. Announce changes through the region that contains it if needed.

## Tokens

`--space-10`, `--space-3`, `--space-5`, `--text-faint`

## Gotchas

- It has generous vertical padding (40px) and is meant to fill a card.

## Migration

Before:

```vue
<td colspan="4" class="empty">{{ t("services.empty") }}</td>
```

After:

```vue
<AppEmptyState :message="t('services.empty')" />
```

- Inside a table body, prefer TablePanel's `empty` prop rather than a table cell.

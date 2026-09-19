---
name: StateBadge
components: [StateBadge]
category: feedback
replaces: [.state-badge]
related: [TagBadge]
---

# StateBadge

Colored dot plus text reporting a resource's condition.

## When to use

- Service running, stopped or failed; audit result good or bad.

## When not to use

- Classifying a resource by kind: TagBadge.

## API

### StateBadge

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `state` | `"active" \| "inactive" \| "failed"` | required | `active` (green), `inactive` (gray) or `failed` (red, bold). |

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | Label text. |

## Examples

```vue
<StateBadge state="failed">Failed</StateBadge>
```

## Accessibility

- State is carried by the text label as well as color; always pass a label.

## Tokens

`--danger-accent`, `--neutral-strong`, `--space-2`, `--success-strong`, `--success-text-badge`, `--text-muted`

## Gotchas

- `failed` is the only state that is also bold, so the state a viewer must notice first stands out even without color.
- Map every non-good state to `failed` or `inactive`; there is no fourth state.

## Migration

Before:

```vue
<span :class="['state-badge', service.active_state === 'failed' ? 'failed' : 'active']">{{ label }}</span>
```

After:

```vue
<StateBadge :state="service.active_state === 'failed' ? 'failed' : 'active'">{{ label }}</StateBadge>
```

- The `row-failed` row highlight is separate and is not part of this component.

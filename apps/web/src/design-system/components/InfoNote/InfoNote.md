---
name: InfoNote
components: [InfoNote]
category: feedback
replaces: [.inline-note]
related: [NoticeBanner]
---

# InfoNote

Low-emphasis supplementary text below a form or table.

## When to use

- Context that was always true, such as which config file controls a list.

## When not to use

- Reporting a result or error: NoticeBanner.

## API

### InfoNote

**Props**

None.

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | Note content. `code` elements inside are tinted. |

## Examples

```vue
<InfoNote>Changes are available only for services listed in <code>/etc/deckox/agent.toml</code>.</InfoNote>
```

## Accessibility

- Renders an `aside` with no live-region role, because it does not report a state change.

## Tokens

`--font-xs`, `--info-bg`, `--info-border`, `--info-code-text`, `--info-text`, `--radius-sm`, `--space-3`

## Gotchas

- It has a 12px top margin and no bottom margin.

## Migration

Before:

```vue
<aside class="inline-note">{{ t("services.allowlist") }}</aside>
```

After:

```vue
<InfoNote>{{ t("services.allowlist") }}</InfoNote>
```

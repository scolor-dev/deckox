---
name: LogList
components: [LogList, LogEntry]
category: data
replaces: [.log-list, .log-entry]
related: [AppModal, StateBadge]
---

# LogList

Scrolling list of journal lines. LogEntry is one line.

## When to use

- The body of a service log viewer.

## When not to use

- Structured tabular data: TablePanel.

## API

### LogList

**Props**

None.

**Events**

None.

**Slots**

| Name | Description |
|---|---|
| `default` | LogEntry elements. |

### LogEntry

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `priority` | `"error" \| "warning" \| "info"` | required | `error` and `warning` get a colored left edge; `info` stays neutral. |
| `priorityLabel` | `string` | required | Translated priority text shown in the meta line. |
| `timestamp` | `string` | required | Already-formatted timestamp text. |
| `isoTimestamp` | `string \| null` | `null` | Machine-readable time for the `datetime` attribute. |
| `process` | `string \| null` | `null` | Process name. |
| `pid` | `number \| null` | `null` | Process id, shown as `process[pid]` when a process is given. |
| `message` | `string` | required | The log message. |

**Events**

None.

**Slots**

None.

## Examples

```vue
<LogList>
  <LogEntry priority="error" priority-label="Error" timestamp="09/18 04:12:03" process="docker" :pid="1042" message="failed to start containerd" />
</LogList>
```

## Accessibility

- An ordered list (`ol` with `li` items); the time is a `time` element.

## Tokens

`--border-subtle`, `--danger-accent`, `--font-2xs`, `--log-accent-neutral`, `--space-1`, `--space-2`, `--space-3`, `--space-4`, `--surface-elevated`, `--surface-muted`, `--text-muted`, `--text-primary`, `--warning-strong`

## Gotchas

- The component owns layout only. Translate labels and format timestamps before passing them.
- The message keeps its whitespace (`pre-wrap`) and breaks long tokens.

## Migration

Before:

```vue
<ol class="log-list">
  <li :class="['log-entry', priorityClass(entry.priority)]">
    <div class="log-meta"><time :datetime="iso">{{ formatted }}</time><span>{{ priorityLabel }}</span></div>
    <pre>{{ entry.message }}</pre>
  </li>
</ol>
```

After:

```vue
<LogList>
  <LogEntry :priority="priorityClass(entry.priority)" :priority-label="priorityLabel" :timestamp="formatted" :iso-timestamp="iso" :process="entry.process" :pid="entry.pid" :message="entry.message" />
</LogList>
```

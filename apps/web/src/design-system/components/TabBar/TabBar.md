---
name: TabBar
components: [TabBar]
category: navigation
replaces: [.settings-tabs, .settings-tab]
related: [AppPagination]
---

# TabBar

Tab strip. Panels are rendered by the caller.

## When to use

- Splitting one screen into a few sections (Settings: display, security, webhook, system).

## When not to use

- Navigating between pages: use the sidebar or links.

## API

### TabBar

**Props**

| Name | Type | Default | Description |
|---|---|---|---|
| `tabs` | `{ key: string; label: string }[]` | required | Tabs as `{ key, label }` pairs, in display order. |
| `modelValue` | `string` | required | Key of the active tab. Use with `v-model`. |
| `label` | `string` | required | Accessible name of the tab list, for example "Settings category". |

**Events**

| Name | Payload | Description |
|---|---|---|
| `update:modelValue` | `[key: string]` | Key of the tab that was clicked. |

**Slots**

None.

## Examples

```vue
<TabBar v-model="tab" label="Settings category" :tabs="[{ key: 'display', label: 'Display' }, { key: 'security', label: 'Security' }]" />
<div v-show="tab === 'display'" role="tabpanel">...</div>
```

## Accessibility

- `role="tablist"` and `role="tab"` with `aria-selected`.
- Each panel you render should have `role="tabpanel"`.
- Roving tabindex: only the selected tab is in the Tab order. Left/Right Arrow move to the previous/next tab with wrap-around, Home/End jump to the first/last tab, and moving selects the tab immediately.
- There is no `aria-controls` link between tab and panel.

## Tokens

`--border-default`, `--brand-primary`, `--font-md`, `--link`, `--space-0-5`, `--space-2`, `--space-4`, `--surface-hover`, `--text-muted`, `--text-primary`

## Gotchas

- It draws only the strip, so panels can be always mounted (`v-show`) or conditionally mounted (`v-if`) as each screen needs.

## Migration

Before:

```vue
<div class="settings-tabs" role="tablist" :aria-label="t('settings.tabsLabel')">
  <button v-for="tab in TAB_KEYS" :key="tab" type="button" role="tab" :class="['settings-tab', { active: activeTab === tab }]" :aria-selected="activeTab === tab" @click="selectTab(tab)">
    {{ t(`settings.tab.${tab}`) }}
  </button>
</div>
```

After:

```vue
<TabBar :model-value="activeTab" :label="t('settings.tabsLabel')" :tabs="tabs" @update:model-value="selectTab" />
```

- Build `tabs` as `{ key, label }` from the translated labels.

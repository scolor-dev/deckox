# Deckox design system

Tokens, components and a dev-only catalog. Nothing here is wired into the real screens yet; the app still uses `src/style.css` and its own views.

## Rules

- Import components from the barrel: `import { AppButton } from "../design-system/components"`. Components import their siblings directly, never the barrel.
- Component files contain no comments. Rationale, constraints and pitfalls live in the component's `.md`.
- Style with tokens only (`var(--...)` from `tokens.css`): no literal colors, spacing, font sizes, radii, shadows or durations. Values off the scale are snapped to it.
- Every component folder holds `<Name>.vue` and `<Name>.md`, and every component is exported from `components/index.ts`. Components that are always used together share one folder, named after the main one.
- Update the `.md` in the same change as the `.vue`. `docs.test.ts` fails when the API tables, tokens, exports or required sections drift from the source.
- A component that renders a bare `nav`, `h1`, `h2`, `p`, `table`, `th`, `td` or `button` inherits the app's global `style.css` rules for those tags. Reset what it must (AppBreadcrumb and AppPagination do).
- Breakpoints cannot be CSS variables; use the values in `breakpoints.ts` (`docs.test.ts` and `breakpoints.test.ts` enforce it).
- Preview in development at `/design-system.html` (`npm run dev`). It is not part of the production build.

## Which component do I use?

| I need to… | Use | Not |
|---|---|---|
| Show a result or error next to the thing it concerns | `NoticeBanner` | `AppToast` |
| Show a message that floats over the page and goes away | `AppToast` in `ToastRegion` | `NoticeBanner` |
| Show permanent helper text under a form or table | `InfoNote` | `NoticeBanner` |
| Ask one yes/no question | `ConfirmDialog` | `AppModal` |
| Show content that needs room (log, form) | `AppModal` | `ConfirmDialog` |
| Put a few actions behind one button | `AppPopover` | `AppTooltip` |
| Explain an icon or terse button on hover | `AppTooltip` | `AppPopover` |
| Press something with a text label | `AppButton` | `AppIconButton` |
| Press something shown only as a glyph | `AppIconButton` | `AppButton` |
| Show a symbol next to text or inside a control | `AppIcon` | a text character such as `×` |
| Join related buttons into one control | `ButtonGroup` | separate `AppButton`s |
| Type one line of text | `TextField` | `TextAreaField` |
| Type several lines of text | `TextAreaField` | `TextField` |
| Choose one of many options | `SelectField` | `RadioGroup` |
| Choose one of a few visible options | `RadioGroup` | `SelectField` |
| Pick a file | `FileField` | `TextField` |
| Toggle an option saved with a form | `AppCheckbox` | `AppSwitch` |
| Toggle a setting that applies immediately | `AppSwitch` | `AppCheckbox` |
| Filter a list by category | `TagToggle` in `TagToggleGroup` | `AppCheckbox` |
| Label a resource by kind | `TagBadge` | `StateBadge` |
| Report a resource's condition | `StateBadge` | `TagBadge` |
| Show a removable value | `AppChip` | `TagBadge` |
| Wait on an action | `AppSpinner` | `AppSkeleton` |
| Reserve space for content that is loading | `AppSkeleton` | `AppSpinner` |
| Show one percentage | `ProgressBar` | `StorageAllocationBar` |
| Show shares of one whole | `StorageAllocationBar` | `ProgressBar` |
| Show a table | `TablePanel` | `AppCard` |
| Let users sort or select table rows | `AppDataTable` inside `TablePanel` | a hand-written `table` |
| Show label/value pairs | `DetailList` | `TablePanel` |
| Show one metric | `MetricCard` | `AppCard` |
| Show journal lines | `LogList` | `TablePanel` |
| Split a screen into sections | `TabBar` | sidebar links |
| Page through a long list | `AppPagination` | `AppButton` ("load more") |
| Show the user's place in a hierarchy | `AppBreadcrumb` | sidebar links |
| Open a screen | `PageHeader` | a plain heading |
| Group content in a bordered panel | `AppCard` | `TablePanel` |
| Separate two blocks | `AppDivider` | a `gap` between cards |
| Space siblings in a row or column | `AppStack` | ad-hoc `display: flex` |
| Flow cards into as many columns as fit | `AppGrid` | fixed-column CSS grids |
| Show an empty region | `AppEmptyState` | an empty table row |

## Components

| Component | Category | Summary |
|---|---|---|
| [`AppButton`](components/AppButton/AppButton.md) | actions | Text button. |
| [`AppIcon`](components/AppIcon/AppIcon.md) | actions | Inline SVG icon that inherits the surrounding text color. |
| [`AppIconButton`](components/AppIconButton/AppIconButton.md) | actions | Small icon-only button. |
| [`ButtonGroup`](components/ButtonGroup/ButtonGroup.md) | actions | Joins AppButtons into one segmented control. |
| [`AppCheckbox`](components/AppCheckbox/AppCheckbox.md) | forms | Labeled checkbox with optional help text. |
| [`AppSwitch`](components/AppSwitch/AppSwitch.md) | forms | On/off switch for settings that take effect immediately. |
| [`FileField`](components/FileField/FileField.md) | forms | Labeled native file picker. |
| [`RadioGroup`](components/RadioGroup/RadioGroup.md) | forms | Fieldset of radio buttons for choosing exactly one option. |
| [`SelectField`](components/SelectField/SelectField.md) | forms | Labeled dropdown for choosing one option from a list. |
| [`TextAreaField`](components/TextAreaField/TextAreaField.md) | forms | Labeled multi-line text input. |
| [`TextField`](components/TextField/TextField.md) | forms | Labeled single-line text input with optional help text. |
| [`AppEmptyState`](components/AppEmptyState/AppEmptyState.md) | feedback | Centered "nothing here yet" message, with room for a follow-up action. |
| [`AppSkeleton`](components/AppSkeleton/AppSkeleton.md) | feedback | Loading placeholder shaped like the content that is coming. |
| [`AppSpinner`](components/AppSpinner/AppSpinner.md) | feedback | Small spinning loading indicator. |
| [`AppToast`](components/AppToast/AppToast.md) | feedback | Transient message that floats over the page. |
| [`InfoNote`](components/InfoNote/InfoNote.md) | feedback | Low-emphasis supplementary text below a form or table. |
| [`NoticeBanner`](components/NoticeBanner/NoticeBanner.md) | feedback | Inline banner reporting the result of something that just happened. |
| [`ProgressBar`](components/ProgressBar/ProgressBar.md) | feedback | Single value against a track. |
| [`StateBadge`](components/StateBadge/StateBadge.md) | feedback | Colored dot plus text reporting a resource's condition. |
| [`AppModal`](components/AppModal/AppModal.md) | overlay | General dialog with a header, scrolling body and optional footer. |
| [`AppPopover`](components/AppPopover/AppPopover.md) | overlay | Floating panel anchored under a trigger, used as a dropdown menu. |
| [`AppTooltip`](components/AppTooltip/AppTooltip.md) | overlay | One-line label that appears on hover or keyboard focus. |
| [`ConfirmDialog`](components/ConfirmDialog/ConfirmDialog.md) | overlay | Small fixed-width dialog that asks one yes/no question. |
| [`AppBreadcrumb`](components/AppBreadcrumb/AppBreadcrumb.md) | navigation | Where the user is in a hierarchy. |
| [`AppPagination`](components/AppPagination/AppPagination.md) | navigation | Page number strip with previous and next buttons. |
| [`TabBar`](components/TabBar/TabBar.md) | navigation | Tab strip. |
| [`AppCard`](components/AppCard/AppCard.md) | layout | Bare bordered panel for arbitrary content. |
| [`AppDataTable`](components/AppDataTable/AppDataTable.md) | data | Table with sortable headers and optional row selection. |
| [`AppDivider`](components/AppDivider/AppDivider.md) | layout | Horizontal rule between blocks of content. |
| [`AppGrid`](components/AppGrid/AppGrid.md) | layout | Responsive grid that fits equal columns to the width. |
| [`PageHeader`](components/PageHeader/PageHeader.md) | layout | The title row every screen opens with: heading and subtitle left, screen-level actions right. |
| [`AppStack`](components/AppStack/AppStack.md) | layout | Flex row or column with a scale-based gap. |
| [`AppChip`](components/AppChip/AppChip.md) | data | Pill for a discrete value, optionally removable. |
| [`DetailList`](components/DetailList/DetailList.md) | data | Two-column label/value grid. |
| [`LogList`](components/LogList/LogList.md) | data | Scrolling list of journal lines. |
| [`MetricCard`](components/MetricCard/MetricCard.md) | data | Tile shell for a single metric: header, value, optional chart or bar, footer and warning. |
| [`StorageAllocationBar`](components/StorageAllocationBar/StorageAllocationBar.md) | data | Stacked multi-segment bar with a legend, for several shares of one whole. |
| [`TablePanel`](components/TablePanel/TablePanel.md) | data | Card and scroll shell around a table. |
| [`TagBadge`](components/TagBadge/TagBadge.md) | data | Small static pill classifying a resource by kind. |
| [`TagToggle`](components/TagToggle/TagToggle.md) | data | Checkbox pill for showing or hiding a category in a list. |

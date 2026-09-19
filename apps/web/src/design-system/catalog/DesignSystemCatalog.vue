<script setup lang="ts">
import { ref } from "vue";
import { preferences } from "../../preferences";
import { breakpoints } from "../breakpoints";
import AppButton from "../components/AppButton.vue";
import AppCard from "../components/AppCard.vue";
import AppCheckbox from "../components/AppCheckbox.vue";
import AppChip from "../components/AppChip.vue";
import AppDivider from "../components/AppDivider.vue";
import AppEmptyState from "../components/AppEmptyState.vue";
import AppIconButton from "../components/AppIconButton.vue";
import ConfirmDialog from "../components/ConfirmDialog.vue";
import DetailList from "../components/DetailList.vue";
import DetailRow from "../components/DetailRow.vue";
import InfoNote from "../components/InfoNote.vue";
import MetricCard from "../components/MetricCard.vue";
import AppModal from "../components/AppModal.vue";
import NoticeBanner from "../components/NoticeBanner.vue";
import AppPopover from "../components/AppPopover.vue";
import AppPopoverItem from "../components/AppPopoverItem.vue";
import ProgressBar from "../components/ProgressBar.vue";
import RadioGroup from "../components/RadioGroup.vue";
import SelectField from "../components/SelectField.vue";
import AppSpinner from "../components/AppSpinner.vue";
import StateBadge from "../components/StateBadge.vue";
import AppSwitch from "../components/AppSwitch.vue";
import TabBar from "../components/TabBar.vue";
import TableToolbar from "../components/TableToolbar.vue";
import TablePanel from "../components/TablePanel.vue";
import TagBadge from "../components/TagBadge.vue";
import TagToggle from "../components/TagToggle.vue";
import TagToggleGroup from "../components/TagToggleGroup.vue";
import TextField from "../components/TextField.vue";
import AppTooltip from "../components/AppTooltip.vue";
import AppToast from "../components/AppToast.vue";
import LogEntry from "../components/LogEntry.vue";
import LogList from "../components/LogList.vue";
import StorageAllocationBar from "../components/StorageAllocationBar.vue";
import ToastRegion from "../components/ToastRegion.vue";
import AppBreadcrumb from "../components/AppBreadcrumb.vue";
import AppPagination from "../components/AppPagination.vue";
import AppSkeleton from "../components/AppSkeleton.vue";
import ButtonGroup from "../components/ButtonGroup.vue";
import FileField from "../components/FileField.vue";
import PageHeader from "../components/PageHeader.vue";
import TextAreaField from "../components/TextAreaField.vue";

const dialogOpen = ref(false);
const modalOpen = ref(false);
const popoverOpen = ref(false);
const chips = ref(["docker.io", "nginx", "git"]);
const activeTab = ref("first");
const tabs = [
  { key: "first", label: "One" },
  { key: "second", label: "Two" },
  { key: "third", label: "Three" },
];

const tagState = ref({ standard: true, deckox: true, product: false });

let toastSeq = 0;
const toasts = ref<{ id: number; tone: "success" | "warning" | "error"; text: string }[]>([]);
function pushToast(tone: "success" | "warning" | "error", text: string) {
  toasts.value.push({ id: (toastSeq += 1), tone, text });
}

const allocation = [
  { key: "root", label: "/", percent: 46, color: "#2a78d6", value: "46%" },
  { key: "home", label: "/home", percent: 22, color: "#eb6834", value: "22%" },
  { key: "var", label: "/var", percent: 12, color: "#1baf7a", value: "12%" },
  { key: "free", label: "Free", percent: 20, color: "var(--border-strong)", value: "20%" },
];

const textValue = ref("docker.io");
const textAreaValue = ref("Restart docker.service every night at 03:00.");
const currentPage = ref(3);
const pickedFile = ref("(none)");
const breadcrumb = [
  { label: "Settings", href: "#" },
  { label: "Security", href: "#" },
  { label: "Two-factor authentication" },
];
const shadowTokens = ["shadow-thumb", "shadow-raised", "shadow-overlay", "shadow-focus"];
const durationTokens = ["duration-fast", "duration-base", "duration-slow"];
const selectValue = ref("auto");
const checkboxValue = ref(true);
const switchValue = ref(true);
const radioValue = ref("auto");
const themeOptions = [
  { value: "auto", label: "Use device setting" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

const spaceTokens = ["space-0-5", "space-1", "space-2", "space-3", "space-4", "space-5", "space-6", "space-8", "space-10", "space-12", "space-16"];
const radiusTokens = ["radius-sm", "radius-md", "radius-badge", "radius-pill"];
const fontTokens = ["font-2xs", "font-xs", "font-sm", "font-md", "font-lg", "font-xl", "font-2xl", "font-3xl"];
const zTokens = ["z-sticky-column", "z-popover", "z-toast", "z-overlay"];

const colorGroups: { name: string; tokens: string[] }[] = [
  { name: "surface", tokens: ["surface-page", "surface-elevated", "surface-subtle", "surface-muted", "surface-hover"] },
  { name: "border", tokens: ["border-default", "border-strong", "border-subtle", "border-faint", "border-track", "border-hover"] },
  { name: "text", tokens: ["text-base", "text-primary", "text-secondary", "text-strong", "text-label", "text-muted", "text-faint", "text-faint-alt"] },
  { name: "brand", tokens: ["brand-primary", "brand-primary-hover", "brand-focus"] },
  { name: "danger", tokens: ["danger-strong", "danger-accent", "danger-text", "danger-bg", "danger-border"] },
  { name: "success", tokens: ["success-strong", "success-text", "success-bg", "success-border"] },
  { name: "warning", tokens: ["warning-strong", "warning-text", "warning-bg", "warning-border"] },
  { name: "tag: standard", tokens: ["tag-standard-bg", "tag-standard-text", "tag-standard-border"] },
  { name: "tag: deckox", tokens: ["tag-deckox-bg", "tag-deckox-text", "tag-deckox-border"] },
  { name: "tag: product", tokens: ["tag-product-bg", "tag-product-text", "tag-product-border"] },
];
</script>

<template>
  <div class="catalog">
    <header class="catalog-header">
      <div>
        <h1>Deckox Design System</h1>
        <p>Dev-only component catalog — not part of the production build.</p>
      </div>
      <label class="theme-picker">
        <span>Theme</span>
        <select v-model="preferences.theme">
          <option value="auto">
            Auto
          </option>
          <option value="light">
            Light
          </option>
          <option value="dark">
            Dark
          </option>
        </select>
      </label>
    </header>

    <section class="catalog-section">
      <h2>Tokens</h2>
      <div
        v-for="group in colorGroups"
        :key="group.name"
        class="token-group"
      >
        <h3>{{ group.name }}</h3>
        <div class="token-swatches">
          <div
            v-for="token in group.tokens"
            :key="token"
            class="token-swatch"
          >
            <span
              class="swatch"
              :style="{ background: `var(--${token})` }"
            />
            <code>--{{ token }}</code>
          </div>
        </div>
      </div>
    </section>

    <section class="catalog-section">
      <h2>Scale tokens</h2>
      <div class="token-group">
        <h3>spacing</h3>
        <div class="token-swatches">
          <div
            v-for="token in spaceTokens"
            :key="token"
            class="token-swatch"
          >
            <span
              class="space-bar"
              :style="{ width: `var(--${token})` }"
            />
            <code>--{{ token }}</code>
          </div>
        </div>
      </div>
      <div class="token-group">
        <h3>radius</h3>
        <div class="token-swatches">
          <div
            v-for="token in radiusTokens"
            :key="token"
            class="token-swatch"
          >
            <span
              class="radius-box"
              :style="{ borderRadius: `var(--${token})` }"
            />
            <code>--{{ token }}</code>
          </div>
        </div>
      </div>
      <div class="token-group">
        <h3>font size</h3>
        <div class="token-swatches">
          <div
            v-for="token in fontTokens"
            :key="token"
            class="token-swatch"
          >
            <span :style="{ fontSize: `var(--${token})` }">Aa</span>
            <code>--{{ token }}</code>
          </div>
        </div>
      </div>
      <div class="token-group">
        <h3>shadow</h3>
        <div class="token-swatches">
          <div
            v-for="token in shadowTokens"
            :key="token"
            class="token-swatch"
          >
            <span
              class="radius-box"
              :style="{ boxShadow: `var(--${token})` }"
            />
            <code>--{{ token }}</code>
          </div>
        </div>
      </div>
      <div class="token-group">
        <h3>motion (hover the boxes)</h3>
        <div class="token-swatches">
          <div
            v-for="token in durationTokens"
            :key="token"
            class="token-swatch"
          >
            <span
              class="motion-box"
              :style="{ transition: `transform var(--${token}) var(--ease-standard)` }"
            />
            <code>--{{ token }}</code>
          </div>
        </div>
      </div>
      <div class="token-group">
        <h3>breakpoint (a CSS variable can't be used in @media, so this lives in breakpoints.ts)</h3>
        <div class="token-swatches">
          <div
            v-for="(value, name) in breakpoints"
            :key="name"
            class="token-swatch"
          >
            <code>{{ name }} = {{ value }}px</code>
          </div>
        </div>
      </div>
      <div class="token-group">
        <h3>z-index (stacking order)</h3>
        <div class="token-swatches">
          <div
            v-for="token in zTokens"
            :key="token"
            class="token-swatch"
          >
            <code>--{{ token }}</code>
          </div>
        </div>
      </div>
    </section>

    <section class="catalog-section">
      <h2>AppButton</h2>
      <div class="row">
        <AppButton>Default</AppButton>
        <AppButton danger>
          Default danger
        </AppButton>
        <AppButton variant="primary">
          Primary
        </AppButton>
        <AppButton
          variant="primary"
          danger
        >
          Primary danger
        </AppButton>
        <AppButton variant="action">
          Action
        </AppButton>
        <AppButton
          variant="action"
          danger
        >
          Action danger
        </AppButton>
        <AppButton variant="logout">
          Log out
        </AppButton>
        <AppButton disabled>
          Disabled
        </AppButton>
      </div>
    </section>

    <section class="catalog-section">
      <h2>StateBadge</h2>
      <div class="row">
        <StateBadge state="active">
          Running
        </StateBadge>
        <StateBadge state="inactive">
          Stopped
        </StateBadge>
        <StateBadge state="failed">
          Failed
        </StateBadge>
      </div>
    </section>

    <section class="catalog-section">
      <h2>NoticeBanner</h2>
      <NoticeBanner tone="error">
        Something went wrong.
      </NoticeBanner>
      <NoticeBanner tone="warning">
        This needs attention.
      </NoticeBanner>
      <NoticeBanner tone="success">
        Completed successfully.
      </NoticeBanner>
    </section>

    <section class="catalog-section">
      <h2>InfoNote</h2>
      <InfoNote>
        Changes are available only for services listed in <code>/etc/deckox/agent.toml</code>.
      </InfoNote>
    </section>

    <section class="catalog-section">
      <h2>TagBadge</h2>
      <div class="row">
        <TagBadge category="standard">
          Standard
        </TagBadge>
        <TagBadge category="deckox">
          Deckox
        </TagBadge>
        <TagBadge category="product">
          Docker
        </TagBadge>
      </div>
    </section>

    <section class="catalog-section">
      <h2>TagToggle / TagToggleGroup</h2>
      <TagToggleGroup label="Visible tags">
        <TagToggle
          category="standard"
          :checked="tagState.standard"
          @update:checked="tagState.standard = $event"
        >
          Standard
        </TagToggle>
        <TagToggle
          category="deckox"
          :checked="tagState.deckox"
          @update:checked="tagState.deckox = $event"
        >
          Deckox
        </TagToggle>
        <TagToggle
          category="product"
          :checked="tagState.product"
          @update:checked="tagState.product = $event"
        >
          Product
        </TagToggle>
      </TagToggleGroup>
    </section>

    <section class="catalog-section">
      <h2>TextField / SelectField</h2>
      <div class="row">
        <TextField
          id="catalog-text-field"
          v-model="textValue"
          label="Package name"
          help="Checked against this host's configured repositories."
        />
        <SelectField
          id="catalog-select-field"
          v-model="selectValue"
          label="Color theme"
          :options="themeOptions"
        />
      </div>
    </section>

    <section class="catalog-section">
      <h2>AppCheckbox / AppSwitch / RadioGroup</h2>
      <div class="row">
        <AppCheckbox
          v-model="checkboxValue"
          label="Live updates"
          help="Connects only while this page is visible."
        />
        <AppSwitch
          v-model="switchValue"
          label="Allow host restart"
        />
      </div>
      <RadioGroup
        v-model="radioValue"
        name="catalog-radio-example"
        label="Color theme"
        :options="themeOptions"
      />
    </section>

    <section class="catalog-section">
      <h2>TabBar</h2>
      <TabBar
        v-model="activeTab"
        label="Example tabs"
        :tabs="tabs"
      />
      <p>Active tab: {{ activeTab }}</p>
    </section>

    <section class="catalog-section">
      <h2>ConfirmDialog</h2>
      <AppButton
        variant="primary"
        @click="dialogOpen = true"
      >
        Open dialog
      </AppButton>
      <ConfirmDialog
        :open="dialogOpen"
        title="Remove docker.io?"
        @close="dialogOpen = false"
      >
        <p>Configuration and data are kept.</p>
        <template #actions>
          <AppButton @click="dialogOpen = false">
            Close
          </AppButton>
          <AppButton
            variant="primary"
            danger
            @click="dialogOpen = false"
          >
            Remove
          </AppButton>
        </template>
      </ConfirmDialog>
    </section>

    <section class="catalog-section">
      <h2>Modal</h2>
      <AppButton
        variant="primary"
        @click="modalOpen = true"
      >
        Open modal
      </AppButton>
      <AppModal
        :open="modalOpen"
        title="Service log: docker.service"
        size="large"
        @close="modalOpen = false"
      >
        <p>Larger, freely-scrolling content goes here (e.g. a log viewer).</p>
        <p>Unlike ConfirmDialog, Modal has its own close (×) button and no fixed narrow width.</p>
        <template #footer>
          <AppButton @click="modalOpen = false">
            Close
          </AppButton>
        </template>
      </AppModal>
    </section>

    <section class="catalog-section">
      <h2>Chip</h2>
      <div class="row">
        <AppChip
          v-for="(chip, index) in chips"
          :key="chip"
          removable
          @remove="chips.splice(index, 1)"
        >
          {{ chip }}
        </AppChip>
        <span
          v-if="chips.length === 0"
          class="hint"
        >(all removed — reload to reset)</span>
      </div>
    </section>

    <section class="catalog-section">
      <h2>Popover / PopoverItem</h2>
      <AppPopover
        :open="popoverOpen"
        align="start"
        @close="popoverOpen = false"
      >
        <template #trigger>
          <AppButton @click="popoverOpen = !popoverOpen">
            Actions ▾
          </AppButton>
        </template>
        <AppPopoverItem @click="popoverOpen = false">
          Restart
        </AppPopoverItem>
        <AppPopoverItem @click="popoverOpen = false">
          View logs
        </AppPopoverItem>
        <AppPopoverItem
          danger
          @click="popoverOpen = false"
        >
          Remove
        </AppPopoverItem>
      </AppPopover>
    </section>

    <section class="catalog-section">
      <h2>Tooltip</h2>
      <div class="row">
        <AppTooltip text="Restart this service">
          <AppButton variant="action">
            Restart
          </AppButton>
        </AppTooltip>
        <AppTooltip
          text="Shown below the trigger"
          placement="bottom"
        >
          <AppButton variant="action">
            Hover me (bottom)
          </AppButton>
        </AppTooltip>
      </div>
    </section>

    <section class="catalog-section">
      <h2>TablePanel / TableToolbar</h2>
      <TablePanel>
        <template #toolbar>
          <TableToolbar count="2 items">
            <template #search>
              <input
                type="search"
                placeholder="Search…"
              >
            </template>
          </TableToolbar>
        </template>
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>State</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>docker.service</td>
              <td>
                <StateBadge state="active">
                  Running
                </StateBadge>
              </td>
              <td>
                <AppButton variant="action">
                  Restart
                </AppButton>
              </td>
            </tr>
            <tr>
              <td>nginx.service</td>
              <td>
                <StateBadge state="failed">
                  Failed
                </StateBadge>
              </td>
              <td>
                <AppButton
                  variant="action"
                  danger
                >
                  Stop
                </AppButton>
              </td>
            </tr>
          </tbody>
        </table>
      </TablePanel>
    </section>

    <section class="catalog-section">
      <h2>AppCard</h2>
      <AppCard>
        <p>A bare bordered panel — TablePanel is this same shell plus a toolbar and table.</p>
      </AppCard>
    </section>

    <section class="catalog-section">
      <h2>DetailList / DetailRow</h2>
      <DetailList>
        <DetailRow term="Hostname">
          deckox-pi
        </DetailRow>
        <DetailRow term="OS">
          Debian 12
        </DetailRow>
        <DetailRow term="Kernel">
          6.1.0-rpi
        </DetailRow>
        <DetailRow term="Uptime">
          14d 3h
        </DetailRow>
      </DetailList>
    </section>

    <section class="catalog-section">
      <h2>ProgressBar / MetricCard</h2>
      <div class="row">
        <MetricCard
          label="Memory"
          meta="7.6 GiB total"
          value="41%"
          footer="3.1 GiB used"
        >
          <ProgressBar :value="41" />
        </MetricCard>
        <MetricCard
          label="Swap"
          meta="2.0 GiB total"
          value="86%"
          warning
          warning-text="Swap usage is high"
        >
          <ProgressBar
            :value="86"
            critical
          />
        </MetricCard>
      </div>
    </section>

    <section class="catalog-section">
      <h2>AppSpinner / AppIconButton</h2>
      <div class="row">
        <AppSpinner />
        <AppIconButton label="Close">
          ×
        </AppIconButton>
        <AppIconButton
          label="Remove"
          size="sm"
          danger
        >
          ×
        </AppIconButton>
      </div>
    </section>

    <section class="catalog-section">
      <h2>AppDivider / AppEmptyState</h2>
      <p>Content above</p>
      <AppDivider />
      <p>Content below</p>
      <AppEmptyState message="No schedules configured." />
    </section>
    <section class="catalog-section">
      <h2>LogList / LogEntry</h2>
      <LogList>
        <LogEntry
          priority="error"
          priority-label="Error"
          timestamp="09/18 04:12:03"
          process="docker"
          :pid="1042"
          message="failed to start containerd: connection refused"
        />
        <LogEntry
          priority="warning"
          priority-label="Warning"
          timestamp="09/18 04:12:04"
          process="docker"
          :pid="1042"
          message="Configured runtime differs from the default"
        />
        <LogEntry
          priority="info"
          priority-label="Info"
          timestamp="09/18 04:12:05"
          process="docker"
          :pid="1042"
          message="API listening on /run/docker.sock"
        />
      </LogList>
    </section>

    <section class="catalog-section">
      <h2>StorageAllocationBar</h2>
      <StorageAllocationBar
        label="Disk usage by mount"
        :segments="allocation"
      />
    </section>

    <section class="catalog-section">
      <h2>AppToast / ToastRegion</h2>
      <div class="row">
        <AppButton @click="pushToast('success', 'nginx.service restarted.')">
          Success toast
        </AppButton>
        <AppButton @click="pushToast('warning', 'Swap usage is above 80%.')">
          Warning toast
        </AppButton>
        <AppButton @click="pushToast('error', 'Could not reach the Agent.')">
          Error toast
        </AppButton>
      </div>
      <ToastRegion>
        <AppToast
          v-for="toast in toasts"
          :key="toast.id"
          :tone="toast.tone"
          dismiss-label="Dismiss"
          @dismiss="toasts = toasts.filter((t) => t.id !== toast.id)"
        >
          {{ toast.text }}
        </AppToast>
      </ToastRegion>
    </section>
    <section class="catalog-section">
      <h2>TextAreaField / FileField</h2>
      <div class="row">
        <TextAreaField
          id="catalog-textarea"
          v-model="textAreaValue"
          label="Note"
          help="Shown next to the schedule."
        />
        <FileField
          id="catalog-file"
          label="Import config"
          accept=".toml"
          :help="`Selected: ${pickedFile}`"
          @select="pickedFile = $event?.name ?? '(none)'"
        />
      </div>
    </section>

    <section class="catalog-section">
      <h2>PageHeader / ButtonGroup</h2>
      <PageHeader
        title="Services"
        subtitle="3 of 5 services running"
      >
        <template #actions>
          <ButtonGroup label="Range">
            <AppButton>Day</AppButton>
            <AppButton>Week</AppButton>
            <AppButton>Month</AppButton>
          </ButtonGroup>
          <AppButton>Refresh</AppButton>
        </template>
      </PageHeader>
    </section>

    <section class="catalog-section">
      <h2>AppSkeleton</h2>
      <div
        class="row"
        aria-busy="true"
      >
        <AppSkeleton variant="circle" />
        <div class="skeleton-stack">
          <AppSkeleton width="60%" />
          <AppSkeleton />
          <AppSkeleton variant="block" />
        </div>
      </div>
    </section>

    <section class="catalog-section">
      <h2>AppPagination / AppBreadcrumb</h2>
      <AppPagination
        v-model:page="currentPage"
        :page-count="12"
        label="Audit log pages"
        prev-label="Previous page"
        next-label="Next page"
      />
      <p class="hint">
        Page {{ currentPage }} of 12
      </p>
      <AppBreadcrumb
        :items="breadcrumb"
        label="Breadcrumb"
      />
    </section>
  </div>
</template>

<style scoped>
.catalog {
  max-width: 1100px;
  margin: 0 auto;
  padding: 32px 24px 96px;
  color: var(--text-base);
  background: var(--surface-page);
  font-family: -apple-system, BlinkMacSystemFont, "Hiragino Sans", "Yu Gothic UI", "Yu Gothic", "Meiryo", "Segoe UI", sans-serif;
}
.catalog-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 32px;
}
.catalog-header h1 { margin: 0; color: var(--text-heading); }
.catalog-header p { margin: 4px 0 0; color: var(--text-muted); font-size: 13px; }
.theme-picker { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--text-label); }
.theme-picker select {
  padding: 6px 8px;
  border: 1px solid var(--border-strong);
  border-radius: 4px;
  color: var(--text-primary);
  background: var(--surface-elevated);
}
.catalog-section {
  margin-bottom: 40px;
  padding-bottom: 32px;
  border-bottom: 1px solid var(--border-default);
}
.catalog-section h2 { margin: 0 0 14px; color: var(--text-heading); font-size: 18px; }
.row { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; margin-bottom: 10px; }
.skeleton-stack { display: grid; flex: 1; min-width: 220px; gap: 8px; }
.hint { color: var(--text-faint); font-size: 12px; }
.token-group { margin-bottom: 16px; }
.token-group h3 { margin: 0 0 8px; color: var(--text-label); font-size: 11px; text-transform: uppercase; letter-spacing: .04em; }
.token-swatches { display: flex; flex-wrap: wrap; gap: 12px; }
.token-swatch { display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--text-secondary); }
.motion-box { display: inline-block; width: 20px; height: 20px; border-radius: 4px; background: var(--brand-primary); }
.motion-box:hover { transform: translateX(24px); }
.space-bar { display: inline-block; height: 10px; background: var(--brand-primary); }
.radius-box { display: inline-block; width: 28px; height: 28px; border: 1px solid var(--border-strong); background: var(--surface-elevated); }
.swatch { display: inline-block; width: 20px; height: 20px; border: 1px solid var(--border-default); border-radius: 4px; }
</style>

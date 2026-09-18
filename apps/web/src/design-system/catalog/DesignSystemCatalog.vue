<script setup lang="ts">
import { ref } from "vue";
import { preferences } from "../../preferences";
import AppButton from "../components/AppButton.vue";
import ConfirmDialog from "../components/ConfirmDialog.vue";
import InfoNote from "../components/InfoNote.vue";
import NoticeBanner from "../components/NoticeBanner.vue";
import StateBadge from "../components/StateBadge.vue";
import TabBar from "../components/TabBar.vue";
import TableToolbar from "../components/TableToolbar.vue";
import TablePanel from "../components/TablePanel.vue";
import TagBadge from "../components/TagBadge.vue";
import TagToggle from "../components/TagToggle.vue";
import TagToggleGroup from "../components/TagToggleGroup.vue";

const dialogOpen = ref(false);
const activeTab = ref("first");
const tabs = [
  { key: "first", label: "One" },
  { key: "second", label: "Two" },
  { key: "third", label: "Three" },
];

const tagState = ref({ standard: true, deckox: true, product: false });

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
.token-group { margin-bottom: 16px; }
.token-group h3 { margin: 0 0 8px; color: var(--text-label); font-size: 11px; text-transform: uppercase; letter-spacing: .04em; }
.token-swatches { display: flex; flex-wrap: wrap; gap: 12px; }
.token-swatch { display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--text-secondary); }
.swatch { display: inline-block; width: 20px; height: 20px; border: 1px solid var(--border-default); border-radius: 4px; }
</style>

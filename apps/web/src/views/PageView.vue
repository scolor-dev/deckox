<script setup lang="ts">
import { computed, onDeactivated, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import {
  AppButton,
  AppEmptyState,
  AppStack,
  ConfirmDialog,
  NoticeBanner,
  PageHeader,
} from "../design-system/components";
import {
  addPage,
  addWidget,
  moveWidget,
  moveWidgetTo,
  pageById,
  removePage,
  removeWidget,
  resizeWidget,
  sameLayout,
  setWidgetConfig,
  shownWidgets,
  updatePage,
} from "../layout/model";
import { navPages, pageTitle } from "../layout/pages";
import { layout, layoutSaving, newerServerLayout, resetLayout, saveLayout, useServerLayout } from "../layout/store";
import { widgetById } from "../modules/registry";
import { widgetAvailable } from "../modules/store";
import PageDialog from "../widgets/PageDialog.vue";
import type { PageDraft } from "../widgets/pageDraft";
import WidgetConfigDialog from "../widgets/WidgetConfigDialog.vue";
import WidgetPalette from "../widgets/WidgetPalette.vue";
import WidgetSlot from "../widgets/WidgetSlot.vue";
import type { ConfigValue, Layout, WidgetDefinition } from "../widgets/types";

const { t } = useI18n();
const route = useRoute();
const router = useRouter();

const editing = ref(false);
const draft = ref<Layout | null>(null);
const paletteOpen = ref(false);
const pageDialog = ref<"new" | "edit" | null>(null);
const configuring = ref<string | null>(null);
const confirmReset = ref(false);

const working = computed(() => draft.value ?? layout.value);
const pageId = computed(() => {
  const requested = route.params.pageId;
  return typeof requested === "string" ? requested : (navPages.value.at(0)?.id ?? "");
});
const page = computed(() => pageById(working.value, pageId.value));
const title = computed(() => (page.value ? pageTitle(page.value, t) : ""));
const screen = computed(() => page.value?.kind === "screen");
const pageLocked = computed(() => page.value?.widgets.some((widget) => widgetById(widget.widget)?.locked === true) ?? false);
const isLocked = (placementId: string) => {
  const placed = page.value?.widgets.find((widget) => widget.id === placementId);
  return placed !== undefined && widgetById(placed.widget)?.locked === true;
};

const placements = computed(() => {
  const current = page.value;
  if (!current) return [];
  return editing.value ? current.widgets : shownWidgets(current, widgetAvailable);
});
const configuringPlacement = computed(
  () => page.value?.widgets.find((widget) => widget.id === configuring.value) ?? null,
);
const pageDraft = computed<PageDraft | null>(() => {
  const current = page.value;
  return current ? { title: pageTitle(current, t), kind: current.kind, rows: current.rows } : null;
});
const gridStyle = computed(() => ({ "--screen-rows": String(page.value?.rows ?? 8) }));

function change(next: Layout) {
  draft.value = next;
}

function startEditing() {
  draft.value = JSON.parse(JSON.stringify(layout.value)) as Layout;
  editing.value = true;
}

function cancelEditing() {
  editing.value = false;
  draft.value = null;
}

async function finishEditing() {
  const next = draft.value;
  const stillThere = next !== null && pageById(next, pageId.value) !== undefined;
  editing.value = false;
  draft.value = null;
  if (next && !sameLayout(next, layout.value)) await saveLayout(next, t);
  if (!stillThere) await router.replace("/");
}

function add(definition: WidgetDefinition) {
  change(addWidget(working.value, pageId.value, definition));
  paletteOpen.value = false;
}

function savePage(values: PageDraft) {
  if (pageDialog.value === "new") {
    const created = addPage(working.value, values.title, values.kind);
    change(updatePage(created.layout, created.pageId, { rows: values.rows }));
    void router.push(`/${created.pageId}`);
  } else {
    change(updatePage(working.value, pageId.value, values));
  }
  pageDialog.value = null;
}

function deletePage() {
  if (pageLocked.value) return;
  change(removePage(working.value, pageId.value));
  pageDialog.value = null;
  void router.replace("/");
}

async function reset() {
  confirmReset.value = false;
  cancelEditing();
  await resetLayout(t);
  await router.replace("/");
}

onDeactivated(cancelEditing);
</script>

<template>
  <div :class="['view', 'widget-page', { 'widget-page--screen': screen }]">
    <PageHeader :title="title">
      <template #actions>
        <AppStack
          direction="row"
          gap="2"
          align="center"
          wrap
        >
          <template v-if="editing">
            <AppButton
              variant="action"
              @click="paletteOpen = true"
            >
              {{ t("layout.addWidget") }}
            </AppButton>
            <AppButton
              variant="action"
              @click="pageDialog = 'edit'"
            >
              {{ t("layout.pageSettings") }}
            </AppButton>
            <AppButton
              variant="action"
              @click="pageDialog = 'new'"
            >
              {{ t("layout.newPage") }}
            </AppButton>
            <AppButton
              variant="action"
              @click="confirmReset = true"
            >
              {{ t("layout.reset") }}
            </AppButton>
            <AppButton @click="cancelEditing">
              {{ t("common.cancel") }}
            </AppButton>
            <AppButton
              variant="primary"
              :disabled="layoutSaving"
              @click="finishEditing"
            >
              {{ t("layout.done") }}
            </AppButton>
          </template>
          <AppButton
            v-else
            @click="startEditing"
          >
            {{ t("layout.edit") }}
          </AppButton>
        </AppStack>
      </template>
    </PageHeader>

    <NoticeBanner
      v-if="newerServerLayout && !editing"
      tone="warning"
    >
      {{ t("layout.serverNewer") }}
      <AppButton
        variant="action"
        @click="useServerLayout"
      >
        {{ t("layout.useServer") }}
      </AppButton>
    </NoticeBanner>

    <div
      v-if="page"
      class="widget-grid-wrap"
    >
      <div
        :class="['widget-grid', screen ? 'widget-grid--screen' : 'widget-grid--scroll']"
        :style="gridStyle"
      >
        <WidgetSlot
          v-for="(placement, index) in placements"
          :key="placement.id"
          :placement="placement"
          :editing="editing"
          :screen="screen"
          :first="index === 0"
          :last="index === placements.length - 1"
          @move="!isLocked(placement.id) && change(moveWidget(working, pageId, placement.id, $event))"
          @reorder="!isLocked($event) && change(moveWidgetTo(working, pageId, $event, placement.id))"
          @remove="!isLocked(placement.id) && change(removeWidget(working, pageId, placement.id))"
          @configure="configuring = placement.id"
          @resize="!isLocked(placement.id) && change(resizeWidget(working, pageId, placement.id, $event, widgetById))"
        />
      </div>
      <AppEmptyState
        v-if="placements.length === 0"
        :message="t('layout.emptyPage')"
      >
        <AppButton
          v-if="!editing"
          variant="action"
          @click="startEditing(); paletteOpen = true"
        >
          {{ t("layout.addWidget") }}
        </AppButton>
      </AppEmptyState>
    </div>

    <WidgetPalette
      :open="paletteOpen"
      @close="paletteOpen = false"
      @add="add"
    />
    <WidgetConfigDialog
      :placement="configuringPlacement"
      @close="configuring = null"
      @change="(key: string, value: ConfigValue) => configuring && change(setWidgetConfig(working, pageId, configuring, key, value))"
    />
    <PageDialog
      :open="pageDialog !== null"
      :initial="pageDialog === 'edit' ? pageDraft : null"
      :can-delete="working.pages.length > 1 && !pageLocked"
      @close="pageDialog = null"
      @save="savePage"
      @delete="deletePage"
    />
    <ConfirmDialog
      :open="confirmReset"
      :title="t('layout.reset')"
      @close="confirmReset = false"
    >
      <p>{{ t("layout.resetConfirm") }}</p>
      <template #actions>
        <AppButton @click="confirmReset = false">
          {{ t("common.cancel") }}
        </AppButton>
        <AppButton
          variant="primary"
          danger
          @click="reset"
        >
          {{ t("layout.reset") }}
        </AppButton>
      </template>
    </ConfirmDialog>
  </div>
</template>

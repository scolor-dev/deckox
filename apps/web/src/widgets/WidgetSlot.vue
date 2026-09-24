<script setup lang="ts">
import { computed, onErrorCaptured, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  AppButton,
  AppCard,
  AppEmptyState,
  AppHeading,
  AppIcon,
  AppIconButton,
  AppStack,
  AppText,
  SelectField,
} from "../design-system/components";
import { spansFor } from "../layout/model";
import { widgetAvailable } from "../modules/store";
import { widgetById } from "../modules/registry";
import { componentOf } from "./asyncWidget";
import { MAX_ROWS, type WidgetPlacement } from "./types";

const props = defineProps<{
  placement: WidgetPlacement;
  editing: boolean;
  screen: boolean;
  first: boolean;
  last: boolean;
}>();

const emit = defineEmits<{
  move: [delta: number];
  remove: [];
  configure: [];
  resize: [size: { w: number; h: number | "auto" }];
  reorder: [fromId: string];
}>();

const { t } = useI18n();
const failed = ref(false);
const over = ref(false);
const DRAG_TYPE = "application/x-deckox-widget";

const definition = computed(() => widgetById(props.placement.widget));
const locked = computed(() => definition.value?.locked === true);
const available = computed(() => widgetAvailable(props.placement.widget));
const title = computed(() => {
  const found = definition.value;
  if (!found) return props.placement.widget;
  return found.titleFromConfig?.(props.placement.config) ?? t(found.titleKey);
});

const spans = computed(() => spansFor(props.placement.w));
const auto = computed(() => props.placement.h === "auto");
const style = computed(() => ({
  "--span-wide": String(spans.value.wide),
  "--span-medium": String(spans.value.medium),
  "--span-narrow": String(spans.value.narrow),
  "--rows": auto.value ? "1" : String(props.placement.h),
}));

const WIDTHS = [2, 3, 4, 6, 8, 12];
const widthOptions = computed(() => [...new Set([...WIDTHS, props.placement.w])]
  .sort((a, b) => a - b)
  .map((value) => ({ value: String(value), label: String(value) })));
const heightOptions = computed(() => {
  const rows = Array.from({ length: MAX_ROWS }, (_, index) => ({ value: String(index + 1), label: String(index + 1) }));
  return props.screen ? rows : [...rows, { value: "auto", label: t("layout.heightAuto") }];
});

function onWidth(value: string) {
  emit("resize", { w: Number(value), h: props.placement.h });
}

function onHeight(value: string) {
  emit("resize", { w: props.placement.w, h: value === "auto" ? "auto" : Number(value) });
}

function onDragStart(event: DragEvent) {
  if (locked.value) return;
  event.dataTransfer?.setData(DRAG_TYPE, props.placement.id);
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
}

function onDragOver(event: DragEvent) {
  if (!props.editing || !event.dataTransfer?.types.includes(DRAG_TYPE)) return;
  event.preventDefault();
  over.value = true;
}

function onDrop(event: DragEvent) {
  over.value = false;
  const from = event.dataTransfer?.getData(DRAG_TYPE);
  if (!props.editing || !from || from === props.placement.id) return;
  event.preventDefault();
  emit("reorder", from);
}

onErrorCaptured(() => {
  failed.value = true;
  return false;
});
</script>

<template>
  <div
    :class="['widget', {
      'widget--editing': editing,
      'widget--over': over,
      'widget--auto': auto && !screen,
      'widget--bare': definition?.chrome === 'bare',
    }]"
    :style="style"
    :data-widget="placement.widget"
    @dragover="onDragOver"
    @dragleave="over = false"
    @drop="onDrop"
  >
    <AppStack
      v-if="editing"
      :class="{ 'widget-drag': !locked }"
      direction="row"
      align="center"
      gap="1"
      wrap
      :draggable="!locked"
      @dragstart="onDragStart"
      @dragend="over = false"
    >
      <AppHeading
        level="3"
        size="sm"
        truncate
      >
        {{ title }}
      </AppHeading>
      <AppText
        v-if="locked"
        tone="muted"
        size="xs"
      >
        {{ t("layout.locked") }}
      </AppText>
      <template v-else>
        <AppIconButton
          v-if="!first"
          size="sm"
          :label="t('layout.moveEarlier')"
          @click="emit('move', -1)"
        >
          <AppIcon name="chevron-left" />
        </AppIconButton>
        <AppIconButton
          v-if="!last"
          size="sm"
          :label="t('layout.moveLater')"
          @click="emit('move', 1)"
        >
          <AppIcon name="chevron-right" />
        </AppIconButton>
        <AppText
          tone="muted"
          size="xs"
        >
          {{ t("layout.width") }}
        </AppText>
        <SelectField
          :id="`width-${placement.id}`"
          :model-value="String(placement.w)"
          :label="t('layout.width')"
          label-hidden
          compact
          :options="widthOptions"
          @update:model-value="onWidth"
        />
        <AppText
          tone="muted"
          size="xs"
        >
          {{ t("layout.height") }}
        </AppText>
        <SelectField
          :id="`height-${placement.id}`"
          :model-value="String(placement.h)"
          :label="t('layout.height')"
          label-hidden
          compact
          :options="heightOptions"
          @update:model-value="onHeight"
        />
        <AppButton
          v-if="definition?.config?.length"
          variant="action"
          @click="emit('configure')"
        >
          {{ t("layout.configure") }}
        </AppButton>
        <AppIconButton
          size="sm"
          danger
          :label="t('layout.removeWidget')"
          @click="emit('remove')"
        >
          <AppIcon name="trash" />
        </AppIconButton>
      </template>
    </AppStack>

    <AppEmptyState
      v-if="!definition"
      compact
      :message="t('layout.unknownWidget', { id: placement.widget })"
    />
    <AppEmptyState
      v-else-if="!available"
      compact
      :message="t('layout.moduleOff', { name: title })"
    />
    <AppEmptyState
      v-else-if="failed"
      compact
      role="alert"
      :message="t('layout.widgetFailed', { name: title })"
    />
    <div
      v-else
      class="widget-body"
    >
      <AppCard
        v-if="definition.chrome === 'frame'"
        :title="title"
      >
        <component
          :is="componentOf(definition)"
          :config="placement.config"
          :size="{ w: placement.w, h: placement.h }"
        />
      </AppCard>
      <component
        :is="componentOf(definition)"
        v-else
        :config="placement.config"
        :size="{ w: placement.w, h: placement.h }"
      />
    </div>
  </div>
</template>

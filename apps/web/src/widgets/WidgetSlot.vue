<script setup lang="ts">
import { computed, onErrorCaptured, ref } from "vue";
import { useI18n } from "vue-i18n";
import { AppButton, AppIcon, AppIconButton, SelectField } from "../design-system/components";
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
}>();

const { t } = useI18n();
const failed = ref(false);

const definition = computed(() => widgetById(props.placement.widget));
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

onErrorCaptured(() => {
  failed.value = true;
  return false;
});
</script>

<template>
  <div
    :class="['widget', {
      'widget--editing': editing,
      'widget--auto': auto && !screen,
      'widget--bare': definition?.chrome === 'bare',
    }]"
    :style="style"
    :data-widget="placement.widget"
  >
    <div
      v-if="editing"
      class="widget-edit-bar"
    >
      <strong class="widget-edit-title">{{ title }}</strong>
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
      <span class="widget-edit-label">{{ t("layout.width") }}</span>
      <SelectField
        :id="`width-${placement.id}`"
        :model-value="String(placement.w)"
        :label="t('layout.width')"
        label-hidden
        compact
        :options="widthOptions"
        @update:model-value="onWidth"
      />
      <span class="widget-edit-label">{{ t("layout.height") }}</span>
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
    </div>

    <div
      v-if="!definition"
      class="widget-message"
    >
      {{ t("layout.unknownWidget", { id: placement.widget }) }}
    </div>
    <div
      v-else-if="!available"
      class="widget-message"
    >
      {{ t("layout.moduleOff", { name: title }) }}
    </div>
    <div
      v-else-if="failed"
      class="widget-message"
      role="alert"
    >
      {{ t("layout.widgetFailed", { name: title }) }}
    </div>
    <template v-else>
      <section
        v-if="definition.chrome === 'frame'"
        class="widget-frame"
        :aria-label="title"
      >
        <h2 class="widget-frame-title">
          {{ title }}
        </h2>
        <div class="widget-body">
          <component
            :is="componentOf(definition)"
            :config="placement.config"
            :size="{ w: placement.w, h: placement.h }"
          />
        </div>
      </section>
      <div
        v-else
        class="widget-body"
      >
        <component
          :is="componentOf(definition)"
          :config="placement.config"
          :size="{ w: placement.w, h: placement.h }"
        />
      </div>
    </template>
  </div>
</template>

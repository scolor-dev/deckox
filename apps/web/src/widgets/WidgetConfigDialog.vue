<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { AppButton, AppModal, AppStack, SelectField, TextAreaField, TextField } from "../design-system/components";
import { widgetById } from "../modules/registry";
import type { ConfigValue, WidgetPlacement } from "./types";

const props = defineProps<{ placement: WidgetPlacement | null }>();
const emit = defineEmits<{ close: []; change: [key: string, value: ConfigValue] }>();

const { t } = useI18n();
const definition = computed(() => (props.placement ? widgetById(props.placement.widget) : undefined));
const fields = computed(() => (definition.value?.config ?? []).map((field) => ({
  field,
  options: field.kind === "select" ? field.options() : null,
})));

function valueOf(key: string, fallback: string) {
  const value = props.placement?.config[key];
  return typeof value === "string" ? value : fallback;
}
</script>

<template>
  <AppModal
    :open="placement !== null && definition !== undefined"
    :title="definition ? t(definition.titleKey) : ''"
    :close-label="t('common.close')"
    @close="emit('close')"
  >
    <AppStack gap="4">
      <template
        v-for="entry in fields"
        :key="entry.field.key"
      >
        <SelectField
          v-if="entry.field.kind === 'select' && entry.options"
          :id="`config-${entry.field.key}`"
          :model-value="valueOf(entry.field.key, entry.field.default)"
          :label="t(entry.field.labelKey)"
          :options="[...entry.options.value]"
          @update:model-value="emit('change', entry.field.key, $event)"
        />
        <TextAreaField
          v-else-if="entry.field.kind === 'text' && entry.field.multiline"
          :id="`config-${entry.field.key}`"
          :model-value="valueOf(entry.field.key, entry.field.default ?? '')"
          :label="t(entry.field.labelKey)"
          :rows="8"
          @update:model-value="emit('change', entry.field.key, $event)"
        />
        <TextField
          v-else-if="entry.field.kind === 'text'"
          :id="`config-${entry.field.key}`"
          :model-value="valueOf(entry.field.key, entry.field.default ?? '')"
          :label="t(entry.field.labelKey)"
          @update:model-value="emit('change', entry.field.key, $event)"
        />
      </template>
    </AppStack>
    <template #footer>
      <AppButton
        variant="primary"
        @click="emit('close')"
      >
        {{ t("layout.done") }}
      </AppButton>
    </template>
  </AppModal>
</template>

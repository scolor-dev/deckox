<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { AppButton, AppModal, AppStack } from "../design-system/components";
import { WEB_MODULES } from "../modules/registry";
import { backendEnabled } from "../modules/store";
import type { WidgetDefinition } from "./types";

defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: []; add: [definition: WidgetDefinition] }>();

const { t } = useI18n();

const groups = computed(() => WEB_MODULES
  .map((module) => ({
    id: module.id,
    title: t(module.titleKey),
    widgets: module.widgets.map((definition) => ({
      definition,
      on: backendEnabled(definition.requires),
      size: `${String(definition.size.default.w)} × ${definition.size.default.h === "auto" ? t("layout.heightAuto") : String(definition.size.default.h)}`,
    })),
  }))
  .filter((group) => group.widgets.length > 0));
</script>

<template>
  <AppModal
    :open="open"
    :title="t('layout.addWidget')"
    :close-label="t('common.close')"
    size="large"
    @close="emit('close')"
  >
    <AppStack gap="5">
      <section
        v-for="group in groups"
        :key="group.id"
        class="palette-group"
        :aria-label="group.title"
      >
        <h3>{{ group.title }}</h3>
        <ul class="palette-list">
          <li
            v-for="entry in group.widgets"
            :key="entry.definition.id"
            class="palette-item"
          >
            <div>
              <strong>{{ t(entry.definition.titleKey) }}</strong>
              <small>{{ t(entry.definition.descriptionKey) }}</small>
              <small>{{ t("layout.defaultSize", { size: entry.size }) }}</small>
            </div>
            <AppButton
              variant="action"
              :disabled="!entry.on"
              @click="emit('add', entry.definition)"
            >
              {{ entry.on ? t("layout.add") : t("layout.moduleOffShort") }}
            </AppButton>
          </li>
        </ul>
      </section>
    </AppStack>
  </AppModal>
</template>

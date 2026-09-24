<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { servicesList } from "../../../data/sources";
import { MetricCard } from "../../../design-system/components";
import type { WidgetConfig } from "../../../widgets/types";

const props = defineProps<{ config: WidgetConfig }>();

const { t } = useI18n();
const { data: services, loading } = servicesList.use();

const id = computed(() => (typeof props.config.service === "string" ? props.config.service.trim() : ""));
const service = computed(() => services.value?.find((entry) => entry.id === id.value) ?? null);
const value = computed(() => {
  if (id.value === "") return t("widgets.services.status.notSet");
  if (service.value) return service.value.active_state;
  return loading.value || services.value === null ? t("common.none") : t("widgets.services.status.notFound");
});
</script>

<template>
  <MetricCard
    :label="id === '' ? t('widgets.services.status.title') : id"
    :meta="service?.description"
    :value="value"
    :footer="service?.sub_state"
    :warning="service?.active_state === 'failed'"
    :warning-text="t('widgets.services.status.failed')"
  />
</template>

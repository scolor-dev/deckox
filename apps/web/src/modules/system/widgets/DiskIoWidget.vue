<script setup lang="ts">
import { computed } from "vue";
import MetricChart from "../../../components/MetricChart.vue";
import { formatBytes } from "../../../api/client";
import { MetricCard } from "../../../design-system/components";
import type { WidgetSize } from "../../../widgets/types";
import { useMetricWidget } from "./metric";

const props = defineProps<{ size: WidgetSize }>();
const { t, locale, metrics, history, roomForChart, limit } = useMetricWidget(() => props.size);
const rate = (value: number) => formatBytes(value, locale.value);
const perSecond = (value: number | null | undefined) =>
  value == null ? t("common.none") : t("overview.perSecond", { value: formatBytes(value, locale.value) });
const maximum = computed(() => Math.max(1, ...history.diskRead.value, ...history.diskWritten.value));
</script>

<template>
  <MetricCard
    :label="t('overview.diskIo')"
    :meta="t('overview.hostTotal')"
    :pairs="[
      { label: t('overview.read'), value: perSecond(metrics?.disk_io?.read_bytes_per_second) },
      { label: t('overview.write'), value: perSecond(metrics?.disk_io?.written_bytes_per_second) },
    ]"
    :footer="`${t('overview.read')} / ${t('overview.write')}`"
  >
    <MetricChart
      v-if="roomForChart"
      :values="history.diskRead.value"
      :secondary-values="history.diskWritten.value"
      :maximum="maximum"
      :label="t('overview.diskIoChart')"
      :limit="limit"
      :value-formatter="rate"
    />
  </MetricCard>
</template>

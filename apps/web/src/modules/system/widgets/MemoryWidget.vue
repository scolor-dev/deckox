<script setup lang="ts">
import { computed } from "vue";
import MetricChart from "../../../components/MetricChart.vue";
import { formatBytes, usagePercentage } from "../../../api/client";
import { MetricCard } from "../../../design-system/components";
import type { WidgetSize } from "../../../widgets/types";
import { useMetricWidget } from "./metric";

const props = defineProps<{ size: WidgetSize }>();
const { t, locale, metrics, history, roomForChart, limit } = useMetricWidget(() => props.size);
const percent = (value: number) => `${String(Math.round(value))}%`;
const memoryPercent = computed(() => {
  const memory = metrics.value?.memory;
  return memory ? usagePercentage(memory.used_bytes, memory.total_bytes) ?? 0 : 0;
});
</script>

<template>
  <MetricCard
    :label="t('overview.memory')"
    :meta="t('overview.total', { value: formatBytes(metrics?.memory.total_bytes ?? 0, locale) })"
    :value="metrics ? t('overview.inUse', { value: formatBytes(metrics.memory.used_bytes, locale) }) : t('common.none')"
    :footer="`${memoryPercent.toFixed(1)}%`"
  >
    <MetricChart
      v-if="roomForChart"
      :values="history.memory.value"
      :maximum="100"
      :label="t('overview.memoryChart')"
      :limit="limit"
      :value-formatter="percent"
    />
  </MetricCard>
</template>

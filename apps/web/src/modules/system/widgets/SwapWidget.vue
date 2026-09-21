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
const swapPercent = computed(() => {
  const memory = metrics.value?.memory;
  return memory ? usagePercentage(memory.swap_used_bytes, memory.swap_total_bytes) : null;
});
</script>

<template>
  <MetricCard
    label="Swap"
    :meta="metrics?.memory.swap_total_bytes ? t('overview.total', { value: formatBytes(metrics.memory.swap_total_bytes, locale) }) : t('overview.notConfigured')"
    :value="swapPercent === null ? t('common.none') : `${swapPercent.toFixed(1)}%`"
    :warning="swapPercent !== null && swapPercent >= 80"
    :warning-text="t('overview.highUsage')"
  >
    <MetricChart
      v-if="roomForChart"
      :values="history.swap.value"
      :maximum="100"
      :label="t('overview.swapChart')"
      :limit="limit"
      :value-formatter="percent"
    />
  </MetricCard>
</template>

<script setup lang="ts">
import MetricChart from "../../../components/MetricChart.vue";
import { MetricCard } from "../../../design-system/components";
import type { WidgetSize } from "../../../widgets/types";
import { useMetricWidget } from "./metric";

const props = defineProps<{ size: WidgetSize }>();
const { t, metrics, history, roomForChart, limit } = useMetricWidget(() => props.size);
const percent = (value: number) => `${String(Math.round(value))}%`;
</script>

<template>
  <MetricCard
    :label="t('overview.cpu')"
    :meta="t('overview.cores', { count: metrics?.cpu.logical_cores ?? t('common.none') })"
    :value="metrics ? `${metrics.cpu.usage_percent.toFixed(1)}%` : '—'"
  >
    <MetricChart
      v-if="roomForChart"
      :values="history.cpu.value"
      :maximum="100"
      :label="t('overview.cpuChart')"
      :limit="limit"
      :value-formatter="percent"
    />
  </MetricCard>
</template>

<script setup lang="ts">
import { computed } from "vue";
import MetricChart from "../../../components/MetricChart.vue";
import { MetricCard } from "../../../design-system/components";
import type { WidgetSize } from "../../../widgets/types";
import { useMetricWidget } from "./metric";

const props = defineProps<{ size: WidgetSize }>();
const { t, metrics, history, roomForChart, limit } = useMetricWidget(() => props.size);
const decimal = (value: number) => value.toFixed(1);
const maximum = computed(() => Math.max(metrics.value?.cpu.logical_cores ?? 1, 1));
</script>

<template>
  <MetricCard
    :label="t('overview.load')"
    :meta="t('overview.fiveMinutes', { value: metrics?.load_average.five_minutes.toFixed(2) ?? t('common.none') })"
    :value="metrics?.load_average.one_minute.toFixed(2) ?? '—'"
    :footer="t('overview.fifteenMinutes', { value: metrics?.load_average.fifteen_minutes.toFixed(2) ?? t('common.none') })"
  >
    <MetricChart
      v-if="roomForChart"
      :values="history.load.value"
      :maximum="maximum"
      :label="t('overview.loadChart')"
      :limit="limit"
      :value-formatter="decimal"
    />
  </MetricCard>
</template>

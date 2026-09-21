<script setup lang="ts">
import { computed } from "vue";
import MetricChart from "../../../components/MetricChart.vue";
import { MetricCard } from "../../../design-system/components";
import type { WidgetSize } from "../../../widgets/types";
import { useMetricWidget } from "./metric";

const props = defineProps<{ size: WidgetSize }>();
const { t, metrics, history, roomForChart, limit } = useMetricWidget(() => props.size);
const celsius = (value: number) => `${String(Math.round(value))}°C`;
const temperature = computed(() => metrics.value?.cpu.temperature_celsius ?? null);
</script>

<template>
  <MetricCard
    :label="t('overview.temperature')"
    meta="CPU"
    :value="temperature === null ? t('common.none') : `${temperature.toFixed(1)} °C`"
    :footer="temperature === null ? t('overview.notAvailable') : t('overview.sensorValue')"
  >
    <MetricChart
      v-if="roomForChart"
      :values="history.temperature.value"
      :maximum="100"
      :label="t('overview.temperatureChart')"
      :limit="limit"
      :value-formatter="celsius"
    />
  </MetricCard>
</template>

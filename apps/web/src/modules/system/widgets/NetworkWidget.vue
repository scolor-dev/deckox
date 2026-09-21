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
const maximum = computed(() => Math.max(1, ...history.networkReceived.value, ...history.networkTransmitted.value));
</script>

<template>
  <MetricCard
    :label="t('overview.network')"
    meta="RX / TX"
    :pairs="[
      { label: 'RX', value: perSecond(metrics?.network?.received_bytes_per_second) },
      { label: 'TX', value: perSecond(metrics?.network?.transmitted_bytes_per_second) },
    ]"
    :footer="`${t('overview.received')} / ${t('overview.transmitted')}`"
  >
    <MetricChart
      v-if="roomForChart"
      :values="history.networkReceived.value"
      :secondary-values="history.networkTransmitted.value"
      :maximum="maximum"
      :label="t('overview.networkChart')"
      :limit="limit"
      :value-formatter="rate"
    />
  </MetricCard>
</template>

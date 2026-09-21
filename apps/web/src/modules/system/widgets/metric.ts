import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { HISTORY_LIMIT, useMetrics } from "../../../data/metrics";
import type { WidgetSize } from "../../../widgets/types";

/** Shared by the metric widgets: the live numbers and whether room is left for a chart. */
export function useMetricWidget(size: () => WidgetSize) {
  const { t, locale } = useI18n();
  const data = useMetrics();
  const roomForChart = computed(() => {
    const { h } = size();
    return h === "auto" || h >= 3;
  });
  return { t, locale, ...data, roomForChart, limit: HISTORY_LIMIT };
}

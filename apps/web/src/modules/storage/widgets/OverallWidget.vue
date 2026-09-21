<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { formatBytes } from "../../../api/client";
import { MetricCard, ProgressBar } from "../../../design-system/components";
import { useStorageUsage } from "../usage";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const { mounts, totalCapacity, totalUsed, overallPercent, busiest } = useStorageUsage();

const footer = computed(() => {
  if (mounts.value.length === 0) return null;
  const detail = t("storage.overallUsageDetail", {
    used: formatBytes(totalUsed.value, locale.value),
    total: formatBytes(totalCapacity.value, locale.value),
  });
  const top = busiest.value;
  if (!top) return detail;
  return `${detail} · ${t("overview.busiestMount", { mount: top.mount_point, percent: top.usage_percent.toFixed(0) })}`;
});
</script>

<template>
  <MetricCard
    :label="t('storage.overallUsage')"
    :meta="t('storage.summary', { count: mounts.length })"
    :value="mounts.length > 0 ? `${overallPercent.toFixed(0)}%` : t('common.none')"
    :footer="footer"
    :warning="(busiest?.usage_percent ?? 0) >= 90"
    :warning-text="t('overview.highUsage')"
  >
    <ProgressBar
      v-if="mounts.length > 0"
      :value="overallPercent"
      :critical="(busiest?.usage_percent ?? 0) >= 90"
      :label="t('storage.overallUsage')"
    />
  </MetricCard>
</template>

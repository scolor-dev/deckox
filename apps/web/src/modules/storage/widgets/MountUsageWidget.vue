<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { formatBytes } from "../../../api/client";
import { MetricCard, ProgressBar } from "../../../design-system/components";
import type { WidgetConfig } from "../../../widgets/types";
import { useStorageUsage } from "../usage";

const props = defineProps<{ config: WidgetConfig }>();

const { t, locale } = useI18n();
const { mounts } = useStorageUsage();

const mount = computed(() => {
  const wanted = typeof props.config.mount === "string" ? props.config.mount : "/";
  return mounts.value.find((entry) => entry.mount_point === wanted) ?? null;
});
</script>

<template>
  <MetricCard
    :label="mount?.mount_point ?? t('storage.mount')"
    :meta="mount?.filesystem_type ?? null"
    :value="mount ? `${mount.usage_percent.toFixed(0)}%` : t('common.none')"
    :footer="mount ? `${t('storage.used', { value: formatBytes(mount.used_bytes, locale) })} / ${formatBytes(mount.total_bytes, locale)}` : t('widgets.storage.mountMissing')"
    :warning="(mount?.usage_percent ?? 0) >= 90"
    :warning-text="t('overview.highUsage')"
  >
    <ProgressBar
      v-if="mount"
      :value="mount.usage_percent"
      :critical="mount.usage_percent >= 90"
      :label="mount.mount_point"
    />
  </MetricCard>
</template>

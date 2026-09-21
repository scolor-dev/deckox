<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { formatBytes } from "../../../api/client";
import { AppCard, AppStack, ProgressBar, StorageAllocationBar } from "../../../design-system/components";
import { useStorageUsage } from "../usage";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const { mounts, counted, totalCapacity, totalUsed, overallPercent } = useStorageUsage();

// Fixed categorical order (never reassigned per filter), validated for
// colorblind-safe adjacent contrast as a stacked-bar set. The 7th slot is
// reserved for an "other" overflow bucket past six named mounts; free space
// uses a neutral gray outside this set rather than an eighth hue.
const ALLOCATION_COLORS = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300"];
const ALLOCATION_OTHER_COLOR = "#4a3aa7";
const ALLOCATION_MAX_SEGMENTS = ALLOCATION_COLORS.length;

interface AllocationSegment {
  key: string;
  label: string;
  usedBytes: number;
  percent: number;
  color: string;
}

const allocationSegments = computed<AllocationSegment[]>(() => {
  const capacity = totalCapacity.value;
  if (capacity <= 0) return [];

  // Which mounts get an individual segment is a function of usage (top by
  // used_bytes); which color a shown mount gets is not — it is keyed by
  // mount_point, a stable identity, so a mount keeps its color across
  // refreshes even when usage jitter reorders the top-N ranking.
  const byUsageDesc = [...counted.value].sort((a, b) => b.used_bytes - a.used_bytes);
  const shown = byUsageDesc
    .slice(0, ALLOCATION_MAX_SEGMENTS)
    .sort((a, b) => a.mount_point.localeCompare(b.mount_point));
  const overflow = byUsageDesc.slice(ALLOCATION_MAX_SEGMENTS);

  const segments = shown.map((mount, index) => ({
    key: `${mount.filesystem}:${mount.mount_point}`,
    label: mount.mount_point,
    usedBytes: mount.used_bytes,
    percent: (mount.used_bytes / capacity) * 100,
    color: ALLOCATION_COLORS[index],
  }));

  if (overflow.length > 0) {
    const overflowUsed = overflow.reduce((sum, mount) => sum + mount.used_bytes, 0);
    segments.push({
      key: "other",
      label: t("storage.other"),
      usedBytes: overflowUsed,
      percent: (overflowUsed / capacity) * 100,
      color: ALLOCATION_OTHER_COLOR,
    });
  }

  return segments;
});

const freeBytes = computed(() => Math.max(0, totalCapacity.value - totalUsed.value));
const freePercent = computed(() => {
  const capacity = totalCapacity.value;
  return capacity > 0 ? (freeBytes.value / capacity) * 100 : 0;
});

const barSegments = computed(() => [
  ...allocationSegments.value.map((segment) => ({
    key: segment.key,
    label: segment.label,
    percent: segment.percent,
    color: segment.color,
    value: `${segment.percent.toFixed(0)}%`,
  })),
  ...(freeBytes.value > 0
    ? [{ key: "free", label: t("storage.free"), percent: freePercent.value, color: "var(--border-strong)", value: `${freePercent.value.toFixed(0)}%` }]
    : []),
]);

</script>

<template>
  <AppCard v-if="mounts.length > 0">
    <AppStack gap="3">
      <AppStack
        direction="row"
        justify="between"
        align="center"
        gap="3"
        wrap
      >
        <AppStack
          direction="row"
          gap="3"
          align="center"
        >
          <span>{{ t("storage.overallUsage") }}</span>
          <strong class="storage-overall">{{ overallPercent.toFixed(0) }}%</strong>
        </AppStack>
        <span class="storage-summary-detail">{{ t("storage.overallUsageDetail", { used: formatBytes(totalUsed, locale), total: formatBytes(totalCapacity, locale) }) }}</span>
      </AppStack>
      <ProgressBar
        :value="overallPercent"
        :critical="overallPercent >= 90"
        :label="t('storage.overallUsage')"
      />
      <StorageAllocationBar
        :segments="barSegments"
        :label="t('storage.allocation')"
      />
    </AppStack>
  </AppCard>
</template>

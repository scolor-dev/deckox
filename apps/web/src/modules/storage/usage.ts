import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { apiErrorKey } from "../../api/errors";
import { capacityMounts, usagePercentage } from "../../api/client";
import { storageDisks, storageMounts } from "../../data/sources";

/** The mounts, and the totals the storage widgets have in common. */
export function useStorageUsage() {
  const { t } = useI18n();
  const source = storageMounts.use();
  const mounts = computed(() => source.data.value ?? []);
  const counted = computed(() => capacityMounts(mounts.value));
  const totalCapacity = computed(() => counted.value.reduce((sum, mount) => sum + mount.total_bytes, 0));
  const totalUsed = computed(() => counted.value.reduce((sum, mount) => sum + mount.used_bytes, 0));
  const overallPercent = computed(() => usagePercentage(totalUsed.value, totalCapacity.value) ?? 0);
  const busiest = computed(() => counted.value.length === 0
    ? null
    : counted.value.reduce((top, mount) => (mount.usage_percent > top.usage_percent ? mount : top)));
  const errorMessage = computed(() => (source.error.value ? t(apiErrorKey(source.error.value, "errors.storage")) : null));
  return {
    mounts,
    counted,
    totalCapacity,
    totalUsed,
    overallPercent,
    busiest,
    loading: source.loading,
    errorMessage,
    refresh: source.refresh,
  };
}

export function useStorageDisks() {
  const source = storageDisks.use();
  return { disks: computed(() => source.data.value ?? []), loading: source.loading, refresh: source.refresh };
}

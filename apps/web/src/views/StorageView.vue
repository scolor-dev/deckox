<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { api, capacityMounts, formatBytes, usagePercentage, type StorageDisk, type StorageMount } from "../api/client";
import DiskPanel from "../components/DiskPanel.vue";
import { apiErrorKey } from "../api/errors";
import {
  AppButton,
  AppCard,
  AppStack,
  NoticeBanner,
  PageHeader,
  ProgressBar,
  StorageAllocationBar,
  TabBar,
  TableToolbar,
  TablePanel,
  TagBadge,
  TagToggle,
  TagToggleGroup,
} from "../design-system/components";
import { preferences, type StorageTagFilterKey } from "../preferences";

const { t, locale } = useI18n();

const TAG_FILTER_KEYS: StorageTagFilterKey[] = ["standard", "other"];

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

const mounts = ref<StorageMount[]>([]);
const disks = ref<StorageDisk[]>([]);
const activeTab = ref("all");

const tabs = computed(() => [
  { key: "all", label: t("storage.tabAll") },
  ...disks.value.map((disk) => ({ key: disk.name, label: `${disk.name} · ${formatBytes(disk.size_bytes, locale.value)}` })),
]);
const activeDisk = computed(() => disks.value.find((disk) => disk.name === activeTab.value) ?? null);

watch(disks, (list) => {
  if (activeTab.value !== "all" && !list.some((disk) => disk.name === activeTab.value)) activeTab.value = "all";
});
const loading = ref(true);
const error = ref<string | null>(null);

const countedMounts = computed(() => capacityMounts(mounts.value));
const totalCapacity = computed(() => countedMounts.value.reduce((sum, mount) => sum + mount.total_bytes, 0));
const totalUsed = computed(() => countedMounts.value.reduce((sum, mount) => sum + mount.used_bytes, 0));
const overallPercent = computed(() => usagePercentage(totalUsed.value, totalCapacity.value) ?? 0);

const allocationSegments = computed<AllocationSegment[]>(() => {
  const capacity = totalCapacity.value;
  if (capacity <= 0) return [];

  // Which mounts get an individual segment is a function of usage (top by
  // used_bytes); which color a shown mount gets is not — it is keyed by
  // mount_point, a stable identity, so a mount keeps its color across
  // refreshes even when usage jitter reorders the top-N ranking.
  const byUsageDesc = [...countedMounts.value].sort((a, b) => b.used_bytes - a.used_bytes);
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

function mountTags(mount: StorageMount): StorageTagFilterKey[] {
  return mount.standard ? ["standard"] : [];
}

function tagLabel(tag: StorageTagFilterKey) {
  return tag === "standard" ? t("storage.tagStandard") : t("storage.tagOther");
}

function filterKeys(tags: StorageTagFilterKey[]): StorageTagFilterKey[] {
  return tags.length ? tags : ["other"];
}

function isTagHidden(tag: StorageTagFilterKey) {
  return preferences.hiddenStorageTags.includes(tag);
}

function toggleTag(tag: StorageTagFilterKey) {
  preferences.hiddenStorageTags = isTagHidden(tag)
    ? preferences.hiddenStorageTags.filter((hidden) => hidden !== tag)
    : [...preferences.hiddenStorageTags, tag];
}

const filteredMounts = computed(() => mounts.value.filter((mount) => {
  const keys = filterKeys(mountTags(mount));
  return !keys.every((key) => isTagHidden(key));
}));

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    const [mountList, diskList] = await Promise.all([api.storage(), api.storageDisks().catch(() => [])]);
    mounts.value = mountList;
    disks.value = diskList;
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.storage"));
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);
</script>

<template>
  <div class="view">
    <PageHeader
      :title="t('storage.title')"
      :subtitle="t('storage.summary', { count: mounts.length })"
    >
      <template #actions>
        <AppButton
          :disabled="loading"
          @click="refresh"
        >
          {{ loading ? t("common.loading") : t("common.refresh") }}
        </AppButton>
      </template>
    </PageHeader>

    <NoticeBanner
      v-if="error"
      tone="error"
    >
      {{ error }}
    </NoticeBanner>

    <TabBar
      v-if="disks.length > 0"
      v-model="activeTab"
      :tabs="tabs"
      :label="t('storage.tabsLabel')"
    />

    <DiskPanel
      v-if="activeDisk"
      :disk="activeDisk"
    />

    <template v-else>
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

      <TablePanel
        :loading="loading && mounts.length === 0"
        :empty="filteredMounts.length === 0"
        :empty-message="t('storage.empty')"
      >
        <template
          v-if="mounts.length > 0"
          #toolbar
        >
          <TableToolbar :count="t('storage.count', { count: filteredMounts.length })">
            <template #filters>
              <TagToggleGroup :label="t('storage.tagVisibility')">
                <TagToggle
                  v-for="tag in TAG_FILTER_KEYS"
                  :key="tag"
                  :category="tag === 'standard' ? 'standard' : 'other'"
                  :checked="!isTagHidden(tag)"
                  @update:checked="toggleTag(tag)"
                >
                  {{ tagLabel(tag) }}
                </TagToggle>
              </TagToggleGroup>
            </template>
          </TableToolbar>
        </template>
        <template #loading>
          {{ t("storage.loading") }}
        </template>
        <table class="storage-table">
          <thead>
            <tr><th>{{ t("storage.mount") }}</th><th>{{ t("storage.filesystem") }}</th><th>{{ t("storage.capacity") }}</th><th>{{ t("storage.usage") }}</th></tr>
          </thead>
          <tbody>
            <tr
              v-for="mount in filteredMounts"
              :key="`${mount.filesystem}:${mount.mount_point}`"
            >
              <td class="path-cell">
                <AppStack
                  direction="row"
                  gap="2"
                  align="center"
                  wrap
                >
                  <strong
                    class="storage-path"
                    :title="mount.mount_point"
                  >{{ mount.mount_point }}</strong>
                  <TagBadge
                    v-for="tag in mountTags(mount)"
                    :key="tag"
                    :category="tag === 'standard' ? 'standard' : 'deckox'"
                  >
                    {{ tagLabel(tag) }}
                  </TagBadge>
                </AppStack>
              </td>
              <td class="filesystem-cell">
                <span :title="mount.filesystem">{{ mount.filesystem }}</span>
                <small>{{ mount.filesystem_type }}</small>
              </td>
              <td class="capacity-cell">
                <strong>{{ formatBytes(mount.total_bytes, locale) }}</strong>
                <small>{{ t("storage.available", { value: formatBytes(mount.available_bytes, locale) }) }}</small>
              </td>
              <td class="storage-usage-cell">
                <AppStack gap="1">
                  <span class="usage-row">
                    <span>{{ mount.usage_percent.toFixed(0) }}%</span>
                    <small>{{ t("storage.used", { value: formatBytes(mount.used_bytes, locale) }) }}</small>
                  </span>
                  <ProgressBar
                    :value="mount.usage_percent"
                    :critical="mount.usage_percent >= 90"
                    :label="mount.mount_point"
                  />
                </AppStack>
              </td>
            </tr>
          </tbody>
        </table>
      </TablePanel>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, formatBytes, usagePercentage, type StorageMount } from "../api/client";
import { apiErrorKey } from "../api/errors";
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
const loading = ref(true);
const error = ref<string | null>(null);

const totalCapacity = computed(() => mounts.value.reduce((sum, mount) => sum + mount.total_bytes, 0));
const totalUsed = computed(() => mounts.value.reduce((sum, mount) => sum + mount.used_bytes, 0));
const overallPercent = computed(() => usagePercentage(totalUsed.value, totalCapacity.value) ?? 0);

const allocationSegments = computed<AllocationSegment[]>(() => {
  const capacity = totalCapacity.value;
  if (capacity <= 0) return [];

  // Which mounts get an individual segment is a function of usage (top by
  // used_bytes); which color a shown mount gets is not — it is keyed by
  // mount_point, a stable identity, so a mount keeps its color across
  // refreshes even when usage jitter reorders the top-N ranking.
  const byUsageDesc = [...mounts.value].sort((a, b) => b.used_bytes - a.used_bytes);
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
    mounts.value = await api.storage();
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
    <header class="view-header">
      <div>
        <h1>{{ t("storage.title") }}</h1>
        <p class="subtitle">
          {{ t("storage.summary", { count: mounts.length }) }}
        </p>
      </div>
      <button
        class="button"
        type="button"
        :disabled="loading"
        @click="refresh"
      >
        {{ loading ? t("common.loading") : t("common.refresh") }}
      </button>
    </header>

    <div
      v-if="error"
      class="notice error"
    >
      {{ error }}
    </div>

    <section
      v-if="mounts.length > 0"
      class="storage-summary"
      aria-labelledby="storage-summary-heading"
    >
      <div class="storage-summary-head">
        <div>
          <span id="storage-summary-heading">{{ t("storage.overallUsage") }}</span>
          <strong>{{ overallPercent.toFixed(0) }}%</strong>
        </div>
        <span class="storage-summary-detail">{{ t("storage.overallUsageDetail", { used: formatBytes(totalUsed, locale), total: formatBytes(totalCapacity, locale) }) }}</span>
      </div>
      <div class="progress storage-summary-bar">
        <span
          :class="{ critical: overallPercent >= 90 }"
          :style="{ width: `${overallPercent}%` }"
        />
      </div>
      <div
        class="storage-allocation-bar"
        role="img"
        :aria-label="t('storage.allocation')"
      >
        <span
          v-for="segment in allocationSegments"
          :key="segment.key"
          :style="{ width: `${segment.percent}%`, background: segment.color }"
          :title="`${segment.label}: ${formatBytes(segment.usedBytes, locale)} (${segment.percent.toFixed(1)}%)`"
        />
      </div>
      <ul class="storage-allocation-legend">
        <li
          v-for="segment in allocationSegments"
          :key="segment.key"
        >
          <span
            class="swatch"
            :style="{ background: segment.color }"
          />
          <span
            class="storage-allocation-label"
            :title="segment.label"
          >{{ segment.label }}</span>
          <span class="storage-allocation-value">{{ segment.percent.toFixed(0) }}%</span>
        </li>
        <li
          v-if="freeBytes > 0"
          class="free"
        >
          <span class="swatch" />
          <span class="storage-allocation-label">{{ t("storage.free") }}</span>
          <span class="storage-allocation-value">{{ freePercent.toFixed(0) }}%</span>
        </li>
      </ul>
    </section>

    <section class="table-panel storage-panel">
      <div
        v-if="mounts.length > 0"
        class="table-toolbar"
      >
        <fieldset class="tag-toggles">
          <legend class="sr-only">
            {{ t("storage.tagVisibility") }}
          </legend>
          <label
            v-for="tag in TAG_FILTER_KEYS"
            :key="tag"
            :class="['tag-toggle', tag, { off: isTagHidden(tag) }]"
          >
            <input
              type="checkbox"
              :checked="!isTagHidden(tag)"
              @change="toggleTag(tag)"
            >
            {{ tagLabel(tag) }}
          </label>
        </fieldset>
        <span class="table-count">{{ t("storage.count", { count: filteredMounts.length }) }}</span>
      </div>

      <div class="table-scroll">
        <table class="storage-table">
          <thead>
            <tr><th>{{ t("storage.mount") }}</th><th>{{ t("storage.filesystem") }}</th><th>{{ t("storage.capacity") }}</th><th>{{ t("storage.usage") }}</th></tr>
          </thead>
          <tbody>
            <tr v-if="loading && mounts.length === 0">
              <td
                colspan="4"
                class="empty"
              >
                {{ t("storage.loading") }}
              </td>
            </tr>
            <tr v-else-if="!loading && mounts.length === 0 && !error">
              <td
                colspan="4"
                class="empty"
              >
                {{ t("storage.empty") }}
              </td>
            </tr>
            <tr v-else-if="filteredMounts.length === 0">
              <td
                colspan="4"
                class="empty"
              >
                {{ t("storage.empty") }}
              </td>
            </tr>
            <tr
              v-for="mount in filteredMounts"
              :key="`${mount.filesystem}:${mount.mount_point}`"
            >
              <td class="path-cell">
                <div class="storage-path-row">
                  <strong
                    class="storage-path"
                    :title="mount.mount_point"
                  >{{ mount.mount_point }}</strong>
                  <span
                    v-if="mountTags(mount).length"
                    class="tag-badges"
                  >
                    <span
                      v-for="tag in mountTags(mount)"
                      :key="tag"
                      :class="['tag-badge', tag]"
                    >{{ tagLabel(tag) }}</span>
                  </span>
                </div>
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
                <div class="usage-row">
                  <span>{{ mount.usage_percent.toFixed(0) }}%</span>
                  <small>{{ t("storage.used", { value: formatBytes(mount.used_bytes, locale) }) }}</small>
                </div>
                <div class="progress">
                  <span
                    :class="{ critical: mount.usage_percent >= 90 }"
                    :style="{ width: `${mount.usage_percent}%` }"
                  />
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { formatBytes, usagePercentage, type StorageDisk } from "../api/client";
import {
  AppCard,
  AppStack,
  ProgressBar,
  StateBadge,
  StorageAllocationBar,
  TablePanel,
  TagBadge,
} from "../design-system/components";

const props = defineProps<{ disk: StorageDisk }>();

const { t, locale } = useI18n();

const SEGMENT_COLORS = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#4a3aa7"];

const kindLabel = computed(() => {
  if (props.disk.rotational === true) return t("storage.diskHdd");
  if (props.disk.rotational === false) return t("storage.diskSsd");
  return null;
});

const topLevelPartitions = computed(() => props.disk.partitions.filter((partition) => partition.kind === "part"));

const segments = computed(() => {
  const size = props.disk.size_bytes;
  if (size <= 0) return [];
  const parts = topLevelPartitions.value.map((partition, index) => ({
    key: partition.path,
    label: partition.name,
    percent: (partition.size_bytes / size) * 100,
    color: SEGMENT_COLORS[index % SEGMENT_COLORS.length],
    value: formatBytes(partition.size_bytes, locale.value),
  }));
  const assigned = topLevelPartitions.value.reduce((sum, partition) => sum + partition.size_bytes, 0);
  const rest = Math.max(0, size - assigned);
  if (rest > size * 0.005) {
    parts.push({
      key: "unallocated",
      label: t("storage.unallocated"),
      percent: (rest / size) * 100,
      color: "var(--border-strong)",
      value: formatBytes(rest, locale.value),
    });
  }
  return parts;
});

const mounted = computed(() => props.disk.partitions.flatMap((partition) => (partition.mount ? [partition.mount] : [])));
const mountedCapacity = computed(() => mounted.value.reduce((sum, mount) => sum + mount.total_bytes, 0));
const mountedUsed = computed(() => mounted.value.reduce((sum, mount) => sum + mount.used_bytes, 0));
const mountedPercent = computed(() => usagePercentage(mountedUsed.value, mountedCapacity.value) ?? 0);

function deviceLabel(kind: string) {
  return kind === "part" ? "" : kind;
}
</script>

<template>
  <AppStack gap="4">
    <AppCard>
      <AppStack gap="3">
        <AppStack
          direction="row"
          justify="between"
          align="center"
          gap="3"
          wrap
        >
          <AppStack
            gap="1"
          >
            <h2>{{ disk.name }}</h2>
            <small class="disk-model">{{ disk.model ?? disk.path }}</small>
          </AppStack>
          <AppStack
            direction="row"
            gap="2"
            align="center"
            wrap
          >
            <TagBadge category="deckox">
              {{ formatBytes(disk.size_bytes, locale) }}
            </TagBadge>
            <TagBadge
              v-if="disk.transport"
              category="standard"
            >
              {{ disk.transport }}
            </TagBadge>
            <TagBadge
              v-if="kindLabel"
              category="standard"
            >
              {{ kindLabel }}
            </TagBadge>
            <TagBadge
              v-if="disk.removable"
              category="product"
            >
              {{ t("storage.diskRemovable") }}
            </TagBadge>
          </AppStack>
        </AppStack>
        <StorageAllocationBar
          v-if="segments.length > 0"
          :segments="segments"
          :label="t('storage.diskLayout', { name: disk.name })"
        />
        <template v-if="mounted.length > 0">
          <AppStack
            direction="row"
            justify="between"
            gap="3"
            wrap
          >
            <span>{{ t("storage.mountedUsage") }}</span>
            <span class="storage-summary-detail">{{ t("storage.overallUsageDetail", { used: formatBytes(mountedUsed, locale), total: formatBytes(mountedCapacity, locale) }) }}</span>
          </AppStack>
          <ProgressBar
            :value="mountedPercent"
            :critical="mountedPercent >= 90"
            :label="t('storage.mountedUsage')"
          />
        </template>
      </AppStack>
    </AppCard>

    <TablePanel
      :empty="disk.partitions.length === 0"
      :empty-message="t('storage.noPartitions')"
    >
      <table>
        <thead>
          <tr>
            <th>{{ t("storage.device") }}</th>
            <th>{{ t("storage.filesystem") }}</th>
            <th>{{ t("storage.capacity") }}</th>
            <th>{{ t("storage.mount") }}</th>
            <th>{{ t("storage.usage") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="partition in disk.partitions"
            :key="partition.path"
          >
            <td>
              <AppStack
                direction="row"
                gap="2"
                align="center"
                wrap
              >
                <strong class="storage-path">{{ partition.name }}</strong>
                <TagBadge
                  v-if="deviceLabel(partition.kind)"
                  category="standard"
                >
                  {{ deviceLabel(partition.kind) }}
                </TagBadge>
              </AppStack>
            </td>
            <td>
              <span>{{ partition.filesystem_type ?? t("common.none") }}</span>
              <small v-if="partition.label">{{ partition.label }}</small>
            </td>
            <td>
              <strong>{{ formatBytes(partition.size_bytes, locale) }}</strong>
            </td>
            <td>
              <span v-if="partition.mount">{{ partition.mount.mount_point }}</span>
              <StateBadge
                v-else
                state="inactive"
              >
                {{ t("storage.unmounted") }}
              </StateBadge>
            </td>
            <td>
              <AppStack
                v-if="partition.mount"
                gap="1"
              >
                <span class="usage-row">
                  <span>{{ partition.mount.usage_percent.toFixed(0) }}%</span>
                  <small>{{ t("storage.used", { value: formatBytes(partition.mount.used_bytes, locale) }) }}</small>
                </span>
                <ProgressBar
                  :value="partition.mount.usage_percent"
                  :critical="partition.mount.usage_percent >= 90"
                  :label="partition.mount.mount_point"
                />
              </AppStack>
              <span v-else>{{ t("common.none") }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </TablePanel>
  </AppStack>
</template>

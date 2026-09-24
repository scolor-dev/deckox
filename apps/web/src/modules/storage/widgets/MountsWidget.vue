<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { formatBytes, type StorageMount } from "../../../api/client";
import {
  AppButton,
  AppStack,
  AppText,
  NoticeBanner,
  ProgressBar,
  SectionHeader,
  TablePanel,
  TableToolbar,
  TagBadge,
  TagToggle,
  TagToggleGroup,
} from "../../../design-system/components";
import { preferences, type StorageTagFilterKey } from "../../../preferences";
import { useStorageUsage } from "../usage";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const { mounts, loading, errorMessage, refresh } = useStorageUsage();

const TAG_FILTER_KEYS: StorageTagFilterKey[] = ["standard", "other"];

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
</script>

<template>
  <AppStack gap="3">
    <SectionHeader :title="t('storage.title')">
      <template #actions>
        <AppButton
          :disabled="loading"
          @click="refresh"
        >
          {{ loading ? t("common.loading") : t("common.refresh") }}
        </AppButton>
      </template>
    </SectionHeader>
    <NoticeBanner
      v-if="errorMessage"
      tone="error"
    >
      {{ errorMessage }}
    </NoticeBanner>
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
              <AppText
                as="small"
                tone="muted"
                size="xs"
              >
                {{ mount.filesystem_type }}
              </AppText>
            </td>
            <td class="capacity-cell">
              <strong>{{ formatBytes(mount.total_bytes, locale) }}</strong>
              <AppText
                as="small"
                tone="muted"
                size="xs"
              >
                {{ t("storage.available", { value: formatBytes(mount.available_bytes, locale) }) }}
              </AppText>
            </td>
            <td class="storage-usage-cell">
              <AppStack gap="1">
                <span class="usage-row">
                  <span>{{ mount.usage_percent.toFixed(0) }}%</span>
                  <AppText
                    as="small"
                    tone="muted"
                    size="xs"
                  >{{ t("storage.used", { value: formatBytes(mount.used_bytes, locale) }) }}</AppText>
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
  </AppStack>
</template>

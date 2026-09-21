<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { formatBytes } from "../../../api/client";
import { AppStack, TabBar } from "../../../design-system/components";
import DiskPanel from "./DiskPanel.vue";
import { useStorageDisks } from "../usage";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const { disks } = useStorageDisks();
const active = ref("");

const tabs = computed(() => disks.value.map((disk) => ({
  key: disk.name,
  label: `${disk.name} · ${formatBytes(disk.size_bytes, locale.value)}`,
})));
const activeDisk = computed(() => disks.value.find((disk) => disk.name === active.value) ?? disks.value.at(0) ?? null);

watch(disks, (list) => {
  if (!list.some((disk) => disk.name === active.value)) active.value = list[0]?.name ?? "";
}, { immediate: true });
</script>

<template>
  <AppStack
    v-if="activeDisk"
    gap="3"
  >
    <TabBar
      v-model="active"
      :tabs="tabs"
      :label="t('storage.tabsLabel')"
    />
    <DiskPanel :disk="activeDisk" />
  </AppStack>
  <p
    v-else
    class="widget-message"
  >
    {{ t("widgets.storage.noDisks") }}
  </p>
</template>

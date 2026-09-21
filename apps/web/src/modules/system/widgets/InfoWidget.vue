<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { serverStatus, systemInfo } from "../../../data/sources";
import { AppButton, AppStack, DetailList, DetailRow } from "../../../design-system/components";
import { writeClipboardText } from "../../../api/client";
import { notify } from "../../../notifications";

defineOptions({ inheritAttrs: false });

const { t } = useI18n();
const { data: system } = systemInfo.use();
const { data: status } = serverStatus.use();

const accessUrls = computed(() => {
  const port = status.value?.port;
  if (!port) return [];
  return (system.value?.lan_addresses ?? []).map((address) => `http://${address}:${String(port)}`);
});

async function copy(url: string) {
  if (await writeClipboardText(url)) notify("success", t("overview.accessUrlCopied"));
  else notify("error", t("overview.accessUrlCopyFailed"));
}
</script>

<template>
  <DetailList>
    <DetailRow :term="t('overview.hostname')">
      {{ system?.hostname ?? t("common.none") }}
    </DetailRow>
    <DetailRow term="OS">
      {{ system ? `${system.operating_system} ${system.os_version ?? ""}` : t("common.none") }}
    </DetailRow>
    <DetailRow :term="t('overview.kernel')">
      {{ system?.kernel_version ?? t("common.none") }}
    </DetailRow>
    <DetailRow :term="t('overview.architecture')">
      {{ system?.architecture ?? t("common.none") }}
    </DetailRow>
    <DetailRow :term="t('overview.timezone')">
      {{ system?.timezone ?? t("common.none") }}
    </DetailRow>
    <DetailRow :term="t('overview.accessUrl')">
      <template v-if="accessUrls.length === 0">
        {{ t("common.none") }}
      </template>
      <AppStack
        v-else
        gap="1"
      >
        <AppStack
          v-for="url in accessUrls"
          :key="url"
          direction="row"
          gap="2"
          align="center"
        >
          <span>{{ url }}</span>
          <AppButton
            variant="action"
            @click="copy(url)"
          >
            {{ t("overview.copyAccessUrl") }}
          </AppButton>
        </AppStack>
      </AppStack>
    </DetailRow>
  </DetailList>
</template>

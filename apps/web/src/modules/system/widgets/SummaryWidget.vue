<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { formatUptime } from "../../../api/client";
import { systemInfo } from "../../../data/sources";
import { AppCard, AppStack, AppText } from "../../../design-system/components";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const { data: system } = systemInfo.use();
</script>

<template>
  <AppCard>
    <AppStack gap="3">
      <strong>{{ system?.hostname ?? t("overview.loadingHost") }}</strong>
      <AppText
        as="small"
        tone="muted"
        size="xs"
      >
        {{ system?.operating_system ?? "Linux" }} {{ system?.os_version ?? "" }}
      </AppText>
      <AppText
        as="small"
        tone="muted"
        size="xs"
      >
        {{ t("overview.uptime") }}: {{ formatUptime(system?.uptime_seconds, locale) }}
      </AppText>
      <AppText
        as="small"
        tone="muted"
        size="xs"
      >
        {{ t("overview.architecture") }}: {{ system?.architecture ?? t("common.none") }}
      </AppText>
    </AppStack>
  </AppCard>
</template>

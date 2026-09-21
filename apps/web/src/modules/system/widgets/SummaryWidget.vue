<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { formatUptime } from "../../../api/client";
import { systemInfo } from "../../../data/sources";
import { AppCard, AppStack } from "../../../design-system/components";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const { data: system } = systemInfo.use();
</script>

<template>
  <AppCard>
    <AppStack gap="3">
      <strong>{{ system?.hostname ?? t("overview.loadingHost") }}</strong>
      <small>{{ system?.operating_system ?? "Linux" }} {{ system?.os_version ?? "" }}</small>
      <small>{{ t("overview.uptime") }}: {{ formatUptime(system?.uptime_seconds, locale) }}</small>
      <small>{{ t("overview.architecture") }}: {{ system?.architecture ?? t("common.none") }}</small>
    </AppStack>
  </AppCard>
</template>

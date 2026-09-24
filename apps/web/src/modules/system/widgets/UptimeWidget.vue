<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { formatUptime } from "../../../api/client";
import { systemInfo } from "../../../data/sources";
import { MetricCard } from "../../../design-system/components";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const { data: system } = systemInfo.use();
</script>

<template>
  <MetricCard
    :label="t('overview.uptime')"
    :meta="system?.hostname"
    :value="system ? formatUptime(system.uptime_seconds, locale) : t('common.none')"
  />
</template>

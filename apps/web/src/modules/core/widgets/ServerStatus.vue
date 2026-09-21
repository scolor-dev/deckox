<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { serverStatus } from "../../../data/sources";
import { AppCard, AppStack, StateBadge } from "../../../design-system/components";

defineOptions({ inheritAttrs: false });

const { t } = useI18n();
const { data: status } = serverStatus.use();
const online = computed(() => Boolean(status.value?.agent));
</script>

<template>
  <AppCard>
    <AppStack gap="3">
      <StateBadge :state="online ? 'active' : 'failed'">
        {{ online ? t("overview.healthy") : t("overview.agentUnavailable") }}
      </StateBadge>
      <small>Deckox {{ t("common.version") }} {{ status?.version ?? t("common.none") }}</small>
      <small>Agent: {{ status?.agent?.hostname ?? t("common.none") }}</small>
    </AppStack>
  </AppCard>
</template>

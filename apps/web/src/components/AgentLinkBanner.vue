<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { NoticeBanner } from "../design-system/components";
import { agentLink, agentState } from "../modules/store";

const { t } = useI18n();
</script>

<template>
  <NoticeBanner
    v-if="agentState === 'unreachable'"
    tone="warning"
  >
    {{ t("agentLink.unreachable") }}
  </NoticeBanner>
  <NoticeBanner
    v-else-if="agentState === 'incompatible'"
    tone="error"
  >
    {{ t("agentLink.incompatible", {
      agent: agentLink.agentVersion ?? t("common.none"),
      agentProtocol: agentLink.agentProtocol ?? t("agentLink.legacy"),
      serverProtocol: agentLink.serverProtocol ?? t("common.none"),
    }) }}
  </NoticeBanner>
</template>

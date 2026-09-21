<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useMetrics } from "../../../data/metrics";
import { AppButton, AppCard, AppStack, StateBadge } from "../../../design-system/components";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const { streamStatus, lastReceivedAt, agentOnline, loading, reconnect, refresh } = useMetrics();

const label = computed(() => {
  if (streamStatus.value === "connected") return t("overview.realtime");
  return streamStatus.value === "paused" ? t("overview.paused") : t("overview.connecting");
});
const lastUpdated = computed(() => {
  if (lastReceivedAt.value === null) return t("overview.notUpdated");
  return new Intl.DateTimeFormat(locale.value, { hour: "2-digit", minute: "2-digit", second: "2-digit" })
    .format(lastReceivedAt.value);
});
const canRecover = computed(() =>
  streamStatus.value === "paused" || streamStatus.value === "reconnecting" || !agentOnline.value);

async function recover() {
  if (streamStatus.value === "reconnecting") reconnect();
  await refresh();
}
</script>

<template>
  <AppCard>
    <AppStack gap="3">
      <span role="status">
        <StateBadge :state="streamStatus === 'connected' ? 'active' : 'inactive'">
          {{ label }}
        </StateBadge>
      </span>
      <small>{{ t("overview.lastUpdated", { time: lastUpdated }) }}</small>
      <AppButton
        v-if="canRecover"
        :disabled="loading"
        @click="recover"
      >
        {{ loading ? t("common.checking") : streamStatus === "reconnecting" ? t("overview.reconnect") : t("common.refresh") }}
      </AppButton>
    </AppStack>
  </AppCard>
</template>

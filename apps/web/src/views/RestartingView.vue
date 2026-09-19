<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api } from "../api/client";
import { AppButton, AppCard, AppIcon, AppSpinner, InfoNote } from "../design-system/components";
import { hasServerRestarted } from "../restart";

const { t } = useI18n();
const phase = ref<"waiting" | "offline" | "ready" | "timeout">("waiting");
const previousInstance = sessionStorage.getItem("deckox:restart-instance");
let observedOffline = false;
let timer: number | null = null;
let stopped = false;
const startedAt = Date.now();

function schedule(delay = 2000) {
  if (!stopped) timer = window.setTimeout(() => void poll(), delay);
}

async function poll() {
  try {
    const health = await api.health();
    if (hasServerRestarted(previousInstance, health.instance_id, observedOffline)) {
      phase.value = "ready";
      sessionStorage.removeItem("deckox:restart-instance");
      timer = window.setTimeout(() => { window.location.replace("/"); }, 1200);
      return;
    }
  } catch {
    observedOffline = true;
    phase.value = "offline";
  }
  if (Date.now() - startedAt >= 5 * 60 * 1000) {
    phase.value = "timeout";
    return;
  }
  schedule();
}

function retry() {
  phase.value = observedOffline ? "offline" : "waiting";
  void poll();
}

onMounted(() => { schedule(1500); });
onBeforeUnmount(() => {
  stopped = true;
  if (timer !== null) window.clearTimeout(timer);
});
</script>

<template>
  <section
    class="restart-page"
    aria-live="polite"
  >
    <AppCard class="restart-panel">
      <AppIcon
        v-if="phase === 'ready'"
        class="restart-ready"
        name="check"
        size="lg"
        :label="t('restart.ready')"
      />
      <AppSpinner
        v-else
        :label="t('restart.title')"
      />
      <h1>{{ t("restart.title") }}</h1>
      <p v-if="phase === 'waiting'">
        {{ t("restart.waiting") }}
      </p>
      <p v-else-if="phase === 'offline'">
        {{ t("restart.offline") }}
      </p>
      <p v-else-if="phase === 'ready'">
        {{ t("restart.ready") }}
      </p>
      <template v-else>
        <p>{{ t("restart.timeout") }}</p>
        <AppButton @click="retry">
          {{ t("restart.retry") }}
        </AppButton>
      </template>
      <InfoNote>{{ t("restart.keepOpen") }}</InfoNote>
    </AppCard>
  </section>
</template>

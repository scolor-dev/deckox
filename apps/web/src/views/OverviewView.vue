<script setup lang="ts">
import { computed, onMounted, ref, toRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  api,
  formatBytes,
  formatUptime,
  type ServerStatus,
  type SystemInfo,
  type SystemMetrics,
} from "../api/client";
import MetricChart from "../components/MetricChart.vue";
import { useRealtimeMetrics } from "../composables/useRealtimeMetrics";
import { notify } from "../notifications";
import { preferences } from "../preferences";
import { apiErrorKey } from "../api/errors";

const emit = defineEmits<{ status: [value: ServerStatus] }>();
const HISTORY_LIMIT = 120;
const { t, locale } = useI18n();

const status = ref<ServerStatus | null>(null);
const system = ref<SystemInfo | null>(null);
const metrics = ref<SystemMetrics | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);
const intervalSeconds = toRef(preferences, "metricsInterval");
const realtimeEnabled = toRef(preferences, "realtimeEnabled");
const cpuHistory = ref<number[]>([]);
const memoryHistory = ref<number[]>([]);
const loadHistory = ref<number[]>([]);

const stream = useRealtimeMetrics(intervalSeconds, realtimeEnabled);
const agentOnline = computed(() => stream.latest.value?.agent_online ?? Boolean(status.value?.agent));
const lastUpdated = computed(() => {
  if (stream.lastReceivedAt.value === null) return t("overview.notUpdated");
  return new Intl.DateTimeFormat(locale.value, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(stream.lastReceivedAt.value);
});
const memoryPercent = computed(() => {
  if (!metrics.value || metrics.value.memory.total_bytes === 0) return 0;
  return metrics.value.memory.used_bytes / metrics.value.memory.total_bytes * 100;
});
const loadMaximum = computed(() => Math.max(metrics.value?.cpu.logical_cores ?? 1, 1));

function appendHistory(target: typeof cpuHistory, value: number) {
  target.value = [...target.value.slice(-(HISTORY_LIMIT - 1)), value];
}

function applyMetrics(value: SystemMetrics) {
  metrics.value = value;
  appendHistory(cpuHistory, value.cpu.usage_percent);
  const percentage = value.memory.total_bytes > 0
    ? value.memory.used_bytes / value.memory.total_bytes * 100
    : 0;
  appendHistory(memoryHistory, percentage);
  appendHistory(loadHistory, value.load_average.one_minute);
}

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    status.value = await api.serverStatus();
    emit("status", status.value);
    const [systemInfo, systemMetrics] = await Promise.all([
      api.systemInfo(),
      api.systemMetrics(),
    ]);
    system.value = systemInfo;
    applyMetrics(systemMetrics);
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.overview"));
  } finally {
    loading.value = false;
  }
}

async function recover() {
  if (stream.status.value === "reconnecting") stream.reconnect();
  await refresh();
}

watch(stream.latest, (event, previous) => {
  if (event?.metrics) applyMetrics(event.metrics);
  if (event?.agent_online && previous?.agent_online === false) void refreshIdentity();
});

async function refreshIdentity() {
  try {
    const [serverStatus, systemInfo] = await Promise.all([
      api.serverStatus(),
      api.systemInfo(),
    ]);
    status.value = serverStatus;
    system.value = systemInfo;
    emit("status", serverStatus);
  } catch {
    // The SSE reconnect loop remains responsible for recovery.
  }
}

let streamWasDisconnected = false;
watch(stream.status, (current, previous) => {
  if (current === "reconnecting" && previous === "connected") {
    streamWasDisconnected = true;
    notify("warning", t("notifications.streamLost"));
  } else if (current === "connected" && streamWasDisconnected) {
    streamWasDisconnected = false;
    notify("success", t("notifications.streamRestored"));
  }
});

onMounted(refresh);
</script>

<template>
  <div class="view">
    <header class="view-header">
      <div>
        <h1>{{ t("overview.title") }}</h1>
        <p class="subtitle">
          {{ system?.hostname ?? status?.agent?.hostname ?? t("overview.loadingHost") }}
        </p>
      </div>
      <div class="header-actions">
        <span
          :class="['stream-state', stream.status.value]"
          role="status"
        >
          {{ stream.status.value === "connected" ? t("overview.realtime") : stream.status.value === "paused" ? t("overview.paused") : t("overview.connecting") }}
        </span>
        <small class="last-updated">{{ t("overview.lastUpdated", { time: lastUpdated }) }}</small>
        <button
          v-if="stream.status.value === 'paused' || stream.status.value === 'reconnecting' || !agentOnline"
          class="button"
          type="button"
          :disabled="loading"
          @click="recover"
        >
          {{ loading ? t("common.checking") : stream.status.value === "reconnecting" ? t("overview.reconnect") : t("common.refresh") }}
        </button>
      </div>
    </header>

    <div
      v-if="error"
      class="notice error"
    >
      {{ error }}
    </div>
    <div
      v-else-if="status?.agent_error || stream.latest.value?.agent_online === false"
      class="notice warning"
    >
      {{ t("overview.agentUnavailable") }}
    </div>

    <section class="server-summary">
      <div class="server-state">
        <span :class="['status-dot', agentOnline ? 'online' : 'offline']" />
        <div>
          <strong>{{ agentOnline ? t("overview.healthy") : t("overview.agentUnavailable") }}</strong>
          <span>{{ system?.operating_system ?? "Linux" }} {{ system?.os_version ?? "" }}</span>
        </div>
      </div>
      <dl>
        <div><dt>{{ t("overview.uptime") }}</dt><dd>{{ formatUptime(system?.uptime_seconds, locale) }}</dd></div>
        <div><dt>{{ t("overview.architecture") }}</dt><dd>{{ system?.architecture ?? t("common.none") }}</dd></div>
      </dl>
    </section>

    <section
      class="metric-grid"
      :aria-label="t('overview.resources')"
    >
      <article class="metric-card">
        <div class="metric-head">
          <span>{{ t("overview.cpu") }}</span><small>{{ t("overview.cores", { count: metrics?.cpu.logical_cores ?? t("common.none") }) }}</small>
        </div>
        <strong>{{ metrics ? `${metrics.cpu.usage_percent.toFixed(1)}%` : "—" }}</strong>
        <MetricChart
          :values="cpuHistory"
          :maximum="100"
          :label="t('overview.cpuChart')"
        />
      </article>
      <article class="metric-card">
        <div class="metric-head">
          <span>{{ t("overview.memory") }}</span><small>{{ t("overview.total", { value: formatBytes(metrics?.memory.total_bytes ?? 0, locale) }) }}</small>
        </div>
        <strong>{{ metrics ? t("overview.inUse", { value: formatBytes(metrics.memory.used_bytes, locale) }) : t("common.none") }}</strong>
        <MetricChart
          :values="memoryHistory"
          :maximum="100"
          :label="t('overview.memoryChart')"
        />
        <small class="metric-foot">{{ memoryPercent.toFixed(1) }}%</small>
      </article>
      <article class="metric-card">
        <div class="metric-head">
          <span>{{ t("overview.load") }}</span><small>{{ t("overview.fiveMinutes", { value: metrics?.load_average.five_minutes.toFixed(2) ?? t("common.none") }) }}</small>
        </div>
        <strong>{{ metrics?.load_average.one_minute.toFixed(2) ?? "—" }}</strong>
        <MetricChart
          :values="loadHistory"
          :maximum="loadMaximum"
          :label="t('overview.loadChart')"
        />
        <small class="metric-foot">{{ t("overview.fifteenMinutes", { value: metrics?.load_average.fifteen_minutes.toFixed(2) ?? t("common.none") }) }}</small>
      </article>
    </section>

    <section class="detail-panel">
      <div class="section-title">
        <h2>{{ t("overview.systemInfo") }}</h2>
      </div>
      <dl class="details">
        <div>
          <dt>{{ t("overview.hostname") }}</dt><dd :title="system?.hostname ?? undefined">
            {{ system?.hostname ?? t("common.none") }}
          </dd>
        </div>
        <div>
          <dt>OS</dt><dd :title="system ? `${system.operating_system} ${system.os_version ?? ''}` : undefined">
            {{ system ? `${system.operating_system} ${system.os_version ?? ""}` : t("common.none") }}
          </dd>
        </div>
        <div>
          <dt>{{ t("overview.kernel") }}</dt><dd :title="system?.kernel_version ?? undefined">
            {{ system?.kernel_version ?? t("common.none") }}
          </dd>
        </div>
        <div><dt>{{ t("overview.architecture") }}</dt><dd>{{ system?.architecture ?? t("common.none") }}</dd></div>
        <div><dt>{{ t("overview.timezone") }}</dt><dd>{{ system?.timezone ?? t("common.none") }}</dd></div>
        <div><dt>Deckox</dt><dd>{{ t("common.version") }} {{ status?.version ?? t("common.none") }}</dd></div>
      </dl>
    </section>
  </div>
</template>

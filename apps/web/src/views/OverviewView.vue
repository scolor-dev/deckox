<script setup lang="ts">
import { computed, ref, toRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useStaleRefresh } from "../composables/useStaleRefresh";
import { staleMsOf } from "../modules/registry";
import {
  api,
  appendMetricHistory,
  capacityMounts,
  formatBytes,
  formatUptime,
  usagePercentage,
  writeClipboardText,
  type ServerStatus,
  type StorageMount,
  type SystemInfo,
  type SystemMetrics,
} from "../api/client";
import MetricChart from "../components/MetricChart.vue";
import {
  AppButton,
  AppCard,
  AppStack,
  DetailList,
  DetailRow,
  MetricCard,
  NoticeBanner,
  PageHeader,
  ProgressBar,
  StateBadge,
} from "../design-system/components";
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
const storageMounts = ref<StorageMount[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
const intervalSeconds = toRef(preferences, "metricsInterval");
const realtimeEnabled = toRef(preferences, "realtimeEnabled");
const cpuHistory = ref<number[]>([]);
const memoryHistory = ref<number[]>([]);
const loadHistory = ref<number[]>([]);
const swapHistory = ref<number[]>([]);
const temperatureHistory = ref<number[]>([]);
const networkReceivedHistory = ref<number[]>([]);
const networkTransmittedHistory = ref<number[]>([]);
const diskReadHistory = ref<number[]>([]);
const diskWrittenHistory = ref<number[]>([]);

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
  const memory = metrics.value?.memory;
  return memory ? usagePercentage(memory.used_bytes, memory.total_bytes) ?? 0 : 0;
});
const loadMaximum = computed(() => Math.max(metrics.value?.cpu.logical_cores ?? 1, 1));
const swapPercent = computed(() => {
  const memory = metrics.value?.memory;
  return memory ? usagePercentage(memory.swap_used_bytes, memory.swap_total_bytes) : null;
});
const temperature = computed(() => metrics.value?.cpu.temperature_celsius ?? null);
const countedMounts = computed(() => capacityMounts(storageMounts.value));
const busiestMount = computed(() => countedMounts.value.length === 0
  ? null
  : countedMounts.value.reduce((busiest, mount) => (
    mount.usage_percent > busiest.usage_percent ? mount : busiest
  )));
// Matches StorageView's "overall usage" exactly (sum across every mount),
// so the two screens never show a different number for the same concept —
// only the per-mount warning below is specific to a single mount.
const totalStorageCapacity = computed(
  () => countedMounts.value.reduce((sum, mount) => sum + mount.total_bytes, 0),
);
const totalStorageUsed = computed(
  () => countedMounts.value.reduce((sum, mount) => sum + mount.used_bytes, 0),
);
const overallStoragePercent = computed(
  () => usagePercentage(totalStorageUsed.value, totalStorageCapacity.value) ?? 0,
);
const storageFooter = computed(() => {
  if (storageMounts.value.length === 0) return null;
  const detail = t("storage.overallUsageDetail", { used: formatBytes(totalStorageUsed.value, locale.value), total: formatBytes(totalStorageCapacity.value, locale.value) });
  const busiest = busiestMount.value;
  if (!busiest) return detail;
  return `${detail} · ${t("overview.busiestMount", { mount: busiest.mount_point, percent: busiest.usage_percent.toFixed(0) })}`;
});
const networkMaximum = computed(() => Math.max(
  1,
  ...networkReceivedHistory.value,
  ...networkTransmittedHistory.value,
));
const diskMaximum = computed(() => Math.max(
  1,
  ...diskReadHistory.value,
  ...diskWrittenHistory.value,
));
const accessUrls = computed(() => {
  const port = status.value?.port;
  if (!port) return [];
  const portText = String(port);
  return (system.value?.lan_addresses ?? []).map((address) => `http://${address}:${portText}`);
});

async function copyAccessUrl(url: string) {
  if (await writeClipboardText(url)) {
    notify("success", t("overview.accessUrlCopied"));
  } else {
    notify("error", t("overview.accessUrlCopyFailed"));
  }
}

function appendHistory(target: typeof cpuHistory, value: number) {
  target.value = appendMetricHistory(target.value, value, HISTORY_LIMIT);
}

function applyMetrics(value: SystemMetrics) {
  metrics.value = value;
  appendHistory(cpuHistory, value.cpu.usage_percent);
  const percentage = value.memory.total_bytes > 0
    ? usagePercentage(value.memory.used_bytes, value.memory.total_bytes) ?? 0
    : 0;
  appendHistory(memoryHistory, percentage);
  appendHistory(loadHistory, value.load_average.one_minute);
  if (value.memory.swap_total_bytes > 0) {
    const percentage = usagePercentage(value.memory.swap_used_bytes, value.memory.swap_total_bytes);
    if (percentage !== null) appendHistory(swapHistory, percentage);
  }
  if (value.cpu.temperature_celsius != null) {
    appendHistory(temperatureHistory, value.cpu.temperature_celsius);
  }
  if (value.network) {
    appendHistory(networkReceivedHistory, value.network.received_bytes_per_second);
    appendHistory(networkTransmittedHistory, value.network.transmitted_bytes_per_second);
  }
  if (value.disk_io) {
    appendHistory(diskReadHistory, value.disk_io.read_bytes_per_second);
    appendHistory(diskWrittenHistory, value.disk_io.written_bytes_per_second);
  }
}

function formatRate(value: number | null | undefined) {
  return value == null ? t("common.none") : t("overview.perSecond", {
    value: formatBytes(value, locale.value),
  });
}

const percentFormatter = (value: number) => `${String(Math.round(value))}%`;
const decimalFormatter = (value: number) => value.toFixed(1);
const temperatureFormatter = (value: number) => `${String(Math.round(value))}°C`;
const rateFormatter = (value: number) => formatBytes(value, locale.value);

async function fetchOverview() {
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
    storageMounts.value = await api.storage().catch(() => storageMounts.value);
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

const refresh = useStaleRefresh(fetchOverview, staleMsOf("overview"));
</script>

<template>
  <div class="view">
    <PageHeader
      :title="t('overview.title')"
      :subtitle="system?.hostname ?? status?.agent?.hostname ?? t('overview.loadingHost')"
    >
      <template #actions>
        <AppStack
          direction="row"
          gap="3"
          align="center"
          wrap
        >
          <span role="status">
            <StateBadge :state="stream.status.value === 'connected' ? 'active' : 'inactive'">
              {{ stream.status.value === "connected" ? t("overview.realtime") : stream.status.value === "paused" ? t("overview.paused") : t("overview.connecting") }}
            </StateBadge>
          </span>
          <small class="last-updated">{{ t("overview.lastUpdated", { time: lastUpdated }) }}</small>
          <AppButton
            v-if="stream.status.value === 'paused' || stream.status.value === 'reconnecting' || !agentOnline"
            :disabled="loading"
            @click="recover"
          >
            {{ loading ? t("common.checking") : stream.status.value === "reconnecting" ? t("overview.reconnect") : t("common.refresh") }}
          </AppButton>
        </AppStack>
      </template>
    </PageHeader>

    <NoticeBanner
      v-if="error"
      tone="error"
    >
      {{ error }}
    </NoticeBanner>
    <NoticeBanner
      v-else-if="status?.agent_error || stream.latest.value?.agent_online === false"
      tone="warning"
    >
      {{ t("overview.agentUnavailable") }}
    </NoticeBanner>

    <AppCard>
      <AppStack
        direction="row"
        justify="between"
        align="center"
        gap="4"
        wrap
      >
        <AppStack gap="1">
          <StateBadge :state="agentOnline ? 'active' : 'failed'">
            {{ agentOnline ? t("overview.healthy") : t("overview.agentUnavailable") }}
          </StateBadge>
          <small class="server-os">{{ system?.operating_system ?? "Linux" }} {{ system?.os_version ?? "" }}</small>
        </AppStack>
        <DetailList>
          <DetailRow :term="t('overview.uptime')">
            {{ formatUptime(system?.uptime_seconds, locale) }}
          </DetailRow>
          <DetailRow :term="t('overview.architecture')">
            {{ system?.architecture ?? t("common.none") }}
          </DetailRow>
        </DetailList>
      </AppStack>
    </AppCard>

    <section
      class="metric-grid"
      :aria-label="t('overview.resources')"
    >
      <MetricCard
        :label="t('overview.cpu')"
        :meta="t('overview.cores', { count: metrics?.cpu.logical_cores ?? t('common.none') })"
        :value="metrics ? `${metrics.cpu.usage_percent.toFixed(1)}%` : '—'"
      >
        <MetricChart
          :values="cpuHistory"
          :maximum="100"
          :label="t('overview.cpuChart')"
          :limit="HISTORY_LIMIT"
          :value-formatter="percentFormatter"
        />
      </MetricCard>
      <MetricCard
        :label="t('overview.memory')"
        :meta="t('overview.total', { value: formatBytes(metrics?.memory.total_bytes ?? 0, locale) })"
        :value="metrics ? t('overview.inUse', { value: formatBytes(metrics.memory.used_bytes, locale) }) : t('common.none')"
        :footer="`${memoryPercent.toFixed(1)}%`"
      >
        <MetricChart
          :values="memoryHistory"
          :maximum="100"
          :label="t('overview.memoryChart')"
          :limit="HISTORY_LIMIT"
          :value-formatter="percentFormatter"
        />
      </MetricCard>
      <MetricCard
        :label="t('overview.load')"
        :meta="t('overview.fiveMinutes', { value: metrics?.load_average.five_minutes.toFixed(2) ?? t('common.none') })"
        :value="metrics?.load_average.one_minute.toFixed(2) ?? '—'"
        :footer="t('overview.fifteenMinutes', { value: metrics?.load_average.fifteen_minutes.toFixed(2) ?? t('common.none') })"
      >
        <MetricChart
          :values="loadHistory"
          :maximum="loadMaximum"
          :label="t('overview.loadChart')"
          :limit="HISTORY_LIMIT"
          :value-formatter="decimalFormatter"
        />
      </MetricCard>
      <MetricCard
        label="Swap"
        :meta="metrics?.memory.swap_total_bytes ? t('overview.total', { value: formatBytes(metrics.memory.swap_total_bytes, locale) }) : t('overview.notConfigured')"
        :value="swapPercent === null ? t('common.none') : `${swapPercent.toFixed(1)}%`"
        :warning="swapPercent !== null && swapPercent >= 80"
        :warning-text="t('overview.highUsage')"
      >
        <MetricChart
          :values="swapHistory"
          :maximum="100"
          :label="t('overview.swapChart')"
          :limit="HISTORY_LIMIT"
          :value-formatter="percentFormatter"
        />
      </MetricCard>
      <MetricCard
        :label="t('overview.network')"
        meta="RX / TX"
        :pairs="[
          { label: 'RX', value: formatRate(metrics?.network?.received_bytes_per_second) },
          { label: 'TX', value: formatRate(metrics?.network?.transmitted_bytes_per_second) },
        ]"
        :footer="`${t('overview.received')} / ${t('overview.transmitted')}`"
      >
        <MetricChart
          :values="networkReceivedHistory"
          :secondary-values="networkTransmittedHistory"
          :maximum="networkMaximum"
          :label="t('overview.networkChart')"
          :limit="HISTORY_LIMIT"
          :value-formatter="rateFormatter"
        />
      </MetricCard>
      <MetricCard
        :label="t('overview.diskIo')"
        :meta="t('overview.hostTotal')"
        :pairs="[
          { label: t('overview.read'), value: formatRate(metrics?.disk_io?.read_bytes_per_second) },
          { label: t('overview.write'), value: formatRate(metrics?.disk_io?.written_bytes_per_second) },
        ]"
        :footer="`${t('overview.read')} / ${t('overview.write')}`"
      >
        <MetricChart
          :values="diskReadHistory"
          :secondary-values="diskWrittenHistory"
          :maximum="diskMaximum"
          :label="t('overview.diskIoChart')"
          :limit="HISTORY_LIMIT"
          :value-formatter="rateFormatter"
        />
      </MetricCard>
      <MetricCard
        :label="t('storage.overallUsage')"
        :meta="t('storage.summary', { count: storageMounts.length })"
        :value="storageMounts.length > 0 ? `${overallStoragePercent.toFixed(0)}%` : t('common.none')"
        :footer="storageFooter"
        :warning="(busiestMount?.usage_percent ?? 0) >= 90"
        :warning-text="t('overview.highUsage')"
      >
        <ProgressBar
          v-if="storageMounts.length > 0"
          :value="overallStoragePercent"
          :critical="(busiestMount?.usage_percent ?? 0) >= 90"
          :label="t('storage.overallUsage')"
        />
      </MetricCard>
      <MetricCard
        :label="t('overview.temperature')"
        meta="CPU"
        :value="temperature === null ? t('common.none') : `${temperature.toFixed(1)} °C`"
        :footer="temperature === null ? t('overview.notAvailable') : t('overview.sensorValue')"
      >
        <MetricChart
          :values="temperatureHistory"
          :maximum="100"
          :label="t('overview.temperatureChart')"
          :limit="HISTORY_LIMIT"
          :value-formatter="temperatureFormatter"
        />
      </MetricCard>
    </section>

    <AppStack gap="3">
      <h2>
        {{ t("overview.systemInfo") }}
      </h2>
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
        <DetailRow term="Deckox">
          {{ t("common.version") }} {{ status?.version ?? t("common.none") }}
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
                @click="copyAccessUrl(url)"
              >
                {{ t("overview.copyAccessUrl") }}
              </AppButton>
            </AppStack>
          </AppStack>
        </DetailRow>
      </DetailList>
    </AppStack>
  </div>
</template>

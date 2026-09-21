import { computed, onActivated, onBeforeUnmount, onDeactivated, onMounted, ref, shallowRef, toRef, watch } from "vue";
import { api, appendMetricHistory, usagePercentage, type SystemMetrics } from "../api/client";
import { createMetricsStream } from "../composables/useRealtimeMetrics";
import { backendEnabled } from "../modules/store";
import { preferences } from "../preferences";
import { serverStatus, systemInfo } from "./sources";

export const HISTORY_LIMIT = 120;

const liveAvailable = computed(() => backendEnabled(["realtime", "system"]));
const stream = createMetricsStream(
  toRef(preferences, "metricsInterval"),
  computed(() => preferences.realtimeEnabled && liveAvailable.value),
);

export const metrics = shallowRef<SystemMetrics | null>(null);
export const history = {
  cpu: ref<number[]>([]),
  memory: ref<number[]>([]),
  load: ref<number[]>([]),
  swap: ref<number[]>([]),
  temperature: ref<number[]>([]),
  networkReceived: ref<number[]>([]),
  networkTransmitted: ref<number[]>([]),
  diskRead: ref<number[]>([]),
  diskWritten: ref<number[]>([]),
};

function push(target: typeof history.cpu, value: number) {
  target.value = appendMetricHistory(target.value, value, HISTORY_LIMIT);
}

function apply(value: SystemMetrics) {
  metrics.value = value;
  push(history.cpu, value.cpu.usage_percent);
  push(history.memory, value.memory.total_bytes > 0
    ? usagePercentage(value.memory.used_bytes, value.memory.total_bytes) ?? 0
    : 0);
  push(history.load, value.load_average.one_minute);
  if (value.memory.swap_total_bytes > 0) {
    const percent = usagePercentage(value.memory.swap_used_bytes, value.memory.swap_total_bytes);
    if (percent !== null) push(history.swap, percent);
  }
  if (value.cpu.temperature_celsius != null) push(history.temperature, value.cpu.temperature_celsius);
  if (value.network) {
    push(history.networkReceived, value.network.received_bytes_per_second);
    push(history.networkTransmitted, value.network.transmitted_bytes_per_second);
  }
  if (value.disk_io) {
    push(history.diskRead, value.disk_io.read_bytes_per_second);
    push(history.diskWritten, value.disk_io.written_bytes_per_second);
  }
}

watch(stream.latest, (event, previous) => {
  if (event?.metrics) apply(event.metrics);
  // The Agent is back: what this page knows about the host may have changed.
  if (event?.agent_online && previous?.agent_online === false) {
    void serverStatus.refresh();
    void systemInfo.refresh();
  }
});

const error = ref<unknown>(null);
const loading = ref(false);

/** Reads one sample, for when there is no live stream or before its first event. */
export async function refreshMetrics() {
  if (!backendEnabled(["system"])) return;
  loading.value = true;
  error.value = null;
  try {
    apply(await api.systemMetrics());
  } catch (cause) {
    error.value = cause;
  } finally {
    loading.value = false;
  }
}

let holders = 0;

/**
 * Call from a metrics widget's `setup`. The stream runs while at least one
 * such widget is on screen and stops when the last one leaves.
 */
export function useMetrics() {
  let holding = false;
  const hold = () => {
    if (holding) return;
    holding = true;
    holders += 1;
    if (holders === 1) stream.start();
    if (metrics.value === null) void refreshMetrics();
  };
  const release = () => {
    if (!holding) return;
    holding = false;
    holders -= 1;
    if (holders === 0) stream.stop();
  };
  onMounted(hold);
  onActivated(hold);
  onDeactivated(release);
  onBeforeUnmount(release);

  const agentOnline = computed(() => stream.latest.value?.agent_online ?? Boolean(serverStatus.data.value?.agent));
  return {
    metrics,
    history,
    error,
    loading,
    agentOnline,
    streamStatus: stream.status,
    lastReceivedAt: stream.lastReceivedAt,
    reconnect: stream.reconnect,
    refresh: refreshMetrics,
  };
}

export function resetMetrics() {
  metrics.value = null;
  for (const series of Object.values(history)) series.value = [];
}

import { onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";
import { type SystemMetrics } from "../api/client";

export type StreamStatus = "paused" | "connecting" | "connected" | "reconnecting";

export interface RealtimeMetricsEvent {
  sequence: number;
  timestamp_ms: number;
  agent_online: boolean;
  metrics: SystemMetrics | null;
  error_code: string | null;
}

const MAX_RECONNECT_DELAY_MS = 30_000;

export function parseMetricsEvent(data: string): RealtimeMetricsEvent | null {
  try {
    const value = JSON.parse(data) as Partial<RealtimeMetricsEvent>;
    if (
      typeof value.sequence !== "number" ||
      typeof value.timestamp_ms !== "number" ||
      typeof value.agent_online !== "boolean"
    ) return null;
    return {
      sequence: value.sequence,
      timestamp_ms: value.timestamp_ms,
      agent_online: value.agent_online,
      metrics: value.metrics ?? null,
      error_code: value.error_code ?? null,
    };
  } catch {
    return null;
  }
}

export function useRealtimeMetrics(
  intervalSeconds: Readonly<Ref<number>>,
  enabled: Readonly<Ref<boolean>>,
) {
  const status = ref<StreamStatus>("paused");
  const latest = ref<RealtimeMetricsEvent | null>(null);
  let eventSource: EventSource | null = null;
  let reconnectTimer: number | null = null;
  let reconnectAttempt = 0;
  let mounted = false;

  function clearReconnectTimer() {
    if (reconnectTimer !== null) window.clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }

  function disconnect(nextStatus: StreamStatus = "paused") {
    clearReconnectTimer();
    eventSource?.close();
    eventSource = null;
    status.value = nextStatus;
  }

  function shouldConnect() {
    return mounted && enabled.value && document.visibilityState === "visible";
  }

  function scheduleReconnect() {
    if (!shouldConnect() || reconnectTimer !== null) return;
    status.value = "reconnecting";
    const delay = Math.min(1_000 * 2 ** reconnectAttempt, MAX_RECONNECT_DELAY_MS);
    reconnectAttempt += 1;
    reconnectTimer = window.setTimeout(() => {
      reconnectTimer = null;
      connect();
    }, delay);
  }

  function connect() {
    if (!shouldConnect()) {
      disconnect();
      return;
    }
    disconnect("connecting");
    const source = new EventSource(
      `/api/v1/events/metrics?interval=${encodeURIComponent(String(intervalSeconds.value))}`,
    );
    eventSource = source;
    source.onopen = () => {
      reconnectAttempt = 0;
      status.value = "connected";
    };
    source.addEventListener("metrics", (event) => {
      if (!(event instanceof MessageEvent)) return;
      const parsed = parseMetricsEvent(String(event.data));
      if (parsed) latest.value = parsed;
    });
    source.onerror = () => {
      source.close();
      if (eventSource === source) eventSource = null;
      scheduleReconnect();
    };
  }

  function handleVisibilityChange() {
    if (document.visibilityState === "visible") connect();
    else disconnect();
  }

  watch([intervalSeconds, enabled], () => {
    connect();
  });

  onMounted(() => {
    mounted = true;
    document.addEventListener("visibilitychange", handleVisibilityChange);
    connect();
  });

  onBeforeUnmount(() => {
    mounted = false;
    document.removeEventListener("visibilitychange", handleVisibilityChange);
    disconnect();
  });

  return { status, latest, reconnect: connect };
}

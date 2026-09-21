import { ref, watch, type Ref } from "vue";
import { api, ApiError, type SystemMetrics } from "../api/client";

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

/**
 * The metrics event stream. It has no component lifecycle of its own: whoever
 * shows metrics calls `start()` and `stop()`, so several widgets can share one
 * connection (see `data/metrics.ts`).
 */
export function createMetricsStream(
  intervalSeconds: Readonly<Ref<number>>,
  enabled: Readonly<Ref<boolean>>,
) {
  const status = ref<StreamStatus>("paused");
  const latest = ref<RealtimeMetricsEvent | null>(null);
  const lastReceivedAt = ref<number | null>(null);
  let eventSource: EventSource | null = null;
  let reconnectTimer: number | null = null;
  let reconnectAttempt = 0;
  let authCheckPending = false;
  let active = false;

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
    return active && enabled.value && document.visibilityState === "visible";
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

  async function checkAuthentication() {
    if (authCheckPending) return;
    authCheckPending = true;
    try {
      const session = await api.authSession();
      if (!session.authenticated) window.dispatchEvent(new Event("deckox:unauthorized"));
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        window.dispatchEvent(new Event("deckox:unauthorized"));
      }
    } finally {
      authCheckPending = false;
    }
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
      if (parsed) {
        latest.value = parsed;
        lastReceivedAt.value = Date.now();
      }
    });
    source.onerror = () => {
      source.close();
      if (eventSource === source) eventSource = null;
      void checkAuthentication();
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

  function start() {
    if (active) return;
    active = true;
    document.addEventListener("visibilitychange", handleVisibilityChange);
    connect();
  }

  function stop() {
    active = false;
    document.removeEventListener("visibilitychange", handleVisibilityChange);
    disconnect();
  }

  return { status, latest, lastReceivedAt, start, stop, reconnect: connect };
}

<script setup lang="ts">
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, type TerminalStatus } from "../api/client";
import { apiErrorKey } from "../api/errors";
import { parseTerminalServerMessage, terminalWebSocketUrl } from "../terminal";

type ConnectionStatus = "disconnected" | "connecting" | "connected";

const { t } = useI18n();
const terminalElement = ref<HTMLElement | null>(null);
const capability = ref<TerminalStatus | null>(null);
const loading = ref(true);
const errorKey = ref<string | null>(null);
const connectionStatus = ref<ConnectionStatus>("disconnected");
let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let socket: WebSocket | null = null;
let resizeObserver: ResizeObserver | null = null;
let inputDisposable: { dispose: () => void } | null = null;
let intentionalClose = false;

function writeNotice(message: string) {
  terminal?.writeln(`\r\n[Deckox] ${message}\r\n`);
}

function sendResize() {
  if (!terminal || socket?.readyState !== WebSocket.OPEN) return;
  socket.send(JSON.stringify({ type: "resize", cols: terminal.cols, rows: terminal.rows }));
}

function fitTerminal() {
  if (!terminalElement.value || !fitAddon) return;
  fitAddon.fit();
  sendResize();
}

function initializeTerminal() {
  if (!terminalElement.value || terminal) return;
  terminal = new Terminal({
    allowProposedApi: false,
    convertEol: true,
    cursorBlink: true,
    fontFamily: "ui-monospace, SFMono-Regular, Consolas, monospace",
    fontSize: 13,
    rows: 28,
    scrollback: 2_000,
    theme: {
      background: "#1e2329",
      foreground: "#e7e9ec",
      cursor: "#f2f3f5",
      selectionBackground: "#41688a99",
    },
  });
  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.open(terminalElement.value);
  inputDisposable = terminal.onData((data) => {
    if (socket?.readyState === WebSocket.OPEN) socket.send(new TextEncoder().encode(data));
  });
  resizeObserver = new ResizeObserver(fitTerminal);
  resizeObserver.observe(terminalElement.value);
  fitTerminal();
}

function disconnect() {
  intentionalClose = true;
  socket?.close(1000, "client closed");
  socket = null;
  connectionStatus.value = "disconnected";
}

function handleServerMessage(event: MessageEvent) {
  if (typeof event.data !== "string") {
    if (event.data instanceof ArrayBuffer) terminal?.write(new Uint8Array(event.data));
    return;
  }
  const message = parseTerminalServerMessage(event.data);
  if (!message) return;
  if (message.type === "ready") {
    connectionStatus.value = "connected";
    sendResize();
    terminal?.focus();
  } else if (message.type === "exit") {
    intentionalClose = true;
    writeNotice(t("console.shellExited"));
  } else {
    intentionalClose = true;
    writeNotice(t(`console.errors.${message.code}`));
  }
}

function connect() {
  if (!capability.value?.enabled || connectionStatus.value !== "disconnected") return;
  intentionalClose = false;
  errorKey.value = null;
  connectionStatus.value = "connecting";
  terminal?.clear();
  writeNotice(t("console.connecting"));
  const nextSocket = new WebSocket(terminalWebSocketUrl(window.location));
  nextSocket.binaryType = "arraybuffer";
  socket = nextSocket;
  nextSocket.onmessage = handleServerMessage;
  nextSocket.onerror = () => {
    errorKey.value = "errors.terminalConnection";
  };
  nextSocket.onclose = () => {
    if (socket === nextSocket) socket = null;
    connectionStatus.value = "disconnected";
    if (!intentionalClose) writeNotice(t("console.disconnected"));
  };
}

async function load() {
  loading.value = true;
  errorKey.value = null;
  try {
    capability.value = await api.terminalStatus();
    if (capability.value.enabled) {
      await nextTick();
      initializeTerminal();
      connect();
    }
  } catch (caught) {
    errorKey.value = apiErrorKey(caught, "errors.terminalStatus");
  } finally {
    loading.value = false;
  }
}

function handleVisibilityChange() {
  if (document.visibilityState === "hidden") disconnect();
}

onMounted(() => {
  document.addEventListener("visibilitychange", handleVisibilityChange);
  void load();
});

onBeforeUnmount(() => {
  document.removeEventListener("visibilitychange", handleVisibilityChange);
  disconnect();
  resizeObserver?.disconnect();
  inputDisposable?.dispose();
  terminal?.dispose();
});
</script>

<template>
  <div class="view console-view">
    <header class="view-header">
      <div>
        <h1>{{ t("console.title") }}</h1>
        <p class="subtitle">
          {{ t("console.subtitle") }}
        </p>
      </div>
      <div class="header-actions">
        <span
          :class="['stream-state', connectionStatus]"
          role="status"
        >
          {{ t(`console.states.${connectionStatus}`) }}
        </span>
        <button
          v-if="capability?.enabled"
          class="button"
          type="button"
          :disabled="connectionStatus === 'connecting'"
          @click="connectionStatus === 'connected' ? disconnect() : connect()"
        >
          {{ connectionStatus === "connected" ? t("console.disconnect") : t("console.connect") }}
        </button>
      </div>
    </header>

    <div
      v-if="errorKey"
      class="notice error"
      role="alert"
    >
      {{ t(errorKey) }}
    </div>
    <div
      v-else-if="!loading && capability && !capability.enabled"
      class="notice warning"
    >
      {{ t("console.disabled") }}
    </div>

    <div
      v-if="capability?.enabled"
      class="terminal-note"
    >
      <strong>{{ t("console.nonRoot") }}</strong>
      <span>{{ t("console.limit", { count: capability?.max_sessions ?? 2, minutes: Math.round((capability?.idle_timeout_seconds ?? 900) / 60) }) }}</span>
    </div>
    <div
      v-if="capability?.enabled"
      ref="terminalElement"
      class="terminal-surface"
      :aria-label="t('console.terminalLabel')"
    />
  </div>
</template>

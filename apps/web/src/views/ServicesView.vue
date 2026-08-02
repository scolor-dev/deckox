<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  api,
  type ServiceLogEntry,
  type ServiceLogPriority,
  type ServiceSummary,
} from "../api/client";
import { apiErrorKey } from "../api/errors";
import { notify } from "../notifications";

const { t, locale } = useI18n();

const services = ref<ServiceSummary[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
const query = ref("");
const pending = ref<string | null>(null);
const logService = ref<ServiceSummary | null>(null);
const logEntries = ref<ServiceLogEntry[]>([]);
const logLines = ref(100);
const logPriority = ref<ServiceLogPriority>("all");
const logLoading = ref(false);
const logError = ref<string | null>(null);

const LOG_LINE_OPTIONS = [50, 100, 200, 500] as const;
const LOG_PRIORITY_OPTIONS: ServiceLogPriority[] = ["all", "error", "warning", "info"];

const filteredServices = computed(() => {
  const needle = query.value.trim().toLowerCase();
  if (!needle) return services.value;
  return services.value.filter((service) =>
    `${service.id} ${service.description}`.toLowerCase().includes(needle),
  );
});

const runningCount = computed(
  () => services.value.filter((service) => service.active_state === "active").length,
);

function activeStateLabel(state: string) {
  return t(state === "active" ? "services.running" : state === "failed" ? "services.failed" : "services.stopped");
}

function unitStateLabel(state: string | null) {
  if (state === "enabled") return t("services.enabled");
  if (state === "disabled") return t("services.disabled");
  if (state === "static") return t("services.static");
  return state ?? t("common.none");
}

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    services.value = await api.services();
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.services"));
  } finally {
    loading.value = false;
  }
}

async function runAction(
  service: ServiceSummary,
  action: "start" | "stop" | "restart" | "enable" | "disable",
) {
  if ((action === "stop" || action === "restart") &&
      !window.confirm(t(action === "stop" ? "services.confirmStop" : "services.confirmRestart", { id: service.id }))) {
    return;
  }
  if (action === "disable" && !window.confirm(t("services.confirmDisable", { id: service.id }))) return;

  pending.value = `${service.id}:${action}`;
  error.value = null;
  try {
    await api.serviceAction(service.id, action);
    notify("success", t("services.completed", { id: service.id }));
    await refresh();
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.serviceAction"));
    notify("error", error.value);
  } finally {
    pending.value = null;
  }
}

function priorityClass(priority: number) {
  if (priority <= 3) return "error";
  if (priority === 4) return "warning";
  return "info";
}

function priorityLabel(priority: number) {
  return t(`services.logPriorityValue.${priorityClass(priority)}`);
}

function formatLogTimestamp(timestampMs: number) {
  const date = new Date(timestampMs);
  if (!Number.isFinite(timestampMs) || timestampMs <= 0 || Number.isNaN(date.valueOf())) {
    return t("common.none");
  }
  return new Intl.DateTimeFormat(locale.value, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(date);
}

function logDateTime(timestampMs: number) {
  const date = new Date(timestampMs);
  return Number.isNaN(date.valueOf()) ? undefined : date.toISOString();
}

async function loadLogs() {
  if (!logService.value) return;
  logLoading.value = true;
  logError.value = null;
  try {
    const result = await api.serviceLogs(
      logService.value.id,
      logLines.value,
      logPriority.value,
    );
    logEntries.value = result.entries;
  } catch (cause) {
    logEntries.value = [];
    logError.value = t(apiErrorKey(cause, "errors.serviceLogs"));
  } finally {
    logLoading.value = false;
  }
}

function openLogs(service: ServiceSummary) {
  logService.value = service;
  logEntries.value = [];
  logError.value = null;
  void loadLogs();
}

function closeLogs() {
  logService.value = null;
  logEntries.value = [];
  logError.value = null;
}

onMounted(refresh);
</script>

<template>
  <div class="view">
    <header class="view-header">
      <div>
        <h1>{{ t("services.title") }}</h1>
        <p class="subtitle">
          {{ t("services.summary", { total: services.length, running: runningCount }) }}
        </p>
      </div>
      <button
        class="button"
        type="button"
        :disabled="loading"
        @click="refresh"
      >
        {{ loading ? t("common.loading") : t("common.refresh") }}
      </button>
    </header>

    <div
      v-if="error"
      class="notice error"
    >
      {{ error }}
    </div>
    <section class="table-panel">
      <div class="table-toolbar">
        <label class="search">
          <span class="sr-only">{{ t("services.search") }}</span>
          <input
            v-model="query"
            type="search"
            :placeholder="t('services.searchPlaceholder')"
          >
        </label>
        <span class="table-count">{{ t("services.count", { count: filteredServices.length }) }}</span>
      </div>

      <div class="table-scroll">
        <table>
          <thead><tr><th>{{ t("services.service") }}</th><th>{{ t("services.state") }}</th><th>{{ t("services.startup") }}</th><th>{{ t("services.actions") }}</th></tr></thead>
          <tbody>
            <tr v-if="loading && services.length === 0">
              <td
                colspan="4"
                class="empty"
              >
                {{ t("services.loading") }}
              </td>
            </tr>
            <tr v-else-if="filteredServices.length === 0">
              <td
                colspan="4"
                class="empty"
              >
                {{ t("services.empty") }}
              </td>
            </tr>
            <tr
              v-for="service in filteredServices"
              :key="service.id"
            >
              <td>
                <strong class="service-name">{{ service.id }}</strong>
                <small>{{ service.description || t("services.noDescription") }}</small>
              </td>
              <td>
                <span :class="['state-badge', service.active_state === 'active' ? 'active' : 'inactive']">
                  {{ activeStateLabel(service.active_state) }}
                </span>
                <small>{{ service.sub_state }}</small>
              </td>
              <td><span class="unit-state">{{ unitStateLabel(service.unit_file_state) }}</span></td>
              <td>
                <div
                  v-if="service.control_allowed"
                  class="actions"
                >
                  <button
                    class="action-button"
                    type="button"
                    :disabled="pending !== null || service.active_state === 'active'"
                    @click="runAction(service, 'start')"
                  >
                    {{ t("services.start") }}
                  </button>
                  <button
                    class="action-button"
                    type="button"
                    :disabled="pending !== null || service.active_state !== 'active'"
                    @click="runAction(service, 'restart')"
                  >
                    {{ t("services.restart") }}
                  </button>
                  <button
                    class="action-button danger"
                    type="button"
                    :disabled="pending !== null || service.active_state !== 'active'"
                    @click="runAction(service, 'stop')"
                  >
                    {{ t("services.stop") }}
                  </button>
                  <button
                    v-if="service.unit_file_state === 'disabled'"
                    class="action-button"
                    type="button"
                    :disabled="pending !== null"
                    @click="runAction(service, 'enable')"
                  >
                    {{ t("services.enable") }}
                  </button>
                  <button
                    v-else-if="service.unit_file_state === 'enabled'"
                    class="action-button"
                    type="button"
                    :disabled="pending !== null"
                    @click="runAction(service, 'disable')"
                  >
                    {{ t("services.disable") }}
                  </button>
                  <button
                    class="action-button"
                    type="button"
                    :disabled="pending !== null"
                    @click="openLogs(service)"
                  >
                    {{ t("services.logs") }}
                  </button>
                </div>
                <span
                  v-else
                  class="locked"
                >{{ t("services.readOnly") }}</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <aside class="inline-note">
      {{ t("services.allowlist") }}
    </aside>

    <div
      v-if="logService"
      class="dialog-backdrop"
      @click.self="closeLogs"
    >
      <section
        class="log-dialog"
        role="dialog"
        aria-modal="true"
        :aria-label="t('services.logTitle', { id: logService.id })"
      >
        <header class="log-dialog-header">
          <div>
            <h2>{{ t("services.logTitle", { id: logService.id }) }}</h2>
            <small>{{ t("services.logDescription") }}</small>
          </div>
          <button
            class="button"
            type="button"
            @click="closeLogs"
          >
            {{ t("common.close") }}
          </button>
        </header>
        <div class="log-toolbar">
          <label>
            <span>{{ t("services.logLines") }}</span>
            <select v-model.number="logLines">
              <option
                v-for="lines in LOG_LINE_OPTIONS"
                :key="lines"
                :value="lines"
              >
                {{ t("services.logLinesValue", { count: lines }) }}
              </option>
            </select>
          </label>
          <label>
            <span>{{ t("services.logPriorityLabel") }}</span>
            <select v-model="logPriority">
              <option
                v-for="priority in LOG_PRIORITY_OPTIONS"
                :key="priority"
                :value="priority"
              >
                {{ t(`services.logPriority.${priority}`) }}
              </option>
            </select>
          </label>
          <button
            class="button"
            type="button"
            :disabled="logLoading"
            @click="loadLogs"
          >
            {{ logLoading ? t("common.loading") : t("common.refresh") }}
          </button>
        </div>
        <div
          v-if="logError"
          class="notice error log-notice"
          role="alert"
        >
          {{ logError }}
        </div>
        <div
          v-else-if="logLoading"
          class="log-empty"
        >
          {{ t("services.logLoading") }}
        </div>
        <div
          v-else-if="logEntries.length === 0"
          class="log-empty"
        >
          {{ t("services.logEmpty") }}
        </div>
        <ol
          v-else
          class="log-list"
        >
          <li
            v-for="(entry, index) in logEntries"
            :key="`${entry.timestamp_ms}-${index}`"
            :class="['log-entry', priorityClass(entry.priority)]"
          >
            <div class="log-meta">
              <time :datetime="logDateTime(entry.timestamp_ms)">{{ formatLogTimestamp(entry.timestamp_ms) }}</time>
              <span>{{ priorityLabel(entry.priority) }}</span>
              <span v-if="entry.process">{{ entry.process }}<template v-if="entry.pid !== null">[{{ entry.pid }}]</template></span>
            </div>
            <pre>{{ entry.message }}</pre>
          </li>
        </ol>
      </section>
    </div>
  </div>
</template>

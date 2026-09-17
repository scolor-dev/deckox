<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  api,
  type CreateScheduleRequest,
  type ScheduleAction,
  type ServiceLogEntry,
  type ServiceLogPriority,
  type ServiceSchedule,
  type ServiceSummary,
} from "../api/client";
import { apiErrorKey } from "../api/errors";
import { notify } from "../notifications";
import { preferences, type ServiceTagFilterKey } from "../preferences";

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
const logDownloading = ref(false);
const logError = ref<string | null>(null);

const LOG_LINE_OPTIONS = [50, 100, 200, 500] as const;
const LOG_PRIORITY_OPTIONS: ServiceLogPriority[] = ["all", "error", "warning", "info"];

const schedules = ref<ServiceSchedule[]>([]);
const schedulesLoading = ref(true);
const schedulesError = ref<string | null>(null);
const schedulePending = ref<string | null>(null);

const scheduleServiceId = ref("");
const scheduleAction = ref<ScheduleAction>("restart");
const scheduleHour = ref(3);
const scheduleMinute = ref(0);
const scheduleWeekdays = ref<number[]>([1, 2, 3, 4, 5, 6, 7]);

// ISO weekday numbers (1 = Monday ... 7 = Sunday), matching ServiceSchedule.
const WEEKDAY_OPTIONS = [1, 2, 3, 4, 5, 6, 7] as const;
const SCHEDULE_ACTION_OPTIONS: ScheduleAction[] = ["start", "stop", "restart"];

function serviceTags(service: ServiceSummary): ServiceTagFilterKey[] {
  const tags: ServiceTagFilterKey[] = [];
  if (service.deckox_managed) tags.push("deckox");
  if (service.standard_system) tags.push("standard");
  if (service.product) tags.push(service.product);
  return tags;
}

function tagLabel(tag: ServiceTagFilterKey) {
  if (tag === "deckox") return t("services.tagDeckox");
  if (tag === "standard") return t("services.tagStandard");
  if (tag === "other") return t("services.tagOther");
  return tag;
}

// "standard"/"deckox"/"other" get their own styling; any other tag is a
// recognized product name and shares one generic "product" style, since the
// set of possible products is open-ended.
function tagClass(tag: ServiceTagFilterKey) {
  return tag === "standard" || tag === "deckox" || tag === "other" ? tag : "product";
}

function filterKeys(tags: ServiceTagFilterKey[]): ServiceTagFilterKey[] {
  return tags.length ? tags : ["other"];
}

function isTagHidden(tag: ServiceTagFilterKey) {
  return preferences.hiddenServiceTags.includes(tag);
}

function toggleTag(tag: ServiceTagFilterKey) {
  preferences.hiddenServiceTags = isTagHidden(tag)
    ? preferences.hiddenServiceTags.filter((hidden) => hidden !== tag)
    : [...preferences.hiddenServiceTags, tag];
}

// Built from whatever tags are actually present on this host (plus the
// fixed "other" bucket for untagged services), rather than a hardcoded list,
// since recognized products vary per host.
const availableTagKeys = computed<ServiceTagFilterKey[]>(() => {
  const productTags = new Set<string>();
  for (const service of services.value) {
    if (service.product) productTags.add(service.product);
  }
  return ["standard", "deckox", ...Array.from(productTags).sort(), "other"];
});

const filteredServices = computed(() => {
  const needle = query.value.trim().toLowerCase();
  return services.value.filter((service) => {
    const tags = serviceTags(service);
    const keys = filterKeys(tags);
    if (keys.every((key) => isTagHidden(key))) return false;
    if (!needle) return true;
    const haystack = [service.id, service.description, ...keys.map(tagLabel)]
      .join(" ")
      .toLowerCase();
    return haystack.includes(needle);
  });
});

const runningCount = computed(
  () => services.value.filter((service) => service.active_state === "active").length,
);

function activeStateLabel(state: string) {
  return t(state === "active" ? "services.running" : state === "failed" ? "services.failed" : "services.stopped");
}

function activeStateClass(state: string) {
  return state === "active" ? "active" : state === "failed" ? "failed" : "inactive";
}

function unitStateLabel(state: string | null) {
  if (state === "enabled") return t("services.enabled");
  if (state === "disabled") return t("services.disabled");
  if (state === "static") return t("services.static");
  return state ?? t("common.none");
}

// Only services already eligible for manual start/stop/restart (and not
// Deckox's own units) can be scheduled — scheduling is automated timing for
// the same operation, not a new permission.
const schedulableServices = computed(() =>
  services.value.filter((service) => service.control_allowed && !service.deckox_managed),
);

const scheduleTime = computed({
  get: () => `${String(scheduleHour.value).padStart(2, "0")}:${String(scheduleMinute.value).padStart(2, "0")}`,
  set: (value: string) => {
    const [hour, minute] = value.split(":").map(Number);
    if (Number.isInteger(hour)) scheduleHour.value = hour;
    if (Number.isInteger(minute)) scheduleMinute.value = minute;
  },
});

function weekdayLabel(day: number) {
  return t(`services.weekday.${String(day)}`);
}

function toggleScheduleWeekday(day: number) {
  scheduleWeekdays.value = scheduleWeekdays.value.includes(day)
    ? scheduleWeekdays.value.filter((existing) => existing !== day)
    : [...scheduleWeekdays.value, day].sort((a, b) => a - b);
}

function scheduleTimeLabel(schedule: ServiceSchedule) {
  return `${String(schedule.hour).padStart(2, "0")}:${String(schedule.minute).padStart(2, "0")}`;
}

function scheduleWeekdaysLabel(schedule: ServiceSchedule) {
  return schedule.weekdays.length === 7
    ? t("services.scheduleEveryDay")
    : schedule.weekdays.map(weekdayLabel).join(t("services.scheduleWeekdaySeparator"));
}

function scheduleLastRunLabel(schedule: ServiceSchedule) {
  if (schedule.last_run_at_ms == null) return t("services.scheduleNeverRun");
  return new Intl.DateTimeFormat(locale.value, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(schedule.last_run_at_ms));
}

async function loadSchedules() {
  schedulesLoading.value = true;
  schedulesError.value = null;
  try {
    schedules.value = await api.schedules();
  } catch (cause) {
    schedulesError.value = t(apiErrorKey(cause, "errors.schedules"));
  } finally {
    schedulesLoading.value = false;
  }
}

async function createSchedule() {
  if (!scheduleServiceId.value || scheduleWeekdays.value.length === 0) return;
  schedulePending.value = "create";
  schedulesError.value = null;
  try {
    const payload: CreateScheduleRequest = {
      service_id: scheduleServiceId.value,
      action: scheduleAction.value,
      hour: scheduleHour.value,
      minute: scheduleMinute.value,
      weekdays: scheduleWeekdays.value,
    };
    await api.createSchedule(payload);
    notify("success", t("services.scheduleCreated"));
    await loadSchedules();
  } catch (cause) {
    schedulesError.value = t(apiErrorKey(cause, "errors.schedules"));
    notify("error", schedulesError.value);
  } finally {
    schedulePending.value = null;
  }
}

async function deleteSchedule(schedule: ServiceSchedule) {
  if (!window.confirm(t("services.confirmDeleteSchedule", { id: schedule.service_id }))) return;
  schedulePending.value = schedule.id;
  schedulesError.value = null;
  try {
    await api.deleteSchedule(schedule.id);
    await loadSchedules();
  } catch (cause) {
    schedulesError.value = t(apiErrorKey(cause, "errors.schedules"));
    notify("error", schedulesError.value);
  } finally {
    schedulePending.value = null;
  }
}

async function toggleScheduleEnabled(schedule: ServiceSchedule) {
  schedulePending.value = schedule.id;
  schedulesError.value = null;
  try {
    await api.setScheduleEnabled(schedule.id, !schedule.enabled);
    await loadSchedules();
  } catch (cause) {
    schedulesError.value = t(apiErrorKey(cause, "errors.schedules"));
    notify("error", schedulesError.value);
  } finally {
    schedulePending.value = null;
  }
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
  action: "start" | "stop" | "restart" | "enable" | "disable" | "allow" | "disallow",
) {
  if ((action === "stop" || action === "restart") &&
      !window.confirm(t(action === "stop" ? "services.confirmStop" : "services.confirmRestart", { id: service.id }))) {
    return;
  }
  if (action === "disable" && !window.confirm(t("services.confirmDisable", { id: service.id }))) return;
  if (action === "allow" && !window.confirm(t("services.confirmAllow", { id: service.id }))) return;
  if (action === "disallow" && !window.confirm(t("services.confirmDisallow", { id: service.id }))) return;

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

async function downloadLogs() {
  if (!logService.value) return;
  logDownloading.value = true;
  logError.value = null;
  try {
    const blob = await api.serviceLogsReport(logService.value.id, logLines.value, logPriority.value);
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `deckox-service-logs-${logService.value.id}.json`;
    anchor.hidden = true;
    document.body.append(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
    notify("success", t("services.logReportSaved"));
  } catch (cause) {
    logError.value = t(apiErrorKey(cause, "errors.serviceLogs"));
    notify("error", logError.value);
  } finally {
    logDownloading.value = false;
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

onMounted(() => {
  void refresh();
  void loadSchedules();
});
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
        <fieldset class="tag-toggles">
          <legend class="sr-only">
            {{ t("services.tagVisibility") }}
          </legend>
          <label
            v-for="tag in availableTagKeys"
            :key="tag"
            :class="['tag-toggle', tagClass(tag), { off: isTagHidden(tag) }]"
          >
            <input
              type="checkbox"
              :checked="!isTagHidden(tag)"
              @change="toggleTag(tag)"
            >
            {{ tagLabel(tag) }}
          </label>
        </fieldset>
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
              :class="{ 'row-failed': service.active_state === 'failed' }"
            >
              <td>
                <strong class="service-name">{{ service.id }}</strong>
                <span
                  v-if="serviceTags(service).length"
                  class="tag-badges"
                >
                  <span
                    v-for="tag in serviceTags(service)"
                    :key="tag"
                    :class="['tag-badge', tagClass(tag)]"
                  >{{ tagLabel(tag) }}</span>
                </span>
                <small>{{ service.description || t("services.noDescription") }}</small>
              </td>
              <td>
                <span :class="['state-badge', activeStateClass(service.active_state)]">
                  {{ activeStateLabel(service.active_state) }}
                </span>
                <small>{{ service.sub_state }}</small>
              </td>
              <td><span class="unit-state">{{ unitStateLabel(service.unit_file_state) }}</span></td>
              <td>
                <div class="actions">
                  <template v-if="service.control_allowed">
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
                    <button
                      class="action-button danger"
                      type="button"
                      :disabled="pending !== null"
                      @click="runAction(service, 'disallow')"
                    >
                      {{ t("services.disallow") }}
                    </button>
                  </template>
                  <template v-else>
                    <button
                      v-if="!service.deckox_managed"
                      class="action-button"
                      type="button"
                      :disabled="pending !== null"
                      @click="runAction(service, 'allow')"
                    >
                      {{ t("services.allow") }}
                    </button>
                    <span class="locked">{{ t("services.readOnly") }}</span>
                  </template>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <aside class="inline-note">
      {{ t("services.allowlist") }}
    </aside>

    <section class="table-panel">
      <div class="table-toolbar">
        <h2 class="panel-title">
          {{ t("services.scheduleTitle") }}
        </h2>
        <span class="table-count">{{ t("services.count", { count: schedules.length }) }}</span>
      </div>

      <div
        v-if="schedulesError"
        class="notice error"
      >
        {{ schedulesError }}
      </div>

      <div class="table-scroll">
        <table>
          <thead>
            <tr>
              <th>{{ t("services.service") }}</th>
              <th>{{ t("services.scheduleAction") }}</th>
              <th>{{ t("services.scheduleWeekdays") }}</th>
              <th>{{ t("services.scheduleTime") }}</th>
              <th>{{ t("services.scheduleLastRun") }}</th>
              <th>{{ t("services.actions") }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="schedulesLoading && schedules.length === 0">
              <td
                colspan="6"
                class="empty"
              >
                {{ t("common.loading") }}
              </td>
            </tr>
            <tr v-else-if="schedules.length === 0">
              <td
                colspan="6"
                class="empty"
              >
                {{ t("services.scheduleEmpty") }}
              </td>
            </tr>
            <tr
              v-for="schedule in schedules"
              :key="schedule.id"
            >
              <td><strong class="service-name">{{ schedule.service_id }}</strong></td>
              <td>{{ t(`services.${schedule.action}`) }}</td>
              <td>{{ scheduleWeekdaysLabel(schedule) }}</td>
              <td>{{ scheduleTimeLabel(schedule) }}</td>
              <td>
                <span>{{ scheduleLastRunLabel(schedule) }}</span>
                <small v-if="schedule.last_result">{{ schedule.last_result }}</small>
              </td>
              <td>
                <div class="actions">
                  <button
                    class="action-button"
                    type="button"
                    :disabled="schedulePending !== null"
                    @click="toggleScheduleEnabled(schedule)"
                  >
                    {{ schedule.enabled ? t("services.scheduleDisable") : t("services.scheduleEnable") }}
                  </button>
                  <button
                    class="action-button danger"
                    type="button"
                    :disabled="schedulePending !== null"
                    @click="deleteSchedule(schedule)"
                  >
                    {{ t("services.scheduleDelete") }}
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <form
        class="schedule-form"
        @submit.prevent="createSchedule"
      >
        <label>
          <span>{{ t("services.service") }}</span>
          <select
            v-model="scheduleServiceId"
            required
          >
            <option
              value=""
              disabled
            >
              {{ t("services.scheduleSelectService") }}
            </option>
            <option
              v-for="service in schedulableServices"
              :key="service.id"
              :value="service.id"
            >
              {{ service.id }}
            </option>
          </select>
        </label>
        <label>
          <span>{{ t("services.scheduleAction") }}</span>
          <select v-model="scheduleAction">
            <option
              v-for="action in SCHEDULE_ACTION_OPTIONS"
              :key="action"
              :value="action"
            >
              {{ t(`services.${action}`) }}
            </option>
          </select>
        </label>
        <label>
          <span>{{ t("services.scheduleTime") }}</span>
          <input
            v-model="scheduleTime"
            type="time"
            required
          >
        </label>
        <fieldset class="schedule-weekdays">
          <legend>{{ t("services.scheduleWeekdays") }}</legend>
          <label
            v-for="day in WEEKDAY_OPTIONS"
            :key="day"
            class="tag-toggle"
          >
            <input
              type="checkbox"
              :checked="scheduleWeekdays.includes(day)"
              @change="toggleScheduleWeekday(day)"
            >
            {{ weekdayLabel(day) }}
          </label>
        </fieldset>
        <button
          class="button"
          type="submit"
          :disabled="schedulePending !== null || !scheduleServiceId || scheduleWeekdays.length === 0"
        >
          {{ t("services.scheduleAdd") }}
        </button>
      </form>
      <aside class="inline-note">
        {{ t("services.scheduleNote") }}
      </aside>
    </section>

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
          <div class="header-actions">
            <button
              class="button"
              type="button"
              :disabled="logDownloading"
              @click="downloadLogs"
            >
              {{ logDownloading ? t("services.logDownloading") : t("services.logDownload") }}
            </button>
            <button
              class="button"
              type="button"
              @click="closeLogs"
            >
              {{ t("common.close") }}
            </button>
          </div>
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

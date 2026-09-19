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
import {
  AppButton,
  AppModal,
  AppStack,
  InfoNote,
  LogEntry,
  LogList,
  NoticeBanner,
  PageHeader,
  SelectField,
  StateBadge,
  TableToolbar,
  TablePanel,
  TagBadge,
  TagToggle,
  TagToggleGroup,
  TextField,
} from "../design-system/components";
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
function badgeCategory(tag: ServiceTagFilterKey) {
  const category = tagClass(tag);
  return category === "other" ? "standard" : category;
}

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

const logLinesValue = computed({
  get: () => String(logLines.value),
  set: (value: string) => {
    logLines.value = Number(value);
  },
});
const logPriorityValue = computed({
  get: () => logPriority.value,
  set: (value: string) => {
    logPriority.value = value as ServiceLogPriority;
  },
});
const scheduleActionValue = computed({
  get: () => scheduleAction.value,
  set: (value: string) => {
    scheduleAction.value = value as ScheduleAction;
  },
});
const logLineOptions = computed(() =>
  LOG_LINE_OPTIONS.map((lines) => ({ value: String(lines), label: t("services.logLinesValue", { count: lines }) })),
);
const logPriorityOptions = computed(() =>
  LOG_PRIORITY_OPTIONS.map((priority) => ({ value: priority, label: t(`services.logPriority.${priority}`) })),
);
const scheduleActionOptions = computed(() =>
  SCHEDULE_ACTION_OPTIONS.map((action) => ({ value: action, label: t(`services.${action}`) })),
);
const scheduleServiceOptions = computed(() => [
  { value: "", label: t("services.scheduleSelectService") },
  ...schedulableServices.value.map((service) => ({ value: service.id, label: service.id })),
]);

function logPriorityKind(priority: number): "error" | "warning" | "info" {
  return priorityClass(priority);
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
    <PageHeader
      :title="t('services.title')"
      :subtitle="t('services.summary', { total: services.length, running: runningCount })"
    >
      <template #actions>
        <AppButton
          :disabled="loading"
          @click="refresh"
        >
          {{ loading ? t("common.loading") : t("common.refresh") }}
        </AppButton>
      </template>
    </PageHeader>

    <NoticeBanner
      v-if="error"
      tone="error"
    >
      {{ error }}
    </NoticeBanner>

    <TablePanel
      :loading="loading && services.length === 0"
      :empty="filteredServices.length === 0"
      :empty-message="t('services.empty')"
    >
      <template #toolbar>
        <TableToolbar :count="t('services.count', { count: filteredServices.length })">
          <template #search>
            <TextField
              id="services-search"
              v-model="query"
              :label="t('services.search')"
              type="search"
              :placeholder="t('services.searchPlaceholder')"
              label-hidden
            />
          </template>
          <template #filters>
            <TagToggleGroup :label="t('services.tagVisibility')">
              <TagToggle
                v-for="tag in availableTagKeys"
                :key="tag"
                :category="tagClass(tag)"
                :checked="!isTagHidden(tag)"
                @update:checked="toggleTag(tag)"
              >
                {{ tagLabel(tag) }}
              </TagToggle>
            </TagToggleGroup>
          </template>
        </TableToolbar>
      </template>
      <template #loading>
        {{ t("services.loading") }}
      </template>
      <table>
        <thead><tr><th>{{ t("services.service") }}</th><th>{{ t("services.state") }}</th><th>{{ t("services.startup") }}</th><th>{{ t("services.actions") }}</th></tr></thead>
        <tbody>
          <tr
            v-for="service in filteredServices"
            :key="service.id"
            :class="{ 'row-failed': service.active_state === 'failed' }"
          >
            <td>
              <AppStack
                direction="row"
                gap="2"
                align="center"
                wrap
              >
                <strong class="service-name">{{ service.id }}</strong>
                <TagBadge
                  v-for="tag in serviceTags(service)"
                  :key="tag"
                  :category="badgeCategory(tag)"
                >
                  {{ tagLabel(tag) }}
                </TagBadge>
              </AppStack>
              <small>{{ service.description || t("services.noDescription") }}</small>
            </td>
            <td>
              <StateBadge :state="activeStateClass(service.active_state)">
                {{ activeStateLabel(service.active_state) }}
              </StateBadge>
              <small>{{ service.sub_state }}</small>
            </td>
            <td><span class="unit-state">{{ unitStateLabel(service.unit_file_state) }}</span></td>
            <td>
              <AppStack
                class="row-actions"
                direction="row"
                gap="2"
                align="center"
              >
                <template v-if="service.control_allowed">
                  <AppButton
                    variant="action"
                    :disabled="pending !== null || service.active_state === 'active'"
                    @click="runAction(service, 'start')"
                  >
                    {{ t("services.start") }}
                  </AppButton>
                  <AppButton
                    variant="action"
                    :disabled="pending !== null || service.active_state !== 'active'"
                    @click="runAction(service, 'restart')"
                  >
                    {{ t("services.restart") }}
                  </AppButton>
                  <AppButton
                    variant="action"
                    danger
                    :disabled="pending !== null || service.active_state !== 'active'"
                    @click="runAction(service, 'stop')"
                  >
                    {{ t("services.stop") }}
                  </AppButton>
                  <AppButton
                    v-if="service.unit_file_state === 'disabled'"
                    variant="action"
                    :disabled="pending !== null"
                    @click="runAction(service, 'enable')"
                  >
                    {{ t("services.enable") }}
                  </AppButton>
                  <AppButton
                    v-else-if="service.unit_file_state === 'enabled'"
                    variant="action"
                    :disabled="pending !== null"
                    @click="runAction(service, 'disable')"
                  >
                    {{ t("services.disable") }}
                  </AppButton>
                  <AppButton
                    variant="action"
                    :disabled="pending !== null"
                    @click="openLogs(service)"
                  >
                    {{ t("services.logs") }}
                  </AppButton>
                  <AppButton
                    variant="action"
                    danger
                    :disabled="pending !== null"
                    @click="runAction(service, 'disallow')"
                  >
                    {{ t("services.disallow") }}
                  </AppButton>
                </template>
                <template v-else>
                  <AppButton
                    v-if="!service.deckox_managed"
                    variant="action"
                    :disabled="pending !== null"
                    @click="runAction(service, 'allow')"
                  >
                    {{ t("services.allow") }}
                  </AppButton>
                  <span class="locked">{{ t("services.readOnly") }}</span>
                </template>
              </AppStack>
            </td>
          </tr>
        </tbody>
      </table>
    </TablePanel>

    <InfoNote>{{ t("services.allowlist") }}</InfoNote>

    <TablePanel
      :loading="schedulesLoading && schedules.length === 0"
      :empty="schedules.length === 0"
      :empty-message="t('services.scheduleEmpty')"
    >
      <template #toolbar>
        <TableToolbar :count="t('services.count', { count: schedules.length })">
          <template #filters>
            <h2>
              {{ t("services.scheduleTitle") }}
            </h2>
          </template>
        </TableToolbar>
      </template>
      <template
        v-if="schedulesError"
        #note
      >
        <NoticeBanner tone="error">
          {{ schedulesError }}
        </NoticeBanner>
      </template>
      <template #loading>
        {{ t("common.loading") }}
      </template>
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
              <AppStack
                class="row-actions"
                direction="row"
                gap="2"
              >
                <AppButton
                  variant="action"
                  :disabled="schedulePending !== null"
                  @click="toggleScheduleEnabled(schedule)"
                >
                  {{ schedule.enabled ? t("services.scheduleDisable") : t("services.scheduleEnable") }}
                </AppButton>
                <AppButton
                  variant="action"
                  danger
                  :disabled="schedulePending !== null"
                  @click="deleteSchedule(schedule)"
                >
                  {{ t("services.scheduleDelete") }}
                </AppButton>
              </AppStack>
            </td>
          </tr>
        </tbody>
      </table>
    </TablePanel>

    <AppStack
      as="form"
      class="schedule-form"
      direction="row"
      gap="3"
      align="end"
      wrap
      @submit.prevent="createSchedule"
    >
      <SelectField
        id="schedule-service"
        v-model="scheduleServiceId"
        :label="t('services.service')"
        :options="scheduleServiceOptions"
      />
      <SelectField
        id="schedule-action"
        v-model="scheduleActionValue"
        :label="t('services.scheduleAction')"
        :options="scheduleActionOptions"
      />
      <TextField
        id="schedule-time"
        v-model="scheduleTime"
        :label="t('services.scheduleTime')"
        type="time"
        required
      />
      <TagToggleGroup :label="t('services.scheduleWeekdays')">
        <TagToggle
          v-for="day in WEEKDAY_OPTIONS"
          :key="day"
          category="other"
          :checked="scheduleWeekdays.includes(day)"
          @update:checked="toggleScheduleWeekday(day)"
        >
          {{ weekdayLabel(day) }}
        </TagToggle>
      </TagToggleGroup>
      <AppButton
        type="submit"
        :disabled="schedulePending !== null || !scheduleServiceId || scheduleWeekdays.length === 0"
      >
        {{ t("services.scheduleAdd") }}
      </AppButton>
    </AppStack>
    <InfoNote>{{ t("services.scheduleNote") }}</InfoNote>

    <AppModal
      :open="logService !== null"
      :title="logService ? t('services.logTitle', { id: logService.id }) : ''"
      size="large"
      @close="closeLogs"
    >
      <template #header-actions>
        <AppButton
          :disabled="logDownloading"
          @click="downloadLogs"
        >
          {{ logDownloading ? t("services.logDownloading") : t("services.logDownload") }}
        </AppButton>
      </template>
      <AppStack gap="3">
        <InfoNote>{{ t("services.logDescription") }}</InfoNote>
        <AppStack
          direction="row"
          gap="3"
          align="end"
          wrap
        >
          <SelectField
            id="log-lines"
            v-model="logLinesValue"
            :label="t('services.logLines')"
            :options="logLineOptions"
          />
          <SelectField
            id="log-priority"
            v-model="logPriorityValue"
            :label="t('services.logPriorityLabel')"
            :options="logPriorityOptions"
          />
          <AppButton
            :disabled="logLoading"
            @click="loadLogs"
          >
            {{ logLoading ? t("common.loading") : t("common.refresh") }}
          </AppButton>
        </AppStack>
        <NoticeBanner
          v-if="logError"
          tone="error"
        >
          {{ logError }}
        </NoticeBanner>
        <InfoNote v-else-if="logLoading">
          {{ t("services.logLoading") }}
        </InfoNote>
        <InfoNote v-else-if="logEntries.length === 0">
          {{ t("services.logEmpty") }}
        </InfoNote>
        <LogList v-else>
          <LogEntry
            v-for="(entry, index) in logEntries"
            :key="`${entry.timestamp_ms}-${index}`"
            :priority="logPriorityKind(entry.priority)"
            :priority-label="priorityLabel(entry.priority)"
            :timestamp="formatLogTimestamp(entry.timestamp_ms)"
            :iso-timestamp="logDateTime(entry.timestamp_ms)"
            :process="entry.process"
            :pid="entry.pid"
            :message="entry.message"
          />
        </LogList>
      </AppStack>
    </AppModal>
  </div>
</template>

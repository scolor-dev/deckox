<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStaleRefresh } from "../../../composables/useStaleRefresh";
import { servicesList } from "../../../data/sources";
import {
  api,
  type CreateScheduleRequest,
  type ScheduleAction,
  type ServiceSchedule,
} from "../../../api/client";
import { apiErrorKey } from "../../../api/errors";
import {
  AppButton,
  AppHeading,
  AppStack,
  AppText,
  InfoNote,
  NoticeBanner,
  SelectField,
  TablePanel,
  TableToolbar,
  TagToggle,
  TagToggleGroup,
  TextField,
} from "../../../design-system/components";
import { notify } from "../../../notifications";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const { data: serviceData } = servicesList.use();
const services = computed(() => serviceData.value ?? []);

const schedules = ref<ServiceSchedule[]>([]);

const schedulesLoading = ref(true);

const schedulesError = ref<string | null>(null);

const schedulePending = ref<string | null>(null);

const scheduleServiceId = ref("");

const scheduleAction = ref<ScheduleAction>("restart");

const scheduleHour = ref(3);

const scheduleMinute = ref(0);

const scheduleWeekdays = ref<number[]>([1, 2, 3, 4, 5, 6, 7]);

const WEEKDAY_OPTIONS = [1, 2, 3, 4, 5, 6, 7] as const;

const SCHEDULE_ACTION_OPTIONS: ScheduleAction[] = ["start", "stop", "restart"];

const scheduleActionValue = computed({
  get: () => scheduleAction.value,
  set: (value: string) => {
    scheduleAction.value = value as ScheduleAction;
  },
});

const scheduleActionOptions = computed(() =>
  SCHEDULE_ACTION_OPTIONS.map((action) => ({ value: action, label: t(`services.${action}`) })),
);

const scheduleServiceOptions = computed(() => [
  { value: "", label: t("services.scheduleSelectService") },
  ...schedulableServices.value.map((service) => ({ value: service.id, label: service.id })),
]);

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

useStaleRefresh(loadSchedules, 30_000);
</script>

<template>
  <AppStack gap="4">
    <TablePanel
      :loading="schedulesLoading && schedules.length === 0"
      :empty="schedules.length === 0"
      :empty-message="t('services.scheduleEmpty')"
    >
      <template #toolbar>
        <TableToolbar :count="t('services.count', { count: schedules.length })">
          <template #filters>
            <AppHeading>
              {{ t("services.scheduleTitle") }}
            </AppHeading>
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
            <td>
              <AppText
                as="strong"
                mono
                strong
                size="xs"
              >
                {{ schedule.service_id }}
              </AppText>
            </td>
            <td>{{ t(`services.${schedule.action}`) }}</td>
            <td>{{ scheduleWeekdaysLabel(schedule) }}</td>
            <td>{{ scheduleTimeLabel(schedule) }}</td>
            <td>
              <span>{{ scheduleLastRunLabel(schedule) }}</span>
              <AppText
                v-if="schedule.last_result"
                as="small"
                tone="muted"
                size="xs"
              >
                {{ schedule.last_result }}
              </AppText>
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
  </AppStack>
</template>

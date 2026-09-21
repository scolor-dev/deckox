<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStaleRefresh } from "../composables/useStaleRefresh";
import { staleMsOf } from "../modules/registry";
import { AUDIT_REPORT_FILENAME, api, type AuditEvent } from "../api/client";
import { apiErrorKey } from "../api/errors";
import {
  AppButton,
  AppStack,
  NoticeBanner,
  PageHeader,
  SelectField,
  StateBadge,
  TableToolbar,
  TablePanel,
} from "../design-system/components";
import { notify } from "../notifications";

const { t, locale } = useI18n();

const events = ref<AuditEvent[]>([]);
const hasMore = ref(false);
const loading = ref(true);
const loadingMore = ref(false);
const downloading = ref(false);
const error = ref<string | null>(null);
const resultFilter = ref("all");
const eventFilter = ref("all");

const resultOptions = computed(() => ["all", ...new Set(events.value.map((entry) => entry.result))]);
const eventOptions = computed(() => ["all", ...new Set(events.value.map((entry) => entry.event))]);

const filteredEvents = computed(() =>
  events.value.filter(
    (entry) =>
      (resultFilter.value === "all" || entry.result === resultFilter.value)
      && (eventFilter.value === "all" || entry.event === eventFilter.value),
  ),
);

// Event/result identifiers are free-form snake_case strings from the Rust
// audit log (there is no fixed enum to map through i18n), so this only
// tidies punctuation for the dropdown rather than translating each one.
const eventFilterOptions = computed(() =>
  eventOptions.value.map((option) => ({ value: option, label: option === "all" ? t("audit.allEvents") : optionLabel(option) })),
);
const resultFilterOptions = computed(() =>
  resultOptions.value.map((option) => ({ value: option, label: option === "all" ? t("audit.allResults") : optionLabel(option) })),
);

function optionLabel(value: string) {
  return value.replaceAll("_", " ");
}

function formatTime(timestampMs: number) {
  return new Intl.DateTimeFormat(locale.value, {
    dateStyle: "medium",
    timeStyle: "medium",
  }).format(timestampMs);
}

async function fetchData() {
  loading.value = true;
  error.value = null;
  try {
    const page = await api.auditEvents();
    events.value = page.events;
    hasMore.value = page.has_more;
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.audit"));
  } finally {
    loading.value = false;
  }
}

async function loadMore() {
  const oldest = events.value.at(-1);
  if (!oldest) return;
  loadingMore.value = true;
  error.value = null;
  try {
    const page = await api.auditEvents(oldest.timestamp_ms);
    events.value = [...events.value, ...page.events];
    hasMore.value = page.has_more;
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.audit"));
  } finally {
    loadingMore.value = false;
  }
}

async function downloadReport() {
  downloading.value = true;
  error.value = null;
  try {
    const blob = await api.auditReport();
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = AUDIT_REPORT_FILENAME;
    anchor.hidden = true;
    document.body.append(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
    notify("success", t("audit.reportSaved"));
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.auditReport"));
    notify("error", error.value);
  } finally {
    downloading.value = false;
  }
}

const refresh = useStaleRefresh(fetchData, staleMsOf("audit"));
</script>

<template>
  <div class="view audit-view">
    <PageHeader
      :title="t('audit.title')"
      :subtitle="t('audit.subtitle')"
    >
      <template #actions>
        <AppStack
          direction="row"
          gap="2"
          wrap
        >
          <AppButton
            :disabled="loading"
            @click="refresh"
          >
            {{ loading ? t("common.loading") : t("common.refresh") }}
          </AppButton>
          <AppButton
            :disabled="downloading"
            @click="downloadReport"
          >
            {{ downloading ? t("audit.downloading") : t("audit.download") }}
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

    <TablePanel
      :loading="loading && events.length === 0"
      :empty="filteredEvents.length === 0"
      :empty-message="t('audit.empty')"
    >
      <template #toolbar>
        <TableToolbar :count="t('audit.count', { count: filteredEvents.length })">
          <template #filters>
            <AppStack
              direction="row"
              gap="2"
              wrap
            >
              <SelectField
                id="audit-event-filter"
                v-model="eventFilter"
                :label="t('audit.filterEvent')"
                :options="eventFilterOptions"
                label-hidden
              />
              <SelectField
                id="audit-result-filter"
                v-model="resultFilter"
                :label="t('audit.filterResult')"
                :options="resultFilterOptions"
                label-hidden
              />
            </AppStack>
          </template>
        </TableToolbar>
      </template>
      <template #loading>
        {{ t("audit.loading") }}
      </template>
      <table>
        <thead>
          <tr>
            <th>{{ t("audit.time") }}</th>
            <th>{{ t("audit.event") }}</th>
            <th>{{ t("audit.actor") }}</th>
            <th>{{ t("audit.sourceIp") }}</th>
            <th>{{ t("audit.result") }}</th>
            <th>{{ t("audit.detail") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(entry, index) in filteredEvents"
            :key="`${entry.timestamp_ms}-${index}`"
            :class="{ 'row-failed': entry.result !== 'success' && entry.result !== 'accepted' }"
          >
            <td>{{ formatTime(entry.timestamp_ms) }}</td>
            <td class="mono">
              {{ entry.event }}
            </td>
            <td>{{ entry.actor }}</td>
            <td class="mono">
              {{ entry.source_ip }}
            </td>
            <td>
              <StateBadge :state="entry.result === 'success' || entry.result === 'accepted' ? 'active' : 'failed'">
                {{ entry.result }}
              </StateBadge>
            </td>
            <td>{{ entry.detail ?? t("common.none") }}</td>
          </tr>
        </tbody>
      </table>
      <template #footer>
        <AppButton
          v-if="hasMore"
          variant="action"
          :disabled="loadingMore"
          @click="loadMore"
        >
          {{ loadingMore ? t("common.loading") : t("audit.loadMore") }}
        </AppButton>
      </template>
    </TablePanel>
  </div>
</template>

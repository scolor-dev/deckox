<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { AUDIT_REPORT_FILENAME, api, type AuditEvent } from "../api/client";
import { apiErrorKey } from "../api/errors";
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

function formatTime(timestampMs: number) {
  return new Intl.DateTimeFormat(locale.value, {
    dateStyle: "medium",
    timeStyle: "medium",
  }).format(timestampMs);
}

async function refresh() {
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

onMounted(refresh);
</script>

<template>
  <div class="view audit-view">
    <header class="view-header">
      <div>
        <h1>{{ t("audit.title") }}</h1>
        <p class="subtitle">
          {{ t("audit.subtitle") }}
        </p>
      </div>
      <div class="header-actions">
        <button
          class="button"
          type="button"
          :disabled="loading"
          @click="refresh"
        >
          {{ loading ? t("common.loading") : t("common.refresh") }}
        </button>
        <button
          class="button"
          type="button"
          :disabled="downloading"
          @click="downloadReport"
        >
          {{ downloading ? t("audit.downloading") : t("audit.download") }}
        </button>
      </div>
    </header>

    <div
      v-if="error"
      class="notice error"
    >
      {{ error }}
    </div>

    <section class="table-panel">
      <div class="table-toolbar">
        <label>
          <span class="sr-only">{{ t("audit.filterEvent") }}</span>
          <select v-model="eventFilter">
            <option
              v-for="option in eventOptions"
              :key="option"
              :value="option"
            >
              {{ option === "all" ? t("audit.allEvents") : option }}
            </option>
          </select>
        </label>
        <label>
          <span class="sr-only">{{ t("audit.filterResult") }}</span>
          <select v-model="resultFilter">
            <option
              v-for="option in resultOptions"
              :key="option"
              :value="option"
            >
              {{ option === "all" ? t("audit.allResults") : option }}
            </option>
          </select>
        </label>
        <span class="table-count">{{ t("audit.count", { count: filteredEvents.length }) }}</span>
      </div>

      <div class="table-scroll">
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
            <tr v-if="loading && events.length === 0">
              <td
                colspan="6"
                class="empty"
              >
                {{ t("audit.loading") }}
              </td>
            </tr>
            <tr v-else-if="filteredEvents.length === 0">
              <td
                colspan="6"
                class="empty"
              >
                {{ t("audit.empty") }}
              </td>
            </tr>
            <tr
              v-for="(entry, index) in filteredEvents"
              :key="`${entry.timestamp_ms}-${index}`"
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
                <span
                  class="state-badge"
                  :class="entry.result === 'success' || entry.result === 'accepted' ? 'active' : 'inactive'"
                >{{ entry.result }}</span>
              </td>
              <td>{{ entry.detail ?? t("common.none") }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <button
        v-if="hasMore"
        class="action-button audit-load-more"
        type="button"
        :disabled="loadingMore"
        @click="loadMore"
      >
        {{ loadingMore ? t("common.loading") : t("audit.loadMore") }}
      </button>
    </section>
  </div>
</template>

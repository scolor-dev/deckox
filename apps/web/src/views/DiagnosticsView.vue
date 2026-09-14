<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  api,
  DIAGNOSTICS_REPORT_FILENAME,
  formatBytes,
  formatUptime,
  type BackupSummary,
  type DeckoxServiceDiagnostic,
  type DiagnosticsResponse,
} from "../api/client";
import { apiErrorKey } from "../api/errors";
import { notify } from "../notifications";

const { t, locale } = useI18n();
const diagnostics = ref<DiagnosticsResponse | null>(null);
const loading = ref(true);
const downloading = ref(false);
const error = ref<string | null>(null);
const backups = ref<BackupSummary[]>([]);
const backupsError = ref<string | null>(null);

const generatedAt = computed(() => {
  if (!diagnostics.value) return t("common.none");
  return new Intl.DateTimeFormat(locale.value, {
    dateStyle: "medium",
    timeStyle: "medium",
  }).format(diagnostics.value.generated_at_ms);
});
const deckoxServices = computed<DeckoxServiceDiagnostic[]>(() => {
  const services = diagnostics.value?.deckox_services;
  if (!services) return [];
  return [
    { id: "deckox-agent.service", state: services.agent },
    { id: "deckox-server.service", state: services.server },
  ];
});

function stateClass(state: string) {
  return state === "active" || state === "running" || state === "ok" ? "active" : "inactive";
}

function enabledLabel(enabled: boolean) {
  return t(enabled ? "diagnostics.enabled" : "diagnostics.disabled");
}

function formatBackupDate(createdAtMs: number) {
  return new Intl.DateTimeFormat(locale.value, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(createdAtMs);
}

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    diagnostics.value = await api.diagnostics();
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.diagnostics"));
  } finally {
    loading.value = false;
  }

  // Kept independent of the diagnostics fetch above (own try/catch) so an
  // Agent hiccup on one doesn't blank out the other.
  try {
    backups.value = await api.backups();
    backupsError.value = null;
  } catch (cause) {
    backupsError.value = t(apiErrorKey(cause, "errors.backups"));
  }
}

async function downloadReport() {
  downloading.value = true;
  error.value = null;
  try {
    const blob = await api.diagnosticsReport();
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = DIAGNOSTICS_REPORT_FILENAME;
    anchor.hidden = true;
    document.body.append(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
    notify("success", t("diagnostics.reportSaved"));
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.diagnosticsReport"));
    notify("error", error.value);
  } finally {
    downloading.value = false;
  }
}

onMounted(refresh);
</script>

<template>
  <div class="view diagnostics-view">
    <header class="view-header">
      <div>
        <h1>{{ t("diagnostics.title") }}</h1>
        <p class="subtitle">
          {{ t("diagnostics.subtitle") }}
        </p>
      </div>
      <div class="header-actions diagnostics-actions">
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
          {{ downloading ? t("diagnostics.downloading") : t("diagnostics.download") }}
        </button>
      </div>
    </header>

    <div
      v-if="error"
      class="notice error"
    >
      {{ error }}
    </div>
    <div
      v-if="diagnostics && !diagnostics.agent.connected"
      class="notice warning"
    >
      {{ t("diagnostics.partial") }}
    </div>

    <p
      v-if="loading && !diagnostics"
      class="diagnostics-loading"
    >
      {{ t("common.loading") }}
    </p>

    <template v-if="diagnostics">
      <p class="diagnostics-time">
        {{ t("diagnostics.generatedAt", { time: generatedAt }) }}
      </p>
      <section class="diagnostics-grid">
        <article class="diagnostics-card">
          <h2>{{ t("diagnostics.server") }}</h2>
          <dl>
            <div><dt>{{ t("diagnostics.status") }}</dt><dd><span :class="['state-badge', stateClass(diagnostics.server.status)]">{{ diagnostics.server.status }}</span></dd></div>
            <div><dt>{{ t("diagnostics.version") }}</dt><dd>{{ diagnostics.server.version }}</dd></div>
          </dl>
        </article>
        <article class="diagnostics-card">
          <h2>{{ t("diagnostics.agent") }}</h2>
          <dl>
            <div><dt>{{ t("diagnostics.status") }}</dt><dd><span :class="['state-badge', diagnostics.agent.connected ? 'active' : 'inactive']">{{ diagnostics.agent.connected ? t("diagnostics.connected") : t("diagnostics.disconnected") }}</span></dd></div>
            <div><dt>{{ t("diagnostics.version") }}</dt><dd>{{ diagnostics.agent.version ?? t("common.none") }}</dd></div>
          </dl>
        </article>
        <article class="diagnostics-card host-card">
          <h2>{{ t("diagnostics.host") }}</h2>
          <dl v-if="diagnostics.host">
            <div>
              <dt>{{ t("diagnostics.hostname") }}</dt><dd :title="diagnostics.host.hostname">
                {{ diagnostics.host.hostname }}
              </dd>
            </div>
            <div><dt>{{ t("diagnostics.os") }}</dt><dd>{{ diagnostics.host.operating_system }} {{ diagnostics.host.os_version ?? "" }}</dd></div>
            <div>
              <dt>{{ t("diagnostics.kernel") }}</dt><dd :title="diagnostics.host.kernel_version">
                {{ diagnostics.host.kernel_version }}
              </dd>
            </div>
            <div><dt>{{ t("diagnostics.architecture") }}</dt><dd>{{ diagnostics.host.architecture }}</dd></div>
            <div><dt>{{ t("diagnostics.uptime") }}</dt><dd>{{ formatUptime(diagnostics.host.uptime_seconds, locale) }}</dd></div>
            <div><dt>{{ t("diagnostics.timezone") }}</dt><dd>{{ diagnostics.host.timezone ?? t("common.none") }}</dd></div>
            <div><dt>{{ t("diagnostics.upgradablePackages") }}</dt><dd>{{ diagnostics.host.upgradable_packages ?? t("diagnostics.upgradablePackagesUnavailable") }}</dd></div>
          </dl>
          <p
            v-else
            class="diagnostics-empty"
          >
            {{ t("diagnostics.noHost") }}
          </p>
        </article>
      </section>

      <section class="detail-panel diagnostics-panel">
        <div class="section-title">
          <h2>{{ t("diagnostics.services") }}</h2>
        </div>
        <div class="table-scroll">
          <table>
            <thead><tr><th>{{ t("diagnostics.service") }}</th><th>{{ t("diagnostics.activeState") }}</th><th>{{ t("diagnostics.startup") }}</th></tr></thead>
            <tbody>
              <tr v-if="deckoxServices.length === 0">
                <td
                  colspan="3"
                  class="empty"
                >
                  {{ t("diagnostics.noServices") }}
                </td>
              </tr>
              <tr
                v-for="service in deckoxServices"
                :key="service.id"
              >
                <td><strong class="service-name">{{ service.id }}</strong></td>
                <td><span :class="['state-badge', stateClass(service.state.active_state)]">{{ service.state.active_state }} / {{ service.state.sub_state }}</span></td>
                <td>{{ service.state.unit_file_state ?? t("common.none") }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section class="detail-panel diagnostics-panel">
        <div class="section-title">
          <h2>{{ t("diagnostics.config") }}</h2>
        </div>
        <dl
          v-if="diagnostics.runtime_config"
          class="details diagnostics-config"
        >
          <div><dt>{{ t("diagnostics.reboot") }}</dt><dd>{{ enabledLabel(diagnostics.runtime_config.reboot_allowed) }}</dd></div>
          <div><dt>{{ t("diagnostics.update") }}</dt><dd>{{ enabledLabel(diagnostics.runtime_config.update_allowed) }}</dd></div>
          <div><dt>{{ t("diagnostics.allowedServices") }}</dt><dd>{{ t("diagnostics.allowedServicesValue", { count: diagnostics.runtime_config.allowed_services_count }) }}</dd></div>
        </dl>
        <p
          v-else
          class="diagnostics-empty diagnostics-panel-empty"
        >
          {{ t("diagnostics.noConfig") }}
        </p>
      </section>

      <section class="detail-panel diagnostics-panel">
        <div class="section-title">
          <h2>{{ t("diagnostics.backups") }}</h2>
        </div>
        <p
          v-if="backupsError"
          class="diagnostics-empty diagnostics-panel-empty"
        >
          {{ backupsError }}
        </p>
        <div
          v-else
          class="table-scroll"
        >
          <table>
            <thead><tr><th>{{ t("diagnostics.backupCreatedAt") }}</th><th>{{ t("diagnostics.backupPreviousVersion") }}</th><th>{{ t("diagnostics.backupSize") }}</th></tr></thead>
            <tbody>
              <tr v-if="backups.length === 0">
                <td
                  colspan="3"
                  class="empty"
                >
                  {{ t("diagnostics.noBackups") }}
                </td>
              </tr>
              <tr
                v-for="backup in backups"
                :key="backup.name"
              >
                <td>{{ backup.created_at_ms === null ? t("common.none") : formatBackupDate(backup.created_at_ms) }}</td>
                <td>{{ backup.previous_version ?? t("common.none") }}</td>
                <td>{{ formatBytes(backup.size_bytes, locale) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, type ServiceSummary } from "../api/client";
import { apiErrorKey } from "../api/errors";
import { notify } from "../notifications";

const { t } = useI18n();

const services = ref<ServiceSummary[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
const query = ref("");
const pending = ref<string | null>(null);

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

async function runAction(service: ServiceSummary, action: "start" | "stop" | "restart") {
  if ((action === "stop" || action === "restart") &&
      !window.confirm(t(action === "stop" ? "services.confirmStop" : "services.confirmRestart", { id: service.id }))) {
    return;
  }

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
  </div>
</template>

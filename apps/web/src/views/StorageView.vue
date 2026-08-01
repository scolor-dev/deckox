<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, formatBytes, type StorageMount } from "../api/client";
import { apiErrorKey } from "../api/errors";

const { t, locale } = useI18n();

const mounts = ref<StorageMount[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    mounts.value = await api.storage();
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.storage"));
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);
</script>

<template>
  <div class="view">
    <header class="view-header">
      <div>
        <h1>{{ t("storage.title") }}</h1>
        <p class="subtitle">
          {{ t("storage.summary", { count: mounts.length }) }}
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

    <section class="table-panel storage-panel">
      <div class="table-scroll">
        <table class="storage-table">
          <thead>
            <tr><th>{{ t("storage.mount") }}</th><th>{{ t("storage.filesystem") }}</th><th>{{ t("storage.capacity") }}</th><th>{{ t("storage.usage") }}</th></tr>
          </thead>
          <tbody>
            <tr v-if="loading && mounts.length === 0">
              <td
                colspan="4"
                class="empty"
              >
                {{ t("storage.loading") }}
              </td>
            </tr>
            <tr v-else-if="!loading && mounts.length === 0 && !error">
              <td
                colspan="4"
                class="empty"
              >
                {{ t("storage.empty") }}
              </td>
            </tr>
            <tr
              v-for="mount in mounts"
              :key="`${mount.filesystem}:${mount.mount_point}`"
            >
              <td class="path-cell">
                <strong
                  class="storage-path"
                  :title="mount.mount_point"
                >{{ mount.mount_point }}</strong>
              </td>
              <td class="filesystem-cell">
                <span :title="mount.filesystem">{{ mount.filesystem }}</span>
                <small>{{ mount.filesystem_type }}</small>
              </td>
              <td class="capacity-cell">
                <strong>{{ formatBytes(mount.total_bytes, locale) }}</strong>
                <small>{{ t("storage.available", { value: formatBytes(mount.available_bytes, locale) }) }}</small>
              </td>
              <td class="storage-usage-cell">
                <div class="usage-row">
                  <span>{{ mount.usage_percent.toFixed(0) }}%</span>
                  <small>{{ t("storage.used", { value: formatBytes(mount.used_bytes, locale) }) }}</small>
                </div>
                <div class="progress">
                  <span
                    :class="{ critical: mount.usage_percent >= 90 }"
                    :style="{ width: `${mount.usage_percent}%` }"
                  />
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>

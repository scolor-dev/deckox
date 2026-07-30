<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, formatBytes, type StorageMount } from "../api/client";

const mounts = ref<StorageMount[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    mounts.value = await api.storage();
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : "ストレージ情報を取得できませんでした";
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
        <h1>ストレージ</h1>
        <p class="subtitle">
          {{ mounts.length }}件のマウント
        </p>
      </div>
      <button
        class="button"
        type="button"
        :disabled="loading"
        @click="refresh"
      >
        {{ loading ? "読込中…" : "更新" }}
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
            <tr><th>マウント先</th><th>ファイルシステム</th><th>容量</th><th>使用状況</th></tr>
          </thead>
          <tbody>
            <tr v-if="loading && mounts.length === 0">
              <td
                colspan="4"
                class="empty"
              >
                ストレージ情報を読み込んでいます…
              </td>
            </tr>
            <tr v-else-if="!loading && mounts.length === 0 && !error">
              <td
                colspan="4"
                class="empty"
              >
                マウントされたファイルシステムはありません。
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
                <strong>{{ formatBytes(mount.total_bytes) }}</strong>
                <small>空き {{ formatBytes(mount.available_bytes) }}</small>
              </td>
              <td class="storage-usage-cell">
                <div class="usage-row">
                  <span>{{ mount.usage_percent.toFixed(0) }}%</span>
                  <small>{{ formatBytes(mount.used_bytes) }} 使用</small>
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

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, formatBytes, type StorageMount } from "../api/client";

const mounts = ref<StorageMount[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);

const totalBytes = computed(() => mounts.value.reduce((sum, mount) => sum + mount.total_bytes, 0));
const usedBytes = computed(() => mounts.value.reduce((sum, mount) => sum + mount.used_bytes, 0));

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
        <p class="eyebrow">FILESYSTEM STORAGE</p>
        <h1>ストレージ</h1>
        <p class="subtitle">{{ formatBytes(usedBytes) }} / {{ formatBytes(totalBytes) }} used</p>
      </div>
      <button class="button secondary" type="button" :disabled="loading" @click="refresh">
        {{ loading ? "読込中…" : "更新" }}
      </button>
    </header>

    <div v-if="error" class="notice error">{{ error }}</div>

    <section class="storage-grid">
      <article v-for="mount in mounts" :key="`${mount.filesystem}:${mount.mount_point}`" class="storage-card">
        <div class="storage-head">
          <div><span class="mount-icon">◆</span><strong>{{ mount.mount_point }}</strong></div>
          <span class="usage">{{ mount.usage_percent.toFixed(0) }}%</span>
        </div>
        <p>{{ mount.filesystem }} · {{ mount.filesystem_type }}</p>
        <div class="progress storage-progress">
          <span :class="{ critical: mount.usage_percent >= 90 }" :style="{ width: `${mount.usage_percent}%` }"></span>
        </div>
        <div class="storage-meta">
          <span>{{ formatBytes(mount.used_bytes) }} 使用中</span>
          <span>{{ formatBytes(mount.available_bytes) }} 空き</span>
          <span>{{ formatBytes(mount.total_bytes) }} 合計</span>
        </div>
      </article>
      <div v-if="loading && mounts.length === 0" class="empty-card">ストレージ情報を読み込んでいます…</div>
      <div v-else-if="!loading && mounts.length === 0 && !error" class="empty-card">マウントされたファイルシステムはありません。</div>
    </section>
  </div>
</template>

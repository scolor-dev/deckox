<script setup lang="ts">
import { onMounted, ref } from "vue";
import {
  api,
  formatBytes,
  formatUptime,
  type ServerStatus,
  type SystemInfo,
  type SystemMetrics,
} from "../api/client";

const emit = defineEmits<{ status: [value: ServerStatus] }>();

const status = ref<ServerStatus | null>(null);
const system = ref<SystemInfo | null>(null);
const metrics = ref<SystemMetrics | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    status.value = await api.serverStatus();
    emit("status", status.value);
    [system.value, metrics.value] = await Promise.all([
      api.systemInfo(),
      api.systemMetrics(),
    ]);
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : "システム情報を取得できませんでした";
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
        <h1>概要</h1>
        <p class="subtitle">
          {{ system?.hostname ?? status?.agent?.hostname ?? "サーバー情報を取得しています" }}
        </p>
      </div>
      <button
        class="button"
        type="button"
        :disabled="loading"
        @click="refresh"
      >
        {{ loading ? "確認中…" : "更新" }}
      </button>
    </header>

    <div
      v-if="error"
      class="notice error"
    >
      {{ error }}
    </div>
    <div
      v-else-if="status?.agent_error"
      class="notice warning"
    >
      Agentに接続できません: {{ status.agent_error }}
    </div>

    <section class="server-summary">
      <div class="server-state">
        <span :class="['status-dot', status?.agent ? 'online' : 'offline']" />
        <div>
          <strong>{{ status?.agent ? "サーバーは正常に動作しています" : "Agentに接続できません" }}</strong>
          <span>{{ system?.operating_system ?? "Linux" }} {{ system?.os_version ?? "" }}</span>
        </div>
      </div>
      <dl>
        <div><dt>稼働時間</dt><dd>{{ formatUptime(system?.uptime_seconds) }}</dd></div>
        <div><dt>アーキテクチャ</dt><dd>{{ system?.architecture ?? "—" }}</dd></div>
      </dl>
    </section>

    <section
      class="metric-grid"
      aria-label="リソース使用状況"
    >
      <article class="metric-card">
        <div class="metric-head">
          <span>CPU使用率</span><small>{{ metrics?.cpu.logical_cores ?? "—" }}コア</small>
        </div>
        <strong>{{ metrics ? `${metrics.cpu.usage_percent.toFixed(1)}%` : "—" }}</strong>
        <div class="progress">
          <span :style="{ width: `${metrics?.cpu.usage_percent ?? 0}%` }" />
        </div>
      </article>
      <article class="metric-card">
        <div class="metric-head">
          <span>メモリ</span><small>全体 {{ formatBytes(metrics?.memory.total_bytes ?? 0) }}</small>
        </div>
        <strong>{{ metrics ? `${formatBytes(metrics.memory.used_bytes)} 使用中` : "—" }}</strong>
        <div class="progress">
          <span :style="{ width: `${metrics ? metrics.memory.used_bytes / metrics.memory.total_bytes * 100 : 0}%` }" />
        </div>
      </article>
      <article class="metric-card">
        <div class="metric-head">
          <span>負荷平均</span><small>5分 {{ metrics?.load_average.five_minutes.toFixed(2) ?? "—" }}</small>
        </div>
        <strong>{{ metrics?.load_average.one_minute.toFixed(2) ?? "—" }}</strong>
        <small class="metric-foot">1分値・15分値 {{ metrics?.load_average.fifteen_minutes.toFixed(2) ?? "—" }}</small>
      </article>
    </section>

    <section class="detail-panel">
      <div class="section-title">
        <h2>システム情報</h2>
      </div>
      <dl class="details">
        <div><dt>ホスト名</dt><dd>{{ system?.hostname ?? "—" }}</dd></div>
        <div><dt>OS</dt><dd>{{ system ? `${system.operating_system} ${system.os_version ?? ""}` : "—" }}</dd></div>
        <div><dt>カーネル</dt><dd>{{ system?.kernel_version ?? "—" }}</dd></div>
        <div><dt>アーキテクチャ</dt><dd>{{ system?.architecture ?? "—" }}</dd></div>
        <div><dt>タイムゾーン</dt><dd>{{ system?.timezone ?? "—" }}</dd></div>
        <div><dt>Deckox</dt><dd>バージョン {{ status?.version ?? "—" }}</dd></div>
      </dl>
    </section>
  </div>
</template>

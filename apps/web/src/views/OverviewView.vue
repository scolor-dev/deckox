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
        <p class="eyebrow">SYSTEM OVERVIEW</p>
        <h1>{{ system?.hostname ?? status?.agent?.hostname ?? "Linux Server" }}</h1>
      </div>
      <button class="button secondary" type="button" :disabled="loading" @click="refresh">
        {{ loading ? "確認中…" : "更新" }}
      </button>
    </header>

    <div v-if="error" class="notice error">{{ error }}</div>
    <div v-else-if="status?.agent_error" class="notice warning">
      Agentに接続できません: {{ status.agent_error }}
    </div>

    <section class="hero-card">
      <div>
        <span :class="['status-dot', status?.agent ? 'online' : 'offline']"></span>
        <span class="status-label">{{ status?.agent ? "SYSTEM ONLINE" : "AGENT OFFLINE" }}</span>
        <h2>{{ system?.operating_system ?? "Linux" }}</h2>
        <p>
          {{ system?.os_version ? `Version ${system.os_version}` : "システム情報を取得しています" }}
          <span v-if="system"> · Kernel {{ system.kernel_version }}</span>
        </p>
      </div>
      <div class="orb" aria-hidden="true"></div>
    </section>

    <section class="metric-grid" aria-label="リソース使用状況">
      <article class="metric-card">
        <div class="metric-head"><span>CPU</span><small>{{ metrics?.cpu.logical_cores ?? "—" }} cores</small></div>
        <strong>{{ metrics ? `${metrics.cpu.usage_percent.toFixed(1)}%` : "—" }}</strong>
        <div class="progress"><span :style="{ width: `${metrics?.cpu.usage_percent ?? 0}%` }"></span></div>
      </article>
      <article class="metric-card">
        <div class="metric-head"><span>MEMORY</span><small>{{ formatBytes(metrics?.memory.total_bytes ?? 0) }}</small></div>
        <strong>{{ metrics ? formatBytes(metrics.memory.used_bytes) : "—" }}</strong>
        <div class="progress"><span :style="{ width: `${metrics ? metrics.memory.used_bytes / metrics.memory.total_bytes * 100 : 0}%` }"></span></div>
      </article>
      <article class="metric-card">
        <div class="metric-head"><span>LOAD 1M</span><small>5m {{ metrics?.load_average.five_minutes.toFixed(2) ?? "—" }}</small></div>
        <strong>{{ metrics?.load_average.one_minute.toFixed(2) ?? "—" }}</strong>
        <small class="metric-foot">15m {{ metrics?.load_average.fifteen_minutes.toFixed(2) ?? "—" }}</small>
      </article>
      <article class="metric-card">
        <div class="metric-head"><span>UPTIME</span><small>{{ system?.architecture ?? "—" }}</small></div>
        <strong>{{ formatUptime(system?.uptime_seconds) }}</strong>
        <small class="metric-foot">{{ system?.timezone ?? "Timezone unknown" }}</small>
      </article>
    </section>

    <section class="detail-panel">
      <div class="section-title">
        <div><p class="eyebrow">HOST DETAILS</p><h2>システム詳細</h2></div>
      </div>
      <dl class="details">
        <div><dt>ホスト名</dt><dd>{{ system?.hostname ?? "—" }}</dd></div>
        <div><dt>OS</dt><dd>{{ system ? `${system.operating_system} ${system.os_version ?? ""}` : "—" }}</dd></div>
        <div><dt>カーネル</dt><dd>{{ system?.kernel_version ?? "—" }}</dd></div>
        <div><dt>アーキテクチャ</dt><dd>{{ system?.architecture ?? "—" }}</dd></div>
        <div><dt>Boot ID</dt><dd class="mono">{{ system?.boot_id ?? "—" }}</dd></div>
        <div><dt>Server</dt><dd>{{ status?.status ?? "—" }} · v{{ status?.version ?? "—" }}</dd></div>
      </dl>
    </section>
  </div>
</template>


<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, type ServiceSummary } from "../api/client";

const services = ref<ServiceSummary[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
const message = ref<string | null>(null);
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

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    services.value = await api.services();
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : "サービス一覧を取得できませんでした";
  } finally {
    loading.value = false;
  }
}

async function runAction(service: ServiceSummary, action: "start" | "stop" | "restart") {
  if ((action === "stop" || action === "restart") &&
      !window.confirm(`${service.id} を${action === "stop" ? "停止" : "再起動"}しますか？`)) {
    return;
  }

  pending.value = `${service.id}:${action}`;
  error.value = null;
  message.value = null;
  try {
    const result = await api.serviceAction(service.id, action);
    message.value = result.message ?? `${service.id} の操作が完了しました`;
    await refresh();
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : "サービス操作に失敗しました";
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
        <p class="eyebrow">SYSTEMD SERVICES</p>
        <h1>サービス</h1>
        <p class="subtitle">{{ runningCount }} / {{ services.length }} active</p>
      </div>
      <button class="button secondary" type="button" :disabled="loading" @click="refresh">
        {{ loading ? "読込中…" : "更新" }}
      </button>
    </header>

    <div v-if="error" class="notice error">{{ error }}</div>
    <div v-if="message" class="notice success">{{ message }}</div>

    <section class="table-panel">
      <div class="table-toolbar">
        <label class="search">
          <span class="sr-only">サービスを検索</span>
          <input v-model="query" type="search" placeholder="サービス名または説明を検索">
        </label>
        <span class="table-count">{{ filteredServices.length }} services</span>
      </div>

      <div class="table-scroll">
        <table>
          <thead><tr><th>サービス</th><th>状態</th><th>自動起動</th><th>操作</th></tr></thead>
          <tbody>
            <tr v-if="loading && services.length === 0"><td colspan="4" class="empty">サービスを読み込んでいます…</td></tr>
            <tr v-else-if="filteredServices.length === 0"><td colspan="4" class="empty">該当するサービスはありません。</td></tr>
            <tr v-for="service in filteredServices" :key="service.id">
              <td>
                <strong class="service-name">{{ service.id }}</strong>
                <small>{{ service.description || "説明なし" }}</small>
              </td>
              <td>
                <span :class="['state-badge', service.active_state === 'active' ? 'active' : 'inactive']">
                  {{ service.active_state }}
                </span>
                <small>{{ service.sub_state }}</small>
              </td>
              <td><span class="unit-state">{{ service.unit_file_state ?? "—" }}</span></td>
              <td>
                <div v-if="service.control_allowed" class="actions">
                  <button class="action-button" type="button" :disabled="pending !== null || service.active_state === 'active'" @click="runAction(service, 'start')">起動</button>
                  <button class="action-button" type="button" :disabled="pending !== null || service.active_state !== 'active'" @click="runAction(service, 'restart')">再起動</button>
                  <button class="action-button danger" type="button" :disabled="pending !== null || service.active_state !== 'active'" @click="runAction(service, 'stop')">停止</button>
                </div>
                <span v-else class="locked">閲覧のみ</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <aside class="inline-note">
      変更操作は <code>/etc/deckox/agent.toml</code> の許可リストに登録されたサービスだけ利用できます。
    </aside>
  </div>
</template>


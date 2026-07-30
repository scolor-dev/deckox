<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { api, type ServerStatus } from "./api/client";
import LoginView from "./views/LoginView.vue";
import OverviewView from "./views/OverviewView.vue";
import ServicesView from "./views/ServicesView.vue";
import StorageView from "./views/StorageView.vue";

type Page = "overview" | "services" | "storage";

const activePage = ref<Page>("overview");
const status = ref<ServerStatus | null>(null);
const menuOpen = ref(false);
const authChecking = ref(true);
const authenticated = ref(false);

const activeComponent = computed(() => ({
  overview: OverviewView,
  services: ServicesView,
  storage: StorageView,
})[activePage.value]);

const pageTitle = computed(() => ({
  overview: "概要",
  services: "サービス",
  storage: "ストレージ",
})[activePage.value]);

function navigate(page: Page) {
  activePage.value = page;
  menuOpen.value = false;
  document.title = `${pageTitle.value} · Deckox`;
}

async function refreshStatus() {
  try {
    status.value = await api.serverStatus();
  } catch {
    status.value = null;
  }
}

async function checkAuthentication() {
  try {
    const session = await api.authSession();
    authenticated.value = session.authenticated;
    if (session.authenticated) await refreshStatus();
  } catch {
    authenticated.value = false;
  } finally {
    authChecking.value = false;
  }
}

function handleAuthenticated() {
  authenticated.value = true;
  void refreshStatus();
}

function handleUnauthorized() {
  authenticated.value = false;
  status.value = null;
  menuOpen.value = false;
}

async function logout() {
  try {
    await api.logout();
  } finally {
    handleUnauthorized();
  }
}

onMounted(() => {
  window.addEventListener("deckox:unauthorized", handleUnauthorized);
  void checkAuthentication();
});

onBeforeUnmount(() => {
  window.removeEventListener("deckox:unauthorized", handleUnauthorized);
});
</script>

<template>
  <main
    v-if="authChecking"
    class="auth-page"
  >
    <p class="auth-loading">
      認証状態を確認しています…
    </p>
  </main>

  <LoginView
    v-else-if="!authenticated"
    @authenticated="handleAuthenticated"
  />

  <div
    v-else
    class="shell"
  >
    <header class="mobile-bar">
      <div class="brand">
        <span class="brand-mark">D</span><span>Deckox</span>
      </div>
      <button
        class="menu-button"
        type="button"
        :aria-expanded="menuOpen"
        aria-label="メニューを開く"
        @click="menuOpen = !menuOpen"
      >
        {{ menuOpen ? "閉じる" : "メニュー" }}
      </button>
    </header>

    <aside :class="['sidebar', { open: menuOpen }]">
      <div class="brand">
        <span class="brand-mark">D</span>
        <div><span>Deckox</span><small>サーバー管理</small></div>
      </div>
      <nav aria-label="メインナビゲーション">
        <button
          type="button"
          :class="{ active: activePage === 'overview' }"
          @click="navigate('overview')"
        >
          概要
        </button>
        <button
          type="button"
          :class="{ active: activePage === 'services' }"
          @click="navigate('services')"
        >
          サービス
        </button>
        <button
          type="button"
          :class="{ active: activePage === 'storage' }"
          @click="navigate('storage')"
        >
          ストレージ
        </button>
      </nav>
      <div class="agent-state">
        <span :class="['status-dot', status?.agent ? 'online' : 'offline']" />
        <div>
          <strong>{{ status?.agent ? "接続中" : "状態を確認できません" }}</strong>
          <small>{{ status?.agent?.hostname ?? "Agent" }}</small>
        </div>
      </div>
      <div class="sidebar-footer">
        <button
          class="logout-button"
          type="button"
          @click="logout"
        >
          ログアウト
        </button>
        <span>バージョン {{ status?.version ?? "0.2.0" }}</span>
      </div>
    </aside>

    <main class="main-content">
      <component
        :is="activeComponent"
        @status="status = $event"
      />
    </main>
  </div>
</template>

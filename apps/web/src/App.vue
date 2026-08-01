<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { RouterLink, RouterView, useRoute } from "vue-router";
import { api, type ServerStatus } from "./api/client";
import LoginView from "./views/LoginView.vue";

const route = useRoute();
const status = ref<ServerStatus | null>(null);
const menuOpen = ref(false);
const authChecking = ref(true);
const authenticated = ref(false);
const loginMessage = ref<string | null>(null);

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
  loginMessage.value = null;
  void refreshStatus();
}

function handleUnauthorized() {
  authenticated.value = false;
  status.value = null;
  menuOpen.value = false;
}

function handlePasswordChanged() {
  loginMessage.value = "パスワードを変更しました。新しいパスワードでログインしてください。";
  handleUnauthorized();
}

async function logout() {
  try {
    await api.logout();
  } finally {
    handleUnauthorized();
  }
}

watch(() => route.fullPath, () => {
  menuOpen.value = false;
  const title = typeof route.meta.title === "string" ? route.meta.title : "Deckox";
  document.title = `${title} · Deckox`;
}, { immediate: true });

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
    :message="loginMessage"
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
        <RouterLink to="/">
          概要
        </RouterLink>
        <RouterLink to="/services">
          サービス
        </RouterLink>
        <RouterLink to="/storage">
          ストレージ
        </RouterLink>
        <RouterLink to="/settings">
          設定
        </RouterLink>
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
        <span>バージョン {{ status?.version ?? "0.2.1" }}</span>
      </div>
    </aside>

    <main class="main-content">
      <RouterView v-slot="{ Component }">
        <component
          :is="Component"
          @status="status = $event"
          @password-changed="handlePasswordChanged"
        />
      </RouterView>
    </main>
  </div>
</template>

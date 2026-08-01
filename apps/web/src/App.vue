<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { RouterLink, RouterView, useRoute } from "vue-router";
import { api, type ServerStatus } from "./api/client";
import NotificationRegion from "./components/NotificationRegion.vue";
import LoginView from "./views/LoginView.vue";

const route = useRoute();
const { t, locale } = useI18n();
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
  loginMessage.value = t("app.passwordChanged");
  handleUnauthorized();
}

async function logout() {
  try {
    await api.logout();
  } finally {
    handleUnauthorized();
  }
}

watch([() => route.fullPath, locale], () => {
  menuOpen.value = false;
  const titleKey = typeof route.meta.titleKey === "string" ? route.meta.titleKey : "nav.overview";
  document.title = `${t(titleKey)} · Deckox`;
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
      {{ t("app.checkingAuth") }}
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
        :aria-label="t('app.openMenu')"
        @click="menuOpen = !menuOpen"
      >
        {{ menuOpen ? t("common.close") : t("app.menu") }}
      </button>
    </header>

    <aside :class="['sidebar', { open: menuOpen }]">
      <div class="brand">
        <span class="brand-mark">D</span>
        <div><span>Deckox</span><small>{{ t("app.serverManagement") }}</small></div>
      </div>
      <nav :aria-label="t('app.mainNavigation')">
        <RouterLink to="/">
          {{ t("nav.overview") }}
        </RouterLink>
        <RouterLink to="/services">
          {{ t("nav.services") }}
        </RouterLink>
        <RouterLink to="/storage">
          {{ t("nav.storage") }}
        </RouterLink>
        <RouterLink to="/settings">
          {{ t("nav.settings") }}
        </RouterLink>
      </nav>
      <div class="agent-state">
        <span :class="['status-dot', status?.agent ? 'online' : 'offline']" />
        <div>
          <strong>{{ status?.agent ? t("app.connected") : t("app.stateUnavailable") }}</strong>
          <small>{{ status?.agent?.hostname ?? "Agent" }}</small>
        </div>
      </div>
      <div class="sidebar-footer">
        <button
          class="logout-button"
          type="button"
          @click="logout"
        >
          {{ t("app.logout") }}
        </button>
        <span>{{ t("common.version") }} {{ status?.version ?? "0.3.0" }}</span>
      </div>
    </aside>

    <main class="main-content">
      <NotificationRegion />
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

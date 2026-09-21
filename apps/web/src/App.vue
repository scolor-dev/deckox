<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { RouterLink, RouterView, useRoute, useRouter } from "vue-router";
import { api, type ServerStatus } from "./api/client";
import NotificationRegion from "./components/NotificationRegion.vue";
import { AppButton } from "./design-system/components";
import LoginView from "./views/LoginView.vue";
import { cachedViewNames, enabledModules, loadModules, resetModules } from "./modules/store";

const route = useRoute();
const router = useRouter();
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
  await loadModules();
}

async function checkAuthentication() {
  try {
    const session = await api.authSession();
    // The manifest is read before the shell shows, so pages never call a
    // module that is switched off while it is still unknown.
    if (session.authenticated) await refreshStatus();
    authenticated.value = session.authenticated;
  } catch {
    authenticated.value = false;
  } finally {
    authChecking.value = false;
  }
}

async function handleAuthenticated() {
  loginMessage.value = null;
  await refreshStatus();
  authenticated.value = true;
}

function handleUnauthorized() {
  authenticated.value = false;
  resetModules();
  status.value = null;
  menuOpen.value = false;
}

function handlePasswordChanged() {
  loginMessage.value = t("app.passwordChanged");
  handleUnauthorized();
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && menuOpen.value) menuOpen.value = false;
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

// A module switched off in agent.toml only shows once the Agent restarts, so
// the manifest is read again whenever the Agent comes back.
watch(() => status.value?.agent != null, (online) => {
  if (online) void loadModules();
});

// A page whose module is switched off is neither shown nor left mounted.
const routeAllowed = computed(() => {
  const id = route.meta.moduleId;
  return typeof id !== "string" || enabledModules.value.some((module) => module.id === id);
});

watch(routeAllowed, (allowed) => {
  if (!allowed) void router.replace("/");
}, { immediate: true });

onMounted(() => {
  window.addEventListener("deckox:unauthorized", handleUnauthorized);
  window.addEventListener("keydown", handleKeydown);
  void checkAuthentication();
});

onBeforeUnmount(() => {
  window.removeEventListener("deckox:unauthorized", handleUnauthorized);
  window.removeEventListener("keydown", handleKeydown);
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
      <AppButton
        variant="menu"
        :aria-expanded="menuOpen"
        :aria-label="t('app.openMenu')"
        @click="menuOpen = !menuOpen"
      >
        {{ menuOpen ? t("common.close") : t("app.menu") }}
      </AppButton>
    </header>

    <div
      v-if="menuOpen"
      class="sidebar-backdrop"
      @click="menuOpen = false"
    />

    <aside :class="['sidebar', { open: menuOpen }]">
      <div class="brand">
        <span class="brand-mark">D</span>
        <div><span>Deckox</span><small>{{ t("app.serverManagement") }}</small></div>
      </div>
      <nav :aria-label="t('app.mainNavigation')">
        <RouterLink
          v-for="module in enabledModules"
          :key="module.id"
          :to="module.path"
        >
          {{ t(module.titleKey) }}
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
        <AppButton
          variant="logout"
          @click="logout"
        >
          {{ t("app.logout") }}
        </AppButton>
        <span>{{ t("common.version") }} {{ status?.version ?? t("common.none") }}</span>
      </div>
    </aside>

    <main class="main-content">
      <NotificationRegion />
      <RouterView
        v-if="routeAllowed"
        v-slot="{ Component }"
      >
        <KeepAlive :include="cachedViewNames">
          <component
            :is="Component"
            @status="status = $event"
            @password-changed="handlePasswordChanged"
          />
        </KeepAlive>
      </RouterView>
    </main>
  </div>
</template>

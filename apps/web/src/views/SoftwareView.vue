<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, type SoftwarePackage } from "../api/client";
import { apiErrorKey } from "../api/errors";
import { notify } from "../notifications";

const { t } = useI18n();

const packages = ref<SoftwarePackage[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
const pending = ref<string | null>(null);

const newPackageName = ref("");
const adding = ref(false);
const addError = ref<string | null>(null);

type ManageAction = "install" | "remove" | "upgrade";

const CONFIRM_KEYS: Record<ManageAction, string> = {
  install: "software.confirmInstall",
  remove: "software.confirmRemove",
  upgrade: "software.confirmUpgrade",
};

const PENDING_LABEL_KEYS: Record<ManageAction, string> = {
  install: "software.installing",
  remove: "software.removing",
  upgrade: "software.upgrading",
};

const armed = ref<{ pkg: SoftwarePackage; action: ManageAction } | null>(null);
const actionPassword = ref("");
const actionError = ref<string | null>(null);
const submitting = ref(false);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    packages.value = await api.software();
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.software"));
  } finally {
    loading.value = false;
  }
}

async function addPackage() {
  const name = newPackageName.value.trim();
  if (!name) return;

  adding.value = true;
  addError.value = null;
  try {
    await api.softwareAllowlist(name, "allow");
    newPackageName.value = "";
    notify("success", t("software.completed", { name }));
    await refresh();
  } catch (cause) {
    addError.value = t(apiErrorKey(cause, "errors.softwareAction"));
  } finally {
    adding.value = false;
  }
}

async function disallow(pkg: SoftwarePackage) {
  if (!window.confirm(t("software.confirmDisallow", { name: pkg.name }))) return;

  pending.value = `${pkg.name}:disallow`;
  error.value = null;
  try {
    await api.softwareAllowlist(pkg.name, "disallow");
    notify("success", t("software.completed", { name: pkg.name }));
    await refresh();
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.softwareAction"));
    notify("error", error.value);
  } finally {
    pending.value = null;
  }
}

function openAction(pkg: SoftwarePackage, action: ManageAction) {
  armed.value = { pkg, action };
  actionPassword.value = "";
  actionError.value = null;
}

function closeAction() {
  armed.value = null;
  actionPassword.value = "";
  actionError.value = null;
}

async function confirmAction() {
  if (!armed.value || submitting.value) return;
  const { pkg, action } = armed.value;
  if (!actionPassword.value) {
    actionError.value = t("software.passwordRequired");
    return;
  }
  if (!window.confirm(t(CONFIRM_KEYS[action], { name: pkg.name }))) return;

  submitting.value = true;
  pending.value = `${pkg.name}:${action}`;
  actionError.value = null;
  try {
    await api.softwareAction(pkg.name, action, actionPassword.value);
    notify("success", t("software.completed", { name: pkg.name }));
    closeAction();
    await refresh();
  } catch (cause) {
    actionError.value = t(apiErrorKey(cause, "errors.softwareAction"));
  } finally {
    submitting.value = false;
    pending.value = null;
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="view">
    <header class="view-header">
      <div>
        <h1>{{ t("software.title") }}</h1>
        <p class="subtitle">
          {{ t("software.subtitle") }}
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
      role="alert"
    >
      {{ error }}
    </div>

    <section class="table-panel">
      <div class="table-toolbar">
        <form
          class="add-software-form"
          @submit.prevent="addPackage"
        >
          <label class="search">
            <span class="sr-only">{{ t("software.addLabel") }}</span>
            <input
              v-model="newPackageName"
              type="text"
              :placeholder="t('software.addPlaceholder')"
              required
            >
          </label>
          <button
            class="button"
            type="submit"
            :disabled="adding"
          >
            {{ adding ? t("software.adding") : t("software.add") }}
          </button>
        </form>
        <span class="table-count">{{ t("software.count", { count: packages.length }) }}</span>
      </div>

      <div
        v-if="addError"
        class="notice error"
        role="alert"
      >
        {{ addError }}
      </div>
      <p class="inline-note">
        {{ t("software.addHelp") }}
      </p>

      <div class="table-scroll">
        <table>
          <thead>
            <tr>
              <th>{{ t("software.name") }}</th>
              <th>{{ t("software.status") }}</th>
              <th>{{ t("software.installedVersion") }}</th>
              <th>{{ t("software.availableVersion") }}</th>
              <th>{{ t("software.actions") }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading && packages.length === 0">
              <td
                colspan="5"
                class="empty"
              >
                {{ t("software.loading") }}
              </td>
            </tr>
            <tr v-else-if="packages.length === 0">
              <td
                colspan="5"
                class="empty"
              >
                {{ t("software.empty") }}
              </td>
            </tr>
            <tr
              v-for="pkg in packages"
              :key="pkg.name"
            >
              <td>
                <strong class="service-name">{{ pkg.name }}</strong>
              </td>
              <td>
                <span :class="['state-badge', pkg.installed ? 'active' : 'inactive']">
                  {{ pkg.installed ? t("software.installed") : t("software.notInstalled") }}
                </span>
                <small v-if="pkg.upgradable">{{ t("software.upgradable") }}</small>
              </td>
              <td>{{ pkg.installed_version ?? t("common.none") }}</td>
              <td>{{ pkg.available_version ?? t("common.none") }}</td>
              <td>
                <div class="actions">
                  <button
                    v-if="!pkg.installed"
                    class="action-button"
                    type="button"
                    :disabled="pending !== null"
                    @click="openAction(pkg, 'install')"
                  >
                    {{ t("software.install") }}
                  </button>
                  <template v-else>
                    <button
                      v-if="pkg.upgradable"
                      class="action-button"
                      type="button"
                      :disabled="pending !== null"
                      @click="openAction(pkg, 'upgrade')"
                    >
                      {{ t("software.upgrade") }}
                    </button>
                    <button
                      class="action-button danger"
                      type="button"
                      :disabled="pending !== null"
                      @click="openAction(pkg, 'remove')"
                    >
                      {{ t("software.remove") }}
                    </button>
                  </template>
                  <button
                    class="action-button danger"
                    type="button"
                    :disabled="pending !== null"
                    @click="disallow(pkg)"
                  >
                    {{ t("software.disallow") }}
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <aside class="inline-note">
      {{ t("software.platformNote") }}
    </aside>

    <div
      v-if="armed"
      class="dialog-backdrop"
      @click.self="closeAction"
    >
      <section
        class="confirm-dialog"
        role="dialog"
        aria-modal="true"
        :aria-label="t(CONFIRM_KEYS[armed.action], { name: armed.pkg.name })"
      >
        <h2>{{ t(`software.${armed.action}`) }}: {{ armed.pkg.name }}</h2>
        <form
          class="settings-form"
          @submit.prevent="confirmAction"
        >
          <label for="software-action-password">{{ t("software.password") }}</label>
          <input
            id="software-action-password"
            v-model="actionPassword"
            type="password"
            autocomplete="current-password"
            required
          >
          <p
            v-if="actionError"
            class="notice error"
            role="alert"
          >
            {{ actionError }}
          </p>
          <div class="dialog-actions">
            <button
              class="button"
              type="button"
              :disabled="submitting"
              @click="closeAction"
            >
              {{ t("common.close") }}
            </button>
            <button
              class="primary-button danger-button"
              type="submit"
              :disabled="submitting"
            >
              {{ submitting ? t(PENDING_LABEL_KEYS[armed.action]) : t(`software.${armed.action}`) }}
            </button>
          </div>
        </form>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, type SoftwarePackage } from "../api/client";
import { apiErrorKey } from "../api/errors";
import PasswordConfirmDialog from "../components/PasswordConfirmDialog.vue";
import { notify } from "../notifications";
import { preferences, type SoftwareTagFilterKey } from "../preferences";

const { t } = useI18n();

const TAG_FILTER_KEYS: SoftwareTagFilterKey[] = ["installed", "not_installed"];

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
const actionError = ref<string | null>(null);
const submitting = ref(false);

function packageTag(pkg: SoftwarePackage): SoftwareTagFilterKey {
  return pkg.installed ? "installed" : "not_installed";
}

function tagLabel(tag: SoftwareTagFilterKey) {
  return tag === "installed" ? t("software.installed") : t("software.notInstalled");
}

function isTagHidden(tag: SoftwareTagFilterKey) {
  return preferences.hiddenSoftwareTags.includes(tag);
}

function toggleTag(tag: SoftwareTagFilterKey) {
  preferences.hiddenSoftwareTags = isTagHidden(tag)
    ? preferences.hiddenSoftwareTags.filter((hidden) => hidden !== tag)
    : [...preferences.hiddenSoftwareTags, tag];
}

const filteredPackages = computed(() =>
  packages.value.filter((pkg) => !isTagHidden(packageTag(pkg))),
);

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
  actionError.value = null;
}

function closeAction() {
  armed.value = null;
  actionError.value = null;
}

async function confirmAction(password: string) {
  if (!armed.value || submitting.value) return;
  const { pkg, action } = armed.value;

  submitting.value = true;
  pending.value = `${pkg.name}:${action}`;
  actionError.value = null;
  try {
    await api.softwareAction(pkg.name, action, password);
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
        <fieldset class="tag-toggles">
          <legend class="sr-only">
            {{ t("software.tagVisibility") }}
          </legend>
          <label
            v-for="tag in TAG_FILTER_KEYS"
            :key="tag"
            :class="['tag-toggle', tag === 'installed' ? 'standard' : 'other', { off: isTagHidden(tag) }]"
          >
            <input
              type="checkbox"
              :checked="!isTagHidden(tag)"
              @change="toggleTag(tag)"
            >
            {{ tagLabel(tag) }}
          </label>
        </fieldset>
        <span class="table-count">{{ t("software.count", { count: filteredPackages.length }) }}</span>
      </div>

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
            <tr v-else-if="filteredPackages.length === 0">
              <td
                colspan="5"
                class="empty"
              >
                {{ t("software.empty") }}
              </td>
            </tr>
            <tr
              v-for="pkg in filteredPackages"
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

    <section class="table-panel">
      <div class="table-toolbar">
        <h2 class="panel-title">
          {{ t("software.addTitle") }}
        </h2>
      </div>

      <div
        v-if="addError"
        class="notice error"
        role="alert"
      >
        {{ addError }}
      </div>

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
      <aside class="inline-note">
        {{ t("software.addHelp") }}
      </aside>
    </section>

    <aside class="inline-note">
      {{ t("software.platformNote") }}
    </aside>

    <PasswordConfirmDialog
      :open="armed !== null"
      :title="armed ? `${t(`software.${armed.action}`)}: ${armed.pkg.name}` : ''"
      :description="armed ? t(CONFIRM_KEYS[armed.action], { name: armed.pkg.name }) : null"
      :confirm-label="armed ? t(`software.${armed.action}`) : ''"
      :pending-label="armed ? t(PENDING_LABEL_KEYS[armed.action]) : ''"
      :submitting="submitting"
      :error-message="actionError"
      @confirm="confirmAction"
      @close="closeAction"
    />
  </div>
</template>

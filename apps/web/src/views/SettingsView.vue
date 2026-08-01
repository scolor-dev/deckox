<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { ApiError, api, type SshKeyList, type SystemCapabilities } from "../api/client";
import { apiErrorKey } from "../api/errors";
import { notify } from "../notifications";
import { preferences } from "../preferences";

const emit = defineEmits<{ passwordChanged: [] }>();
const { t } = useI18n();

const currentPassword = ref("");
const newPassword = ref("");
const passwordConfirmation = ref("");
const submitting = ref(false);
const error = ref<string | null>(null);
const sshKeys = ref<SshKeyList | null>(null);
const sshLoading = ref(true);
const sshErrorKey = ref<string | null>(null);
const publicKey = ref("");
const addingKey = ref(false);
const removingKeyId = ref<string | null>(null);
const systemCapabilities = ref<SystemCapabilities | null>(null);
const systemErrorKey = ref<string | null>(null);
const rebootPassword = ref("");
const rebooting = ref(false);

function displaySettingsChanged() {
  notify("success", t("settings.saved"));
}

async function changePassword() {
  error.value = null;
  if (newPassword.value.length < 12) {
    error.value = t("settings.passwordTooShort");
    return;
  }
  if (newPassword.value !== passwordConfirmation.value) {
    error.value = t("settings.passwordMismatch");
    return;
  }

  submitting.value = true;
  try {
    await api.changePassword(currentPassword.value, newPassword.value);
    currentPassword.value = "";
    newPassword.value = "";
    passwordConfirmation.value = "";
    emit("passwordChanged");
  } catch (caught) {
    error.value = t(apiErrorKey(caught, "errors.password"));
  } finally {
    submitting.value = false;
  }
}

async function loadSystemCapabilities() {
  systemErrorKey.value = null;
  try {
    systemCapabilities.value = await api.systemCapabilities();
  } catch (caught) {
    systemErrorKey.value = apiErrorKey(caught, "errors.systemCapabilities");
  }
}

async function rebootSystem() {
  systemErrorKey.value = null;
  if (!rebootPassword.value) {
    systemErrorKey.value = "settings.rebootPasswordRequired";
    return;
  }
  if (!window.confirm(t("settings.confirmReboot"))) return;

  rebooting.value = true;
  try {
    await api.rebootSystem(rebootPassword.value);
    rebootPassword.value = "";
    notify("warning", t("settings.rebootAccepted"));
  } catch (caught) {
    systemErrorKey.value = apiErrorKey(caught, "errors.reboot");
  } finally {
    rebooting.value = false;
  }
}

async function loadSshKeys() {
  sshLoading.value = true;
  sshErrorKey.value = null;
  try {
    sshKeys.value = await api.sshKeys();
  } catch (caught) {
    sshErrorKey.value = apiErrorKey(caught, "errors.sshLoad");
  } finally {
    sshLoading.value = false;
  }
}

async function addSshKey() {
  sshErrorKey.value = null;
  if (!publicKey.value.trim()) {
    sshErrorKey.value = "settings.keyRequired";
    return;
  }
  addingKey.value = true;
  try {
    const added = await api.addSshKey(publicKey.value.trim());
    publicKey.value = "";
    await loadSshKeys();
    notify("success", t("settings.keyAdded", { label: added.comment ?? added.fingerprint }));
  } catch (caught) {
    sshErrorKey.value = apiErrorKey(caught, "errors.sshAdd");
  } finally {
    addingKey.value = false;
  }
}

async function removeSshKey(keyId: string, label: string) {
  if (!window.confirm(t("settings.confirmRemove", { label }))) return;
  removingKeyId.value = keyId;
  sshErrorKey.value = null;
  try {
    const removed = await api.removeSshKey(keyId);
    await loadSshKeys();
    notify("success", t("settings.keyRemoved", { label: removed.comment ?? removed.fingerprint }));
  } catch (caught) {
    sshErrorKey.value = caught instanceof ApiError && caught.status === 409
      ? "settings.lastKey"
      : apiErrorKey(caught, "errors.sshRemove");
  } finally {
    removingKeyId.value = null;
  }
}

onMounted(() => {
  void loadSystemCapabilities();
  void loadSshKeys();
});
</script>

<template>
  <section class="view settings-view">
    <header class="view-header">
      <div>
        <h1>{{ t("settings.title") }}</h1>
        <p class="subtitle">
          {{ t("settings.subtitle") }}
        </p>
      </div>
    </header>

    <section
      class="settings-section"
      aria-labelledby="display-heading"
    >
      <div class="settings-description">
        <h2 id="display-heading">
          {{ t("settings.display") }}
        </h2>
        <p>{{ t("settings.displayDescription") }}</p>
      </div>
      <div class="settings-form">
        <label for="language">{{ t("settings.language") }}</label>
        <select
          id="language"
          v-model="preferences.locale"
          @change="displaySettingsChanged"
        >
          <option value="auto">
            {{ t("settings.languageAuto") }}
          </option>
          <option value="ja">
            {{ t("settings.japanese") }}
          </option>
          <option value="en">
            {{ t("settings.english") }}
          </option>
        </select>

        <label class="checkbox-field">
          <input
            v-model="preferences.realtimeEnabled"
            type="checkbox"
            @change="displaySettingsChanged"
          >
          <span>{{ t("settings.realtime") }}</span>
        </label>
        <small>{{ t("settings.realtimeHelp") }}</small>

        <label for="metrics-interval">{{ t("settings.interval") }}</label>
        <select
          id="metrics-interval"
          v-model.number="preferences.metricsInterval"
          :disabled="!preferences.realtimeEnabled"
          @change="displaySettingsChanged"
        >
          <option
            v-for="seconds in [1, 2, 5]"
            :key="seconds"
            :value="seconds"
          >
            {{ t("settings.intervalValue", { seconds }) }}
          </option>
        </select>
      </div>
    </section>

    <section
      class="settings-section"
      aria-labelledby="system-operations-heading"
    >
      <div class="settings-description">
        <h2 id="system-operations-heading">
          {{ t("settings.systemOperations") }}
        </h2>
        <p>{{ t("settings.systemOperationsDescription") }}</p>
      </div>
      <form
        class="settings-form"
        @submit.prevent="rebootSystem"
      >
        <div
          v-if="systemCapabilities && !systemCapabilities.reboot_allowed"
          class="notice warning"
        >
          {{ t("settings.rebootDisabled") }}
        </div>
        <template v-else>
          <label for="reboot-password">{{ t("settings.rebootPassword") }}</label>
          <input
            id="reboot-password"
            v-model="rebootPassword"
            type="password"
            autocomplete="current-password"
            required
          >
          <small>{{ t("settings.rebootHelp") }}</small>
          <p
            v-if="systemErrorKey"
            class="notice error"
            role="alert"
          >
            {{ t(systemErrorKey) }}
          </p>
          <button
            class="primary-button danger-button settings-submit"
            type="submit"
            :disabled="rebooting || !systemCapabilities?.reboot_allowed"
          >
            {{ rebooting ? t("settings.rebooting") : t("settings.reboot") }}
          </button>
        </template>
      </form>
    </section>

    <section
      class="settings-section"
      aria-labelledby="password-heading"
    >
      <div class="settings-description">
        <h2 id="password-heading">
          {{ t("settings.password") }}
        </h2>
        <p>{{ t("settings.passwordDescription") }}</p>
      </div>
      <form
        class="settings-form"
        @submit.prevent="changePassword"
      >
        <label for="current-password">{{ t("settings.currentPassword") }}</label>
        <input
          id="current-password"
          v-model="currentPassword"
          type="password"
          autocomplete="current-password"
          required
        >

        <label for="new-password">{{ t("settings.newPassword") }}</label>
        <input
          id="new-password"
          v-model="newPassword"
          type="password"
          autocomplete="new-password"
          minlength="12"
          required
        >
        <small>{{ t("settings.passwordRule") }}</small>

        <label for="password-confirmation">{{ t("settings.passwordConfirmation") }}</label>
        <input
          id="password-confirmation"
          v-model="passwordConfirmation"
          type="password"
          autocomplete="new-password"
          minlength="12"
          required
        >

        <p
          v-if="error"
          class="notice error"
          role="alert"
        >
          {{ error }}
        </p>
        <button
          class="primary-button settings-submit"
          type="submit"
          :disabled="submitting"
        >
          {{ submitting ? t("settings.changing") : t("settings.changePassword") }}
        </button>
        <p class="settings-help">
          {{ t("settings.relogin") }}
        </p>
      </form>
    </section>

    <section
      class="settings-section"
      aria-labelledby="ssh-heading"
    >
      <div class="settings-description">
        <h2 id="ssh-heading">
          {{ t("settings.ssh") }}
        </h2>
        <p>{{ t("settings.sshDescription") }}</p>
      </div>
      <div class="settings-form ssh-settings">
        <p
          v-if="sshErrorKey"
          class="notice error"
          role="alert"
        >
          {{ t(sshErrorKey) }}
        </p>
        <p
          v-if="sshLoading"
          class="settings-help"
        >
          {{ t("settings.checkingKeys") }}
        </p>
        <div
          v-else-if="!sshKeys?.enabled"
          class="notice warning"
        >
          {{ t("settings.sshDisabled") }}
        </div>
        <template v-else>
          <p class="managed-user">
            {{ t("settings.managedUser") }} <strong class="mono">{{ sshKeys.managed_user }}</strong>
          </p>

          <div
            v-if="sshKeys.keys.length"
            class="ssh-key-list"
          >
            <article
              v-for="key in sshKeys.keys"
              :key="key.id"
              class="ssh-key-item"
            >
              <div>
                <strong>{{ key.comment ?? t("settings.noComment") }}</strong>
                <span class="mono">{{ key.key_type }}</span>
                <code>{{ key.fingerprint }}</code>
              </div>
              <button
                class="action-button danger"
                type="button"
                :disabled="removingKeyId !== null"
                @click="removeSshKey(key.id, key.comment ?? key.fingerprint)"
              >
                {{ removingKeyId === key.id ? t("settings.deleting") : t("settings.delete") }}
              </button>
            </article>
          </div>
          <p
            v-else
            class="settings-help"
          >
            {{ t("settings.noKeys") }}
          </p>

          <form
            class="ssh-add-form"
            @submit.prevent="addSshKey"
          >
            <label for="public-key">{{ t("settings.addKey") }}</label>
            <textarea
              id="public-key"
              v-model="publicKey"
              rows="4"
              :placeholder="t('settings.keyPlaceholder')"
              spellcheck="false"
              required
            />
            <small>{{ t("settings.publicOnly") }}</small>
            <button
              class="primary-button settings-submit"
              type="submit"
              :disabled="addingKey"
            >
              {{ addingKey ? t("settings.adding") : t("settings.addKey") }}
            </button>
          </form>
        </template>
      </div>
    </section>
  </section>
</template>

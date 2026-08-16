<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import {
  api,
  buildUpdateCommand,
  safeReleaseUrl,
  writeClipboardText,
  type SystemCapabilities,
  type UpdateStatus,
} from "../api/client";
import { apiErrorKey } from "../api/errors";
import { notify } from "../notifications";
import { preferences } from "../preferences";

const emit = defineEmits<{ passwordChanged: [] }>();
const { t, locale } = useI18n();
const router = useRouter();

const currentPassword = ref("");
const newPassword = ref("");
const passwordConfirmation = ref("");
const submitting = ref(false);
const error = ref<string | null>(null);
const systemCapabilities = ref<SystemCapabilities | null>(null);
const systemErrorKey = ref<string | null>(null);
const rebootPassword = ref("");
const rebooting = ref(false);
const updateStatus = ref<UpdateStatus | null>(null);
const updateChecking = ref(false);
const updateErrorKey = ref<string | null>(null);
const updateCommand = computed(() => buildUpdateCommand(updateStatus.value?.latest_version));
const releaseUrl = computed(() => safeReleaseUrl(updateStatus.value?.release_url));
const updateCheckedAt = computed(() => {
  const checkedAt = updateStatus.value?.checked_at_ms;
  if (checkedAt == null) return null;
  return new Intl.DateTimeFormat(locale.value, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(checkedAt);
});

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
    const health = await api.health().catch(() => null);
    await api.rebootSystem(rebootPassword.value);
    rebootPassword.value = "";
    if (health) sessionStorage.setItem("deckox:restart-instance", health.instance_id);
    await router.push({ name: "restarting" });
  } catch (caught) {
    systemErrorKey.value = apiErrorKey(caught, "errors.reboot");
  } finally {
    rebooting.value = false;
  }
}

async function checkForUpdate() {
  updateChecking.value = true;
  updateErrorKey.value = null;
  try {
    updateStatus.value = await api.updateStatus();
  } catch (caught) {
    updateErrorKey.value = apiErrorKey(caught, "errors.updateCheck");
  } finally {
    updateChecking.value = false;
  }
}

async function copyUpdateCommand() {
  if (!updateCommand.value) {
    updateErrorKey.value = "settings.updateCommandUnavailable";
    return;
  }
  updateErrorKey.value = null;
  if (await writeClipboardText(updateCommand.value)) {
    notify("success", t("settings.updateCommandCopied"));
  } else {
    updateErrorKey.value = "settings.updateCopyFailed";
    notify("error", t("settings.updateCopyFailed"));
  }
}

onMounted(() => {
  void loadSystemCapabilities();
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
      aria-labelledby="update-heading"
    >
      <div class="settings-description">
        <h2 id="update-heading">
          {{ t("settings.update") }}
        </h2>
        <p>{{ t("settings.updateDescription") }}</p>
      </div>
      <div class="settings-form update-settings">
        <p
          v-if="updateErrorKey"
          class="notice error"
          role="alert"
        >
          {{ t(updateErrorKey) }}
        </p>
        <template v-if="updateStatus">
          <dl class="update-versions">
            <div><dt>{{ t("settings.currentVersion") }}</dt><dd>{{ updateStatus.current_version }}</dd></div>
            <div><dt>{{ t("settings.latestVersion") }}</dt><dd>{{ updateStatus.latest_version ?? t("common.none") }}</dd></div>
          </dl>
          <div
            v-if="updateStatus.status === 'available' && updateStatus.update_available"
            class="notice success update-notice"
          >
            {{ t("settings.updateAvailable") }}
          </div>
          <div
            v-else-if="updateStatus.status === 'up_to_date'"
            class="notice success update-notice"
          >
            {{ t("settings.upToDate") }}
          </div>
          <div
            v-else
            class="notice warning update-notice"
          >
            {{ t("settings.updateUnavailable") }}
          </div>
          <small v-if="updateCheckedAt">{{ t("settings.updateCheckedAt", { time: updateCheckedAt }) }}</small>
          <a
            v-if="releaseUrl"
            class="release-link"
            :href="releaseUrl"
            target="_blank"
            rel="noopener noreferrer"
          >{{ t("settings.openRelease") }}</a>
          <div
            v-if="updateStatus.update_available && updateCommand"
            class="update-command"
          >
            <code>{{ updateCommand }}</code>
            <button
              class="action-button"
              type="button"
              @click="copyUpdateCommand"
            >
              {{ t("settings.copyCommand") }}
            </button>
            <small>{{ t("settings.manualUpdateOnly") }}</small>
          </div>
        </template>
        <p
          v-else-if="!updateChecking"
          class="settings-help"
        >
          {{ t("settings.updateNotChecked") }}
        </p>
        <button
          class="button update-check-button"
          type="button"
          :disabled="updateChecking"
          @click="checkForUpdate"
        >
          {{ updateChecking ? t("settings.checkingUpdate") : t("settings.checkUpdate") }}
        </button>
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
  </section>
</template>

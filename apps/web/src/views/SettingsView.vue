<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import {
  api,
  buildUpdateCommand,
  safeReleaseUrl,
  writeClipboardText,
  type ServerStatus,
  type SystemCapabilities,
  type TotpStatus,
  type UpdateStatus,
} from "../api/client";
import { apiErrorKey } from "../api/errors";
import ForgotPasswordHelp from "../components/ForgotPasswordHelp.vue";
import PasswordConfirmDialog from "../components/PasswordConfirmDialog.vue";
import { notify } from "../notifications";
import { preferences } from "../preferences";

const emit = defineEmits<{ passwordChanged: [] }>();
const { t, locale } = useI18n();
const router = useRouter();
const route = useRoute();

type SettingsTab = "display" | "security" | "webhook" | "system";
const TAB_KEYS: SettingsTab[] = ["display", "security", "webhook", "system"];

function isSettingsTab(value: unknown): value is SettingsTab {
  return typeof value === "string" && (TAB_KEYS as string[]).includes(value);
}

const activeTab = ref<SettingsTab>(isSettingsTab(route.query.tab) ? route.query.tab : "display");

function selectTab(tab: SettingsTab) {
  activeTab.value = tab;
  void router.replace({ query: { ...route.query, tab } });
}

const currentPassword = ref("");
const newPassword = ref("");
const passwordConfirmation = ref("");
const submitting = ref(false);
const error = ref<string | null>(null);
const systemCapabilities = ref<SystemCapabilities | null>(null);
const rebootDialogOpen = ref(false);
const rebooting = ref(false);
const rebootError = ref<string | null>(null);
const updateStatus = ref<UpdateStatus | null>(null);
const updateChecking = ref(false);
const updateErrorKey = ref<string | null>(null);
const updateDialogOpen = ref(false);
const updating = ref(false);
const updateDialogError = ref<string | null>(null);
const totpStatus = ref<TotpStatus | null>(null);
const totpLoading = ref(true);
const totpErrorKey = ref<string | null>(null);
const totpSetupSecret = ref<string | null>(null);
const totpConfirmCode = ref("");
const totpConfirming = ref(false);
const totpRecoveryCodes = ref<string[] | null>(null);
const totpDisablePassword = ref("");
const totpDisableCode = ref("");
const totpDisabling = ref(false);
const serverStatus = ref<ServerStatus | null>(null);
const webhookTesting = ref(false);
const webhookErrorKey = ref<string | null>(null);
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

async function loadTotpStatus() {
  totpLoading.value = true;
  totpErrorKey.value = null;
  try {
    totpStatus.value = await api.totpStatus();
  } catch (caught) {
    totpErrorKey.value = apiErrorKey(caught, "errors.totpStatus");
  } finally {
    totpLoading.value = false;
  }
}

async function startTotpSetup() {
  totpErrorKey.value = null;
  try {
    const setup = await api.totpSetup();
    totpSetupSecret.value = setup.secret_base32;
  } catch (caught) {
    totpErrorKey.value = apiErrorKey(caught, "errors.totpSetup");
  }
}

function cancelTotpSetup() {
  totpSetupSecret.value = null;
  totpConfirmCode.value = "";
  totpErrorKey.value = null;
}

async function copyTotpSecret() {
  if (!totpSetupSecret.value) return;
  if (await writeClipboardText(totpSetupSecret.value)) {
    notify("success", t("settings.totpSecretCopied"));
  } else {
    notify("error", t("settings.totpCopyFailed"));
  }
}

async function confirmTotpSetup() {
  totpErrorKey.value = null;
  totpConfirming.value = true;
  try {
    const result = await api.totpConfirm(totpConfirmCode.value);
    totpRecoveryCodes.value = result.recovery_codes;
    totpConfirmCode.value = "";
    totpSetupSecret.value = null;
    await loadTotpStatus();
  } catch (caught) {
    totpErrorKey.value = apiErrorKey(caught, "errors.totpConfirm");
  } finally {
    totpConfirming.value = false;
  }
}

function acknowledgeRecoveryCodes() {
  totpRecoveryCodes.value = null;
  notify("success", t("settings.totpEnabled"));
}

async function disableTotp() {
  totpErrorKey.value = null;
  if (!window.confirm(t("settings.confirmTotpDisable"))) return;

  totpDisabling.value = true;
  try {
    totpStatus.value = await api.totpDisable(totpDisablePassword.value, totpDisableCode.value);
    totpDisablePassword.value = "";
    totpDisableCode.value = "";
    notify("success", t("settings.totpDisabled"));
  } catch (caught) {
    totpErrorKey.value = apiErrorKey(caught, "errors.totpDisable");
  } finally {
    totpDisabling.value = false;
  }
}

async function loadSystemCapabilities() {
  try {
    systemCapabilities.value = await api.systemCapabilities();
  } catch (caught) {
    rebootError.value = t(apiErrorKey(caught, "errors.systemCapabilities"));
  }
}

function openRebootDialog() {
  rebootError.value = null;
  rebootDialogOpen.value = true;
}

function closeRebootDialog() {
  rebootDialogOpen.value = false;
  rebootError.value = null;
}

async function rebootSystem(password: string) {
  rebooting.value = true;
  rebootError.value = null;
  try {
    const health = await api.health().catch(() => null);
    await api.rebootSystem(password);
    if (health) sessionStorage.setItem("deckox:restart-instance", health.instance_id);
    rebootDialogOpen.value = false;
    await router.push({ name: "restarting" });
  } catch (caught) {
    rebootError.value = t(apiErrorKey(caught, "errors.reboot"));
  } finally {
    rebooting.value = false;
  }
}

function openUpdateDialog() {
  updateDialogError.value = null;
  updateDialogOpen.value = true;
}

function closeUpdateDialog() {
  updateDialogOpen.value = false;
  updateDialogError.value = null;
}

async function triggerUpdate(password: string) {
  updating.value = true;
  updateDialogError.value = null;
  try {
    const health = await api.health().catch(() => null);
    await api.triggerUpdate(password);
    if (health) sessionStorage.setItem("deckox:restart-instance", health.instance_id);
    updateDialogOpen.value = false;
    await router.push({ name: "restarting" });
  } catch (caught) {
    updateDialogError.value = t(apiErrorKey(caught, "errors.updateTrigger"));
  } finally {
    updating.value = false;
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

async function loadServerStatus() {
  try {
    serverStatus.value = await api.serverStatus();
  } catch {
    // Overview already surfaces connectivity problems; this section only
    // needs webhook_configured, so a failed fetch just leaves it unknown.
  }
}

async function testWebhook() {
  webhookErrorKey.value = null;
  webhookTesting.value = true;
  try {
    await api.testWebhook();
    notify("success", t("settings.webhookTestSuccess"));
  } catch (caught) {
    webhookErrorKey.value = apiErrorKey(caught, "errors.webhookTestFailed");
    notify("error", t(webhookErrorKey.value));
  } finally {
    webhookTesting.value = false;
  }
}

onMounted(() => {
  void loadSystemCapabilities();
  void loadTotpStatus();
  void loadServerStatus();
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

    <div
      class="settings-tabs"
      role="tablist"
      :aria-label="t('settings.tabsLabel')"
    >
      <button
        v-for="tab in TAB_KEYS"
        :key="tab"
        type="button"
        role="tab"
        :aria-selected="activeTab === tab"
        :class="['settings-tab', { active: activeTab === tab }]"
        @click="selectTab(tab)"
      >
        {{ t(`settings.tab.${tab}`) }}
      </button>
    </div>

    <div
      v-show="activeTab === 'display'"
      role="tabpanel"
    >
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
    </div>

    <div
      v-show="activeTab === 'security'"
      role="tabpanel"
    >
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
          <ForgotPasswordHelp />

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
        aria-labelledby="totp-heading"
      >
        <div class="settings-description">
          <h2 id="totp-heading">
            {{ t("settings.totp") }}
          </h2>
          <p>{{ t("settings.totpDescription") }}</p>
        </div>
        <div class="settings-form">
          <p
            v-if="totpErrorKey"
            class="notice error"
            role="alert"
          >
            {{ t(totpErrorKey) }}
          </p>
          <p
            v-if="totpLoading"
            class="settings-help"
          >
            {{ t("settings.totpChecking") }}
          </p>

          <template v-else-if="totpRecoveryCodes">
            <div class="notice warning">
              <p>{{ t("settings.totpRecoveryCodesIntro") }}</p>
              <pre class="recovery-codes"><code>{{ totpRecoveryCodes.join("\n") }}</code></pre>
            </div>
            <button
              class="primary-button settings-submit"
              type="button"
              @click="acknowledgeRecoveryCodes"
            >
              {{ t("settings.totpRecoveryCodesAck") }}
            </button>
          </template>

          <template v-else-if="totpSetupSecret">
            <p>{{ t("settings.totpSetupIntro") }}</p>
            <dl class="totp-secret">
              <div>
                <dt>{{ t("settings.totpSecret") }}</dt><dd class="mono">
                  {{ totpSetupSecret }}
                </dd>
              </div>
            </dl>
            <button
              class="action-button"
              type="button"
              @click="copyTotpSecret"
            >
              {{ t("settings.totpCopySecret") }}
            </button>
            <form
              class="totp-confirm-form"
              @submit.prevent="confirmTotpSetup"
            >
              <label for="totp-confirm-code">{{ t("settings.totpConfirmCode") }}</label>
              <input
                id="totp-confirm-code"
                v-model="totpConfirmCode"
                type="text"
                inputmode="numeric"
                autocomplete="one-time-code"
                required
              >
              <button
                class="primary-button settings-submit"
                type="submit"
                :disabled="totpConfirming || totpConfirmCode.length === 0"
              >
                {{ totpConfirming ? t("settings.totpConfirming") : t("settings.totpConfirmSubmit") }}
              </button>
              <button
                class="action-button"
                type="button"
                @click="cancelTotpSetup"
              >
                {{ t("settings.cancel") }}
              </button>
            </form>
          </template>

          <template v-else-if="totpStatus?.enabled">
            <p class="settings-help">
              {{ t("settings.totpEnabledStatus", { count: totpStatus.recovery_codes_remaining }) }}
            </p>
            <form
              class="settings-form"
              @submit.prevent="disableTotp"
            >
              <label for="totp-disable-password">{{ t("settings.currentPassword") }}</label>
              <input
                id="totp-disable-password"
                v-model="totpDisablePassword"
                type="password"
                autocomplete="current-password"
                required
              >
              <label for="totp-disable-code">{{ t("login.totpCode") }}</label>
              <input
                id="totp-disable-code"
                v-model="totpDisableCode"
                type="text"
                inputmode="numeric"
                autocomplete="one-time-code"
                required
              >
              <button
                class="primary-button danger-button settings-submit"
                type="submit"
                :disabled="totpDisabling"
              >
                {{ totpDisabling ? t("settings.totpDisabling") : t("settings.totpDisable") }}
              </button>
            </form>
          </template>

          <button
            v-else
            class="primary-button settings-submit"
            type="button"
            @click="startTotpSetup"
          >
            {{ t("settings.totpEnable") }}
          </button>
        </div>
      </section>
    </div>

    <div
      v-show="activeTab === 'webhook'"
      role="tabpanel"
    >
      <section
        class="settings-section"
        aria-labelledby="webhook-heading"
      >
        <div class="settings-description">
          <h2 id="webhook-heading">
            {{ t("settings.webhookTitle") }}
          </h2>
          <p>{{ t("settings.webhookDescription") }}</p>
        </div>
        <div class="settings-form">
          <template v-if="serverStatus">
            <div
              v-if="!serverStatus.webhook_configured"
              class="notice warning"
            >
              {{ t("settings.webhookNotConfigured") }}
            </div>
            <template v-else>
              <div class="notice success">
                {{ t("settings.webhookConfigured") }}
              </div>
              <p
                v-if="webhookErrorKey"
                class="notice error"
                role="alert"
              >
                {{ t(webhookErrorKey) }}
              </p>
              <button
                class="button"
                type="button"
                :disabled="webhookTesting"
                @click="testWebhook"
              >
                {{ webhookTesting ? t("settings.webhookTesting") : t("settings.webhookTest") }}
              </button>
            </template>
          </template>
        </div>
      </section>
    </div>

    <div
      v-show="activeTab === 'system'"
      role="tabpanel"
    >
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
            <template v-if="updateStatus.update_available && systemCapabilities?.update_allowed">
              <small>{{ t("settings.updateHelp") }}</small>
              <button
                class="primary-button danger-button settings-submit"
                type="button"
                @click="openUpdateDialog"
              >
                {{ t("settings.updateNow") }}
              </button>
            </template>
            <div
              v-else-if="updateStatus.update_available && updateCommand"
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
        <div class="settings-form">
          <div
            v-if="systemCapabilities && !systemCapabilities.reboot_allowed"
            class="notice warning"
          >
            {{ t("settings.rebootDisabled") }}
          </div>
          <template v-else>
            <small>{{ t("settings.rebootHelp") }}</small>
            <button
              class="primary-button danger-button settings-submit"
              type="button"
              :disabled="!systemCapabilities?.reboot_allowed"
              @click="openRebootDialog"
            >
              {{ t("settings.reboot") }}
            </button>
          </template>
        </div>
      </section>
    </div>

    <PasswordConfirmDialog
      :open="rebootDialogOpen"
      :title="t('settings.reboot')"
      :description="t('settings.confirmReboot')"
      :confirm-label="t('settings.reboot')"
      :pending-label="t('settings.rebooting')"
      :submitting="rebooting"
      :error-message="rebootError"
      @confirm="rebootSystem"
      @close="closeRebootDialog"
    />

    <PasswordConfirmDialog
      :open="updateDialogOpen"
      :title="t('settings.updateNow')"
      :description="t('settings.confirmUpdate')"
      :confirm-label="t('settings.updateNow')"
      :pending-label="t('settings.updating')"
      :submitting="updating"
      :error-message="updateDialogError"
      @confirm="triggerUpdate"
      @close="closeUpdateDialog"
    />
  </section>
</template>

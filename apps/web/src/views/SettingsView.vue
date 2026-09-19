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
import {
  AppButton,
  AppCheckbox,
  AppStack,
  DetailList,
  DetailRow,
  InfoNote,
  NoticeBanner,
  PageHeader,
  SelectField,
  TabBar,
  TextField,
} from "../design-system/components";
import { notify } from "../notifications";
import { preferences, type LocalePreference, type MetricsInterval, type ThemePreference } from "../preferences";

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

const tabs = computed(() => TAB_KEYS.map((key) => ({ key, label: t(`settings.tab.${key}`) })));

function handleTabChange(key: string) {
  if (isSettingsTab(key)) selectTab(key);
}

const localeValue = computed({
  get: () => preferences.locale,
  set: (value: string) => {
    preferences.locale = value as LocalePreference;
    displaySettingsChanged();
  },
});
const themeValue = computed({
  get: () => preferences.theme,
  set: (value: string) => {
    preferences.theme = value as ThemePreference;
    displaySettingsChanged();
  },
});
const realtimeValue = computed({
  get: () => preferences.realtimeEnabled,
  set: (value: boolean) => {
    preferences.realtimeEnabled = value;
    displaySettingsChanged();
  },
});
const intervalValue = computed({
  get: () => String(preferences.metricsInterval),
  set: (value: string) => {
    preferences.metricsInterval = Number(value) as MetricsInterval;
    displaySettingsChanged();
  },
});

const localeOptions = computed(() => [
  { value: "auto", label: t("settings.languageAuto") },
  { value: "ja", label: t("settings.japanese") },
  { value: "en", label: t("settings.english") },
]);
const themeOptions = computed(() => [
  { value: "auto", label: t("settings.themeAuto") },
  { value: "light", label: t("settings.themeLight") },
  { value: "dark", label: t("settings.themeDark") },
]);
const intervalOptions = computed(() =>
  [1, 2, 5].map((seconds) => ({ value: String(seconds), label: t("settings.intervalValue", { seconds }) })),
);

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
    <PageHeader
      :title="t('settings.title')"
      :subtitle="t('settings.subtitle')"
    />

    <TabBar
      :tabs="tabs"
      :model-value="activeTab"
      :label="t('settings.tabsLabel')"
      @update:model-value="handleTabChange"
    />

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
        <AppStack gap="3">
          <SelectField
            id="language"
            v-model="localeValue"
            :label="t('settings.language')"
            :options="localeOptions"
          />
          <SelectField
            id="theme"
            v-model="themeValue"
            :label="t('settings.theme')"
            :options="themeOptions"
          />
          <AppCheckbox
            v-model="realtimeValue"
            :label="t('settings.realtime')"
            :help="t('settings.realtimeHelp')"
          />
          <SelectField
            id="metrics-interval"
            v-model="intervalValue"
            :label="t('settings.interval')"
            :options="intervalOptions"
            :disabled="!preferences.realtimeEnabled"
          />
        </AppStack>
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
        <AppStack
          as="form"
          gap="3"
          @submit.prevent="changePassword"
        >
          <TextField
            id="current-password"
            v-model="currentPassword"
            :label="t('settings.currentPassword')"
            type="password"
            autocomplete="current-password"
            required
          />
          <ForgotPasswordHelp />
          <TextField
            id="new-password"
            v-model="newPassword"
            :label="t('settings.newPassword')"
            :help="t('settings.passwordRule')"
            type="password"
            autocomplete="new-password"
            minlength="12"
            required
          />
          <TextField
            id="password-confirmation"
            v-model="passwordConfirmation"
            :label="t('settings.passwordConfirmation')"
            type="password"
            autocomplete="new-password"
            minlength="12"
            required
          />
          <NoticeBanner
            v-if="error"
            tone="error"
          >
            {{ error }}
          </NoticeBanner>
          <AppStack
            align="start"
            gap="2"
          >
            <AppButton
              variant="primary"
              type="submit"
              :disabled="submitting"
            >
              {{ submitting ? t("settings.changing") : t("settings.changePassword") }}
            </AppButton>
            <InfoNote>{{ t("settings.relogin") }}</InfoNote>
          </AppStack>
        </AppStack>
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
        <AppStack
          gap="3"
          align="start"
        >
          <NoticeBanner
            v-if="totpErrorKey"
            tone="error"
          >
            {{ t(totpErrorKey) }}
          </NoticeBanner>
          <InfoNote v-if="totpLoading">
            {{ t("settings.totpChecking") }}
          </InfoNote>

          <template v-else-if="totpRecoveryCodes">
            <NoticeBanner tone="warning">
              <p>{{ t("settings.totpRecoveryCodesIntro") }}</p>
              <pre class="recovery-codes"><code>{{ totpRecoveryCodes.join("\n") }}</code></pre>
            </NoticeBanner>
            <AppButton
              variant="primary"
              @click="acknowledgeRecoveryCodes"
            >
              {{ t("settings.totpRecoveryCodesAck") }}
            </AppButton>
          </template>

          <template v-else-if="totpSetupSecret">
            <p>{{ t("settings.totpSetupIntro") }}</p>
            <p class="totp-secret">
              <small>{{ t("settings.totpSecret") }}</small>
              <code>{{ totpSetupSecret }}</code>
            </p>
            <AppButton
              variant="action"
              @click="copyTotpSecret"
            >
              {{ t("settings.totpCopySecret") }}
            </AppButton>
            <AppStack
              as="form"
              gap="3"
              @submit.prevent="confirmTotpSetup"
            >
              <TextField
                id="totp-confirm-code"
                v-model="totpConfirmCode"
                :label="t('settings.totpConfirmCode')"
                inputmode="numeric"
                autocomplete="one-time-code"
                required
              />
              <AppStack
                direction="row"
                gap="2"
                wrap
              >
                <AppButton
                  variant="primary"
                  type="submit"
                  :disabled="totpConfirming || totpConfirmCode.length === 0"
                >
                  {{ totpConfirming ? t("settings.totpConfirming") : t("settings.totpConfirmSubmit") }}
                </AppButton>
                <AppButton
                  variant="action"
                  @click="cancelTotpSetup"
                >
                  {{ t("settings.cancel") }}
                </AppButton>
              </AppStack>
            </AppStack>
          </template>

          <template v-else-if="totpStatus?.enabled">
            <InfoNote>
              {{ t("settings.totpEnabledStatus", { count: totpStatus.recovery_codes_remaining }) }}
            </InfoNote>
            <AppStack
              as="form"
              gap="3"
              @submit.prevent="disableTotp"
            >
              <TextField
                id="totp-disable-password"
                v-model="totpDisablePassword"
                :label="t('settings.currentPassword')"
                type="password"
                autocomplete="current-password"
                required
              />
              <TextField
                id="totp-disable-code"
                v-model="totpDisableCode"
                :label="t('login.totpCode')"
                inputmode="numeric"
                autocomplete="one-time-code"
                required
              />
              <AppButton
                variant="primary"
                type="submit"
                danger
                :disabled="totpDisabling"
              >
                {{ totpDisabling ? t("settings.totpDisabling") : t("settings.totpDisable") }}
              </AppButton>
            </AppStack>
          </template>

          <AppButton
            v-else
            variant="primary"
            @click="startTotpSetup"
          >
            {{ t("settings.totpEnable") }}
          </AppButton>
        </AppStack>
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
        <AppStack
          gap="3"
          align="start"
        >
          <template v-if="serverStatus">
            <NoticeBanner
              v-if="!serverStatus.webhook_configured"
              tone="warning"
            >
              {{ t("settings.webhookNotConfigured") }}
            </NoticeBanner>
            <template v-else>
              <NoticeBanner tone="success">
                {{ t("settings.webhookConfigured") }}
              </NoticeBanner>
              <NoticeBanner
                v-if="webhookErrorKey"
                tone="error"
              >
                {{ t(webhookErrorKey) }}
              </NoticeBanner>
              <AppButton
                :disabled="webhookTesting"
                @click="testWebhook"
              >
                {{ webhookTesting ? t("settings.webhookTesting") : t("settings.webhookTest") }}
              </AppButton>
            </template>
          </template>
        </AppStack>
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
        <AppStack
          class="update-settings"
          gap="3"
          align="start"
        >
          <NoticeBanner
            v-if="updateErrorKey"
            tone="error"
          >
            {{ t(updateErrorKey) }}
          </NoticeBanner>
          <template v-if="updateStatus">
            <DetailList>
              <DetailRow :term="t('settings.currentVersion')">
                {{ updateStatus.current_version }}
              </DetailRow>
              <DetailRow :term="t('settings.latestVersion')">
                {{ updateStatus.latest_version ?? t("common.none") }}
              </DetailRow>
            </DetailList>
            <NoticeBanner
              v-if="updateStatus.status === 'available' && updateStatus.update_available"
              tone="success"
            >
              {{ t("settings.updateAvailable") }}
            </NoticeBanner>
            <NoticeBanner
              v-else-if="updateStatus.status === 'up_to_date'"
              tone="success"
            >
              {{ t("settings.upToDate") }}
            </NoticeBanner>
            <NoticeBanner
              v-else
              tone="warning"
            >
              {{ t("settings.updateUnavailable") }}
            </NoticeBanner>
            <InfoNote v-if="updateCheckedAt">
              {{ t("settings.updateCheckedAt", { time: updateCheckedAt }) }}
            </InfoNote>
            <a
              v-if="releaseUrl"
              class="release-link"
              :href="releaseUrl"
              target="_blank"
              rel="noopener noreferrer"
            >{{ t("settings.openRelease") }}</a>
            <template v-if="updateStatus.update_available && systemCapabilities?.update_allowed">
              <InfoNote>{{ t("settings.updateHelp") }}</InfoNote>
              <AppButton
                variant="primary"
                danger
                @click="openUpdateDialog"
              >
                {{ t("settings.updateNow") }}
              </AppButton>
            </template>
            <AppStack
              v-else-if="updateStatus.update_available && updateCommand"
              gap="2"
              align="start"
            >
              <code>{{ updateCommand }}</code>
              <AppButton
                variant="action"
                @click="copyUpdateCommand"
              >
                {{ t("settings.copyCommand") }}
              </AppButton>
              <InfoNote>{{ t("settings.manualUpdateOnly") }}</InfoNote>
            </AppStack>
          </template>
          <InfoNote v-else-if="!updateChecking">
            {{ t("settings.updateNotChecked") }}
          </InfoNote>
          <AppButton
            :disabled="updateChecking"
            @click="checkForUpdate"
          >
            {{ updateChecking ? t("settings.checkingUpdate") : t("settings.checkUpdate") }}
          </AppButton>
        </AppStack>
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
        <AppStack
          gap="3"
          align="start"
        >
          <NoticeBanner
            v-if="systemCapabilities && !systemCapabilities.reboot_allowed"
            tone="warning"
          >
            {{ t("settings.rebootDisabled") }}
          </NoticeBanner>
          <template v-else>
            <InfoNote>{{ t("settings.rebootHelp") }}</InfoNote>
            <AppButton
              variant="primary"
              danger
              :disabled="!systemCapabilities?.reboot_allowed"
              @click="openRebootDialog"
            >
              {{ t("settings.reboot") }}
            </AppButton>
          </template>
        </AppStack>
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

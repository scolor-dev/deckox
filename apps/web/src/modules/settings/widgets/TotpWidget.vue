<script setup lang="ts">
import { onActivated, onDeactivated, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, writeClipboardText, type TotpStatus } from "../../../api/client";
import { apiErrorKey } from "../../../api/errors";
import {
  AppButton,
  AppStack,
  AppText,
  InfoNote,
  NoticeBanner,
  TextField,
} from "../../../design-system/components";
import { notify } from "../../../notifications";

defineOptions({ inheritAttrs: false });

const { t } = useI18n();

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

let mounted = false;

onMounted(() => {
  mounted = true;
  void loadTotpStatus();
});

onActivated(() => {
  if (mounted) mounted = false;
  else void loadTotpStatus();
});

onDeactivated(() => {
  totpSetupSecret.value = null;
  totpConfirmCode.value = "";
  totpRecoveryCodes.value = null;
  totpDisablePassword.value = "";
  totpDisableCode.value = "";
  totpErrorKey.value = null;
});
</script>

<template>
  <AppStack
    gap="3"
    align="start"
  >
    <AppText
      as="p"
      tone="muted"
      size="sm"
    >
      {{ t("settings.totpDescription") }}
    </AppText>
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
        <AppText
          as="p"
          block
        >
          {{ t("settings.totpRecoveryCodesIntro") }}
        </AppText>
        <AppText
          as="pre"
          mono
          preserve
          block
        >
          {{ totpRecoveryCodes.join("\n") }}
        </AppText>
      </NoticeBanner>
      <AppButton
        variant="primary"
        @click="acknowledgeRecoveryCodes"
      >
        {{ t("settings.totpRecoveryCodesAck") }}
      </AppButton>
    </template>

    <template v-else-if="totpSetupSecret">
      <AppText
        as="p"
        block
      >
        {{ t("settings.totpSetupIntro") }}
      </AppText>
      <AppStack gap="1">
        <AppText
          tone="faint"
          size="xs"
        >
          {{ t("settings.totpSecret") }}
        </AppText>
        <AppText
          mono
          strong
          size="sm"
        >
          {{ totpSetupSecret }}
        </AppText>
      </AppStack>
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
</template>

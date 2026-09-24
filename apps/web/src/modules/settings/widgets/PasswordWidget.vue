<script setup lang="ts">
import { onDeactivated, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api } from "../../../api/client";
import { apiErrorKey } from "../../../api/errors";
import ForgotPasswordHelp from "../../../components/ForgotPasswordHelp.vue";
import {
  AppButton,
  AppStack,
  AppText,
  InfoNote,
  NoticeBanner,
  TextField,
} from "../../../design-system/components";
import { announcePasswordChange } from "../../../session";

defineOptions({ inheritAttrs: false });

const { t } = useI18n();

const currentPassword = ref("");
const newPassword = ref("");
const passwordConfirmation = ref("");
const submitting = ref(false);
const error = ref<string | null>(null);

function forget() {
  currentPassword.value = "";
  newPassword.value = "";
  passwordConfirmation.value = "";
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
    forget();
    announcePasswordChange();
  } catch (caught) {
    error.value = t(apiErrorKey(caught, "errors.password"));
  } finally {
    submitting.value = false;
  }
}

onDeactivated(() => {
  forget();
  error.value = null;
});
</script>

<template>
  <AppStack
    as="form"
    gap="3"
    @submit.prevent="changePassword"
  >
    <AppText
      as="p"
      tone="muted"
      size="sm"
    >
      {{ t("settings.passwordDescription") }}
    </AppText>
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
</template>

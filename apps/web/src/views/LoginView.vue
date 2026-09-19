<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { api } from "../api/client";
import { apiErrorKey } from "../api/errors";
import ForgotPasswordHelp from "../components/ForgotPasswordHelp.vue";
import { AppButton, AppStack, NoticeBanner, TextField } from "../design-system/components";

defineProps<{
  message?: string | null;
}>();

const emit = defineEmits<{
  authenticated: [];
}>();
const { t } = useI18n();

const password = ref("");
const totpCode = ref("");
const step = ref<"password" | "totp">("password");
const submitting = ref(false);
const errorMessage = ref("");

async function submitPassword() {
  submitting.value = true;
  errorMessage.value = "";

  try {
    const status = await api.login(password.value);
    password.value = "";
    if (status.totp_required) {
      step.value = "totp";
    } else {
      emit("authenticated");
    }
  } catch (error) {
    errorMessage.value = t(apiErrorKey(error, "errors.login"));
  } finally {
    submitting.value = false;
  }
}

async function submitTotp() {
  submitting.value = true;
  errorMessage.value = "";

  try {
    await api.loginTotp(totpCode.value);
    totpCode.value = "";
    emit("authenticated");
  } catch (error) {
    errorMessage.value = t(apiErrorKey(error, "errors.login"));
  } finally {
    submitting.value = false;
  }
}

function backToPassword() {
  step.value = "password";
  totpCode.value = "";
  errorMessage.value = "";
}
</script>

<template>
  <main class="auth-page">
    <section
      class="login-card"
      aria-labelledby="login-title"
    >
      <div class="login-brand">
        <span class="brand-mark">D</span>
        <div>
          <strong>Deckox</strong>
          <small>{{ t("app.serverManagement") }}</small>
        </div>
      </div>

      <h1 id="login-title">
        {{ t("login.title") }}
      </h1>
      <p v-if="step === 'password'">
        {{ t("login.description") }}
      </p>
      <p v-else>
        {{ t("login.totpDescription") }}
      </p>
      <NoticeBanner
        v-if="message"
        tone="success"
      >
        {{ message }}
      </NoticeBanner>

      <AppStack
        v-if="step === 'password'"
        as="form"
        gap="3"
        @submit.prevent="submitPassword"
      >
        <TextField
          id="password"
          v-model="password"
          :label="t('login.password')"
          type="password"
          name="password"
          autocomplete="current-password"
          required
          autofocus
        />
        <NoticeBanner
          v-if="errorMessage"
          tone="error"
        >
          {{ errorMessage }}
        </NoticeBanner>
        <AppButton
          variant="primary"
          type="submit"
          :disabled="submitting || password.length === 0"
        >
          {{ submitting ? t("login.submitting") : t("login.submit") }}
        </AppButton>
      </AppStack>

      <AppStack
        v-else
        as="form"
        gap="3"
        @submit.prevent="submitTotp"
      >
        <TextField
          id="totp-code"
          v-model="totpCode"
          :label="t('login.totpCode')"
          inputmode="numeric"
          autocomplete="one-time-code"
          :placeholder="t('login.totpPlaceholder')"
          :help="t('login.totpHelp')"
          required
          autofocus
        />
        <NoticeBanner
          v-if="errorMessage"
          tone="error"
        >
          {{ errorMessage }}
        </NoticeBanner>
        <AppButton
          variant="primary"
          type="submit"
          :disabled="submitting || totpCode.length === 0"
        >
          {{ submitting ? t("login.submitting") : t("login.totpSubmit") }}
        </AppButton>
        <AppButton
          variant="action"
          @click="backToPassword"
        >
          {{ t("login.totpBack") }}
        </AppButton>
      </AppStack>

      <ForgotPasswordHelp v-if="step === 'password'" />
    </section>
  </main>
</template>

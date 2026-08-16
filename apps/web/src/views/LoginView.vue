<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { api } from "../api/client";
import { apiErrorKey } from "../api/errors";
import ForgotPasswordHelp from "../components/ForgotPasswordHelp.vue";

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
      <p
        v-if="message"
        class="notice success login-notice"
        role="status"
      >
        {{ message }}
      </p>

      <form
        v-if="step === 'password'"
        @submit.prevent="submitPassword"
      >
        <label for="password">{{ t("login.password") }}</label>
        <input
          id="password"
          v-model="password"
          type="password"
          name="password"
          autocomplete="current-password"
          required
          autofocus
        >
        <p
          v-if="errorMessage"
          class="login-error"
          role="alert"
        >
          {{ errorMessage }}
        </p>
        <button
          class="primary-button"
          type="submit"
          :disabled="submitting || password.length === 0"
        >
          {{ submitting ? t("login.submitting") : t("login.submit") }}
        </button>
      </form>

      <form
        v-else
        @submit.prevent="submitTotp"
      >
        <label for="totp-code">{{ t("login.totpCode") }}</label>
        <input
          id="totp-code"
          v-model="totpCode"
          type="text"
          inputmode="numeric"
          autocomplete="one-time-code"
          :placeholder="t('login.totpPlaceholder')"
          required
          autofocus
        >
        <small>{{ t("login.totpHelp") }}</small>
        <p
          v-if="errorMessage"
          class="login-error"
          role="alert"
        >
          {{ errorMessage }}
        </p>
        <button
          class="primary-button"
          type="submit"
          :disabled="submitting || totpCode.length === 0"
        >
          {{ submitting ? t("login.submitting") : t("login.totpSubmit") }}
        </button>
        <button
          class="action-button"
          type="button"
          @click="backToPassword"
        >
          {{ t("login.totpBack") }}
        </button>
      </form>

      <ForgotPasswordHelp v-if="step === 'password'" />
    </section>
  </main>
</template>

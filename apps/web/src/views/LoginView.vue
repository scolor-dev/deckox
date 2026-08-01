<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { api } from "../api/client";
import { apiErrorKey } from "../api/errors";

defineProps<{
  message?: string | null;
}>();

const emit = defineEmits<{
  authenticated: [];
}>();
const { t } = useI18n();

const password = ref("");
const submitting = ref(false);
const errorMessage = ref("");

async function submit() {
  submitting.value = true;
  errorMessage.value = "";

  try {
    await api.login(password.value);
    password.value = "";
    emit("authenticated");
  } catch (error) {
    errorMessage.value = t(apiErrorKey(error, "errors.login"));
  } finally {
    submitting.value = false;
  }
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
      <p>{{ t("login.description") }}</p>
      <p
        v-if="message"
        class="notice success login-notice"
        role="status"
      >
        {{ message }}
      </p>

      <form @submit.prevent="submit">
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
    </section>
  </main>
</template>

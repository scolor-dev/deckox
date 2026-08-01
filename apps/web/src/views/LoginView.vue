<script setup lang="ts">
import { ref } from "vue";
import { ApiError, api } from "../api/client";

defineProps<{
  message?: string | null;
}>();

const emit = defineEmits<{
  authenticated: [];
}>();

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
    errorMessage.value = error instanceof ApiError
      ? error.message
      : "ログインできませんでした。しばらくしてから再度お試しください。";
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
          <small>サーバー管理</small>
        </div>
      </div>

      <h1 id="login-title">
        管理画面にログイン
      </h1>
      <p>管理者パスワードを入力してください。</p>
      <p
        v-if="message"
        class="notice success login-notice"
        role="status"
      >
        {{ message }}
      </p>

      <form @submit.prevent="submit">
        <label for="password">パスワード</label>
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
          {{ submitting ? "確認中…" : "ログイン" }}
        </button>
      </form>
    </section>
  </main>
</template>

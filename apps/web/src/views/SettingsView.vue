<script setup lang="ts">
import { ref } from "vue";
import { ApiError, api } from "../api/client";

const emit = defineEmits<{ passwordChanged: [] }>();

const currentPassword = ref("");
const newPassword = ref("");
const passwordConfirmation = ref("");
const submitting = ref(false);
const error = ref<string | null>(null);

async function changePassword() {
  error.value = null;
  if (newPassword.value.length < 12) {
    error.value = "新しいパスワードは12文字以上で入力してください。";
    return;
  }
  if (newPassword.value !== passwordConfirmation.value) {
    error.value = "新しいパスワードが確認欄と一致しません。";
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
    if (caught instanceof ApiError && caught.code === "invalid_current_password") {
      error.value = "現在のパスワードが正しくありません。";
    } else {
      error.value = caught instanceof Error ? caught.message : "パスワードを変更できませんでした。";
    }
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <section class="view settings-view">
    <header class="view-header">
      <div>
        <h1>設定</h1>
        <p class="subtitle">
          Deckoxとサーバーの管理設定
        </p>
      </div>
    </header>

    <section
      class="settings-section"
      aria-labelledby="password-heading"
    >
      <div class="settings-description">
        <h2 id="password-heading">
          管理者パスワード
        </h2>
        <p>管理画面へログインするときのパスワードを変更します。</p>
      </div>
      <form
        class="settings-form"
        @submit.prevent="changePassword"
      >
        <label for="current-password">現在のパスワード</label>
        <input
          id="current-password"
          v-model="currentPassword"
          type="password"
          autocomplete="current-password"
          required
        >

        <label for="new-password">新しいパスワード</label>
        <input
          id="new-password"
          v-model="newPassword"
          type="password"
          autocomplete="new-password"
          minlength="12"
          required
        >
        <small>12文字以上で入力してください。</small>

        <label for="password-confirmation">新しいパスワード（確認）</label>
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
          {{ submitting ? "変更しています…" : "パスワードを変更" }}
        </button>
        <p class="settings-help">
          変更後は、すべての端末で再ログインが必要です。
        </p>
      </form>
    </section>
  </section>
</template>

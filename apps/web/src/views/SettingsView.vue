<script setup lang="ts">
import { onMounted, ref } from "vue";
import { ApiError, api, type SshKeyList } from "../api/client";

const emit = defineEmits<{ passwordChanged: [] }>();

const currentPassword = ref("");
const newPassword = ref("");
const passwordConfirmation = ref("");
const submitting = ref(false);
const error = ref<string | null>(null);
const sshKeys = ref<SshKeyList | null>(null);
const sshLoading = ref(true);
const sshError = ref<string | null>(null);
const sshSuccess = ref<string | null>(null);
const publicKey = ref("");
const addingKey = ref(false);
const removingKeyId = ref<string | null>(null);

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
    } else if (caught instanceof ApiError && caught.code === "rate_limited") {
      error.value = "確認に何度も失敗したため、5分ほど待ってから再度お試しください。";
    } else {
      error.value = caught instanceof Error ? caught.message : "パスワードを変更できませんでした。";
    }
  } finally {
    submitting.value = false;
  }
}

async function loadSshKeys() {
  sshLoading.value = true;
  sshError.value = null;
  sshSuccess.value = null;
  try {
    sshKeys.value = await api.sshKeys();
  } catch (caught) {
    sshError.value = caught instanceof Error ? caught.message : "SSH公開鍵を取得できませんでした。";
  } finally {
    sshLoading.value = false;
  }
}

async function addSshKey() {
  sshError.value = null;
  if (!publicKey.value.trim()) {
    sshError.value = "追加するSSH公開鍵を入力してください。";
    return;
  }
  addingKey.value = true;
  try {
    const added = await api.addSshKey(publicKey.value.trim());
    publicKey.value = "";
    await loadSshKeys();
    sshSuccess.value = `${added.comment ?? added.fingerprint} を追加しました。`;
  } catch (caught) {
    sshError.value = caught instanceof Error ? caught.message : "SSH公開鍵を追加できませんでした。";
  } finally {
    addingKey.value = false;
  }
}

async function removeSshKey(keyId: string, label: string) {
  if (!window.confirm(`${label} を削除しますか？`)) return;
  removingKeyId.value = keyId;
  sshError.value = null;
  sshSuccess.value = null;
  try {
    const removed = await api.removeSshKey(keyId);
    await loadSshKeys();
    sshSuccess.value = `${removed.comment ?? removed.fingerprint} を削除しました。`;
  } catch (caught) {
    if (caught instanceof ApiError && caught.status === 409) {
      sshError.value = "最後のSSH公開鍵は削除できません。先に別の鍵を追加してください。";
    } else {
      sshError.value = caught instanceof Error ? caught.message : "SSH公開鍵を削除できませんでした。";
    }
  } finally {
    removingKeyId.value = null;
  }
}

onMounted(() => {
  void loadSshKeys();
});
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

    <section
      class="settings-section"
      aria-labelledby="ssh-heading"
    >
      <div class="settings-description">
        <h2 id="ssh-heading">
          SSH公開鍵
        </h2>
        <p>指定したLinuxユーザーへのSSH接続を許可する公開鍵を管理します。</p>
      </div>
      <div class="settings-form ssh-settings">
        <p
          v-if="sshError"
          class="notice error"
          role="alert"
        >
          {{ sshError }}
        </p>
        <p
          v-if="sshSuccess"
          class="notice success"
          role="status"
        >
          {{ sshSuccess }}
        </p>
        <p
          v-if="sshLoading"
          class="settings-help"
        >
          SSH公開鍵を確認しています…
        </p>
        <div
          v-else-if="!sshKeys?.enabled"
          class="notice warning"
        >
          SSH公開鍵管理は無効です。Agent設定の <code>ssh.managed_user</code> に、管理する非rootユーザーを指定してください。
        </div>
        <template v-else>
          <p class="managed-user">
            対象ユーザー <strong class="mono">{{ sshKeys.managed_user }}</strong>
          </p>

          <div
            v-if="sshKeys.keys.length"
            class="ssh-key-list"
          >
            <article
              v-for="key in sshKeys.keys"
              :key="key.id"
              class="ssh-key-item"
            >
              <div>
                <strong>{{ key.comment ?? "コメントなし" }}</strong>
                <span class="mono">{{ key.key_type }}</span>
                <code>{{ key.fingerprint }}</code>
              </div>
              <button
                class="action-button danger"
                type="button"
                :disabled="removingKeyId !== null"
                @click="removeSshKey(key.id, key.comment ?? key.fingerprint)"
              >
                {{ removingKeyId === key.id ? "削除中…" : "削除" }}
              </button>
            </article>
          </div>
          <p
            v-else
            class="settings-help"
          >
            Deckoxが管理しているSSH公開鍵はありません。
          </p>

          <form
            class="ssh-add-form"
            @submit.prevent="addSshKey"
          >
            <label for="public-key">公開鍵を追加</label>
            <textarea
              id="public-key"
              v-model="publicKey"
              rows="4"
              placeholder="ssh-ed25519 AAAA... device-name"
              spellcheck="false"
              required
            />
            <small>秘密鍵は入力しないでください。OpenSSH形式の公開鍵1本だけを受け付けます。</small>
            <button
              class="primary-button settings-submit"
              type="submit"
              :disabled="addingKey"
            >
              {{ addingKey ? "追加しています…" : "公開鍵を追加" }}
            </button>
          </form>
        </template>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { writeClipboardText } from "../api/client";
import { notify } from "../notifications";

const { t } = useI18n();

const command =
  "printf '%s' '新しいパスワード' | sudo -u deckox deckox-server reset-password\nsudo systemctl restart deckox-server";

async function copyCommand() {
  if (await writeClipboardText(command)) {
    notify("success", t("common.forgotPasswordCopied"));
  } else {
    notify("error", t("common.forgotPasswordCopyFailed"));
  }
}
</script>

<template>
  <details class="forgot-password">
    <summary>{{ t("common.forgotPassword") }}</summary>
    <p>{{ t("common.forgotPasswordIntro") }}</p>
    <pre><code>{{ command }}</code></pre>
    <button
      class="action-button"
      type="button"
      @click="copyCommand"
    >
      {{ t("common.forgotPasswordCopy") }}
    </button>
  </details>
</template>

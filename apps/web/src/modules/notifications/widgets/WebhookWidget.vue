<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { api } from "../../../api/client";
import { serverStatus as statusSource } from "../../../data/sources";
import { apiErrorKey } from "../../../api/errors";
import {
  AppButton,
  AppStack,
  InfoNote,
  NoticeBanner,
} from "../../../design-system/components";
import { notify } from "../../../notifications";

defineOptions({ inheritAttrs: false });

const { t } = useI18n();
const { data: serverStatus } = statusSource.use();

const webhookTesting = ref(false);

const webhookErrorKey = ref<string | null>(null);

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
</script>

<template>
  <AppStack
    gap="3"
    align="start"
  >
    <InfoNote>{{ t("settings.webhookDescription") }}</InfoNote>
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
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { api } from "../../../api/client";
import { systemCapabilities as capabilitiesSource } from "../../../data/sources";
import { apiErrorKey } from "../../../api/errors";
import PasswordConfirmDialog from "../../../components/PasswordConfirmDialog.vue";
import {
  AppButton,
  AppStack,
  InfoNote,
  NoticeBanner,
} from "../../../design-system/components";

defineOptions({ inheritAttrs: false });

const { t } = useI18n();
const router = useRouter();
const { data: systemCapabilities } = capabilitiesSource.use();

const rebootDialogOpen = ref(false);

const rebooting = ref(false);

const rebootError = ref<string | null>(null);

function openRebootDialog() {
  rebootError.value = null;
  rebootDialogOpen.value = true;
}

function closeRebootDialog() {
  rebootDialogOpen.value = false;
  rebootError.value = null;
}

async function rebootSystem(password: string) {
  rebooting.value = true;
  rebootError.value = null;
  try {
    const health = await api.health().catch(() => null);
    await api.rebootSystem(password);
    if (health) sessionStorage.setItem("deckox:restart-instance", health.instance_id);
    rebootDialogOpen.value = false;
    await router.push({ name: "restarting" });
  } catch (caught) {
    rebootError.value = t(apiErrorKey(caught, "errors.reboot"));
  } finally {
    rebooting.value = false;
  }
}
</script>

<template>
  <AppStack
    gap="3"
    align="start"
  >
    <InfoNote>{{ t("settings.systemOperationsDescription") }}</InfoNote>
    <NoticeBanner
      v-if="systemCapabilities && !systemCapabilities.reboot_allowed"
      tone="warning"
    >
      {{ t("settings.rebootDisabled") }}
    </NoticeBanner>
    <template v-else>
      <InfoNote>{{ t("settings.rebootHelp") }}</InfoNote>
      <AppButton
        variant="primary"
        danger
        :disabled="!systemCapabilities?.reboot_allowed"
        @click="openRebootDialog"
      >
        {{ t("settings.reboot") }}
      </AppButton>
    </template>
    <PasswordConfirmDialog
      :open="rebootDialogOpen"
      :title="t('settings.reboot')"
      :description="t('settings.confirmReboot')"
      :confirm-label="t('settings.reboot')"
      :pending-label="t('settings.rebooting')"
      :submitting="rebooting"
      :error-message="rebootError"
      @confirm="rebootSystem"
      @close="closeRebootDialog"
    />
  </AppStack>
</template>

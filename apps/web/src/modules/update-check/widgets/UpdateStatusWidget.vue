<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import {
  api,
  buildUpdateCommand,
  safeReleaseUrl,
  writeClipboardText,
  type UpdateStatus,
} from "../../../api/client";
import { systemCapabilities as capabilitiesSource } from "../../../data/sources";
import { apiErrorKey } from "../../../api/errors";
import PasswordConfirmDialog from "../../../components/PasswordConfirmDialog.vue";
import {
  AppButton,
  AppStack,
  DetailList,
  DetailRow,
  InfoNote,
  NoticeBanner,
} from "../../../design-system/components";
import { backendEnabled } from "../../../modules/store";
import { notify } from "../../../notifications";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const router = useRouter();
const { data: systemCapabilities } = capabilitiesSource.use();
const updateNowOn = computed(() => backendEnabled(["update"]));

const updateStatus = ref<UpdateStatus | null>(null);

const updateChecking = ref(false);

const updateErrorKey = ref<string | null>(null);

const updateDialogOpen = ref(false);

const updating = ref(false);

const updateDialogError = ref<string | null>(null);

const updateCommand = computed(() => buildUpdateCommand(updateStatus.value?.latest_version));

const releaseUrl = computed(() => safeReleaseUrl(updateStatus.value?.release_url));

const updateCheckedAt = computed(() => {
  const checkedAt = updateStatus.value?.checked_at_ms;
  if (checkedAt == null) return null;
  return new Intl.DateTimeFormat(locale.value, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(checkedAt);
});

function openUpdateDialog() {
  updateDialogError.value = null;
  updateDialogOpen.value = true;
}

function closeUpdateDialog() {
  updateDialogOpen.value = false;
  updateDialogError.value = null;
}

async function triggerUpdate(password: string) {
  updating.value = true;
  updateDialogError.value = null;
  try {
    const health = await api.health().catch(() => null);
    await api.triggerUpdate(password);
    if (health) sessionStorage.setItem("deckox:restart-instance", health.instance_id);
    updateDialogOpen.value = false;
    await router.push({ name: "restarting" });
  } catch (caught) {
    updateDialogError.value = t(apiErrorKey(caught, "errors.updateTrigger"));
  } finally {
    updating.value = false;
  }
}

async function checkForUpdate() {
  updateChecking.value = true;
  updateErrorKey.value = null;
  try {
    updateStatus.value = await api.updateStatus();
  } catch (caught) {
    updateErrorKey.value = apiErrorKey(caught, "errors.updateCheck");
  } finally {
    updateChecking.value = false;
  }
}

async function copyUpdateCommand() {
  if (!updateCommand.value) {
    updateErrorKey.value = "settings.updateCommandUnavailable";
    return;
  }
  updateErrorKey.value = null;
  if (await writeClipboardText(updateCommand.value)) {
    notify("success", t("settings.updateCommandCopied"));
  } else {
    updateErrorKey.value = "settings.updateCopyFailed";
    notify("error", t("settings.updateCopyFailed"));
  }
}
</script>

<template>
  <AppStack
    class="update-settings"
    gap="3"
    align="start"
  >
    <InfoNote>{{ t("settings.updateDescription") }}</InfoNote>
    <NoticeBanner
      v-if="updateErrorKey"
      tone="error"
    >
      {{ t(updateErrorKey) }}
    </NoticeBanner>
    <template v-if="updateStatus">
      <DetailList>
        <DetailRow :term="t('settings.currentVersion')">
          {{ updateStatus.current_version }}
        </DetailRow>
        <DetailRow :term="t('settings.latestVersion')">
          {{ updateStatus.latest_version ?? t("common.none") }}
        </DetailRow>
      </DetailList>
      <NoticeBanner
        v-if="updateStatus.status === 'available' && updateStatus.update_available"
        tone="success"
      >
        {{ t("settings.updateAvailable") }}
      </NoticeBanner>
      <NoticeBanner
        v-else-if="updateStatus.status === 'up_to_date'"
        tone="success"
      >
        {{ t("settings.upToDate") }}
      </NoticeBanner>
      <NoticeBanner
        v-else
        tone="warning"
      >
        {{ t("settings.updateUnavailable") }}
      </NoticeBanner>
      <InfoNote v-if="updateCheckedAt">
        {{ t("settings.updateCheckedAt", { time: updateCheckedAt }) }}
      </InfoNote>
      <a
        v-if="releaseUrl"
        class="release-link"
        :href="releaseUrl"
        target="_blank"
        rel="noopener noreferrer"
      >{{ t("settings.openRelease") }}</a>
      <template v-if="updateStatus.update_available && updateNowOn && systemCapabilities?.update_allowed">
        <InfoNote>{{ t("settings.updateHelp") }}</InfoNote>
        <AppButton
          variant="primary"
          danger
          @click="openUpdateDialog"
        >
          {{ t("settings.updateNow") }}
        </AppButton>
      </template>
      <AppStack
        v-else-if="updateStatus.update_available && updateCommand"
        gap="2"
        align="start"
      >
        <code>{{ updateCommand }}</code>
        <AppButton
          variant="action"
          @click="copyUpdateCommand"
        >
          {{ t("settings.copyCommand") }}
        </AppButton>
        <InfoNote>{{ t("settings.manualUpdateOnly") }}</InfoNote>
      </AppStack>
    </template>
    <InfoNote v-else-if="!updateChecking">
      {{ t("settings.updateNotChecked") }}
    </InfoNote>
    <AppButton
      :disabled="updateChecking"
      @click="checkForUpdate"
    >
      {{ updateChecking ? t("settings.checkingUpdate") : t("settings.checkUpdate") }}
    </AppButton>
    <PasswordConfirmDialog
      :open="updateDialogOpen"
      :title="t('settings.updateNow')"
      :description="t('settings.confirmUpdate')"
      :confirm-label="t('settings.updateNow')"
      :pending-label="t('settings.updating')"
      :submitting="updating"
      :error-message="updateDialogError"
      @confirm="triggerUpdate"
      @close="closeUpdateDialog"
    />
  </AppStack>
</template>

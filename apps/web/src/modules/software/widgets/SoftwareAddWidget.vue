<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { api } from "../../../api/client";
import { apiErrorKey } from "../../../api/errors";
import { softwarePackages } from "../../../data/sources";
import {
  AppButton,
  AppCard,
  AppHeading,
  AppStack,
  InfoNote,
  NoticeBanner,
  TextField,
} from "../../../design-system/components";
import { notify } from "../../../notifications";

defineOptions({ inheritAttrs: false });

const { t } = useI18n();

const newPackageName = ref("");

const adding = ref(false);

const addError = ref<string | null>(null);

async function addPackage() {
  const name = newPackageName.value.trim();
  if (!name) return;

  adding.value = true;
  addError.value = null;
  try {
    await api.softwareAllowlist(name, "allow");
    newPackageName.value = "";
    notify("success", t("software.completed", { name }));
    await softwarePackages.refresh();
  } catch (cause) {
    addError.value = t(apiErrorKey(cause, "errors.softwareAction"));
  } finally {
    adding.value = false;
  }
}
</script>

<template>
  <AppCard>
    <AppStack gap="3">
      <AppHeading>
        {{ t("software.addTitle") }}
      </AppHeading>
      <NoticeBanner
        v-if="addError"
        tone="error"
      >
        {{ addError }}
      </NoticeBanner>
      <AppStack
        as="form"
        direction="row"
        gap="2"
        align="end"
        wrap
        @submit.prevent="addPackage"
      >
        <TextField
          id="software-package-name"
          v-model="newPackageName"
          :label="t('software.addLabel')"
          :placeholder="t('software.addPlaceholder')"
          required
        />
        <AppButton
          type="submit"
          :disabled="adding"
        >
          {{ adding ? t("software.adding") : t("software.add") }}
        </AppButton>
      </AppStack>
      <InfoNote>{{ t("software.addHelp") }}</InfoNote>
    </AppStack>
  </AppCard>
</template>

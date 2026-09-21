<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { api, type InstalledSoftware } from "../../../api/client";
import { apiErrorKey } from "../../../api/errors";
import { installedSoftware, softwarePackages } from "../../../data/sources";
import {
  AppButton,
  AppStack,
  InfoNote,
  NoticeBanner,
  TablePanel,
  TextField,
  StateBadge,
  TableToolbar,
} from "../../../design-system/components";
import { notify } from "../../../notifications";

defineOptions({ inheritAttrs: false });

const { t } = useI18n();

const installed = installedSoftware.use();
const managedSource = softwarePackages.use();
const installedLoading = installed.loading;
const installedError = ref<string | null>(null);
const installedPackages = computed(() => {
  const managed = managedSource.data.value ? new Set(managedSource.data.value.map((pkg) => pkg.name)) : null;
  return (installed.data.value ?? []).map((pkg) => (managed ? { ...pkg, managed: managed.has(pkg.name) } : pkg));
});
watch(installed.error, (cause) => {
  installedError.value = cause ? t(apiErrorKey(cause, "errors.software")) : null;
});

const INSTALLED_LIMIT = 100;

const installedQuery = ref("");

const managing = ref<string | null>(null);

const installedMatches = computed(() => {
  const query = installedQuery.value.trim().toLowerCase();
  return query === ""
    ? installedPackages.value
    : installedPackages.value.filter((pkg) => pkg.name.toLowerCase().includes(query));
});

const installedShown = computed(() => installedMatches.value.slice(0, INSTALLED_LIMIT));

async function manageInstalled(pkg: InstalledSoftware) {
  managing.value = pkg.name;
  installedError.value = null;
  try {
    await api.softwareAllowlist(pkg.name, "allow");
    pkg.managed = true;
    notify("success", t("software.completed", { name: pkg.name }));
    await softwarePackages.refresh();
  } catch (cause) {
    installedError.value = t(apiErrorKey(cause, "errors.softwareAction"));
  } finally {
    managing.value = null;
  }
}
</script>

<template>
  <AppStack gap="3">
    <h2>{{ t("software.installedTitle") }}</h2>
    <InfoNote>{{ t("software.installedHelp") }}</InfoNote>
    <NoticeBanner
      v-if="installedError"
      tone="error"
    >
      {{ installedError }}
    </NoticeBanner>
    <TablePanel
      :loading="installedLoading"
      :empty="installedShown.length === 0"
      :empty-message="t('software.installedEmpty')"
    >
      <template #toolbar>
        <TableToolbar :count="t('software.installedCount', { shown: installedShown.length, total: installedMatches.length })">
          <template #search>
            <TextField
              id="software-installed-search"
              v-model="installedQuery"
              :label="t('software.installedSearch')"
              type="search"
              :placeholder="t('software.installedSearchPlaceholder')"
              label-hidden
            />
          </template>
        </TableToolbar>
      </template>
      <template #loading>
        {{ t("software.installedLoading") }}
      </template>
      <table>
        <thead>
          <tr>
            <th>{{ t("software.name") }}</th>
            <th>{{ t("software.installedVersion") }}</th>
            <th>{{ t("software.actions") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="pkg in installedShown"
            :key="pkg.name"
          >
            <td><strong class="service-name">{{ pkg.name }}</strong></td>
            <td>{{ pkg.version }}</td>
            <td>
              <StateBadge
                v-if="pkg.managed"
                state="active"
              >
                {{ t("software.managed") }}
              </StateBadge>
              <AppButton
                v-else
                variant="action"
                :disabled="managing !== null"
                @click="manageInstalled(pkg)"
              >
                {{ t("software.manage") }}
              </AppButton>
            </td>
          </tr>
        </tbody>
      </table>
      <template
        v-if="installedMatches.length > INSTALLED_LIMIT"
        #footer
      >
        <InfoNote>{{ t("software.installedLimited", { limit: INSTALLED_LIMIT }) }}</InfoNote>
      </template>
    </TablePanel>
  </AppStack>
</template>

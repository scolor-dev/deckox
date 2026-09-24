<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, formatBytes, type BackupSummary } from "../../../api/client";
import { apiErrorKey } from "../../../api/errors";
import { useStaleRefresh } from "../../../composables/useStaleRefresh";
import {
  AppHeading,
  AppStack,
  InfoNote,
  TablePanel,
  TableToolbar,
} from "../../../design-system/components";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const backups = ref<BackupSummary[]>([]);
const backupsError = ref<string | null>(null);

function formatBackupDate(createdAtMs: number) {
  return new Intl.DateTimeFormat(locale.value, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(createdAtMs);
}

async function loadBackups() {
  try {
    backups.value = await api.backups();
    backupsError.value = null;
  } catch (cause) {
    backupsError.value = t(apiErrorKey(cause, "errors.backups"));
  }
}

useStaleRefresh(loadBackups, 60_000);
</script>

<template>
  <AppStack gap="3">
    <AppStack
      v-if="backupsError"
      gap="3"
    >
      <AppHeading>
        {{ t("diagnostics.backups") }}
      </AppHeading>
      <InfoNote>{{ backupsError }}</InfoNote>
    </AppStack>
    <TablePanel
      v-else
      :empty="backups.length === 0"
      :empty-message="t('diagnostics.noBackups')"
    >
      <template #toolbar>
        <TableToolbar>
          <template #filters>
            <AppHeading>
              {{ t("diagnostics.backups") }}
            </AppHeading>
          </template>
        </TableToolbar>
      </template>
      <table>
        <thead><tr><th>{{ t("diagnostics.backupCreatedAt") }}</th><th>{{ t("diagnostics.backupPreviousVersion") }}</th><th>{{ t("diagnostics.backupSize") }}</th></tr></thead>
        <tbody>
          <tr
            v-for="backup in backups"
            :key="backup.name"
          >
            <td>{{ backup.created_at_ms === null ? t("common.none") : formatBackupDate(backup.created_at_ms) }}</td>
            <td>{{ backup.previous_version ?? t("common.none") }}</td>
            <td>{{ formatBytes(backup.size_bytes, locale) }}</td>
          </tr>
        </tbody>
      </table>
    </TablePanel>
  </AppStack>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStaleRefresh } from "../../../composables/useStaleRefresh";
import {
  api,
  DIAGNOSTICS_REPORT_FILENAME,
  formatUptime,
  type DeckoxServiceDiagnostic,
  type DiagnosticsResponse,
} from "../../../api/client";
import { apiErrorKey } from "../../../api/errors";
import {
  AppButton,
  AppHeading,
  AppStack,
  AppText,
  DetailList,
  DetailRow,
  InfoNote,
  NoticeBanner,
  SectionHeader,
  StateBadge,
  TablePanel,
  TableToolbar,
} from "../../../design-system/components";
import { notify } from "../../../notifications";

defineOptions({ inheritAttrs: false });

const { t, locale } = useI18n();
const diagnostics = ref<DiagnosticsResponse | null>(null);
const loading = ref(true);
const downloading = ref(false);
const error = ref<string | null>(null);

const generatedAt = computed(() => {
  if (!diagnostics.value) return t("common.none");
  return new Intl.DateTimeFormat(locale.value, {
    dateStyle: "medium",
    timeStyle: "medium",
  }).format(diagnostics.value.generated_at_ms);
});
const deckoxServices = computed<DeckoxServiceDiagnostic[]>(() => {
  const services = diagnostics.value?.deckox_services;
  if (!services) return [];
  return [
    { id: "deckox-agent.service", state: services.agent },
    { id: "deckox-server.service", state: services.server },
  ];
});

function stateClass(state: string) {
  return state === "active" || state === "running" || state === "ok" ? "active" : "inactive";
}

function enabledLabel(enabled: boolean) {
  return t(enabled ? "diagnostics.enabled" : "diagnostics.disabled");
}

async function fetchData() {
  loading.value = true;
  error.value = null;
  try {
    diagnostics.value = await api.diagnostics();
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.diagnostics"));
  } finally {
    loading.value = false;
  }
}

async function downloadReport() {
  downloading.value = true;
  error.value = null;
  try {
    const blob = await api.diagnosticsReport();
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = DIAGNOSTICS_REPORT_FILENAME;
    anchor.hidden = true;
    document.body.append(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
    notify("success", t("diagnostics.reportSaved"));
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.diagnosticsReport"));
    notify("error", error.value);
  } finally {
    downloading.value = false;
  }
}

const refresh = useStaleRefresh(fetchData, 60_000);
</script>

<template>
  <AppStack gap="4">
    <SectionHeader :title="t('diagnostics.title')">
      <template #actions>
        <AppStack
          direction="row"
          gap="2"
          wrap
        >
          <AppButton
            :disabled="loading"
            @click="refresh"
          >
            {{ loading ? t("common.loading") : t("common.refresh") }}
          </AppButton>
          <AppButton
            :disabled="downloading"
            @click="downloadReport"
          >
            {{ downloading ? t("diagnostics.downloading") : t("diagnostics.download") }}
          </AppButton>
        </AppStack>
      </template>
    </SectionHeader>

    <NoticeBanner
      v-if="error"
      tone="error"
    >
      {{ error }}
    </NoticeBanner>
    <NoticeBanner
      v-if="diagnostics && !diagnostics.agent.connected"
      tone="warning"
    >
      {{ t("diagnostics.partial") }}
    </NoticeBanner>

    <InfoNote v-if="loading && !diagnostics">
      {{ t("common.loading") }}
    </InfoNote>

    <AppStack
      v-if="diagnostics"
      gap="4"
    >
      <InfoNote>{{ t("diagnostics.generatedAt", { time: generatedAt }) }}</InfoNote>
      <AppStack gap="4">
        <AppStack gap="3">
          <AppHeading>
            {{ t("diagnostics.server") }}
          </AppHeading>
          <DetailList>
            <DetailRow :term="t('diagnostics.status')">
              <StateBadge :state="stateClass(diagnostics.server.status)">
                {{ diagnostics.server.status }}
              </StateBadge>
            </DetailRow>
            <DetailRow :term="t('diagnostics.version')">
              {{ diagnostics.server.version }}
            </DetailRow>
          </DetailList>
        </AppStack>
        <AppStack gap="3">
          <AppHeading>
            {{ t("diagnostics.agent") }}
          </AppHeading>
          <DetailList>
            <DetailRow :term="t('diagnostics.status')">
              <StateBadge :state="diagnostics.agent.connected ? 'active' : 'inactive'">
                {{ diagnostics.agent.connected ? t("diagnostics.connected") : t("diagnostics.disconnected") }}
              </StateBadge>
            </DetailRow>
            <DetailRow :term="t('diagnostics.version')">
              {{ diagnostics.agent.version ?? t("common.none") }}
            </DetailRow>
          </DetailList>
        </AppStack>
      </AppStack>

      <AppStack gap="3">
        <AppHeading>
          {{ t("diagnostics.host") }}
        </AppHeading>
        <DetailList v-if="diagnostics.host">
          <DetailRow :term="t('diagnostics.hostname')">
            {{ diagnostics.host.hostname }}
          </DetailRow>
          <DetailRow :term="t('diagnostics.os')">
            {{ diagnostics.host.operating_system }} {{ diagnostics.host.os_version ?? "" }}
          </DetailRow>
          <DetailRow :term="t('diagnostics.kernel')">
            {{ diagnostics.host.kernel_version }}
          </DetailRow>
          <DetailRow :term="t('diagnostics.architecture')">
            {{ diagnostics.host.architecture }}
          </DetailRow>
          <DetailRow :term="t('diagnostics.uptime')">
            {{ formatUptime(diagnostics.host.uptime_seconds, locale) }}
          </DetailRow>
          <DetailRow :term="t('diagnostics.timezone')">
            {{ diagnostics.host.timezone ?? t("common.none") }}
          </DetailRow>
          <DetailRow :term="t('diagnostics.upgradablePackages')">
            {{ diagnostics.host.upgradable_packages ?? t("diagnostics.upgradablePackagesUnavailable") }}
          </DetailRow>
        </DetailList>
        <InfoNote v-else>
          {{ t("diagnostics.noHost") }}
        </InfoNote>
      </AppStack>

      <TablePanel
        :empty="deckoxServices.length === 0"
        :empty-message="t('diagnostics.noServices')"
      >
        <template #toolbar>
          <TableToolbar>
            <template #filters>
              <AppHeading>
                {{ t("diagnostics.services") }}
              </AppHeading>
            </template>
          </TableToolbar>
        </template>
        <table>
          <thead><tr><th>{{ t("diagnostics.service") }}</th><th>{{ t("diagnostics.activeState") }}</th><th>{{ t("diagnostics.startup") }}</th></tr></thead>
          <tbody>
            <tr
              v-for="service in deckoxServices"
              :key="service.id"
            >
              <td>
                <AppText
                  as="strong"
                  mono
                  strong
                  size="xs"
                >
                  {{ service.id }}
                </AppText>
              </td>
              <td>
                <StateBadge :state="stateClass(service.state.active_state)">
                  {{ service.state.active_state }} / {{ service.state.sub_state }}
                </StateBadge>
              </td>
              <td>{{ service.state.unit_file_state ?? t("common.none") }}</td>
            </tr>
          </tbody>
        </table>
      </TablePanel>

      <AppStack gap="3">
        <AppHeading>
          {{ t("diagnostics.config") }}
        </AppHeading>
        <DetailList v-if="diagnostics.runtime_config">
          <DetailRow :term="t('diagnostics.reboot')">
            {{ enabledLabel(diagnostics.runtime_config.reboot_allowed) }}
          </DetailRow>
          <DetailRow :term="t('diagnostics.update')">
            {{ enabledLabel(diagnostics.runtime_config.update_allowed) }}
          </DetailRow>
          <DetailRow :term="t('diagnostics.allowedServices')">
            {{ t("diagnostics.allowedServicesValue", { count: diagnostics.runtime_config.allowed_services_count }) }}
          </DetailRow>
        </DetailList>
        <InfoNote v-else>
          {{ t("diagnostics.noConfig") }}
        </InfoNote>
      </AppStack>
    </AppStack>
  </AppStack>
</template>

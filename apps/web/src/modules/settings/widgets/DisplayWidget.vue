<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { AppCheckbox, AppStack, AppText, SelectField } from "../../../design-system/components";
import { notify } from "../../../notifications";
import {
  preferences,
  type LocalePreference,
  type MetricsInterval,
  type ThemePreference,
} from "../../../preferences";
import { backendEnabled } from "../../store";

defineOptions({ inheritAttrs: false });

const { t } = useI18n();
const liveOn = computed(() => backendEnabled(["realtime", "system"]));

function saved() {
  notify("success", t("settings.saved"));
}

const localeValue = computed({
  get: () => preferences.locale,
  set: (value: string) => {
    preferences.locale = value as LocalePreference;
    saved();
  },
});
const themeValue = computed({
  get: () => preferences.theme,
  set: (value: string) => {
    preferences.theme = value as ThemePreference;
    saved();
  },
});
const realtimeValue = computed({
  get: () => preferences.realtimeEnabled,
  set: (value: boolean) => {
    preferences.realtimeEnabled = value;
    saved();
  },
});
const intervalValue = computed({
  get: () => String(preferences.metricsInterval),
  set: (value: string) => {
    preferences.metricsInterval = Number(value) as MetricsInterval;
    saved();
  },
});

const localeOptions = computed(() => [
  { value: "auto", label: t("settings.languageAuto") },
  { value: "ja", label: t("settings.japanese") },
  { value: "en", label: t("settings.english") },
]);
const themeOptions = computed(() => [
  { value: "auto", label: t("settings.themeAuto") },
  { value: "light", label: t("settings.themeLight") },
  { value: "dark", label: t("settings.themeDark") },
]);
const intervalOptions = computed(() =>
  [1, 2, 5].map((seconds) => ({ value: String(seconds), label: t("settings.intervalValue", { seconds }) })),
);
</script>

<template>
  <AppStack gap="3">
    <AppText
      as="p"
      tone="muted"
      size="sm"
    >
      {{ t("settings.displayDescription") }}
    </AppText>
    <SelectField
      id="language"
      v-model="localeValue"
      :label="t('settings.language')"
      :options="localeOptions"
    />
    <SelectField
      id="theme"
      v-model="themeValue"
      :label="t('settings.theme')"
      :options="themeOptions"
    />
    <AppCheckbox
      v-if="liveOn"
      v-model="realtimeValue"
      :label="t('settings.realtime')"
      :help="t('settings.realtimeHelp')"
    />
    <SelectField
      v-if="liveOn"
      id="metrics-interval"
      v-model="intervalValue"
      :label="t('settings.interval')"
      :options="intervalOptions"
      :disabled="!preferences.realtimeEnabled"
    />
  </AppStack>
</template>

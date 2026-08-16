import { reactive, watch } from "vue";

export type LocalePreference = "auto" | "ja" | "en";
export type MetricsInterval = 1 | 2 | 5;
export type ServiceTagFilterKey = "standard" | "deckox" | "other";

const SERVICE_TAG_FILTER_KEYS: ServiceTagFilterKey[] = ["standard", "deckox", "other"];

export interface Preferences {
  locale: LocalePreference;
  realtimeEnabled: boolean;
  metricsInterval: MetricsInterval;
  hiddenServiceTags: ServiceTagFilterKey[];
}

const STORAGE_KEY = "deckox:preferences";
const DEFAULT_PREFERENCES: Preferences = {
  locale: "auto",
  realtimeEnabled: true,
  metricsInterval: 1,
  hiddenServiceTags: [],
};

export function normalizePreferences(value: unknown): Preferences {
  if (typeof value !== "object" || value === null) return { ...DEFAULT_PREFERENCES };
  const candidate = value as Partial<Preferences>;
  return {
    locale: candidate.locale === "ja" || candidate.locale === "en" || candidate.locale === "auto"
      ? candidate.locale
      : DEFAULT_PREFERENCES.locale,
    realtimeEnabled: typeof candidate.realtimeEnabled === "boolean"
      ? candidate.realtimeEnabled
      : DEFAULT_PREFERENCES.realtimeEnabled,
    metricsInterval: candidate.metricsInterval === 2 || candidate.metricsInterval === 5
      ? candidate.metricsInterval
      : DEFAULT_PREFERENCES.metricsInterval,
    hiddenServiceTags: Array.isArray(candidate.hiddenServiceTags)
      ? candidate.hiddenServiceTags.filter(
          (tag): tag is ServiceTagFilterKey =>
            typeof tag === "string" && SERVICE_TAG_FILTER_KEYS.includes(tag),
        )
      : [...DEFAULT_PREFERENCES.hiddenServiceTags],
  };
}

export function resolveLocale(
  preference: LocalePreference,
  browserLanguage: string,
): "ja" | "en" {
  if (preference !== "auto") return preference;
  return browserLanguage.toLowerCase().startsWith("ja") ? "ja" : "en";
}

function loadPreferences(): Preferences {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    return stored ? normalizePreferences(JSON.parse(stored) as unknown) : { ...DEFAULT_PREFERENCES };
  } catch {
    return { ...DEFAULT_PREFERENCES };
  }
}

export const preferences = reactive<Preferences>(loadPreferences());

watch(preferences, (value) => {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch {
    // The selected settings remain active for this tab when storage is unavailable.
  }
}, { deep: true });

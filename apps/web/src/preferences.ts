import { reactive, watch } from "vue";

export type LocalePreference = "auto" | "ja" | "en";
export type ThemePreference = "auto" | "light" | "dark";
export type MetricsInterval = 1 | 2 | 5;
// "standard" and "deckox"/"other" are fixed categories; any other string is
// a recognized product name (e.g. "Docker"), which is open-ended, so this
// stays a plain string rather than a literal union.
export type ServiceTagFilterKey = string;
export type StorageTagFilterKey = string;
// A package is always exactly one of these two, unlike the open-ended
// service/storage tag sets above.
export type SoftwareTagFilterKey = "installed" | "not_installed";

export interface Preferences {
  locale: LocalePreference;
  theme: ThemePreference;
  realtimeEnabled: boolean;
  metricsInterval: MetricsInterval;
  hiddenServiceTags: ServiceTagFilterKey[];
  hiddenStorageTags: StorageTagFilterKey[];
  hiddenSoftwareTags: SoftwareTagFilterKey[];
}

const STORAGE_KEY = "deckox:preferences";
const DEFAULT_PREFERENCES: Preferences = {
  locale: "auto",
  theme: "auto",
  realtimeEnabled: true,
  metricsInterval: 1,
  hiddenServiceTags: [],
  hiddenStorageTags: [],
  hiddenSoftwareTags: [],
};

export function normalizePreferences(value: unknown): Preferences {
  if (typeof value !== "object" || value === null) return { ...DEFAULT_PREFERENCES };
  const candidate = value as Partial<Preferences>;
  return {
    locale: candidate.locale === "ja" || candidate.locale === "en" || candidate.locale === "auto"
      ? candidate.locale
      : DEFAULT_PREFERENCES.locale,
    theme: candidate.theme === "light" || candidate.theme === "dark" || candidate.theme === "auto"
      ? candidate.theme
      : DEFAULT_PREFERENCES.theme,
    realtimeEnabled: typeof candidate.realtimeEnabled === "boolean"
      ? candidate.realtimeEnabled
      : DEFAULT_PREFERENCES.realtimeEnabled,
    metricsInterval: candidate.metricsInterval === 2 || candidate.metricsInterval === 5
      ? candidate.metricsInterval
      : DEFAULT_PREFERENCES.metricsInterval,
    hiddenServiceTags: Array.isArray(candidate.hiddenServiceTags)
      ? candidate.hiddenServiceTags.filter(isTagString)
      : [...DEFAULT_PREFERENCES.hiddenServiceTags],
    hiddenStorageTags: Array.isArray(candidate.hiddenStorageTags)
      ? candidate.hiddenStorageTags.filter(isTagString)
      : [...DEFAULT_PREFERENCES.hiddenStorageTags],
    hiddenSoftwareTags: Array.isArray(candidate.hiddenSoftwareTags)
      ? candidate.hiddenSoftwareTags.filter(isSoftwareTag)
      : [...DEFAULT_PREFERENCES.hiddenSoftwareTags],
  };
}

function isTagString(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= 64;
}

function isSoftwareTag(value: unknown): value is SoftwareTagFilterKey {
  return value === "installed" || value === "not_installed";
}

export function resolveLocale(
  preference: LocalePreference,
  browserLanguage: string,
): "ja" | "en" {
  if (preference !== "auto") return preference;
  return browserLanguage.toLowerCase().startsWith("ja") ? "ja" : "en";
}

export function resolveTheme(
  preference: ThemePreference,
  prefersDark: boolean,
): "light" | "dark" {
  if (preference !== "auto") return preference;
  return prefersDark ? "dark" : "light";
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

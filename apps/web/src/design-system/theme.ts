import { watch } from "vue";
import { preferences, resolveTheme } from "../preferences";

const DARK_MEDIA_QUERY = "(prefers-color-scheme: dark)";

function applyTheme() {
  if (typeof document === "undefined") return;
  const prefersDark = typeof window !== "undefined" && window.matchMedia(DARK_MEDIA_QUERY).matches;
  document.documentElement.dataset.theme = resolveTheme(preferences.theme, prefersDark);
}

/**
 * Wires `preferences.theme` to `<html data-theme>`, which design-system/
 * tokens.css keys its dark-mode overrides on. Also re-applies on a live OS
 * theme change while the preference is "auto" — `prefers-color-scheme`
 * itself already handles that in CSS, but a stale `data-theme="light"`
 * (or "dark") on `<html>` from a *previous* auto resolution would keep
 * overriding the media query otherwise. Call once from main.ts; the
 * inline script in index.html handles the very first paint before this
 * module ever runs.
 */
export function initTheme() {
  applyTheme();
  watch(() => preferences.theme, applyTheme, { flush: "sync" });
  if (typeof window === "undefined") return;
  window.matchMedia(DARK_MEDIA_QUERY).addEventListener("change", () => {
    if (preferences.theme === "auto") applyTheme();
  });
}

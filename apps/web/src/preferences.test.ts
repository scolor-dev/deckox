import { describe, expect, it } from "vitest";
import { normalizePreferences, resolveLocale, resolveTheme } from "./preferences";

describe("preferences", () => {
  it("normalizes stored values to supported choices", () => {
    expect(normalizePreferences({
      locale: "en",
      theme: "dark",
      realtimeEnabled: false,
      metricsInterval: 5,
      hiddenServiceTags: ["standard", "Docker", 42, ""],
      hiddenStorageTags: ["standard", 42],
      hiddenSoftwareTags: ["installed", "bogus", 42],
    })).toEqual({
      locale: "en",
      theme: "dark",
      realtimeEnabled: false,
      metricsInterval: 5,
      hiddenServiceTags: ["standard", "Docker"],
      hiddenStorageTags: ["standard"],
      hiddenSoftwareTags: ["installed"],
    });
    expect(normalizePreferences({
      locale: "fr",
      theme: "sepia",
      realtimeEnabled: "yes",
      metricsInterval: 3,
    })).toEqual({
      locale: "auto",
      theme: "auto",
      realtimeEnabled: true,
      metricsInterval: 1,
      hiddenServiceTags: [],
      hiddenStorageTags: [],
      hiddenSoftwareTags: [],
    });
  });

  it("uses Japanese only for Japanese device locales in automatic mode", () => {
    expect(resolveLocale("auto", "ja-JP")).toBe("ja");
    expect(resolveLocale("auto", "en-US")).toBe("en");
    expect(resolveLocale("ja", "en-US")).toBe("ja");
  });

  it("follows the OS dark-mode setting only in automatic mode", () => {
    expect(resolveTheme("auto", true)).toBe("dark");
    expect(resolveTheme("auto", false)).toBe("light");
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });
});

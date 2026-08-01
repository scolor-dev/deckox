import { describe, expect, it } from "vitest";
import { normalizePreferences, resolveLocale } from "./preferences";

describe("preferences", () => {
  it("normalizes stored values to supported choices", () => {
    expect(normalizePreferences({
      locale: "en",
      realtimeEnabled: false,
      metricsInterval: 5,
    })).toEqual({ locale: "en", realtimeEnabled: false, metricsInterval: 5 });
    expect(normalizePreferences({
      locale: "fr",
      realtimeEnabled: "yes",
      metricsInterval: 3,
    })).toEqual({ locale: "auto", realtimeEnabled: true, metricsInterval: 1 });
  });

  it("uses Japanese only for Japanese device locales in automatic mode", () => {
    expect(resolveLocale("auto", "ja-JP")).toBe("ja");
    expect(resolveLocale("auto", "en-US")).toBe("en");
    expect(resolveLocale("ja", "en-US")).toBe("ja");
  });
});

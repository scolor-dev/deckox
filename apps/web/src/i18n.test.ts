import { describe, expect, it } from "vitest";
import { messages } from "./i18n";

function messageKeys(value: object, prefix = ""): string[] {
  return Object.entries(value).flatMap(([key, child]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    return typeof child === "object" && child !== null
      ? messageKeys(child as object, path)
      : [path];
  });
}

describe("translations", () => {
  it("has the same message keys in Japanese and English", () => {
    expect(messageKeys(messages.ja).sort()).toEqual(messageKeys(messages.en).sort());
  });
});

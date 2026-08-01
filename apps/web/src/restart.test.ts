import { describe, expect, it } from "vitest";
import { hasServerRestarted } from "./restart";

describe("hasServerRestarted", () => {
  it("detects a new server instance without requiring a failed poll", () => {
    expect(hasServerRestarted("old", "new", false)).toBe(true);
  });

  it("does not accept the old server instance", () => {
    expect(hasServerRestarted("same", "same", true)).toBe(false);
  });

  it("falls back to an offline-to-online transition", () => {
    expect(hasServerRestarted(null, "new", true)).toBe(true);
  });
});

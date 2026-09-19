import { describe, expect, it } from "vitest";
import { breakpoints } from "./breakpoints";

const components = import.meta.glob("./components/*.vue", {
  query: "?raw",
  import: "default",
  eager: true,
});

describe("breakpoints", () => {
  it("are the only widths components use in max-width media queries", () => {
    const allowed = new Set<number>(Object.values(breakpoints));
    const offenders: string[] = [];
    let found = 0;
    for (const [file, source] of Object.entries(components)) {
      for (const match of source.matchAll(/@media\s*\(\s*max-width:\s*(\d+)px\s*\)/g)) {
        found += 1;
        if (!allowed.has(Number(match[1]))) offenders.push(`${file}: ${match[0]}`);
      }
    }
    // Guards against a vacuous pass (glob matched nothing / regex drifted).
    expect(found).toBeGreaterThan(0);
    expect(offenders).toEqual([]);
  });
});

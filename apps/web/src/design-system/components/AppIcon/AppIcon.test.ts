import { describe, expect, it } from "vitest";
import doc from "./AppIcon.md?raw";
import { ICON_PATHS } from "./icons";

describe("AppIcon", () => {
  it("documents exactly the icons that exist", () => {
    const row = /\| `name` \| `IconName` \| required \| (.+) \|/.exec(doc)?.[1] ?? "";
    const documented = [...row.matchAll(/`([\w-]+)`/g)].map((match) => match[1]);
    expect(documented.sort()).toEqual(Object.keys(ICON_PATHS).sort());
  });

  it("only defines path data made of drawing commands", () => {
    for (const [name, paths] of Object.entries(ICON_PATHS)) {
      expect(paths.length, name).toBeGreaterThan(0);
      for (const path of paths) expect(path, name).toMatch(/^[MmLlHhVvCcSsQqTtAaZz\d\s.,-]+$/);
    }
  });
});

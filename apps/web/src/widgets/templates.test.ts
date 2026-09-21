import { describe, expect, it } from "vitest";

const sources = import.meta.glob(["../modules/**/*.vue", "./*.vue", "../views/*.vue"], {
  query: "?raw",
  import: "default",
  eager: true,
});

describe("widget components", () => {
  it("scans every module and widget component", () => {
    expect(Object.keys(sources).length).toBeGreaterThan(20);
  });

  it.each(Object.entries(sources))("%s imports every component its template uses", (_path, source) => {
    const [script = "", ...rest] = source.split("</script>");
    const template = rest.join("</script>");
    const used = [...template.matchAll(/<([A-Z][A-Za-z0-9]+)/g)].map((match) => match[1]);
    const missing = [...new Set(used)].filter((name) => !new RegExp(`\\b${name}\\b`).test(script));
    expect(missing).toEqual([]);
  });
});

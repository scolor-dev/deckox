import { describe, expect, it } from "vitest";
import appStyles from "../style.css?raw";

const vueModules = import.meta.glob(["../App.vue", "../views/*.vue", "../components/*.vue"], { query: "?raw", import: "default", eager: true });

const styleBlocks: [string, string][] = [
  ["style.css", appStyles],
  ...Object.entries(vueModules).map(([path, source]): [string, string] => [
    path,
    [...source.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((match) => match[1]).join("\n"),
  ]),
];

const SCALED = /(?:^|[;{\s])(font-size|border-radius|gap|row-gap|column-gap|padding(?:-top|-right|-bottom|-left)?|margin(?:-top|-right|-bottom|-left)?)\s*:\s*([^;}]*)/g;

describe("production styles use design tokens", () => {
  it("scans the app stylesheet and every component style block", () => {
    expect(styleBlocks.length).toBeGreaterThan(5);
    expect(appStyles.length).toBeGreaterThan(1000);
  });

  it.each(styleBlocks)("%s has no color literals", (_path, css) => {
    const withoutComments = css.replace(/\/\*[\s\S]*?\*\//g, "");
    expect(withoutComments.match(/#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/g) ?? []).toEqual([]);
  });

  it.each(styleBlocks)("%s has no px literals for font size, radius, gap, padding or margin", (_path, css) => {
    const withoutComments = css.replace(/\/\*[\s\S]*?\*\//g, "");
    const offenders = [...withoutComments.matchAll(SCALED)]
      .filter((match) => /(?<![\w-])-?[1-9]\d*(?:\.\d+)?px/.test(match[2].replace(/var\([^)]*\)/g, "").replace(/(?:min|max|calc|clamp)\([^)]*\)/g, "")))
      .map((match) => `${match[1]}: ${match[2].trim()}`);
    expect(offenders).toEqual([]);
  });
});

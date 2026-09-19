import { describe, expect, it } from "vitest";
import tokensSource from "./tokens.css?raw";

type Palette = Partial<Record<string, string>>;

function blockAfter(header: string, from = 0): string {
  const start = tokensSource.indexOf(header, from);
  if (start === -1) throw new Error(`missing block: ${header}`);
  const open = tokensSource.indexOf("{", start);
  let depth = 0;
  for (let i = open; i < tokensSource.length; i += 1) {
    if (tokensSource[i] === "{") depth += 1;
    else if (tokensSource[i] === "}") {
      depth -= 1;
      if (depth === 0) return tokensSource.slice(open + 1, i);
    }
  }
  throw new Error(`unbalanced block: ${header}`);
}

function colors(block: string): Palette {
  const palette: Record<string, string> = {};
  for (const match of block.matchAll(/--([\w-]+):\s*(#[0-9a-fA-F]{3}|#[0-9a-fA-F]{6})\s*;/g)) palette[match[1]] = match[2];
  return palette;
}

const light = colors(blockAfter("\n:root {"));
const darkAuto = colors(blockAfter(':root:not([data-theme="light"])'));
const darkExplicit = colors(blockAfter(':root[data-theme="dark"]'));
const dark: Palette = { ...light, ...darkExplicit };

function channel(value: number) {
  const c = value / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function luminance(hex: string) {
  const full = hex.replace(/^#(.)(.)(.)$/, "#$1$1$2$2$3$3");
  const [r, g, b] = [1, 3, 5].map((i) => channel(parseInt(full.slice(i, i + 2), 16)));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: string, b: string) {
  const [high, low] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (high + 0.05) / (low + 0.05);
}

const SURFACES = ["surface-page", "surface-elevated", "surface-hover"];

const TEXT_AA = ["text-base", "text-primary", "text-secondary", "text-strong", "text-label", "text-muted", "link"];
const TEXT_SOFT = ["text-faint", "text-faint-alt"];

interface Pair {
  foreground: string;
  background: string;
  minimum: number;
}

const pairs: Pair[] = [
  ...TEXT_AA.flatMap((foreground) => SURFACES.map((background) => ({ foreground, background, minimum: 4.5 }))),
  ...TEXT_SOFT.flatMap((foreground) => SURFACES.map((background) => ({ foreground, background, minimum: 3 }))),
  ...["danger", "success", "warning", "info"].map((tone) => ({ foreground: `${tone}-text`, background: `${tone}-bg`, minimum: 4.5 })),
  ...["standard", "deckox", "product"].map((tag) => ({ foreground: `tag-${tag}-text`, background: `tag-${tag}-bg`, minimum: 4.5 })),
  { foreground: "tag-off-text", background: "tag-off-bg", minimum: 3 },
  { foreground: "text-inverse", background: "brand-primary", minimum: 4.5 },
  { foreground: "text-inverse", background: "brand-primary-hover", minimum: 4.5 },
  { foreground: "text-inverse", background: "danger-strong", minimum: 4.5 },
  { foreground: "text-inverse", background: "danger-strong-hover", minimum: 4.5 },
  { foreground: "brand-primary", background: "surface-elevated", minimum: 3 },
  { foreground: "brand-focus", background: "surface-elevated", minimum: 3 },
  { foreground: "border-hover", background: "surface-elevated", minimum: 3 },
];

describe("token contrast", () => {
  it("keeps the automatic and explicit dark palettes identical", () => {
    expect(darkAuto).toEqual(darkExplicit);
    expect(Object.keys(darkExplicit).length).toBeGreaterThan(50);
  });

  it.each([["light", light], ["dark", dark]] as const)("%s palette meets the contrast contract", (_name, palette) => {
    const failures: string[] = [];
    for (const { foreground, background, minimum } of pairs) {
      const fg = palette[foreground];
      const bg = palette[background];
      if (fg === undefined || bg === undefined) {
        failures.push(`${foreground} on ${background}: token is not a plain hex color`);
        continue;
      }
      const ratio = contrast(fg, bg);
      if (ratio < minimum) failures.push(`${foreground} (${fg}) on ${background} (${bg}): ${ratio.toFixed(2)} < ${String(minimum)}`);
    }
    expect(failures).toEqual([]);
  });

  it("computes WCAG ratios correctly", () => {
    expect(contrast("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrast("#777777", "#ffffff")).toBeCloseTo(4.48, 2);
  });
});

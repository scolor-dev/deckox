import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { build } from "vite";

await build({ configFile: "vite.catalog.config.ts", logLevel: "warn" });

const dir = "dist-catalog/.build";
const cssFile = readdirSync(dir).find((name) => name.endsWith(".css"));
if (cssFile === undefined) throw new Error("catalog build produced no stylesheet");
const css = readFileSync(join(dir, cssFile), "utf8");
const js = readFileSync(join(dir, "catalog.js"), "utf8").replace(/<\/script/gi, "<\\/script");

const page = `<title>Deckox Design System</title>
<style>${css}</style>
<div id="app"></div>
<script>${js}</script>
`;
const out = "dist-catalog/deckox-design-system.html";
writeFileSync(out, page);
console.log(`${out} (${String(Math.round(page.length / 1024))} kB)`);

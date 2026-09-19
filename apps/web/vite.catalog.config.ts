import { fileURLToPath } from "node:url";
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

const root = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  root,
  plugins: [vue()],
  define: { "process.env.NODE_ENV": '"production"' },
  build: {
    outDir: "dist-catalog/.build",
    emptyOutDir: true,
    cssCodeSplit: false,
    lib: {
      entry: "src/design-system/catalog/artifact/entry.ts",
      formats: ["iife"],
      name: "DeckoxDesignSystem",
      fileName: () => "catalog.js",
    },
  },
});

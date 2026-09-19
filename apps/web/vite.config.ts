import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  test: {
    css: { include: [/design-system\/tokens\.css/, /src\/style\.css/] },
  },
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: "http://127.0.0.1:8080",
        ws: true,
      },
      "/healthz": "http://127.0.0.1:8080"
    }
  }
});

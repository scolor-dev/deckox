import { createApp } from "vue";
import { initTheme } from "../theme";
import DesignSystemCatalog from "./DesignSystemCatalog.vue";
import "../../style.css";

// Dev-only preview of the design-system components in isolation — never
// wired into vite.config.ts's build input, so it never ships in the
// production bundle (npm run build only ever sees index.html). Reachable
// during `npm run dev` at /design-system.html.
initTheme();
createApp(DesignSystemCatalog).mount("#app");

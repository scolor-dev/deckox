import { createApp } from "vue";
import App from "./App.vue";
import { initTheme } from "./design-system/theme";
import { i18n } from "./i18n";
import { router } from "./router";
import "./style.css";

initTheme();
createApp(App).use(i18n).use(router).mount("#app");

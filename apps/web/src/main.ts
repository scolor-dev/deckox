import { createApp } from "vue";
import App from "./App.vue";
import { initTheme } from "./design-system/theme";
import { i18n } from "./i18n";
import { mergeModuleMessages } from "./modules/messages";
import { router } from "./router";
import "./style.css";

initTheme();
mergeModuleMessages(i18n);
createApp(App).use(i18n).use(router).mount("#app");

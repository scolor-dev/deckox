import { createRouter, createWebHistory } from "vue-router";
import OverviewView from "./views/OverviewView.vue";
import ServicesView from "./views/ServicesView.vue";
import SettingsView from "./views/SettingsView.vue";
import StorageView from "./views/StorageView.vue";

export const routes = [
  { path: "/", name: "overview", component: OverviewView, meta: { titleKey: "nav.overview" } },
  { path: "/services", name: "services", component: ServicesView, meta: { titleKey: "nav.services" } },
  { path: "/storage", name: "storage", component: StorageView, meta: { titleKey: "nav.storage" } },
  { path: "/settings", name: "settings", component: SettingsView, meta: { titleKey: "nav.settings" } },
] as const;

export const router = createRouter({
  history: createWebHistory(),
  routes: [...routes],
});

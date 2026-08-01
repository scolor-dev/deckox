import { createRouter, createWebHistory } from "vue-router";
import OverviewView from "./views/OverviewView.vue";
import ServicesView from "./views/ServicesView.vue";
import SettingsView from "./views/SettingsView.vue";
import StorageView from "./views/StorageView.vue";

export const routes = [
  { path: "/", name: "overview", component: OverviewView, meta: { title: "概要" } },
  { path: "/services", name: "services", component: ServicesView, meta: { title: "サービス" } },
  { path: "/storage", name: "storage", component: StorageView, meta: { title: "ストレージ" } },
  { path: "/settings", name: "settings", component: SettingsView, meta: { title: "設定" } },
] as const;

export const router = createRouter({
  history: createWebHistory(),
  routes: [...routes],
});

import { createRouter, createWebHistory, type RouteRecordRaw } from "vue-router";
import { WEB_MODULES } from "./modules/registry";

const moduleRoutes: RouteRecordRaw[] = WEB_MODULES.map((module) => ({
  path: module.path,
  name: module.id,
  component: module.load,
  meta: { titleKey: module.titleKey, moduleId: module.id },
}));

export const routes: RouteRecordRaw[] = [
  ...moduleRoutes,
  {
    path: "/restarting",
    name: "restarting",
    component: () => import("./views/RestartingView.vue"),
    meta: { titleKey: "restart.title" },
  },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
});

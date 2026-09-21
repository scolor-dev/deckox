import { createRouter, createWebHistory } from "vue-router";

/**
 * Screens are pages of widgets (`/:pageId`), laid out by the user; only
 * settings and the restart wait are routes of their own.
 */
export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/settings",
      name: "settings",
      component: () => import("./views/SettingsView.vue"),
      meta: { titleKey: "nav.settings" },
    },
    {
      path: "/restarting",
      name: "restarting",
      component: () => import("./views/RestartingView.vue"),
      meta: { titleKey: "restart.title" },
    },
    {
      path: "/:pageId?",
      name: "page",
      component: () => import("./views/PageView.vue"),
    },
  ],
});

import type { WebModule } from "../../widgets/types";

/** Checking GitHub Releases for a newer Deckox, from the Server's `update-check` module. */
const updateCheck: WebModule = {
  id: "update-check",
  order: 60,
  titleKey: "widgets.updateCheck.title",
  widgets: [
    {
      id: "update-check.status",
      titleKey: "widgets.updateCheck.status.title",
      descriptionKey: "widgets.updateCheck.status.description",
      requires: ["update-check"],
      chrome: "frame",
      component: () => import("./widgets/UpdateStatusWidget.vue"),
      size: { default: { w: 6, h: "auto" }, min: { w: 4, h: 3 } },
    },
  ],
  messages: {
    ja: {
      widgets: {
        updateCheck: {
          title: "更新",
          status: { title: "Deckoxの更新", description: "新しいバージョンがあるかを確認し、許可されている場合は管理画面から更新します。" },
        },
      },
    },
    en: {
      widgets: {
        updateCheck: {
          title: "Updates",
          status: { title: "Deckox updates", description: "Check for a newer version and, where allowed, update from here." },
        },
      },
    },
  },
};

export default updateCheck;

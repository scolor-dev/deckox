import type { WebModule } from "../../widgets/types";

/** Restarting the host, from the Agent's `power` module. */
const power: WebModule = {
  id: "power",
  order: 70,
  titleKey: "widgets.power.title",
  widgets: [
    {
      id: "power.restart",
      titleKey: "widgets.power.restart.title",
      descriptionKey: "widgets.power.restart.description",
      requires: ["power"],
      chrome: "frame",
      component: () => import("./widgets/RestartWidget.vue"),
      size: { default: { w: 6, h: "auto" }, min: { w: 4, h: 3 } },
    },
  ],
  messages: {
    ja: {
      widgets: {
        power: {
          title: "電源",
          restart: { title: "ホストの再起動", description: "Linuxホスト全体を再起動します。管理者パスワードの再確認が必要です。" },
        },
      },
    },
    en: {
      widgets: {
        power: {
          title: "Power",
          restart: { title: "Restart the host", description: "Restart the whole Linux host. The administrator password is asked for again." },
        },
      },
    },
  },
};

export default power;

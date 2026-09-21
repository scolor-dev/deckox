import type { WebModule } from "../../widgets/types";

/** systemd services and their schedules, from the Agent's `services` and `schedules` modules. */
const services: WebModule = {
  id: "services",
  order: 30,
  titleKey: "widgets.services.title",
  widgets: [
    {
      id: "services.list",
      titleKey: "widgets.services.list.title",
      descriptionKey: "widgets.services.list.description",
      requires: ["services"],
      chrome: "bare",
      component: () => import("./widgets/ServicesListWidget.vue"),
      size: { default: { w: 12, h: "auto" }, min: { w: 6, h: 3 } },
    },
    {
      id: "services.schedules",
      titleKey: "widgets.services.schedules.title",
      descriptionKey: "widgets.services.schedules.description",
      requires: ["services", "schedules"],
      chrome: "bare",
      component: () => import("./widgets/SchedulesWidget.vue"),
      size: { default: { w: 12, h: "auto" }, min: { w: 6, h: 3 } },
    },
  ],
  messages: {
    ja: {
      widgets: {
        services: {
          title: "サービス",
          list: { title: "サービス一覧", description: "systemdのサービスの状態を確認し、許可したものを起動・停止・再起動できます。" },
          schedules: { title: "スケジュール", description: "許可したサービスを、曜日と時刻を決めて自動で操作します。" },
        },
      },
    },
    en: {
      widgets: {
        services: {
          title: "Services",
          list: { title: "Services", description: "See systemd services and start, stop or restart the ones you allowed." },
          schedules: { title: "Schedules", description: "Run an action on an allowed service at a set day and time." },
        },
      },
    },
  },
};

export default services;

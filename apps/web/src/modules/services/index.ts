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
    {
      id: "services.status",
      titleKey: "widgets.services.status.title",
      descriptionKey: "widgets.services.status.description",
      requires: ["services"],
      chrome: "bare",
      component: () => import("./widgets/ServiceStatusWidget.vue"),
      size: { default: { w: 4, h: 2 }, min: { w: 2, h: 2 } },
      config: [
        { key: "service", kind: "text", labelKey: "widgets.services.status.field", default: "docker.service" },
      ],
      titleFromConfig: (config) => (typeof config.service === "string" && config.service !== "" ? config.service : null),
    },
  ],
  messages: {
    ja: {
      widgets: {
        services: {
          title: "サービス",
          list: { title: "サービス一覧", description: "systemdのサービスの状態を確認し、許可したものを起動・停止・再起動できます。" },
          schedules: { title: "スケジュール", description: "許可したサービスを、曜日と時刻を決めて自動で操作します。" },
          status: {
            title: "サービスの状態",
            description: "指定した1つのサービスの状態を、小さなタイルで表示します。",
            field: "サービス名(例: docker.service)",
            notSet: "サービス名を設定してください",
            notFound: "見つかりません",
            failed: "サービスが失敗しています",
          },
        },
      },
    },
    en: {
      widgets: {
        services: {
          title: "Services",
          list: { title: "Services", description: "See systemd services and start, stop or restart the ones you allowed." },
          schedules: { title: "Schedules", description: "Run an action on an allowed service at a set day and time." },
          status: {
            title: "Service status",
            description: "The state of one chosen service, as a small tile.",
            field: "Service name (for example docker.service)",
            notSet: "Set a service name",
            notFound: "Not found",
            failed: "The service has failed",
          },
        },
      },
    },
  },
};

export default services;

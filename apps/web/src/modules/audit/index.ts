import type { WebModule } from "../../widgets/types";

/** The record of administrative actions, from the Server's `audit` module. */
const audit: WebModule = {
  id: "audit",
  order: 50,
  titleKey: "widgets.audit.title",
  widgets: [
    {
      id: "audit.log",
      titleKey: "widgets.audit.log.title",
      descriptionKey: "widgets.audit.log.description",
      requires: ["audit"],
      chrome: "bare",
      component: () => import("./widgets/AuditLogWidget.vue"),
      size: { default: { w: 12, h: "auto" }, min: { w: 6, h: 3 } },
    },
  ],
  messages: {
    ja: {
      widgets: {
        audit: {
          title: "監査ログ",
          log: { title: "監査ログ", description: "ログイン、サービス操作、再起動などの操作の記録を、新しい順に表示します。" },
        },
      },
    },
    en: {
      widgets: {
        audit: {
          title: "Audit log",
          log: { title: "Audit log", description: "A record of logins, service actions, restarts and more, newest first." },
        },
      },
    },
  },
};

export default audit;

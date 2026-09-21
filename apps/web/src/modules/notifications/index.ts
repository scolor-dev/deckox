import type { WebModule } from "../../widgets/types";

/** Webhook notifications, from the Server's `notifications` module. */
const notifications: WebModule = {
  id: "notifications",
  order: 80,
  titleKey: "widgets.notifications.title",
  widgets: [
    {
      id: "notifications.webhook",
      titleKey: "widgets.notifications.webhook.title",
      descriptionKey: "widgets.notifications.webhook.description",
      requires: ["notifications"],
      chrome: "frame",
      component: () => import("./widgets/WebhookWidget.vue"),
      size: { default: { w: 6, h: "auto" }, min: { w: 4, h: 2 } },
    },
  ],
  messages: {
    ja: {
      widgets: {
        notifications: {
          title: "通知",
          webhook: { title: "Webhook通知", description: "Webhookの設定状態を確認し、テスト通知を送ります。" },
        },
      },
    },
    en: {
      widgets: {
        notifications: {
          title: "Notifications",
          webhook: { title: "Webhook notifications", description: "See whether a webhook is set up and send a test notification." },
        },
      },
    },
  },
};

export default notifications;

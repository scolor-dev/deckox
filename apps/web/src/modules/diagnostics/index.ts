import type { WebModule } from "../../widgets/types";

/** Health checks and the backups kept before self-updates. */
const diagnostics: WebModule = {
  id: "diagnostics",
  order: 40,
  titleKey: "widgets.diagnostics.title",
  widgets: [
    {
      id: "diagnostics.summary",
      titleKey: "widgets.diagnostics.summary.title",
      descriptionKey: "widgets.diagnostics.summary.description",
      requires: ["diagnostics"],
      chrome: "bare",
      component: () => import("./widgets/DiagnosticsSummaryWidget.vue"),
      size: { default: { w: 12, h: "auto" }, min: { w: 6, h: 3 } },
    },
    {
      id: "diagnostics.backups",
      titleKey: "widgets.diagnostics.backups.title",
      descriptionKey: "widgets.diagnostics.backups.description",
      requires: ["backups"],
      chrome: "bare",
      component: () => import("./widgets/BackupsWidget.vue"),
      size: { default: { w: 12, h: "auto" }, min: { w: 6, h: 3 } },
    },
  ],
  messages: {
    ja: {
      widgets: {
        diagnostics: {
          title: "診断",
          summary: { title: "診断", description: "Server・Agent・ホストの状態と設定を確認し、診断レポートを保存できます。" },
          backups: { title: "バックアップ", description: "自己更新の前に残されたバックアップの一覧です。" },
        },
      },
    },
    en: {
      widgets: {
        diagnostics: {
          title: "Diagnostics",
          summary: { title: "Diagnostics", description: "State and settings of the Server, Agent and host, with a report you can save." },
          backups: { title: "Backups", description: "The backups kept before self-updates." },
        },
      },
    },
  },
};

export default diagnostics;

import type { WebModule } from "../../widgets/types";

/** Packages managed through the host's package manager, from the Agent's `software` module. */
const software: WebModule = {
  id: "software",
  order: 35,
  titleKey: "widgets.software.title",
  widgets: [
    {
      id: "software.list",
      titleKey: "widgets.software.list.title",
      descriptionKey: "widgets.software.list.description",
      requires: ["software"],
      chrome: "bare",
      component: () => import("./widgets/SoftwareListWidget.vue"),
      size: { default: { w: 12, h: "auto" }, min: { w: 6, h: 3 } },
    },
    {
      id: "software.add",
      titleKey: "widgets.software.add.title",
      descriptionKey: "widgets.software.add.description",
      requires: ["software"],
      chrome: "bare",
      component: () => import("./widgets/SoftwareAddWidget.vue"),
      size: { default: { w: 6, h: "auto" }, min: { w: 4, h: 2 } },
    },
    {
      id: "software.installed",
      titleKey: "widgets.software.installed.title",
      descriptionKey: "widgets.software.installed.description",
      requires: ["software"],
      chrome: "bare",
      component: () => import("./widgets/SoftwareInstalledWidget.vue"),
      size: { default: { w: 12, h: "auto" }, min: { w: 6, h: 3 } },
    },
  ],
  messages: {
    ja: {
      widgets: {
        software: {
          title: "ソフトウェア",
          list: { title: "管理中のソフトウェア", description: "管理対象のパッケージの状態と、インストール・削除・更新の操作です。" },
          add: { title: "管理対象の追加", description: "パッケージ名を入力して、管理対象に加えます。" },
          installed: { title: "インストール済み一覧", description: "ホストにインストールされている、すべてのパッケージから探して、管理対象に加えます。" },
        },
      },
    },
    en: {
      widgets: {
        software: {
          title: "Software",
          list: { title: "Managed software", description: "State of managed packages, with install, remove and upgrade." },
          add: { title: "Add to management", description: "Type a package name to manage it." },
          installed: { title: "Installed packages", description: "Search every package installed on the host and add one to management." },
        },
      },
    },
  },
};

export default software;

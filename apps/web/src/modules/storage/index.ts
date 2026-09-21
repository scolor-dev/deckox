import { computed } from "vue";
import { storageMounts } from "../../data/sources";
import type { WebModule } from "../../widgets/types";

/** The mounts and disks of the host, from the Agent's `storage` module. */
const storage: WebModule = {
  id: "storage",
  order: 20,
  titleKey: "widgets.storage.title",
  widgets: [
    {
      id: "storage.overall",
      titleKey: "widgets.storage.overall.title",
      descriptionKey: "widgets.storage.overall.description",
      requires: ["storage"],
      chrome: "bare",
      component: () => import("./widgets/OverallWidget.vue"),
      size: { default: { w: 4, h: 4 }, min: { w: 2, h: 2 } },
    },
    {
      id: "storage.mount-usage",
      titleKey: "widgets.storage.mountUsage.title",
      descriptionKey: "widgets.storage.mountUsage.description",
      requires: ["storage"],
      chrome: "bare",
      component: () => import("./widgets/MountUsageWidget.vue"),
      size: { default: { w: 2, h: 2 }, min: { w: 2, h: 2 }, max: { w: 12, h: 4 } },
      config: [
        {
          key: "mount",
          kind: "select",
          labelKey: "widgets.storage.fieldMount",
          default: "/",
          options: () => computed(() => (storageMounts.data.value ?? []).map((mount) => ({
            value: mount.mount_point,
            label: mount.mount_point,
          }))),
        },
      ],
      titleFromConfig: (config) => (typeof config.mount === "string" ? config.mount : null),
    },
    {
      id: "storage.allocation",
      titleKey: "widgets.storage.allocation.title",
      descriptionKey: "widgets.storage.allocation.description",
      requires: ["storage"],
      chrome: "bare",
      component: () => import("./widgets/AllocationWidget.vue"),
      size: { default: { w: 12, h: "auto" }, min: { w: 4, h: 2 } },
    },
    {
      id: "storage.mounts",
      titleKey: "widgets.storage.mounts.title",
      descriptionKey: "widgets.storage.mounts.description",
      requires: ["storage"],
      chrome: "bare",
      component: () => import("./widgets/MountsWidget.vue"),
      size: { default: { w: 12, h: "auto" }, min: { w: 6, h: 3 } },
    },
    {
      id: "storage.disks",
      titleKey: "widgets.storage.disks.title",
      descriptionKey: "widgets.storage.disks.description",
      requires: ["storage"],
      chrome: "bare",
      component: () => import("./widgets/DisksWidget.vue"),
      size: { default: { w: 12, h: "auto" }, min: { w: 6, h: 3 } },
    },
  ],
  messages: {
    ja: {
      widgets: {
        storage: {
          title: "ストレージ", fieldMount: "マウント先", mountMissing: "このマウント先は見つかりません",
          noDisks: "ディスクの情報を取得できません。",
          overall: { title: "全体の使用率", description: "すべてのマウントを合わせた使用率を、数値と帯で表示します。" },
          mountUsage: { title: "特定のマウントの使用率", description: "選んだマウント先ひとつの使用率を表示します。" },
          allocation: { title: "容量の内訳", description: "全体の使用率と、マウントごとの内訳を1本の帯で表示します。" },
          mounts: { title: "マウント一覧", description: "マウントごとの容量と使用率を、表で表示します。" },
          disks: { title: "ディスクとパーティション", description: "ディスクごとに、パーティションの並びと使用状況を表示します。" },
        },
      },
    },
    en: {
      widgets: {
        storage: {
          title: "Storage", fieldMount: "Mount point", mountMissing: "This mount point was not found",
          noDisks: "Disk information is not available.",
          overall: { title: "Overall usage", description: "Usage of all mounts together, as a number and a bar." },
          mountUsage: { title: "Usage of one mount", description: "Usage of the one mount point you choose." },
          allocation: { title: "Capacity breakdown", description: "Overall usage and a one-bar breakdown by mount." },
          mounts: { title: "Mounts", description: "Capacity and usage of each mount, as a table." },
          disks: { title: "Disks and partitions", description: "Partition layout and usage for each disk." },
        },
      },
    },
  },
};

export default storage;

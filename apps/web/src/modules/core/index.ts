import { computed } from "vue";
import type { WebModule } from "../../widgets/types";

/**
 * The blocks every page can use whichever modules are on: text, headings,
 * notices and dividers, plus the state of the Server itself.
 */
const core: WebModule = {
  id: "core",
  order: 0,
  titleKey: "widgets.core.title",
  widgets: [
    {
      id: "core.text",
      titleKey: "widgets.core.text.title",
      descriptionKey: "widgets.core.text.description",
      requires: [],
      chrome: "bare",
      component: () => import("./widgets/TextBlock.vue"),
      size: { default: { w: 6, h: 2 }, min: { w: 2, h: 1 } },
      config: [
        { key: "title", kind: "text", labelKey: "widgets.core.fieldTitle", default: "" },
        { key: "body", kind: "text", labelKey: "widgets.core.fieldText", multiline: true, default: "" },
      ],
      titleFromConfig: (config) => (typeof config.title === "string" && config.title !== "" ? config.title : null),
    },
    {
      id: "core.heading",
      titleKey: "widgets.core.heading.title",
      descriptionKey: "widgets.core.heading.description",
      requires: [],
      chrome: "bare",
      component: () => import("./widgets/HeadingBlock.vue"),
      size: { default: { w: 12, h: 1 }, min: { w: 2, h: 1 }, max: { w: 12, h: 2 } },
      config: [{ key: "text", kind: "text", labelKey: "widgets.core.fieldText", default: "" }],
      titleFromConfig: (config) => (typeof config.text === "string" && config.text !== "" ? config.text : null),
    },
    {
      id: "core.notice",
      titleKey: "widgets.core.notice.title",
      descriptionKey: "widgets.core.notice.description",
      requires: [],
      chrome: "bare",
      component: () => import("./widgets/NoticeBlock.vue"),
      size: { default: { w: 12, h: 1 }, min: { w: 2, h: 1 } },
      config: [
        {
          key: "tone",
          kind: "select",
          labelKey: "widgets.core.fieldTone",
          default: "warning",
          options: () => computed(() => [
            { value: "success", label: "success" },
            { value: "warning", label: "warning" },
            { value: "error", label: "error" },
          ]),
        },
        { key: "body", kind: "text", labelKey: "widgets.core.fieldText", multiline: true, default: "" },
      ],
    },
    {
      id: "core.divider",
      titleKey: "widgets.core.divider.title",
      descriptionKey: "widgets.core.divider.description",
      requires: [],
      chrome: "bare",
      component: () => import("./widgets/DividerBlock.vue"),
      size: { default: { w: 12, h: 1 }, min: { w: 2, h: 1 }, max: { w: 12, h: 1 } },
    },
    {
      id: "core.jobs",
      titleKey: "widgets.core.jobs.title",
      descriptionKey: "widgets.core.jobs.description",
      requires: [],
      chrome: "frame",
      component: () => import("./widgets/JobsWidget.vue"),
      size: { default: { w: 6, h: "auto" }, min: { w: 4, h: 2 } },
    },
    {
      id: "core.server-status",
      titleKey: "widgets.core.serverStatus.title",
      descriptionKey: "widgets.core.serverStatus.description",
      requires: [],
      chrome: "bare",
      component: () => import("./widgets/ServerStatus.vue"),
      size: { default: { w: 6, h: 2 }, min: { w: 3, h: 2 } },
    },
  ],
  messages: {
    ja: {
      widgets: {
        core: {
          title: "基本",
          fieldTitle: "見出し", fieldText: "文章", fieldTone: "種類",
          text: { title: "文章", description: "見出しと文章を置きます。メモや説明に使います。" },
          heading: { title: "見出し", description: "ページを区切る大きな見出しです。" },
          notice: { title: "お知らせ", description: "目立たせたい一文を、色つきで置きます。" },
          divider: { title: "区切り線", description: "ウィジェットの間に線を引きます。" },
          jobs: { title: "実行中の操作", description: "インストールやサービス操作など、時間のかかる操作の進み具合と結果を表示します。" },
          serverStatus: { title: "サーバーの状態", description: "Deckox Serverの状態と、Agentへの接続を表示します。" },
        },
      },
    },
    en: {
      widgets: {
        core: {
          title: "Basics",
          fieldTitle: "Heading", fieldText: "Text", fieldTone: "Tone",
          text: { title: "Text", description: "A heading and some text, for notes and explanations." },
          heading: { title: "Heading", description: "A large heading that separates parts of a page." },
          notice: { title: "Notice", description: "One sentence to stand out, in colour." },
          divider: { title: "Divider", description: "A line between widgets." },
          jobs: { title: "Operations in progress", description: "Progress and result of long operations such as installs and service actions." },
          serverStatus: { title: "Server status", description: "The state of the Deckox Server and its connection to the Agent." },
        },
      },
    },
  },
};

export default core;

import type { WebModule } from "../../widgets/types";

/**
 * The admin's own settings: display, password and two-factor authentication.
 * They live on the Server and the browser, not in an Agent module, so they
 * need nothing switched on, and every one is locked in place.
 */
const settings: WebModule = {
  id: "settings",
  order: 90,
  titleKey: "widgets.settings.title",
  widgets: [
    {
      id: "settings.display",
      titleKey: "widgets.settings.display.title",
      descriptionKey: "widgets.settings.display.description",
      requires: [],
      chrome: "frame",
      locked: true,
      component: () => import("./widgets/DisplayWidget.vue"),
      size: { default: { w: 6, h: "auto" }, min: { w: 4, h: 2 } },
    },
    {
      id: "settings.password",
      titleKey: "widgets.settings.password.title",
      descriptionKey: "widgets.settings.password.description",
      requires: [],
      chrome: "frame",
      locked: true,
      component: () => import("./widgets/PasswordWidget.vue"),
      size: { default: { w: 6, h: "auto" }, min: { w: 4, h: 2 } },
    },
    {
      id: "settings.totp",
      titleKey: "widgets.settings.totp.title",
      descriptionKey: "widgets.settings.totp.description",
      requires: [],
      chrome: "frame",
      locked: true,
      component: () => import("./widgets/TotpWidget.vue"),
      size: { default: { w: 6, h: "auto" }, min: { w: 4, h: 2 } },
    },
  ],
  messages: {
    ja: {
      widgets: {
        settings: {
          title: "設定",
          display: { title: "表示とリアルタイム更新", description: "このブラウザで使う表示言語・配色・更新方法を設定します。" },
          password: { title: "パスワード", description: "管理者パスワードを変更します。変更後は全セッションが失効します。" },
          totp: { title: "二要素認証(TOTP)", description: "認証アプリによる二要素認証の有効化・無効化と、リカバリーコードを扱います。" },
        },
      },
    },
    en: {
      widgets: {
        settings: {
          title: "Settings",
          display: { title: "Display and live updates", description: "Language, theme and update method used by this browser." },
          password: { title: "Password", description: "Change the administrator password. Every session is signed out afterwards." },
          totp: { title: "Two-factor authentication (TOTP)", description: "Turn authenticator-app two-factor sign-in on or off, and handle recovery codes." },
        },
      },
    },
  },
};

export default settings;

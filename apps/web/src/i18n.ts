import { watch } from "vue";
import { createI18n } from "vue-i18n";
import { preferences, resolveLocale } from "./preferences";

export const messages = {
  ja: {
    common: { refresh: "更新", loading: "読込中…", checking: "確認中…", close: "閉じる", dismiss: "通知を閉じる", version: "バージョン", none: "—" },
    app: {
      checkingAuth: "認証状態を確認しています…", serverManagement: "サーバー管理", menu: "メニュー",
      openMenu: "メニューを開く", mainNavigation: "メインナビゲーション", connected: "接続中",
      stateUnavailable: "状態を確認できません", logout: "ログアウト", passwordChanged: "パスワードを変更しました。新しいパスワードでログインしてください。",
    },
    nav: { overview: "概要", services: "サービス", storage: "ストレージ", console: "コンソール", settings: "設定" },
    restart: { title: "再起動しています", waiting: "再起動の開始を待っています…", offline: "コンピューターの起動を待っています…", ready: "再接続できました。ログイン画面へ戻ります。", timeout: "5分以内に再接続できませんでした。コンピューターの状態を確認してください。", retry: "もう一度確認", keepOpen: "この画面は閉じずにお待ちください。" },
    login: { title: "管理画面にログイン", description: "管理者パスワードを入力してください。", password: "パスワード", submit: "ログイン", submitting: "確認中…" },
    overview: {
      title: "概要", loadingHost: "サーバー情報を取得しています", realtime: "リアルタイム", paused: "一時停止", connecting: "接続中", reconnect: "再接続",
      agentUnavailable: "Agentに接続できません", healthy: "サーバーは正常に動作しています", uptime: "稼働時間", architecture: "アーキテクチャ",
      resources: "リソース使用状況", cpu: "CPU使用率", cores: "{count}コア", cpuChart: "CPU使用率の推移", memory: "メモリ",
      total: "全体 {value}", inUse: "{value} 使用中", memoryChart: "メモリ使用率の推移", load: "負荷平均", fiveMinutes: "5分 {value}",
      loadChart: "1分間負荷平均の推移", fifteenMinutes: "15分 {value}", systemInfo: "システム情報", hostname: "ホスト名", kernel: "カーネル", timezone: "タイムゾーン",
    },
    services: {
      title: "サービス", summary: "全{total}件のうち{running}件が稼働中", search: "サービスを検索", searchPlaceholder: "サービス名または説明を検索",
      count: "{count}件", service: "サービス", state: "状態", startup: "自動起動", actions: "操作", loading: "サービスを読み込んでいます…",
      empty: "該当するサービスはありません。", noDescription: "説明なし", running: "稼働中", failed: "異常", stopped: "停止中",
      enabled: "有効", disabled: "無効", static: "固定", start: "起動", restart: "再起動", stop: "停止", readOnly: "閲覧のみ",
      confirmStop: "{id} を停止しますか？", confirmRestart: "{id} を再起動しますか？", completed: "{id} の操作が完了しました。",
      allowlist: "変更操作は /etc/deckox/agent.toml の許可リストに登録されたサービスだけ利用できます。",
    },
    storage: {
      title: "ストレージ", summary: "{count}件のマウント", mount: "マウント先", filesystem: "ファイルシステム", capacity: "容量", usage: "使用状況",
      loading: "ストレージ情報を読み込んでいます…", empty: "マウントされたファイルシステムはありません。", available: "空き {value}", used: "{value} 使用",
    },
    console: {
      title: "コンソール", subtitle: "Deckoxの非rootユーザーでLinuxシェルを操作します", connect: "接続", disconnect: "切断", connecting: "コンソールへ接続しています…",
      disconnected: "コンソールとの接続が終了しました。", shellExited: "シェルが終了しました。", disabled: "Webコンソールはサーバー設定で無効です。",
      nonRoot: "このコンソールは専用の非rootユーザーに隔離されています。", limit: "同時{count}接続まで・{minutes}分間操作がない場合は自動終了します。", terminalLabel: "Linux Webコンソール",
      states: { disconnected: "未接続", connecting: "接続中", connected: "接続済み" },
      errors: { terminal_start_failed: "シェルを開始できませんでした。", terminal_io_failed: "コンソールの入出力でエラーが発生しました。", terminal_idle_timeout: "無操作時間が上限に達したため終了しました。" },
    },
    settings: {
      title: "設定", subtitle: "Deckoxとサーバーの管理設定", display: "表示とリアルタイム更新", displayDescription: "このブラウザで使う表示言語と更新方法を設定します。",
      language: "表示言語", languageAuto: "端末の設定に合わせる", japanese: "日本語", english: "English", realtime: "リアルタイム更新", realtimeHelp: "概要画面を表示している間だけ接続します。",
      interval: "更新間隔", intervalValue: "{seconds}秒", saved: "表示設定を保存しました。",
      systemOperations: "システム操作", systemOperationsDescription: "Linuxホスト全体に影響する操作です。実行前に管理者パスワードを再確認します。",
      reboot: "コンピューターを再起動", rebooting: "再起動を要求しています…", rebootPassword: "管理者パスワード", rebootPasswordRequired: "管理者パスワードを入力してください。",
      rebootHelp: "保存していない作業がないことを確認してください。", rebootDisabled: "再起動はAgent設定で無効です。[system] allow_reboot = true を設定してください。",
      confirmReboot: "コンピューターを再起動します。接続が一時的に切断されます。実行しますか？", rebootAccepted: "再起動を受け付けました。しばらくしてから再接続してください。",
      password: "管理者パスワード", passwordDescription: "管理画面へログインするときのパスワードを変更します。", currentPassword: "現在のパスワード",
      newPassword: "新しいパスワード", passwordRule: "12文字以上で入力してください。", passwordConfirmation: "新しいパスワード（確認）",
      changing: "変更しています…", changePassword: "パスワードを変更", relogin: "変更後は、すべての端末で再ログインが必要です。",
      passwordTooShort: "新しいパスワードは12文字以上で入力してください。", passwordMismatch: "新しいパスワードが確認欄と一致しません。",
      ssh: "SSH公開鍵", sshDescription: "指定したLinuxユーザーへのSSH接続を許可する公開鍵を管理します。", checkingKeys: "SSH公開鍵を確認しています…",
      sshDisabled: "SSH公開鍵管理は無効です。Agent設定の ssh.managed_user に、管理する非rootユーザーを指定してください。", managedUser: "対象ユーザー",
      noComment: "コメントなし", deleting: "削除中…", delete: "削除", noKeys: "Deckoxが管理しているSSH公開鍵はありません。", addKey: "公開鍵を追加",
      keyPlaceholder: "ssh-ed25519 AAAA... device-name", publicOnly: "秘密鍵は入力しないでください。OpenSSH形式の公開鍵1本だけを受け付けます。",
      adding: "追加しています…", keyRequired: "追加するSSH公開鍵を入力してください。", keyAdded: "{label} を追加しました。", keyRemoved: "{label} を削除しました。",
      confirmRemove: "{label} を削除しますか？", lastKey: "最後のSSH公開鍵は削除できません。先に別の鍵を追加してください。",
    },
    notifications: { streamLost: "リアルタイム更新が切断されました。再接続しています。", streamRestored: "リアルタイム更新に再接続しました。" },
    errors: {
      login: "ログインできませんでした。しばらくしてから再度お試しください。", overview: "システム情報を取得できませんでした。", services: "サービス一覧を取得できませんでした。",
      serviceAction: "サービス操作に失敗しました。", storage: "ストレージ情報を取得できませんでした。", password: "パスワードを変更できませんでした。",
      systemCapabilities: "システム操作の設定を取得できませんでした。", reboot: "コンピューターを再起動できませんでした。",
      terminalStatus: "コンソールの設定を取得できませんでした。", terminalConnection: "コンソールへ接続できませんでした。",
      sshLoad: "SSH公開鍵を取得できませんでした。", sshAdd: "SSH公開鍵を追加できませんでした。", sshRemove: "SSH公開鍵を削除できませんでした。",
      agentUnavailable: "Agentに接続できません。", badRequest: "入力内容を確認してください。", conflict: "現在の状態では操作できません。", forbidden: "この操作は許可されていません。",
      internal: "内部エラーが発生しました。", invalidCredentials: "パスワードが正しくありません。", invalidCurrentPassword: "現在のパスワードが正しくありません。",
      invalidNewPassword: "新しいパスワードの条件を確認してください。", notFound: "対象が見つかりません。", rateLimited: "確認に何度も失敗したため、5分ほど待ってから再度お試しください。",
      unavailable: "一時的に利用できません。", unauthorized: "ログインの有効期限が切れました。",
    },
    time: { dayHour: "{days}日 {hours}時間", hourMinute: "{hours}時間 {minutes}分", minute: "{minutes}分" },
  },
  en: {
    common: { refresh: "Refresh", loading: "Loading…", checking: "Checking…", close: "Close", dismiss: "Dismiss notification", version: "Version", none: "—" },
    app: {
      checkingAuth: "Checking authentication…", serverManagement: "Server management", menu: "Menu",
      openMenu: "Open menu", mainNavigation: "Main navigation", connected: "Connected",
      stateUnavailable: "Status unavailable", logout: "Log out", passwordChanged: "Password changed. Sign in with your new password.",
    },
    nav: { overview: "Overview", services: "Services", storage: "Storage", console: "Console", settings: "Settings" },
    restart: { title: "Restarting", waiting: "Waiting for the restart to begin…", offline: "Waiting for the computer to start…", ready: "Reconnected. Returning to sign in.", timeout: "Could not reconnect within five minutes. Check the computer status.", retry: "Check again", keepOpen: "Keep this page open while Deckox reconnects." },
    login: { title: "Sign in to Deckox", description: "Enter the administrator password.", password: "Password", submit: "Sign in", submitting: "Checking…" },
    overview: {
      title: "Overview", loadingHost: "Loading server information", realtime: "Live", paused: "Paused", connecting: "Connecting", reconnect: "Reconnect",
      agentUnavailable: "Cannot connect to Agent", healthy: "Server is operating normally", uptime: "Uptime", architecture: "Architecture",
      resources: "Resource usage", cpu: "CPU usage", cores: "{count} cores", cpuChart: "CPU usage history", memory: "Memory",
      total: "Total {value}", inUse: "{value} in use", memoryChart: "Memory usage history", load: "Load average", fiveMinutes: "5 min {value}",
      loadChart: "One-minute load average history", fifteenMinutes: "15 min {value}", systemInfo: "System information", hostname: "Hostname", kernel: "Kernel", timezone: "Time zone",
    },
    services: {
      title: "Services", summary: "{running} of {total} services running", search: "Search services", searchPlaceholder: "Search by service name or description",
      count: "{count}", service: "Service", state: "State", startup: "Startup", actions: "Actions", loading: "Loading services…",
      empty: "No matching services.", noDescription: "No description", running: "Running", failed: "Failed", stopped: "Stopped",
      enabled: "Enabled", disabled: "Disabled", static: "Static", start: "Start", restart: "Restart", stop: "Stop", readOnly: "Read only",
      confirmStop: "Stop {id}?", confirmRestart: "Restart {id}?", completed: "Completed the operation for {id}.",
      allowlist: "Changes are available only for services listed in /etc/deckox/agent.toml.",
    },
    storage: {
      title: "Storage", summary: "{count} mounts", mount: "Mount point", filesystem: "File system", capacity: "Capacity", usage: "Usage",
      loading: "Loading storage information…", empty: "No mounted file systems.", available: "{value} available", used: "{value} used",
    },
    console: {
      title: "Console", subtitle: "Use a Linux shell as the non-root Deckox user", connect: "Connect", disconnect: "Disconnect", connecting: "Connecting to the console…",
      disconnected: "The console connection has ended.", shellExited: "The shell has exited.", disabled: "The web console is disabled in the server configuration.",
      nonRoot: "This console is isolated under a dedicated non-root user.", limit: "Up to {count} sessions; closes after {minutes} minutes without input.", terminalLabel: "Linux web console",
      states: { disconnected: "Disconnected", connecting: "Connecting", connected: "Connected" },
      errors: { terminal_start_failed: "Could not start the shell.", terminal_io_failed: "A console input/output error occurred.", terminal_idle_timeout: "The console closed after reaching the idle time limit." },
    },
    settings: {
      title: "Settings", subtitle: "Deckox and server settings", display: "Display and live updates", displayDescription: "Configure the language and update behavior for this browser.",
      language: "Language", languageAuto: "Use device language", japanese: "日本語", english: "English", realtime: "Live updates", realtimeHelp: "Connects only while the Overview page is visible.",
      interval: "Update interval", intervalValue: "{seconds} sec", saved: "Display settings saved.",
      systemOperations: "System operations", systemOperationsDescription: "These operations affect the entire Linux host. The administrator password is required again.",
      reboot: "Restart computer", rebooting: "Requesting restart…", rebootPassword: "Administrator password", rebootPasswordRequired: "Enter the administrator password.",
      rebootHelp: "Make sure all work is saved before continuing.", rebootDisabled: "Restart is disabled in the Agent configuration. Set [system] allow_reboot = true.",
      confirmReboot: "Restart this computer? The connection will be temporarily unavailable.", rebootAccepted: "Restart accepted. Reconnect after the computer starts again.",
      password: "Administrator password", passwordDescription: "Change the password used to sign in to Deckox.", currentPassword: "Current password",
      newPassword: "New password", passwordRule: "Use at least 12 characters.", passwordConfirmation: "Confirm new password",
      changing: "Changing…", changePassword: "Change password", relogin: "You will need to sign in again on every device.",
      passwordTooShort: "The new password must contain at least 12 characters.", passwordMismatch: "The new passwords do not match.",
      ssh: "SSH public keys", sshDescription: "Manage public keys allowed to connect to the configured Linux user.", checkingKeys: "Checking SSH public keys…",
      sshDisabled: "SSH key management is disabled. Set ssh.managed_user in the Agent configuration to a non-root user.", managedUser: "Managed user",
      noComment: "No comment", deleting: "Deleting…", delete: "Delete", noKeys: "Deckox is not managing any SSH public keys.", addKey: "Add public key",
      keyPlaceholder: "ssh-ed25519 AAAA... device-name", publicOnly: "Enter one OpenSSH public key. Never enter a private key.",
      adding: "Adding…", keyRequired: "Enter an SSH public key to add.", keyAdded: "Added {label}.", keyRemoved: "Removed {label}.",
      confirmRemove: "Remove {label}?", lastKey: "The last SSH public key cannot be removed. Add another key first.",
    },
    notifications: { streamLost: "Live updates disconnected. Reconnecting…", streamRestored: "Live updates reconnected." },
    errors: {
      login: "Could not sign in. Try again shortly.", overview: "Could not load system information.", services: "Could not load services.",
      serviceAction: "The service operation failed.", storage: "Could not load storage information.", password: "Could not change the password.",
      systemCapabilities: "Could not load system operation settings.", reboot: "Could not restart the computer.",
      terminalStatus: "Could not load console settings.", terminalConnection: "Could not connect to the console.",
      sshLoad: "Could not load SSH public keys.", sshAdd: "Could not add the SSH public key.", sshRemove: "Could not remove the SSH public key.",
      agentUnavailable: "Cannot connect to Agent.", badRequest: "Check the information you entered.", conflict: "This operation is not available in the current state.", forbidden: "This operation is not permitted.",
      internal: "An internal error occurred.", invalidCredentials: "The password is incorrect.", invalidCurrentPassword: "The current password is incorrect.",
      invalidNewPassword: "Check the new password requirements.", notFound: "The requested item was not found.", rateLimited: "Too many failed attempts. Wait about five minutes and try again.",
      unavailable: "Temporarily unavailable.", unauthorized: "Your session has expired.",
    },
    time: { dayHour: "{days}d {hours}h", hourMinute: "{hours}h {minutes}m", minute: "{minutes}m" },
  },
} as const;

export const i18n = createI18n({
  legacy: false,
  locale: resolveLocale(preferences.locale, navigator.language),
  fallbackLocale: "en",
  messages,
});

function syncLocale() {
  const locale = resolveLocale(preferences.locale, navigator.language);
  i18n.global.locale.value = locale;
  if (typeof document !== "undefined") document.documentElement.lang = locale;
}

watch(() => preferences.locale, syncLocale, { flush: "sync" });
syncLocale();

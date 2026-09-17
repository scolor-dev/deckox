# Deckox

Deckoxは、Linuxをブラウザから安全に管理するためのWeb管理基盤です。SSHの代わりに
任意のコマンドを実行するWebコンソールは提供せず、あらかじめ許可した操作だけを
ブラウザから行えるようにする設計です。

## できること

- CPU・メモリ・Swap・ネットワーク送受信速度・ディスクI/O速度・ディスク使用率を
  リアルタイム表示（SVGグラフ、日本語・英語の表示切替）
- 許可リスト付きのsystemdサービス管理（起動・停止・再起動・有効化・無効化、
  journalログ閲覧、曜日・時刻を指定したスケジュール実行）
- パッケージ名を指定してのソフトウェア管理（ホストに設定済みのリポジトリで
  実在確認したものだけをインストール・削除・最新版へ更新、apt・dnf対応）
- 管理者パスワード変更、任意のTOTP二要素認証、パスワード再確認付きのホスト再起動
- ログイン・設定変更・サービス操作などを記録する監査ログ（閲覧・JSON保存）
- 安全な固定項目だけを含むシステム診断（JSON保存対応）、更新の確認、
  Webhookによる異常通知

実装済み機能の詳しい一覧は[`docs/features.html`](docs/features.html)、
設計・アーキテクチャは[`docs/architecture.html`](docs/architecture.html)を
参照してください。

## インストール

対応OSはLinux（x86_64・ARM64、`uname -m`から自動判定）です。

```bash
curl -fsSL \
  https://raw.githubusercontent.com/scolor-dev/deckox/main/packaging/scripts/install.sh \
  | sudo sh
```

初回インストールでは、対話端末が使える場合に待受アドレス（ローカルのみ/
LAN内のこのホストのアドレス）・ホスト再起動の許可・更新適用の許可を尋ねます。
無人インストール時は質問せず、従来どおりの既定値（ローカルのみ、両許可とも
無効）のまま導入されます。初回だけランダムな管理者パスワードがターミナルへ
一度だけ表示されるので、必ず控えてください。

バージョン指定・dry-run・アンインストール・ローカル配布物の検証など、
インストーラーの詳しい使い方は
[`docs/installation.html`](docs/installation.html)を参照してください。

## 使い始める

既定ではServerは`127.0.0.1:8080`だけに待ち受けます（インストール時にLAN内の
アドレスを選んでいれば、そのアドレスで待ち受けます）。別端末から一時的に
確認する場合はSSHトンネルを使います。

```bash
ssh -L 8080:127.0.0.1:8080 user@server
```

ブラウザで`http://127.0.0.1:8080/`を開き、インストール時に表示された
パスワードでログインします。ログイン後は「設定」画面からパスワード変更・
TOTP二要素認証の有効化ができます。管理画面へ入れなくなった場合は、SSH接続
したコンソールから次のサブコマンドで復旧できます。

```bash
printf '%s' '新しいパスワード' | sudo -u deckox deckox-server reset-password
sudo -u deckox deckox-server disable-totp   # TOTPだけを無効化する場合
sudo systemctl restart deckox-server
```

ホスト再起動・管理画面からの更新適用は、初期状態ではいずれも無効です。
インストール時の対話プロンプト（または無人インストールなら
`DECKOX_ALLOW_REBOOT`・`DECKOX_ALLOW_UPDATE`環境変数）で有効化するか選べます。
既にインストール済みのホストで後から変更する場合や、LAN内から常時アクセス
できるようにする場合の手順は
[`docs/installation.html`](docs/installation.html#agent-config-title)を
参照してください。

## セキュリティに関する注意

`deckox-server`は専用の`deckox`ユーザーで動作し、強い権限が必要な
`deckox-agent`とは別プロセスに分離されています。外部TCPポートは公開せず、
両者はUnixソケットだけで通信します。認証はArgon2idパスワード（任意で
TOTP二要素認証）、CookieはHttpOnly・SameSite=Strict、ログインとパスワード
再確認には試行制限があります。Agentは任意のシェルコマンドを受け付けず、
許可済みの型付き操作だけを実行します。

**TLS終端は実装していません。** 信頼できるLANまたはSSHトンネル内だけで
利用し、ルーターのポート転送や外部公開には使用しないでください。

## ドキュメント

| ページ | 内容 |
| --- | --- |
| [`docs/index.html`](docs/index.html) | ドキュメント全体の入口、実装範囲の状況 |
| [`docs/architecture.html`](docs/architecture.html) | 実行時アーキテクチャ、Server–Agent通信、セキュリティ境界 |
| [`docs/features.html`](docs/features.html) | 実装済み機能の詳細一覧（API・画面・共有型） |
| [`docs/installation.html`](docs/installation.html) | インストーラーの詳しい使い方、運用手順 |
| [`docs/development.html`](docs/development.html) | リポジトリ構成、ローカル開発、検証コマンド、CI |

## 開発に参加する

Rust（Cargo Workspace）とVue（Vite）で構成されています。リポジトリ構成、
ローカルでのAgent/Server起動方法、Vue開発サーバー、検証コマンド、CIの内容は
[`docs/development.html`](docs/development.html)にまとめています。

## ライセンス

[MIT License](LICENSE)

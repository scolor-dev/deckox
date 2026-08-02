# Deckox

Deckoxは、Linuxをブラウザから安全に管理するためのWeb管理基盤です。

現在は次の最小構成と、Agentによるシステム情報・リソース・ストレージ取得、
許可リスト付きsystemdサービス管理とjournalログ閲覧、管理者パスワード変更、SSH公開鍵管理、
パスワード再確認付きのホスト再起動を提供します。SSEによるCPU・メモリ・Swap・
ネットワーク送受信速度・ディスクI/O速度のリアルタイムメトリクス、任意取得のCPU温度、
軽量SVGグラフ、日本語・英語の表示切替、再起動後の自動再接続、画面ごとのURLと
ブラウザ別表示設定、最終更新時刻、Agent復旧時の状態再取得も実装済みです。任意コマンドを実行するWebコンソールは提供しません。

```text
Vue管理画面
    ↕ REST / SSE
deckox-server（Axum API + Vue配信）
    ↓ HTTP over Unix socket
deckox-agent（Linux操作）
    ↓
Linux
```

概要画面を開いている間は、既存のCPU・メモリ・負荷平均に加え、Swap使用率、
ネットワーク送受信速度、ディスク読み書き速度をSSEで更新します。CPU温度は
ホストから取得できる場合だけ表示します。Swap使用率が80%以上になると警告表示へ
切り替わります。取得できない追加メトリクスは`—`または非表示となり、CPUやメモリなど
取得可能な値の更新は継続します。管理画面の購読者が0件になるとAgentからの採取も停止します。

設計・実装済み機能・導入方法のHTMLドキュメントは
[`docs/index.html`](docs/index.html)から参照できます。

## リポジトリ構成

```text
apps/
├── server/          Axum Web APIサーバー
├── agent/           Linux管理Agent
└── web/             Vue + TypeScript
crates/
└── protocol/        Server・Agent間の共有型
packaging/
├── config/          Linux向け設定
├── scripts/         インストーラー
└── systemd/         systemdユニット
scripts/
└── package-release.sh
```

## ローカル開発

Rust側はAgent用のUnixソケットと、Server用の管理者パスワードハッシュが
必要です。通常のLinuxではAgentが`/run/deckox/agent.sock`を使用します。
一般ユーザーで試す場合は、両方に同じ一時ソケットを指定します。

```bash
printf '%s' '開発用パスワード' \
  | cargo run --quiet --package deckox-server -- hash-password \
  > /tmp/deckox-admin-password.hash

DECKOX_AGENT_SOCKET=/tmp/deckox-agent.sock cargo run --package deckox-agent
DECKOX_AGENT_SOCKET=/tmp/deckox-agent.sock \
DECKOX_WEB_DIR="$PWD/apps/web/dist" \
DECKOX_ADMIN_PASSWORD_HASH_FILE=/tmp/deckox-admin-password.hash \
cargo run --package deckox-server
```

Vue側:

```bash
cd apps/web
npm install
npm run dev
```

Vite開発サーバーは`/api`を`http://127.0.0.1:8080`へ転送します。

本番用フロントエンド:

```bash
cd apps/web
npm ci
npm run build
```

## 検証

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features --locked -- \
  -D warnings -W clippy::pedantic -W clippy::nursery

cd apps/web
npm ci
npm run lint
npm run typecheck
npm run test
npm run build
```

## CI

`main`・`develop`へのpushとPull RequestでGitHub Actionsの通常CIが動きます。

- Rust: `fmt`、厳格なClippy、ワークスペーステスト
- Vue: 依存関係の固定インストール、ESLint、型チェック、単体テスト、本番ビルド
- Packaging: インストール・リリース用シェルの構文確認

## GitHubからインストール

`v0.3.6`のようなタグをpushすると、GitHub ActionsがLinux x86-64・ARM64向け
バイナリ、Vue、設定、systemdユニットをまとめ、GitHub Releaseへ公開します。

```bash
git tag v0.3.6
git push origin v0.3.6
```

Release公開後、Linuxサーバーでは次のコマンドでインストールできます。

```bash
curl -fsSL \
  https://raw.githubusercontent.com/scolor-dev/deckox/main/packaging/scripts/install.sh \
  | sudo sh
```

スクリプトを確認してから実行する場合:

```bash
curl -fsSLO \
  https://raw.githubusercontent.com/scolor-dev/deckox/main/packaging/scripts/install.sh
less install.sh
sudo sh install.sh
```

特定バージョンをインストールする場合:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/scolor-dev/deckox/main/packaging/scripts/install.sh \
  | sudo DECKOX_VERSION=v0.3.6 sh
```

ローカルで作成した配布物を検証する場合は、アーカイブと同じ場所に
`.sha256`ファイルを置いて指定できます。

```bash
sudo DECKOX_ARCHIVE=/tmp/deckox-aarch64-unknown-linux-musl.tar.gz \
  sh install.sh
```

インストール後:

```bash
systemctl status deckox-server deckox-agent
journalctl -u deckox-server -u deckox-agent -f
```

初回インストール時には、ランダムな管理者パスワードがターミナルへ一度だけ
表示されます。更新時は既存のパスワードが維持されます。ログイン後の
「設定」画面から、現在のパスワードを確認して12文字以上の新しいパスワードへ
変更できます。現在のパスワード確認に5分間で5回失敗すると一時的に制限され、
変更後はすべてのセッションが失効します。ログイン画面には変更完了が表示されます。

管理画面へ入れない場合にパスワードを再設定する手順:

```bash
printf '%s' '新しいパスワード' \
  | sudo /usr/local/bin/deckox-server hash-password \
  | sudo tee /var/lib/deckox/admin-password.hash >/dev/null
sudo chown deckox:deckox /var/lib/deckox/admin-password.hash
sudo chmod 0600 /var/lib/deckox/admin-password.hash
sudo systemctl restart deckox-server
```

SSH公開鍵管理を有効にするには、`/etc/deckox/agent.toml`へ管理対象の
非rootローカルユーザーを指定します。

```toml
[ssh]
managed_user = "sorac"
```

```bash
sudo systemctl restart deckox-agent
```

設定画面ではOpenSSH形式の公開鍵を追加・削除できます。秘密鍵は受け付けず、
既存の`authorized_keys`はDeckox管理ブロック外に保持します。SSH接続手段を
失わないよう、外部の鍵を含めて最後の1本になる鍵は削除できません。Agentは
`.ssh`をシンボリックリンクを辿らずに開き、同じディレクトリFDを基準として
一時ファイルの作成、権限設定、同期、置換を行います。

ホスト再起動は初期状態では無効です。利用する場合は
`/etc/deckox/agent.toml`で明示的に許可し、Agentを再起動します。

```toml
[system]
allow_reboot = true
```

```bash
sudo systemctl restart deckox-agent
```

設定画面から再起動するときは管理者パスワードを再入力します。要求後は専用画面が
Webサーバーの新しいプロセス識別子を確認し、復帰後にログイン画面へ戻ります。
パスワード確認の試行制限はパスワード変更と共通です。

systemdサービスは、一覧と状態を確認したうえで、Agent設定の完全一致許可リストに
登録した対象だけを起動・停止・再起動・有効化・無効化できます。同じ許可対象について
journalログを最大500行まで表示し、全件・エラー・警告・情報のpriorityで絞り込めます。ログのファイルダウンロードと
任意のjournalctl引数指定には対応していません。

v0.3.3ではWebコンソールを削除しました。Linuxの対話操作には通常のSSHを利用し、
Deckoxからは許可された管理APIだけを実行します。v0.3.2から更新すると、旧Terminal
サービスとバイナリを撤去し、Deckox管理と確認できた専用ユーザー・グループも削除します。
`/var/lib/deckox-terminal`にファイルがある場合、そのディレクトリは保存されます。

Serverは既定で`127.0.0.1:8080`だけに待ち受けます。別端末から一時的に
確認する場合は、SSHトンネルを利用します。

```bash
ssh -L 8080:127.0.0.1:8080 user@server
```

LAN内の端末から常時アクセスする場合は、サーバーのLANアドレスだけへ
待受先を上書きします。次の例ではサーバーのアドレスを`192.168.1.21`と
しています。

```bash
sudo systemctl edit deckox-server
```

```ini
[Service]
Environment=DECKOX_LISTEN_ADDR=192.168.1.21:8080
```

保存後に反映します。

```bash
sudo systemctl restart deckox-server
```

同じLANの端末から`http://192.168.1.21:8080/`を開き、管理者パスワードで
ログインできます。TLS終端は未実装なので、信頼できるLANまたはSSHトンネル
内だけで利用し、ルーターのポート転送や外部公開には使用しないでください。

配置先:

```text
/usr/local/bin/deckox-server
/usr/local/bin/deckox-agent
/usr/local/share/deckox/web/
/etc/deckox/
/var/lib/deckox/
/run/deckox/agent.sock
```

対応アーキテクチャ:

```text
x86_64 / amd64  → x86_64-unknown-linux-musl
aarch64 / arm64 → aarch64-unknown-linux-musl
```

インストーラーが`uname -m`から自動判定するため、どちらも同じインストール
コマンドを利用できます。

## Docker

Docker Composeは開発・UI確認用にも利用できます。

```bash
docker compose up --build
```

`http://127.0.0.1:8080/`を開き、開発用パスワード`deckox`でログインします。
これはローカル開発専用の固定値です。
Composeはハッシュを環境変数で直接渡すため、設定画面からのパスワード変更は
利用できません。

コンテナ内のAgentはLinuxホストのsystemdなどを管理できません。Linux全体を
管理する本番用途では、systemdサービスとしてインストールしてください。
対話操作はコンテナ内にも提供しません。

## セキュリティ

`deckox-server`は専用の`deckox`ユーザーで動作します。強い権限が必要になる
`deckox-agent`は別プロセスとし、外部TCPポートを公開せずUnixソケットだけで
Serverと通信します。

Serverは単一管理者のArgon2idパスワード認証と、12時間のメモリ内セッション
を提供します。CookieはHttpOnly・SameSite=Strictです。ログインとパスワード
再確認には送信元IP単位の試行制限があります。認証、パスワード変更、
サービス操作、SSH公開鍵操作、ホスト再起動は
リクエストID付きでjournalへ記録します。
Agentは任意のシェルコマンドを受け付けず、許可済みの型付き操作だけを
実行します。サービスログ閲覧も完全一致許可リスト、500行の上限、全件・
エラー・警告・情報のpriority選択肢に制限されます。任意コマンドや対話シェルを
受け付けるWebコンソールは提供しません。

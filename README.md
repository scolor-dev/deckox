# Deckox

Deckoxは、Linuxをブラウザから安全に管理するためのWeb管理基盤です。

現在は次の最小構成と、Agentによるシステム情報・リソース・ストレージ取得、
許可リスト付きsystemdサービス管理を提供します。

```text
Vue管理画面
    ↓ HTTP
deckox-server（Axum API + Vue配信）
    ↓ HTTP over Unix socket
deckox-agent（Linux操作）
    ↓
Linux
```

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
npm run build
```

## CI

`main`・`develop`へのpushとPull RequestでGitHub Actionsの通常CIが動きます。

- Rust: `fmt`、厳格なClippy、ワークスペーステスト
- Vue: 依存関係の固定インストール、ESLint、型チェック、本番ビルド
- Packaging: インストール・リリース用シェルの構文確認

## GitHubからインストール

`v0.2.0`のようなタグをpushすると、GitHub ActionsがLinux x86-64・ARM64向け
バイナリ、Vue、設定、systemdユニットをまとめ、GitHub Releaseへ公開します。

```bash
git tag v0.2.0
git push origin v0.2.0
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
  | sudo DECKOX_VERSION=v0.2.0 sh
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
表示されます。更新時は既存のパスワードが維持されます。パスワードを再設定
する場合:

```bash
printf '%s' '新しいパスワード' \
  | sudo /usr/local/bin/deckox-server hash-password \
  | sudo tee /etc/deckox/admin-password.hash >/dev/null
sudo chown root:deckox /etc/deckox/admin-password.hash
sudo chmod 0640 /etc/deckox/admin-password.hash
sudo systemctl restart deckox-server
```

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

コンテナ内のAgentはLinuxホストのsystemdなどを管理できません。Linux全体を
管理する本番用途では、systemdサービスとしてインストールしてください。

## セキュリティ

`deckox-server`は専用の`deckox`ユーザーで動作します。強い権限が必要になる
`deckox-agent`は別プロセスとし、外部TCPポートを公開せずUnixソケットだけで
Serverと通信します。

Serverは単一管理者のArgon2idパスワード認証と、12時間のメモリ内セッション
を提供します。CookieはHttpOnly・SameSite=Strictです。サービス操作と認証
イベントはリクエストID付きでjournalへ記録します。Agentは任意のシェル
コマンドを受け付けず、許可済みの型付き操作だけを実行します。

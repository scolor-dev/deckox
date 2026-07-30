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

Rust側:

```bash
cargo run --package deckox-agent
cargo run --package deckox-server
```

通常のLinuxではAgentが`/run/deckox/agent.sock`を使用します。一般ユーザーで
試す場合は、両方に同じ一時ソケットを指定します。

```bash
DECKOX_AGENT_SOCKET=/tmp/deckox-agent.sock cargo run --package deckox-agent
DECKOX_AGENT_SOCKET=/tmp/deckox-agent.sock \
DECKOX_WEB_DIR="$PWD/apps/web/dist" \
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
cargo clippy --workspace --all-targets --all-features -- -D warnings

cd apps/web
npm ci
npm run typecheck
npm run build
```

## GitHubからインストール

`v0.1.0`のようなタグをpushすると、GitHub ActionsがLinux x86-64・ARM64向け
バイナリ、Vue、設定、systemdユニットをまとめ、GitHub Releaseへ公開します。

```bash
git tag v0.1.0
git push origin v0.1.0
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
  | sudo DECKOX_VERSION=v0.1.0 sh
```

インストール後:

```bash
systemctl status deckox-server deckox-agent
journalctl -u deckox-server -u deckox-agent -f
```

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

コンテナ内のAgentはLinuxホストのsystemdなどを管理できません。Linux全体を
管理する本番用途では、systemdサービスとしてインストールしてください。

## セキュリティ

`deckox-server`は専用の`deckox`ユーザーで動作します。強い権限が必要になる
`deckox-agent`は別プロセスとし、外部TCPポートを公開せずUnixソケットだけで
Serverと通信します。

現在のAgentは状態取得だけを提供します。今後のシステム操作APIでは、任意の
シェルコマンドを受け付けず、許可済みの型付き操作だけを実装します。

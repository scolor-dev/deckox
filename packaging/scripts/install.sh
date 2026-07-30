#!/bin/sh
set -eu

REPOSITORY="${DECKOX_REPOSITORY:-scolor-dev/deckox}"
VERSION="${DECKOX_VERSION:-latest}"
LOCAL_ARCHIVE="${DECKOX_ARCHIVE:-}"
BASE_URL="https://github.com/${REPOSITORY}/releases"

if [ "$(id -u)" -ne 0 ]; then
  echo "deckox installer must run as root (use sudo)" >&2
  exit 1
fi

for command in awk cp getent groupadd install mktemp systemctl tar uname useradd; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "required command not found: $command" >&2
    exit 1
  fi
done
if [ -z "$LOCAL_ARCHIVE" ] && ! command -v curl >/dev/null 2>&1; then
  echo "required command not found: curl" >&2
  exit 1
fi

case "$(uname -m)" in
  x86_64|amd64) target="x86_64-unknown-linux-musl" ;;
  aarch64|arm64) target="aarch64-unknown-linux-musl" ;;
  *)
    echo "unsupported architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

asset="deckox-${target}.tar.gz"
if [ "$VERSION" = "latest" ]; then
  download_url="${BASE_URL}/latest/download/${asset}"
else
  download_url="${BASE_URL}/download/${VERSION}/${asset}"
fi

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT INT TERM

if [ -n "$LOCAL_ARCHIVE" ]; then
  if [ ! -f "$LOCAL_ARCHIVE" ] || [ ! -f "${LOCAL_ARCHIVE}.sha256" ]; then
    echo "local archive and checksum are required: ${LOCAL_ARCHIVE}{,.sha256}" >&2
    exit 1
  fi
  echo "Installing local Deckox archive for ${target}..."
  cp "$LOCAL_ARCHIVE" "${work_dir}/${asset}"
  cp "${LOCAL_ARCHIVE}.sha256" "${work_dir}/${asset}.sha256"
else
  echo "Downloading Deckox ${VERSION} for ${target}..."
  curl --fail --location --proto '=https' --tlsv1.2 \
    --output "${work_dir}/${asset}" "$download_url"
  curl --fail --location --proto '=https' --tlsv1.2 \
    --output "${work_dir}/${asset}.sha256" "${download_url}.sha256"
fi

expected="$(awk '{print $1}' "${work_dir}/${asset}.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "${work_dir}/${asset}" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "${work_dir}/${asset}" | awk '{print $1}')"
else
  echo "sha256sum or shasum is required" >&2
  exit 1
fi

if [ "$expected" != "$actual" ]; then
  echo "checksum verification failed" >&2
  exit 1
fi

mkdir "${work_dir}/release"
tar -xzf "${work_dir}/${asset}" -C "${work_dir}/release"

if ! getent group deckox >/dev/null 2>&1; then
  groupadd --system deckox
fi
if ! id deckox >/dev/null 2>&1; then
  useradd --system --gid deckox --home-dir /var/lib/deckox \
    --shell /usr/sbin/nologin deckox
fi

install -d -m 0750 -o deckox -g deckox /etc/deckox /var/lib/deckox
install -d -m 0755 /usr/local/share/deckox/web
install -m 0755 "${work_dir}/release/bin/deckox-server" /usr/local/bin/deckox-server
install -m 0755 "${work_dir}/release/bin/deckox-agent" /usr/local/bin/deckox-agent
cp -R "${work_dir}/release/web/." /usr/local/share/deckox/web/

if [ ! -f /etc/deckox/server.toml ]; then
  install -m 0640 -o root -g deckox \
    "${work_dir}/release/config/server.toml" /etc/deckox/server.toml
fi
if [ ! -f /etc/deckox/agent.toml ]; then
  install -m 0640 -o root -g deckox \
    "${work_dir}/release/config/agent.toml" /etc/deckox/agent.toml
fi

install -m 0644 "${work_dir}/release/systemd/deckox-agent.service" \
  /etc/systemd/system/deckox-agent.service
install -m 0644 "${work_dir}/release/systemd/deckox-server.service" \
  /etc/systemd/system/deckox-server.service

systemctl daemon-reload
systemctl enable deckox-agent.service deckox-server.service
systemctl restart deckox-agent.service
systemctl restart deckox-server.service

echo
echo "Deckox has been installed."
echo "Deckox is listening on http://127.0.0.1:8080/"
echo "For remote access, configure authentication/TLS or use an SSH tunnel."
echo "Check status with: systemctl status deckox-server deckox-agent"

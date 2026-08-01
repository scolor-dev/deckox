#!/bin/sh
set -eu

target="${1:-x86_64-unknown-linux-musl}"
case "$target" in
  x86_64-unknown-linux-musl|aarch64-unknown-linux-musl) ;;
  *)
    echo "unsupported release target: $target" >&2
    exit 1
    ;;
esac

root_dir="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
output_dir="${root_dir}/dist"
stage_dir="${output_dir}/deckox-${target}"
archive="${output_dir}/deckox-${target}.tar.gz"

cd "${root_dir}/apps/web"
npm ci
npm run build

cd "$root_dir"
cargo build --release --locked --target "$target" \
  --package deckox-server --package deckox-agent

rm -rf "$stage_dir"
mkdir -p "$stage_dir/bin" "$stage_dir/web" "$stage_dir/config" "$stage_dir/systemd"
install -m 0755 "target/${target}/release/deckox-server" "$stage_dir/bin/"
install -m 0755 "target/${target}/release/deckox-agent" "$stage_dir/bin/"
cp -R apps/web/dist/. "$stage_dir/web/"
cp packaging/config/*.toml "$stage_dir/config/"
cp packaging/systemd/*.service "$stage_dir/systemd/"

COPYFILE_DISABLE=1 tar -czf "$archive" -C "$stage_dir" .

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$archive" > "${archive}.sha256"
else
  shasum -a 256 "$archive" > "${archive}.sha256"
fi

echo "Created ${archive}"

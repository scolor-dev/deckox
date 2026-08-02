#!/bin/sh
set -eu

root_dir="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
installer="${root_dir}/packaging/scripts/install.sh"
test_dir="$(mktemp -d)"
trap 'rm -rf "$test_dir"' EXIT INT TERM
fake_bin="${test_dir}/fake-bin"
state_dir="${test_dir}/state"
mkdir -p "$fake_bin" "$state_dir"

fail() {
  echo "installer test failed: $*" >&2
  exit 1
}

assert_file() {
  [ -f "$1" ] || fail "expected file: $1"
}

assert_missing() {
  [ ! -e "$1" ] || fail "expected missing path: $1"
}

assert_contains() {
  grep -F "$2" "$1" >/dev/null || fail "expected '$2' in $1"
}

write_fake() {
  name="$1"
  shift
  file="${fake_bin}/${name}"
  {
    echo '#!/bin/sh'
    printf '%s\n' "$@"
  } > "$file"
  chmod +x "$file"
}

write_fake uname 'echo "${TEST_ARCH:-x86_64}"'
write_fake id '
if [ "${1:-}" = "-u" ]; then echo 0; exit 0; fi
if [ "${1:-}" = "-gn" ]; then echo deckox; exit 0; fi
exit 1'
write_fake getent 'exit 1'
write_fake groupadd 'echo "groupadd $*" >> "$TEST_STATE/commands"'
write_fake groupdel 'echo "groupdel $*" >> "$TEST_STATE/commands"'
write_fake useradd 'echo "useradd $*" >> "$TEST_STATE/commands"'
write_fake userdel 'echo "userdel $*" >> "$TEST_STATE/commands"'
write_fake systemctl '
echo "systemctl $*" >> "$TEST_STATE/commands"
if [ "${TEST_FAIL_HEALTH:-0}" = 1 ] && [ "$*" = "is-active --quiet deckox-server.service" ]; then exit 1; fi
exit 0'
write_fake curl 'echo "curl $*" >> "$TEST_STATE/commands"; exit 99'
write_fake date 'echo 20260802T000000Z'

test_path="${fake_bin}:/usr/bin:/bin:/usr/sbin:/sbin"

checksum() {
  archive="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$archive" > "${archive}.sha256"
  else
    shasum -a 256 "$archive" > "${archive}.sha256"
  fi
}

make_archive() {
  version="$1"
  label="$2"
  stage="${test_dir}/stage-${version}-${label}"
  archive="${test_dir}/deckox-x86_64-unknown-linux-musl-${version}-${label}.tar.gz"
  mkdir -p "$stage/bin" "$stage/web" "$stage/config" "$stage/systemd"
  printf '%s\n' "$version" > "$stage/VERSION"
  printf '#!/bin/sh\nif [ "${1:-}" = hash-password ]; then echo hash-%s; fi\n' "$label" > "$stage/bin/deckox-server"
  printf '#!/bin/sh\necho agent-%s\n' "$label" > "$stage/bin/deckox-agent"
  chmod +x "$stage/bin/deckox-server" "$stage/bin/deckox-agent"
  printf '<!doctype html><title>%s</title>\n' "$label" > "$stage/web/index.html"
  printf 'listen_addr = "127.0.0.1:8080"\n' > "$stage/config/server.toml"
  printf 'socket = "/run/deckox/agent.sock"\n' > "$stage/config/agent.toml"
  cp "${root_dir}/packaging/systemd/deckox-agent.service" "$stage/systemd/"
  cp "${root_dir}/packaging/systemd/deckox-server.service" "$stage/systemd/"
  tar -czf "$archive" -C "$stage" .
  checksum "$archive"
  echo "$archive"
}

run_installer() {
  fake_root="$1"
  archive="$2"
  version="$3"
  shift 3
  PATH="$test_path" TEST_STATE="$state_dir" DECKOX_ROOT="$fake_root" \
    DECKOX_ARCHIVE="$archive" DECKOX_VERSION="v${version}" \
    sh "$installer" "$@"
}

# Dry runs are non-root, do not invoke curl/systemctl, and report both architectures.
: > "$state_dir/commands"
dry_root="${test_dir}/dry-root"
PATH="$test_path" TEST_STATE="$state_dir" TEST_ARCH=x86_64 DECKOX_ROOT="$dry_root" \
  DECKOX_VERSION=v9.9.9 sh "$installer" --dry-run > "$test_dir/dry-x86.out"
assert_contains "$test_dir/dry-x86.out" "x86_64-unknown-linux-musl"
assert_contains "$test_dir/dry-x86.out" "/download/v9.9.9/deckox-x86_64-unknown-linux-musl.tar.gz"
assert_missing "$dry_root"
[ ! -s "$state_dir/commands" ] || fail "dry run executed a changing command"
PATH="$test_path" TEST_STATE="$state_dir" TEST_ARCH=aarch64 DECKOX_ROOT="$dry_root" \
  sh "$installer" --dry-run > "$test_dir/dry-arm.out"
assert_contains "$test_dir/dry-arm.out" "aarch64-unknown-linux-musl"
if PATH="$test_path" TEST_STATE="$state_dir" TEST_ARCH=riscv64 DECKOX_ROOT="$dry_root" \
  sh "$installer" --dry-run > "$test_dir/dry-unsupported.out" 2>&1; then
  fail "unsupported architecture succeeded"
fi
assert_contains "$test_dir/dry-unsupported.out" "unsupported architecture"

# Version reporting is read-only and includes the selected and installed package.
version_root="${test_dir}/version-root"
mkdir -p "$version_root/usr/local/share/deckox"
printf '1.2.3\n' > "$version_root/usr/local/share/deckox/VERSION"
PATH="$test_path" TEST_STATE="$state_dir" DECKOX_ROOT="$version_root" DECKOX_VERSION=v2.0.0 \
  sh "$installer" --version > "$test_dir/version.out"
assert_contains "$test_dir/version.out" "Deckox installer 0.3.8"
assert_contains "$test_dir/version.out" "Requested package: v2.0.0"
assert_contains "$test_dir/version.out" "Installed package: 1.2.3"

# A checksum mismatch stops before any filesystem mutation.
bad_archive="$(make_archive 1.0.0 bad)"
printf '%064d  %s\n' 0 "$bad_archive" > "${bad_archive}.sha256"
bad_root="${test_dir}/bad-root"
if run_installer "$bad_root" "$bad_archive" 1.0.0 > "$test_dir/bad.out" 2>&1; then
  fail "checksum mismatch succeeded"
fi
assert_contains "$test_dir/bad.out" "checksum verification failed"
assert_missing "$bad_root"

# Initial install creates managed files without overwriting future persistent data.
archive_v1="$(make_archive 1.0.0 old)"

# An initial deployment failure removes the generated password and new managed files.
failed_initial_root="${test_dir}/failed-initial-root"
if TEST_FAIL_HEALTH=1 run_installer "$failed_initial_root" "$archive_v1" 1.0.0 \
  > "$test_dir/failed-initial.out" 2>&1; then
  fail "failed initial health check succeeded"
fi
unset TEST_FAIL_HEALTH
assert_missing "$failed_initial_root/var/lib/deckox/admin-password.hash"
assert_missing "$failed_initial_root/usr/local/bin/deckox-agent"
assert_missing "$failed_initial_root/usr/local/share/deckox/VERSION"

# A partial managed-artifact set is reported and left untouched.
partial_root="${test_dir}/partial-root"
mkdir -p "$partial_root/etc/systemd/system" "$partial_root/usr/local/bin"
cp "${root_dir}/packaging/systemd/deckox-agent.service" "$partial_root/etc/systemd/system/"
printf '#!/bin/sh\necho partial-agent\n' > "$partial_root/usr/local/bin/deckox-agent"
chmod +x "$partial_root/usr/local/bin/deckox-agent"
if run_installer "$partial_root" "$archive_v1" 1.0.0 > "$test_dir/partial.out" 2>&1; then
  fail "partial installation was overwritten"
fi
assert_contains "$test_dir/partial.out" "Installation type: partial"
assert_contains "$test_dir/partial.out" "no files were changed"
assert_contains "$partial_root/usr/local/bin/deckox-agent" "partial-agent"
assert_missing "$partial_root/usr/local/bin/deckox-server"

install_root="${test_dir}/install-root"
mkdir -p "$install_root/etc/deckox" "$install_root/var/lib/deckox"
printf 'keep-config\n' > "$install_root/etc/deckox/agent.toml"
printf 'keep-password\n' > "$install_root/var/lib/deckox/admin-password.hash"
run_installer "$install_root" "$archive_v1" 1.0.0 > "$test_dir/initial.out"
assert_contains "$test_dir/initial.out" "Installation type: initial"
assert_contains "$install_root/usr/local/share/deckox/VERSION" "1.0.0"
assert_contains "$install_root/etc/deckox/agent.toml" "keep-config"
assert_contains "$install_root/var/lib/deckox/admin-password.hash" "keep-password"

# A verified update backs up managed artifacts and preserves config/password data.
archive_v2="$(make_archive 1.1.0 new)"
run_installer "$install_root" "$archive_v2" 1.1.0 > "$test_dir/update.out"
assert_contains "$test_dir/update.out" "Installation type: update"
backup_dir="$install_root/var/lib/deckox/backups/20260802T000000Z-1.0.0"
assert_file "$backup_dir/VERSION"
assert_file "$backup_dir/deckox-server"
assert_file "$backup_dir/web/index.html"
assert_contains "$install_root/usr/local/share/deckox/VERSION" "1.1.0"
assert_contains "$install_root/etc/deckox/agent.toml" "keep-config"
assert_contains "$install_root/var/lib/deckox/admin-password.hash" "keep-password"

# Failed health verification restores the complete previous managed set.
archive_v3="$(make_archive 1.2.0 broken)"
if TEST_FAIL_HEALTH=1 run_installer "$install_root" "$archive_v3" 1.2.0 > "$test_dir/rollback.out" 2>&1; then
  fail "failed health check succeeded"
fi
unset TEST_FAIL_HEALTH
assert_contains "$test_dir/rollback.out" "restoring the previous managed artifacts"
assert_contains "$install_root/usr/local/share/deckox/VERSION" "1.1.0"
assert_contains "$install_root/usr/local/bin/deckox-agent" "agent-new"
assert_contains "$install_root/usr/local/share/deckox/web/index.html" "new"

# Uninstall removes only confirmed managed artifacts and keeps persistent state.
run_installer "$install_root" "$archive_v2" 1.1.0 --uninstall > "$test_dir/uninstall.out"
assert_missing "$install_root/usr/local/bin/deckox-agent"
assert_missing "$install_root/usr/local/bin/deckox-server"
assert_missing "$install_root/usr/local/share/deckox/web"
assert_missing "$install_root/usr/local/share/deckox/VERSION"
assert_file "$install_root/etc/deckox/agent.toml"
assert_file "$install_root/var/lib/deckox/admin-password.hash"
assert_file "$backup_dir/VERSION"
assert_contains "$test_dir/uninstall.out" "deckox user/group"

# Unconfirmed unit/binary paths are preserved rather than guessed to be Deckox-owned.
foreign_root="${test_dir}/foreign-root"
mkdir -p "$foreign_root/etc/systemd/system" "$foreign_root/usr/local/bin" \
  "$foreign_root/usr/local/share/deckox/web"
printf '[Service]\nExecStart=/opt/other/server\nUser=other\n' > "$foreign_root/etc/systemd/system/deckox-server.service"
printf 'foreign\n' > "$foreign_root/usr/local/bin/deckox-server"
printf '1.0.0\n' > "$foreign_root/usr/local/share/deckox/VERSION"
run_installer "$foreign_root" "$archive_v2" 1.1.0 --uninstall > "$test_dir/foreign.out" 2>&1
assert_file "$foreign_root/etc/systemd/system/deckox-server.service"
assert_file "$foreign_root/usr/local/bin/deckox-server"
assert_file "$foreign_root/usr/local/share/deckox/VERSION"
assert_contains "$test_dir/foreign.out" "Preserved unconfirmed server"

echo "Installer safety tests passed."

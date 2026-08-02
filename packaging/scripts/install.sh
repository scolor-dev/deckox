#!/bin/sh
set -eu

REPOSITORY="${DECKOX_REPOSITORY:-scolor-dev/deckox}"
INSTALLER_VERSION="0.3.8"
REQUESTED_VERSION="${DECKOX_VERSION:-latest}"
LOCAL_ARCHIVE="${DECKOX_ARCHIVE:-}"
ROOT="${DECKOX_ROOT:-}"
BASE_URL="https://github.com/${REPOSITORY}/releases"
initial_password=""
terminal_cleanup="not_installed"
mode="install"

usage() {
  echo "Usage: install.sh [--version | --dry-run | --uninstall]"
}

case "${1:-}" in
  "") ;;
  --version) mode="version" ;;
  --dry-run) mode="dry-run" ;;
  --uninstall) mode="uninstall" ;;
  -h|--help) usage; exit 0 ;;
  *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
esac
if [ "$#" -gt 1 ]; then
  echo "only one option may be specified" >&2
  exit 2
fi

if [ -n "$ROOT" ]; then
  case "$ROOT" in
    /*) ;;
    *) echo "DECKOX_ROOT must be an absolute path" >&2; exit 1 ;;
  esac
  if [ "$ROOT" = "/" ]; then
    echo "DECKOX_ROOT must not be /" >&2
    exit 1
  fi
  ROOT="${ROOT%/}"
fi

bin_dir="${ROOT}/usr/local/bin"
share_dir="${ROOT}/usr/local/share/deckox"
web_dir="${share_dir}/web"
version_file="${share_dir}/VERSION"
config_dir="${ROOT}/etc/deckox"
data_dir="${ROOT}/var/lib/deckox"
backup_root="${data_dir}/backups"
systemd_dir="${ROOT}/etc/systemd/system"
agent_unit="${systemd_dir}/deckox-agent.service"
server_unit="${systemd_dir}/deckox-server.service"
agent_binary="${bin_dir}/deckox-agent"
server_binary="${bin_dir}/deckox-server"

installed_version() {
  if [ -f "$version_file" ]; then
    awk 'NR == 1 { print; exit }' "$version_file"
  else
    echo "not installed"
  fi
}

if [ "$mode" = "version" ]; then
  echo "Deckox installer ${INSTALLER_VERSION}"
  echo "Requested package: ${REQUESTED_VERSION}"
  echo "Installed package: $(installed_version)"
  exit 0
fi

if [ "$mode" = "uninstall" ]; then
  target="not-required"
else
  machine="$(uname -m)"
  case "$machine" in
    x86_64|amd64) target="x86_64-unknown-linux-musl" ;;
    aarch64|arm64) target="aarch64-unknown-linux-musl" ;;
    *) echo "unsupported architecture: ${machine}" >&2; exit 1 ;;
  esac
fi

asset="deckox-${target}.tar.gz"
if [ "$REQUESTED_VERSION" = "latest" ]; then
  download_url="${BASE_URL}/latest/download/${asset}"
else
  download_url="${BASE_URL}/download/${REQUESTED_VERSION}/${asset}"
fi

if [ "$mode" = "dry-run" ]; then
  echo "Deckox installation dry run (no downloads or changes)"
  echo "Architecture: ${target}"
  echo "Requested package: ${REQUESTED_VERSION}"
  echo "Installed package: $(installed_version)"
  if [ -n "$LOCAL_ARCHIVE" ]; then
    echo "Source: local archive ${LOCAL_ARCHIVE}"
  else
    echo "Archive URL: ${download_url}"
    echo "Checksum URL: ${download_url}.sha256"
  fi
  echo "Managed binaries: ${server_binary}, ${agent_binary}"
  echo "Managed web files: ${web_dir}"
  echo "Managed units: ${server_unit}, ${agent_unit}"
  echo "Managed version marker: ${version_file}"
  echo "Preserved: ${config_dir}, ${data_dir}, deckox user and group"
  exit 0
fi

if [ "$(id -u)" -ne 0 ]; then
  echo "deckox installer must run as root (use sudo)" >&2
  exit 1
fi

is_managed_unit() {
  unit_file="$1"
  executable="$2"
  account="$3"
  [ -f "$unit_file" ] && [ ! -L "$unit_file" ] && awk -v executable="$executable" -v account="$account" '
    $0 == "ExecStart=" executable { executable_match = 1 }
    $0 == "User=" account { account_match = 1 }
    END { exit !(executable_match && account_match) }
  ' "$unit_file"
}

remove_managed_installation() {
  removed=""
  agent_managed=false
  server_managed=false
  if is_managed_unit "$agent_unit" /usr/local/bin/deckox-agent root; then
    agent_managed=true
  fi
  if is_managed_unit "$server_unit" /usr/local/bin/deckox-server deckox; then
    server_managed=true
  fi

  if [ "$server_managed" = true ]; then
    systemctl disable deckox-server.service >/dev/null 2>&1 || true
    systemctl stop deckox-server.service >/dev/null 2>&1 || true
    rm -f "$server_unit" "$server_binary"
    removed="${removed} server"
  elif [ -e "$server_unit" ] || [ -e "$server_binary" ]; then
    echo "Preserved unconfirmed server unit or binary." >&2
  fi
  if [ "$agent_managed" = true ]; then
    systemctl disable deckox-agent.service >/dev/null 2>&1 || true
    systemctl stop deckox-agent.service >/dev/null 2>&1 || true
    rm -f "$agent_unit" "$agent_binary"
    removed="${removed} agent"
  elif [ -e "$agent_unit" ] || [ -e "$agent_binary" ]; then
    echo "Preserved unconfirmed agent unit or binary." >&2
  fi

  if [ "$agent_managed" = true ] && [ "$server_managed" = true ]; then
    rm -rf "$web_dir"
    rm -f "$version_file"
    removed="${removed} web"
  elif [ -e "$web_dir" ] || [ -e "$version_file" ]; then
    echo "Preserved web files because the complete Deckox installation could not be confirmed." >&2
  fi
  systemctl daemon-reload
  if [ -n "$removed" ]; then
    echo "Removed Deckox managed artifacts:${removed}."
  else
    echo "No confirmed Deckox managed artifacts were found."
  fi
  echo "Preserved ${config_dir}, ${data_dir}, and the deckox user/group."
}

if [ "$mode" = "uninstall" ]; then
  for command in awk id rm systemctl uname; do
    if ! command -v "$command" >/dev/null 2>&1; then
      echo "required command not found: $command" >&2
      exit 1
    fi
  done
  remove_managed_installation
  exit 0
fi

for command in awk cp date getent groupadd groupdel id install mkdir mktemp od rm rmdir systemctl tar tr uname useradd userdel; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "required command not found: $command" >&2
    exit 1
  fi
done
if [ -z "$LOCAL_ARCHIVE" ] && ! command -v curl >/dev/null 2>&1; then
  echo "required command not found: curl" >&2
  exit 1
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
  echo "Downloading Deckox ${REQUESTED_VERSION} for ${target}..."
  curl --fail --location --proto '=https' --tlsv1.2 --output "${work_dir}/${asset}" "$download_url"
  curl --fail --location --proto '=https' --tlsv1.2 --output "${work_dir}/${asset}.sha256" "${download_url}.sha256"
fi

expected="$(awk 'NR == 1 { print $1; exit }' "${work_dir}/${asset}.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "${work_dir}/${asset}" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "${work_dir}/${asset}" | awk '{print $1}')"
else
  echo "sha256sum or shasum is required" >&2
  exit 1
fi
if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
  echo "checksum verification failed" >&2
  exit 1
fi

mkdir "${work_dir}/release"
tar -xzf "${work_dir}/${asset}" -C "${work_dir}/release"
release_dir="${work_dir}/release"
for required in VERSION bin/deckox-server bin/deckox-agent web/index.html config/server.toml config/agent.toml systemd/deckox-agent.service systemd/deckox-server.service; do
  if [ ! -f "${release_dir}/${required}" ]; then
    echo "invalid release archive: missing ${required}" >&2
    exit 1
  fi
done
package_version="$(awk 'NR == 1 { print; exit }' "${release_dir}/VERSION")"
if ! printf '%s\n' "$package_version" | awk -F. '
  NF != 3 { exit 1 }
  $1 !~ /^[0-9]+$/ || $2 !~ /^[0-9]+$/ || $3 !~ /^[0-9]+$/ { exit 1 }
'; then
  echo "invalid release archive VERSION" >&2
  exit 1
fi
if [ "$REQUESTED_VERSION" != "latest" ] && [ "${REQUESTED_VERSION#v}" != "$package_version" ]; then
  echo "release VERSION does not match requested version" >&2
  exit 1
fi

for path in "$agent_unit" "$server_unit" "$agent_binary" "$server_binary" "$web_dir" \
  "$version_file" "$config_dir" "$data_dir" "${config_dir}/server.toml" \
  "${config_dir}/agent.toml" "${config_dir}/admin-password.hash" \
  "${data_dir}/admin-password.hash"; do
  if [ -L "$path" ]; then
    echo "refusing to use symlinked installation path: ${path}" >&2
    exit 1
  fi
done

for pair in "$agent_unit|/usr/local/bin/deckox-agent|root" "$server_unit|/usr/local/bin/deckox-server|deckox"; do
  unit_file="${pair%%|*}"
  rest="${pair#*|}"
  executable="${rest%%|*}"
  account="${rest#*|}"
  if [ -e "$unit_file" ] && ! is_managed_unit "$unit_file" "$executable" "$account"; then
    echo "refusing to replace unconfirmed unit: ${unit_file}" >&2
    exit 1
  fi
done
if [ -e "$agent_binary" ] && [ ! -e "$agent_unit" ] && [ ! -f "$version_file" ]; then
  echo "refusing to replace unconfirmed binary: ${agent_binary}" >&2
  exit 1
fi
if [ -e "$server_binary" ] && [ ! -e "$server_unit" ] && [ ! -f "$version_file" ]; then
  echo "refusing to replace unconfirmed binary: ${server_binary}" >&2
  exit 1
fi

existing_count=0
for path in "$agent_unit" "$server_unit" "$agent_binary" "$server_binary" "$web_dir" "$version_file"; do
  if [ -e "$path" ]; then existing_count=$((existing_count + 1)); fi
done
if [ "$existing_count" -eq 0 ]; then
  install_kind="initial"
elif [ -f "$agent_unit" ] && [ -f "$server_unit" ] \
  && [ -f "$agent_binary" ] && [ -f "$server_binary" ] && [ -d "$web_dir" ]; then
  install_kind="update"
else
  install_kind="partial"
fi
echo "Installation type: ${install_kind}"
if [ "$install_kind" = "partial" ]; then
  echo "partial Deckox installation detected; no files were changed" >&2
  echo "restore or remove the incomplete managed artifacts before retrying" >&2
  exit 1
fi

backup_dir=""
if [ "$install_kind" != "initial" ]; then
  old_version="$(installed_version)"
  case "$old_version" in
    ''|*[!0-9.]*) old_version="unknown" ;;
  esac
  timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
  backup_dir="${backup_root}/${timestamp}-${old_version}"
  if [ -e "$backup_dir" ]; then
    echo "backup destination already exists: ${backup_dir}" >&2
    exit 1
  fi
  install -d -m 0750 "$backup_dir"
  for path in "$agent_unit" "$server_unit" "$agent_binary" "$server_binary" "$version_file"; do
    if [ -f "$path" ]; then cp -p "$path" "$backup_dir/"; fi
  done
  if [ -d "$web_dir" ]; then cp -R "$web_dir" "$backup_dir/web"; fi
  echo "Backed up managed artifacts to ${backup_dir}"
fi

install_file() {
  file_mode="$1" owner="$2" group="$3" source="$4" destination="$5"
  if [ -n "$ROOT" ]; then
    install -m "$file_mode" "$source" "$destination"
  else
    install -m "$file_mode" -o "$owner" -g "$group" "$source" "$destination"
  fi
}
install_directory() {
  directory_mode="$1" owner="$2" group="$3" destination="$4"
  if [ -n "$ROOT" ]; then
    install -d -m "$directory_mode" "$destination"
  else
    install -d -m "$directory_mode" -o "$owner" -g "$group" "$destination"
  fi
}

restore_managed_artifacts() {
  rm -f "$agent_unit" "$server_unit" "$agent_binary" "$server_binary" "$version_file"
  rm -rf "$web_dir"
  if [ -n "$backup_dir" ]; then
    if [ -f "${backup_dir}/deckox-agent.service" ]; then cp -p "${backup_dir}/deckox-agent.service" "$agent_unit"; fi
    if [ -f "${backup_dir}/deckox-server.service" ]; then cp -p "${backup_dir}/deckox-server.service" "$server_unit"; fi
    if [ -f "${backup_dir}/deckox-agent" ]; then cp -p "${backup_dir}/deckox-agent" "$agent_binary"; fi
    if [ -f "${backup_dir}/deckox-server" ]; then cp -p "${backup_dir}/deckox-server" "$server_binary"; fi
    if [ -f "${backup_dir}/VERSION" ]; then cp -p "${backup_dir}/VERSION" "$version_file"; fi
    if [ -d "${backup_dir}/web" ]; then cp -R "${backup_dir}/web" "$web_dir"; fi
  fi
  systemctl daemon-reload >/dev/null 2>&1 || true
  systemctl restart deckox-agent.service >/dev/null 2>&1 || true
  systemctl restart deckox-server.service >/dev/null 2>&1 || true
}

deployment_started=false
deployment_committed=false
password_created=false
finish() {
  status="$?"
  trap - EXIT INT TERM
  if [ "$status" -ne 0 ] && [ "$deployment_started" = true ] && [ "$deployment_committed" = false ]; then
    echo "Deckox installation failed; restoring the previous managed artifacts." >&2
    restore_managed_artifacts
    if [ "$password_created" = true ]; then rm -f "${data_dir}/admin-password.hash"; fi
  fi
  rm -rf "$work_dir"
  exit "$status"
}
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

if ! getent group deckox >/dev/null 2>&1; then groupadd --system deckox; fi
if ! id deckox >/dev/null 2>&1; then
  useradd --system --gid deckox --home-dir /var/lib/deckox --shell /usr/sbin/nologin deckox
fi

install_directory 0750 root deckox "$config_dir"
install_directory 0750 deckox deckox "$data_dir"
install_directory 0755 root root "$share_dir"
install_directory 0755 root root "$bin_dir"
deployment_started=true
rm -rf "$web_dir"
install_directory 0755 root root "$web_dir"
install_file 0755 root root "${release_dir}/bin/deckox-server" "$server_binary"
install_file 0755 root root "${release_dir}/bin/deckox-agent" "$agent_binary"
cp -R "${release_dir}/web/." "$web_dir/"
install_file 0644 root root "${release_dir}/VERSION" "$version_file"

if [ ! -f "${config_dir}/server.toml" ]; then
  install_file 0640 root deckox "${release_dir}/config/server.toml" "${config_dir}/server.toml"
fi
if [ ! -f "${config_dir}/agent.toml" ]; then
  install_file 0640 root deckox "${release_dir}/config/agent.toml" "${config_dir}/agent.toml"
fi
if [ -f "${config_dir}/admin-password.hash" ] && [ ! -f "${data_dir}/admin-password.hash" ]; then
  install_file 0600 deckox deckox "${config_dir}/admin-password.hash" "${data_dir}/admin-password.hash"
  password_created=true
fi
if [ ! -f "${data_dir}/admin-password.hash" ]; then
  initial_password="$(od -An -N 12 -tx1 /dev/urandom | tr -d ' \n')"
  printf '%s' "$initial_password" | "$server_binary" hash-password > "${work_dir}/admin-password.hash"
  install_file 0600 deckox deckox "${work_dir}/admin-password.hash" "${data_dir}/admin-password.hash"
  password_created=true
fi

install_directory 0755 root root "$systemd_dir"
install_file 0644 root root "${release_dir}/systemd/deckox-agent.service" "$agent_unit"
install_file 0644 root root "${release_dir}/systemd/deckox-server.service" "$server_unit"

if ! systemctl daemon-reload \
  || ! systemctl enable deckox-agent.service deckox-server.service \
  || ! systemctl restart deckox-agent.service \
  || ! systemctl restart deckox-server.service \
  || ! systemctl is-active --quiet deckox-agent.service \
  || ! systemctl is-active --quiet deckox-server.service; then
  echo "Deckox failed to start." >&2
  exit 1
fi
deployment_committed=true

legacy_terminal_unit="${systemd_dir}/deckox-terminal.service"
if [ -f "$legacy_terminal_unit" ] && awk '
  $0 == "ExecStart=/usr/local/bin/deckox-terminal" { executable = 1 }
  $0 == "User=deckox-terminal" { account = 1 }
  END { exit !(executable && account) }
' "$legacy_terminal_unit"; then
  systemctl disable deckox-terminal.service >/dev/null 2>&1 || true
  systemctl kill --signal=KILL --kill-who=all deckox-terminal.service >/dev/null 2>&1 || true
  systemctl stop deckox-terminal.service >/dev/null 2>&1 || true
  rm -f "$legacy_terminal_unit" "${bin_dir}/deckox-terminal"
  terminal_cleanup="service_removed"
  if [ -z "$ROOT" ] && id deckox-terminal >/dev/null 2>&1; then
    terminal_home="$(getent passwd deckox-terminal | awk -F: '{print $6}')"
    terminal_shell="$(getent passwd deckox-terminal | awk -F: '{print $7}')"
    terminal_group="$(id -gn deckox-terminal)"
    if [ "$terminal_home" = "/var/lib/deckox-terminal" ] \
      && { [ "$terminal_shell" = "/usr/sbin/nologin" ] || [ "$terminal_shell" = "/sbin/nologin" ]; } \
      && [ "$terminal_group" = "deckox-terminal" ]; then
      if userdel deckox-terminal; then
        terminal_cleanup="account_removed"
        if rmdir /var/lib/deckox-terminal 2>/dev/null; then terminal_cleanup="all_removed"; fi
        if getent group deckox-terminal >/dev/null 2>&1; then groupdel deckox-terminal || true; fi
      fi
    fi
  fi
  systemctl daemon-reload || echo "Warning: systemd reload failed after legacy cleanup." >&2
fi

echo
echo "Deckox ${package_version} has been installed (${install_kind})."
echo "Deckox is listening on http://127.0.0.1:8080/"
if [ -n "$initial_password" ]; then
  echo "Initial administrator password: ${initial_password}"
  echo "Store this password now. It is not shown again."
else
  echo "The existing administrator password was preserved."
fi
echo "For remote access, use an SSH tunnel or a trusted LAN."
if [ "$terminal_cleanup" != "not_installed" ]; then
  echo "The obsolete web terminal was removed (${terminal_cleanup})."
fi
echo "Check status with: systemctl status deckox-server deckox-agent"

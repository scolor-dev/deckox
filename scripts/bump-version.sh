#!/bin/sh
set -eu

new_version=${1:-}
if ! printf '%s\n' "$new_version" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Usage: scripts/bump-version.sh vX.Y.Z" >&2
  exit 2
fi

new_number=${new_version#v}
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
cd "$repo_root"

fail() {
  echo "bump-version: $*" >&2
  exit 1
}

# Portable in-place sed: GNU sed and BSD sed both accept a suffix glued to
# -i, but only GNU sed also accepts a bare -i with no suffix.
replace_in_file() {
  file=$1
  pattern=$2
  [ -f "$file" ] || fail "expected file not found: $file"
  sed -i.bak -e "$pattern" "$file"
  rm -f "$file.bak"
}

command -v git >/dev/null 2>&1 || fail "git is required"
command -v cargo >/dev/null 2>&1 || fail "cargo is required"
command -v npm >/dev/null 2>&1 || fail "npm is required"

[ "$(git branch --show-current)" = "develop" ] || fail "start from the develop branch"
[ -z "$(git status --porcelain)" ] || fail "the worktree is not clean"

old_number=$(awk -F '"' '/^version = "/ { print $2; exit }' Cargo.toml)
[ -n "$old_number" ] || fail "could not read the current version from Cargo.toml"
[ "$old_number" != "$new_number" ] || fail "already at $new_number"
old_version="v$old_number"

echo "bump-version: $old_version -> $new_version"

# Cargo.toml / Cargo.lock. Cargo.lock is regenerated via cargo, never
# text-replaced: it also pins unrelated third-party crates that can
# coincidentally share the same version string as this project.
replace_in_file Cargo.toml "s/^version = \"$old_number\"\$/version = \"$new_number\"/"
cargo check --workspace --quiet

# apps/web/package.json / package-lock.json. Same reasoning as Cargo.lock:
# package-lock.json also pins third-party packages, so only npm itself is
# allowed to touch it (and only the top-level version fields change, since
# no dependency changed).
replace_in_file apps/web/package.json "s/^  \"version\": \"$old_number\",\$/  \"version\": \"$new_number\",/"
(cd apps/web && npm install --package-lock-only --silent)

# README.md and docs/*.html/installation commands: the release tag always
# appears as `v$old_number`, which is specific enough to replace safely.
replace_in_file README.md "s/v$old_number/v$new_number/g"
replace_in_file docs/installation.html "s/v$old_number/v$new_number/g"

# docs/*.html sidebar footer, present in every page under docs/.
for doc in docs/*.html; do
  replace_in_file "$doc" "s/<br>Version $old_number<\/div>/<br>Version $new_number<\/div>/"
done

replace_in_file packaging/scripts/install.sh "s/INSTALLER_VERSION=\"$old_number\"/INSTALLER_VERSION=\"$new_number\"/"
replace_in_file scripts/test-installer.sh "s/Deckox installer $old_number/Deckox installer $new_number/"

echo "bump-version: checking for leftover references to $old_number"
leftover=$(git grep -n -F -- "$old_number" -- . ':!Cargo.lock' ':!apps/web/package-lock.json' 2>/dev/null || true)
if [ -n "$leftover" ]; then
  echo "$leftover" >&2
  fail "found references to $old_number above that this script does not know how to update; extend the script, then re-run"
fi

echo "bump-version: running verification"
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- \
  -D warnings -W clippy::pedantic -W clippy::nursery
cargo test --workspace
sh scripts/test-installer.sh
(
  cd apps/web
  npm ci
  npm run lint
  npm run typecheck
  npm run test
  npm run build
)

git add \
  Cargo.toml Cargo.lock \
  apps/web/package.json apps/web/package-lock.json \
  README.md docs/*.html \
  packaging/scripts/install.sh scripts/test-installer.sh
git commit -m "chore: ${new_version}への更新に向けてバージョンを更新"

echo "bump-version: committed the $new_version version bump on develop"

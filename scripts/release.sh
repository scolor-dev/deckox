#!/bin/sh
set -eu

version=${1:-}
if ! printf '%s\n' "$version" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Usage: scripts/release.sh vX.Y.Z" >&2
  exit 2
fi

release_number=${version#v}
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
cd "$repo_root"

fail() {
  echo "release: $*" >&2
  exit 1
}

wait_for_run() {
  workflow=$1
  commit=$2
  branch=${3:-}
  attempt=0
  run_id=

  while [ "$attempt" -lt 30 ]; do
    if [ -n "$branch" ]; then
      run_id=$(gh run list \
        --workflow "$workflow" \
        --branch "$branch" \
        --commit "$commit" \
        --limit 1 \
        --json databaseId \
        --jq '.[0].databaseId // empty')
    else
      run_id=$(gh run list \
        --workflow "$workflow" \
        --commit "$commit" \
        --limit 1 \
        --json databaseId \
        --jq '.[0].databaseId // empty')
    fi

    if [ -n "$run_id" ]; then
      gh run watch "$run_id" --exit-status
      return
    fi

    attempt=$((attempt + 1))
    sleep 2
  done

  fail "$workflow workflow did not start for $commit"
}

command -v git >/dev/null 2>&1 || fail "git is required"
command -v gh >/dev/null 2>&1 || fail "GitHub CLI is required"
gh auth status >/dev/null 2>&1 || fail "GitHub CLI is not authenticated"

[ "$(git branch --show-current)" = "develop" ] || fail "start from the develop branch"
[ -z "$(git status --porcelain)" ] || fail "the worktree is not clean"

manifest_version=$(awk -F '"' '/^version = "/ { print $2; exit }' Cargo.toml)
[ "$manifest_version" = "$release_number" ] || \
  fail "Cargo.toml version is $manifest_version, expected $release_number"

echo "release: fetching branches and tags"
git fetch origin main develop --tags

[ "$(git rev-parse develop)" = "$(git rev-parse origin/develop)" ] || \
  fail "local develop is not synchronized with origin/develop"
[ "$(git rev-parse main)" = "$(git rev-parse origin/main)" ] || \
  fail "local main is not synchronized with origin/main"

if git show-ref --verify --quiet "refs/tags/$version"; then
  fail "tag $version already exists"
fi

develop_sha=$(git rev-parse develop)
ci_state=$(gh run list \
  --workflow CI \
  --branch develop \
  --commit "$develop_sha" \
  --limit 1 \
  --json status,conclusion \
  --jq '.[0] | "\(.status) \(.conclusion)"')
[ "$ci_state" = "completed success" ] || \
  fail "develop CI for $develop_sha is not successful (state: ${ci_state:-missing})"

echo "release: publishing $version from verified develop commit $develop_sha"
git switch main
git merge --squash develop
git diff --cached --check

staged_tree=$(git write-tree)
develop_tree=$(git rev-parse 'develop^{tree}')
[ "$staged_tree" = "$develop_tree" ] || fail "squashed main tree differs from develop"

git commit -m "chore: release Deckox $version"
main_sha=$(git rev-parse HEAD)
git push origin main

echo "release: waiting for main CI"
wait_for_run CI "$main_sha" main

git tag -a "$version" -m "Deckox $version"
git push origin "$version"

echo "release: waiting for cross-architecture release artifacts"
wait_for_run Release "$main_sha"

assets=$(gh release view "$version" --json assets --jq '.assets[].name')
for expected in \
  "deckox-x86_64-unknown-linux-musl.tar.gz" \
  "deckox-x86_64-unknown-linux-musl.tar.gz.sha256" \
  "deckox-aarch64-unknown-linux-musl.tar.gz" \
  "deckox-aarch64-unknown-linux-musl.tar.gz.sha256"
do
  printf '%s\n' "$assets" | awk -v expected="$expected" \
    '$0 == expected { found = 1 } END { exit !found }' || \
    fail "GitHub Release is missing $expected"
done

echo "release: synchronizing release history back to develop"
git switch develop
git merge --no-ff main -m "chore: mainの${version}リリース履歴を同期"
[ "$(git rev-parse 'develop^{tree}')" = "$(git rev-parse 'main^{tree}')" ] || \
  fail "develop tree differs from the released main tree"
git push origin develop

release_url=$(gh release view "$version" --json url --jq '.url')
echo "release: completed $version"
echo "release: $release_url"

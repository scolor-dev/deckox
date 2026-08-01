# Deckox development instructions

## Repository conventions

- Work on `develop`. Keep `main` as release-only history with one squash commit per version.
- Write conventional commit subjects in Japanese on `develop`.
- Write release commit subjects on `main` in English: `chore: release Deckox vX.Y.Z`.
- Keep Rust warnings clean under the strict Clippy configuration used by CI.
- Keep Vue lint, type checking, tests, and production build clean.
- Document only implemented behavior. Documentation belongs in `docs/` as HTML; use SVG for diagrams.
- Support both `x86_64` and `aarch64` Linux release artifacts.

## Standard release workflow

When the user explicitly asks to deploy, release to `main`, or publish the current version:

1. Determine the version from the repository and confirm the worktree is clean on `develop`.
2. Run `scripts/release.sh vX.Y.Z` instead of reconstructing the Git/GitHub procedure manually.
3. Do not rerun local checks when the exact `origin/develop` commit already has a successful `CI` workflow run. The release script enforces this condition.
4. The script must stop on any failed CI or release workflow. Diagnose the failure; do not bypass it.
5. Do not replace or rewrite an existing tag or GitHub Release.
6. After completion, remain on `develop` and report the release URL and the Raspberry Pi update command.

The release script performs the fixed sequence: fetch and preflight checks, squash `develop` into `main`, wait for `main` CI, create and push an annotated tag, wait for the Release workflow, verify all four architecture archives/checksums, and merge the release commit back into `develop`. It intentionally does not wait for the final `develop` synchronization CI because that tree has already passed `main` CI.

# Deckox development instructions

## Repository conventions

- Work on `develop`. Keep `main` as release-only history with one squash commit per version.
- Write conventional commit subjects in Japanese on `develop`.
- Write release commit subjects on `main` in English: `chore: release Deckox vX.Y.Z`.
- Keep Rust warnings clean under the strict Clippy configuration used by CI.
- Keep Vue lint, type checking, tests, and production build clean.
- Document only implemented behavior. Documentation belongs in `docs/` as HTML; use SVG for diagrams.
- Support both `x86_64` and `aarch64` Linux release artifacts.

## Design system

The Vue design system lives in `apps/web/src/design-system/`. Read `README.md` there first; each component has its own `.md` next to its `.vue`.

- Build UI from the components exported by `design-system/components/index.ts`. Pick the component with the "Which component do I use?" table and read its `.md` (API, accessibility, gotchas) before writing markup. Do not hand-write a button, dialog, table, badge or form field that a component already covers.
- Never write color, spacing, radius, font-size, shadow, z-index or duration literals in Vue or CSS. Use the tokens in `design-system/tokens.css`. A new color token must be defined in all three layers (light `:root`, the `prefers-color-scheme` block and `:root[data-theme="dark"]`) and must pass `contrast.test.ts`.
- If a component lacks something, extend the component (props, `.md`, tests) instead of adding screen-specific CSS around it.
- Component rules enforced by tests: no comments in component `.vue` files, only defined tokens, `.md` in sync with props/events/slots/tokens, every export in `components/index.ts`, breakpoints only through `breakpoints.ts`, `prefers-reduced-motion` handling for anything that animates, own `:focus-visible` styles for anything interactive, and no axe violations in the catalog.
- Keyboard and focus behavior (focus trap in dialogs, arrow keys in tabs and menus) is part of the contract; changing it needs a matching test in `keyboard.test.ts`.
- Preview components with `npm run dev` at `/design-system.html`. `npm run build:catalog` writes the single-file catalog to `apps/web/dist-catalog/` for publishing as an Artifact.

## Version bump

Before releasing, the version recorded in the repository (`Cargo.toml`, etc.) must already match the target tag; `scripts/release.sh` refuses otherwise.

1. Run `scripts/bump-version.sh vX.Y.Z` on a clean `develop` instead of editing version strings by hand.
2. The script updates `Cargo.toml`/`Cargo.lock` via `cargo check`, `apps/web/package.json`/`package-lock.json` via `npm install --package-lock-only`, and the `vOLD` / `Version OLD` references in `README.md`, `docs/*.html`, `packaging/scripts/install.sh`, and `scripts/test-installer.sh`. Lockfiles are never text-replaced, because they also pin unrelated third-party packages that can coincidentally share the same version string.
3. It then greps the whole tree (excluding the lockfiles) for the old version number and fails loudly if anything is left over — a sign a new file needs adding to the script. Extend the script's replacement list when that happens; do not hand-edit around it just once.
4. It runs the full verification suite (the same commands as `## 検証` in `README.md`) and creates one commit (`chore: vX.Y.Zへの更新に向けてバージョンを更新`) only if everything passes. It does not push.

## Standard release workflow

When the user explicitly asks to deploy, release to `main`, or publish the current version:

1. Determine the version from the repository and confirm the worktree is clean on `develop`. Run `scripts/bump-version.sh vX.Y.Z` first if the version has not been bumped yet.
2. Run `scripts/release.sh vX.Y.Z` instead of reconstructing the Git/GitHub procedure manually.
3. Do not rerun local checks when the exact `origin/develop` commit already has a successful `CI` workflow run. The release script enforces this condition.
4. The script must stop on any failed CI or release workflow. Diagnose the failure; do not bypass it.
5. Do not replace or rewrite an existing tag or GitHub Release.
6. After completion, remain on `develop` and report the release URL and the Raspberry Pi update command.

The release script performs the fixed sequence: fetch and preflight checks, squash `develop` into `main`, wait for `main` CI, create and push an annotated tag, wait for the Release workflow, verify all four architecture archives/checksums, and merge the release commit back into `develop`. It intentionally does not wait for the final `develop` synchronization CI because that tree has already passed `main` CI.

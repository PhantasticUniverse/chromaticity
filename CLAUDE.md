<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# CLAUDE.md

Project memory for Claude Code and other coding agents. Everything below is non-obvious
context that a code-reader cannot derive from the tree. Read it on turn one; keep it short
(context budget).

## Project at a glance

Chromaticity is a deterministic, audio-reactive live-visuals runtime and authoring
environment (macOS + Windows desktop). Toolkit: Tauri 2 + Svelte 5 + TypeScript on the UI
side; Rust 2024 edition on the engine side. `docs/adr/ADR-018-non-scene-ui-toolkit.md` is
the normative toolkit decision. Phase 1 entered April 2026; engine crates (runtime, scene,
audio, integrations) arrive in subsequent PRs.

Authoritative planning corpus lives under `docs/`. Two documents to read first:
`docs/43-development-standards.md` (standards) and
`docs/29-cleanroom-legal-policy-baseline.md` (clean-room SOP).

## Fast commands

Frontend, from repo root:

```
npm ci                     # once, after clone
npm run format:check       # prettier
npm run lint               # eslint .    (config is authoritative scope)
npm run tsc                # tsc --noEmit (app + tsconfig.node.json)
npm run check              # svelte-check
npm run dev                # vite dev server on localhost:1420
```

Rust, from repo root:

```
cargo check   --workspace --all-targets
cargo clippy  --workspace --all-targets -- -D warnings
cargo test    --workspace --all-targets --no-fail-fast
cargo fmt --all -- --check
cargo deny   check {advisories,licenses,bans,sources}   # needs cargo-deny
cargo hakari generate --diff                            # needs cargo-hakari
```

Policy / evidence:

```
bash scripts/evidence-tag-lint.sh    # Phase-1 stub; passes trivially
bash scripts/adr-lint.sh             # 17 Phase-0 grace warnings expected
bash scripts/trailer-check.sh HEAD   # DCO + Profile-Impact trailers
npm run commitlint:last              # Conventional Commits check
```

The cargo target directory is shared at the workspace root `/target/`; there is no
per-crate `target/` to clean.

## Workspace layout

```
chromaticity/
├── app/
│   └── shell/            # Tauri 2 binary (chromaticity-app crate)
├── engine/               # reserved: runtime/, scene/, audio/
├── integrations/         # reserved: adapter crates
├── release-eng/          # reserved: signing / notarization / updater
├── third_party/          # reserved: vendored-with-attribution
├── workspace-hack/       # cargo-hakari placeholder (at root by convention)
├── src/                  # Svelte web root; may migrate under app/ui/ later
├── dist/                 # Vite build output — .gitkeep is load-bearing
├── docs/                 # normative planning corpus + ADRs
├── scripts/              # policy / CI shell helpers
├── .github/workflows/    # rust, frontend, sbom, evidence-tag-lint, policy
├── Cargo.toml            # workspace root (edition 2024, MSRV 1.85.0)
└── package.json          # @chromaticity/app
```

`Contents/` (clean-room reference material) and `chromaticity-planning/` (planning
archive) live on disk but are `.gitignored`, `.prettierignored`, and excluded from ESLint.
Source tooling MUST NOT parse them.

## Commits — every commit carries two mandatory trailers

Conventional Commits with Chromaticity vocabulary. Every commit ends with:

```
Signed-off-by: <Name> <email>
Profile-Impact: none | compat-syn-v1-{minor,major} | native-chroma-v1-{minor,major}
```

Missing trailers fail `scripts/trailer-check.sh` and block merge. `compat-syn-*` = impact
on the Synesthesia-compat API surface; `native-chroma-*` = impact on the
Chromaticity-native surface; `none` = no user-visible API / behavior change.

**Types** (commitlint-enforced): `feat` `fix` `perf` `refactor` `docs` `test` `build` `ci`
`chore` `revert`.

**Scopes** (commitlint-enforced; additions require a `policy`-scoped PR editing
`commitlint.config.cjs`): `runtime` `scene` `audio` `integration` `authoring` `library`
`ui` `docs` `ux` `entitlement` `observability` `release-eng` `policy` `repo`.

**Shape:** header ≤ 100 chars; subject ≤ 72 chars, lower-case, no trailing period;
body / footer ≤ 100 chars per line. Scope is mandatory on every commit.

## Unique conventions and constraints

**Rust lints (workspace-wide):**

- `unsafe_code = "forbid"` at the workspace root.
- `clippy::unwrap_used = "deny"`, `clippy::expect_used = "deny"`. Tests opt out with
  `#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]` at module level.
- MSRV pinned to 1.85.0 in `rust-toolchain.toml`. Bumping needs a `policy`-scoped PR.

**TypeScript / Svelte:**

- `@typescript-eslint/no-explicit-any: "error"`.
- `tsconfig.json` sets `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`,
  `verbatimModuleSyntax`, and `moduleResolution: "bundler"`.
- Svelte 5 runes mode (`compilerOptions.runes: true`). Use `$state`, `$derived`, `$effect` —
  not Svelte 4 stores.

**REUSE 3.2:**

- Every new source file carries inline `SPDX-License-Identifier` and
  `SPDX-FileCopyrightText` headers. Binary / JSON / markdown files use HTML-comment or
  path-rule coverage via `REUSE.toml`.
- First-party Chromaticity code: `LicenseRef-Chromaticity-Proprietary`. Docs under `docs/`
  are `CC-BY-4.0` (override rule in `REUSE.toml`).

**Lefthook:**

- Requires `evilmartians/lefthook` v2.1.6 or later. `brew install lefthook` ships the
  right package. Do NOT use the npm package `@arkweid/lefthook` — it is a separate
  abandoned project whose v0.7.7 binary blocks commits against modern `lefthook.yml`
  syntax.
- `--no-verify` is for emergencies only. Fix the underlying issue instead.

## Never touch without explicit direction

- `Contents/` — clean-room reference material; source tooling must not parse it (see
  `docs/29-cleanroom-legal-policy-baseline.md`).
- `chromaticity-planning/` — archive-only planning corpus.
- `LICENSES/` — canonical third-party license text; preserve upstream formatting.
- `docs/adr/*` and `docs/29-cleanroom-legal-policy-baseline.md` — non-delegable; AO-only
  approval.
- Anything under `**/*.synScene` or `Contents/Resources/release/`.

## Load-bearing files that look optional

- `dist/.gitkeep` — Tauri's `generate_context!()` macro expands at `cargo check` time and
  reads `frontendDist` via `build.rs`. A missing `dist/` triggers a compile-time panic.
  `.gitignore` tracks only the sentinel and ignores everything else inside.
- `app/shell/icons/icon.png` — Tauri config requires a window icon even when
  `bundle.active = false`. The 93-byte transparent placeholder is intentional.
- `Cargo.lock` — tracked (required for reproducible builds).
- `rust-toolchain.toml` — honored by CI and local builds. Do not override with
  `+stable` / `+nightly` without a work-package requirement.

## Anti-patterns we already learned

- Don't add features, refactor, or introduce abstractions beyond what the task requires.
  Three similar lines beats a premature abstraction. No half-finished implementations.
- Don't run `npx eslint .` through tooling that bypasses `eslint.config.js`. The config's
  `ignores` protect against the `Contents/` 23k-error landmine; third-party integrations
  that don't load the config will blow up.
- Don't bypass lefthook with `--no-verify`; fix the underlying issue.
- Don't use `.expect()` / `.unwrap()` in non-test Rust code — clippy will deny.
- Don't widen clippy or ESLint rules without a `policy`-scoped PR.
- Don't update `CODEOWNERS` outside a `policy`-scoped commit.
- Don't touch `docs/adr/ADR-*.md` without AO sign-off.
- Don't hand-edit `CHANGELOG.md` — `release-please` populates per-release sections on
  merges to `main`.

## GitHub operations — known gotchas

Surfaced landing the first protected-branch PR in this repo; none are obvious from the
tree.

- **Bare hash-number references in commit bodies break commitlint.** Patterns like
  `PR #2` or `fixes #17` are treated by `conventional-commits-parser` as GitHub
  references, and the parser then mis-identifies the footer boundary and fails
  `footer-leading-blank`. Write "the present PR", "PR number 2", or "this PR" in body
  prose instead. Subject lines are unaffected.
- **`gh pr edit --title` may silently no-op** due to a GraphQL Projects-classic
  deprecation error — the command returns zero plus a warning but the title does not
  change. Use
  `gh api -X PATCH repos/OWNER/REPO/pulls/<N> -f title='<new title>'` instead.
- **`pull_request` workflows do not retrigger on title / body edits by default** — the
  trigger types are `[opened, synchronize, reopened]`. To re-run CI after a title edit
  (e.g., to clear a commitlint title failure), close + reopen the PR:
  `gh pr close <N>; gh pr reopen <N>`. `gh run rerun` does not work here — it replays
  the cached event payload with the old title.
- **On `pull_request` events, `HEAD` in a checked-out workflow is the synthetic merge
  commit** at `refs/pull/<N>/merge`, which carries GitHub's auto-generated message with
  no DCO / Profile-Impact trailers. Any CI step that runs against `HEAD` misses the PR's
  actual commits. Walk `$BASE_SHA..$HEAD_SHA` via `git rev-list` instead. The
  `trailer-check` job was fixed after this surfaced on the first PR.
- **`dorny/paths-filter@v3` needs `pull-requests: read` on the workflow token.** On
  `pull_request` events the action calls
  `GET /repos/{owner}/{repo}/pulls/{number}/files` to enumerate changed paths; with
  only the default `contents: read`, it fails with "Resource not accessible by
  integration" and silently degrades to "no matches" on fork PRs. Grant
  `pull-requests: read` workflow-wide alongside `contents: read` whenever a workflow
  uses `dorny/paths-filter` (or any action that hits the pulls REST API) in an in-job
  filter. The three per-workflow `*-summary` aggregators in `rust.yml` /
  `frontend.yml` / `sbom.yml` depend on this scope.

## Where authoritative answers live

- **Dev standards**: `docs/43-development-standards.md`.
- **Evidence registry**: `docs/26-evidence-artifact-registry.md`.
- **ADRs**: `docs/adr/` (ADR-018 for UI stack, ADR-014 for release signing, ADR-013 for
  network defaults, ADR-011 for rendering backend).
- **Phase-0 placeholder inventory**: `docs/00-phase-0-placeholder-inventory.md`.
- **Risk register**: `docs/13-risk-register.md`.
- **Clean-room / legal baseline**: `docs/29-cleanroom-legal-policy-baseline.md`.
- **Scope-vocabulary source of truth**: `commitlint.config.cjs`.
- **CI tree**: `.github/workflows/` (rust, frontend, sbom, evidence-tag-lint, policy).

## When this file is wrong

Do not trust memory over the tree. If a command, path, or invariant here disagrees with
the code or git log, the tree wins — edit this file in a follow-up `docs(repo)` commit.
Stale project memory is worse than none.

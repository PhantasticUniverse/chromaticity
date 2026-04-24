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
the normative toolkit decision. Phase 1 entered April 2026; engine and integration crates
(runtime, scene, audio, observability, osc-adapter) are scaffolded and receiving
substantive implementation in successive slices.

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
bash scripts/evidence-tag-lint.sh    # strict on tier tokens, evidence-tag coverage (engine/**, integrations/**), and EV-ID validity against docs/26
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
├── engine/               # runtime/, scene/, audio/, observability/
├── integrations/         # osc-adapter/ (more adapter crates to follow)
├── release-eng/          # reserved: signing / notarization / updater
├── third_party/          # reserved: vendored-with-attribution
├── workspace-hack/       # cargo-hakari managed crate (at root by convention)
├── .config/hakari.toml   # cargo-hakari config; platforms mirror deny.toml graph.targets
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
- **Merge queue is NOT active (plan-gated).** The five required-check workflows
  (`rust.yml`, `sbom.yml`, `frontend.yml`, `policy.yml`, `evidence-tag-lint.yml`)
  already trigger on `merge_group` and plumb `merge_group.base_sha` /
  `merge_group.head_sha` through the range-walking jobs, so they're
  forward-compatible. The ruleset that would actually enable the queue
  (`merge_queue` rule type on `refs/heads/main`) is rejected by the API
  because merge queue on private repositories requires GitHub Team or
  Enterprise Cloud; the current plan (Pro) does not include it. Consequences
  for today:
  - Merges on `main` still go through the normal squash-merge path under
    branch protection's `strict: true` + `required_linear_history: true`.
    Slice-style parallel lanes still pay the O(N²) rebase cascade.
  - Workflow surface is prepared: if the plan is ever upgraded to Team or
    higher, creating the ruleset via
    `POST /repos/.../rulesets` (with a `merge_queue` rule targeting
    `refs/heads/main`, `merge_method=SQUASH`, `grouping_strategy=ALLGREEN`)
    is the only remaining step; no workflow changes needed at that point.
  - The parallel-dep-adder recipe below remains the manual-rebase path that
    the queue would otherwise absorb.
    Compat patterns already in place for eventual queue activation:
  - Range-walking jobs (`dco`, `trailer-check`, `commit-convention-lint`'s
    PR-range step) read base/head from the null-coalescing expression
    `${{ github.event.pull_request.base.sha || github.event.merge_group.base_sha }}`
    so the same job body works for both event types.
  - `dorny/paths-filter` jobs gate the filter on
    `github.event_name == 'pull_request'` and combine with a "non-PR bypass"
    step that forces `output=true` on push or merge_group events.
  - `commit-convention-lint`'s `Lint head commit subject` step handles
    `push` and `merge_group` events by linting subject only, not body or
    footer — PR-body-composed squash commits contain markdown that
    regularly violates body/footer length rules written for hand-authored
    commit messages.
- **Squash-merge commits compose from the PR title + PR body, NOT from concatenated
  commit messages.** The repo setting (Settings → General → Squash merging) is
  "Default to pull request title and description". Consequences for PR authors:
  - The PR body becomes the main-side commit body verbatim.
    `scripts/trailer-check.sh` validates `Signed-off-by` and `Profile-Impact`
    on the main-side commit, so both trailers MUST live at the end of the PR
    body (the `.github/pull_request_template.md` stub seeds them).
  - `git interpret-trailers` only recognises a trailer block as the LAST
    paragraph. Do not add prose after the Trailers section of a PR body; any
    post-trailer content terminates the trailer zone and re-breaks
    `trailer-check`.
  - Branch commits are no longer constrained to single-commit-per-PR — the
    old `---------` divider failure mode only arose when the squash template
    concatenated commit messages, which is no longer the case. Branches can
    carry however many commits are useful for review; only the merge commit's
    shape matters for trailer-check.
  - The branch-level commits themselves still need `Signed-off-by` (the `dco`
    required-context validates every commit in the PR range, not just the
    merge commit). Lefthook's pre-push `trailer-check` validates the branch
    HEAD commit message.
- **`gh pr update-branch` breaks DCO and `trailer-check`.** The command asks GitHub to
  update the PR branch from its base, which creates a SERVER-side merge commit signed
  only by GitHub (`GitHub <noreply@github.com>`, no `Signed-off-by`, no
  `Profile-Impact`). That commit fails the `dco` and `trailer-check` required contexts
  on the next CI run and blocks merge. When a laggard PR goes out of date under
  `strict: true` branch protection, use `git rebase origin/main` in a local worktree
  followed by `git push --force-with-lease=<branch>:<old-sha>`; never the GitHub web
  button, never `gh pr update-branch`. Applies equally to the "Update branch" button
  in the PR UI.

## Agent parallelism playbook

Use sub-agents in parallel when work bundles are branch-independent; keep serial
execution when they share choke points. Cheap rule set, derived from the PR #4 → #5 → #6
re-harden sequence post-mortem:

**Parallel-safe** (spawn one sub-agent per bundle, each on its own branch off latest
`main`):

- Per-subtree implementation work under a single CODEOWNERS entry (`engine/<x>/`,
  `app/<x>/`, `integrations/<x>/`). Each subtree's PR has a self-contained diff.
- Per-subtree dep-adding work — introducing new external crates in parallel subtree
  PRs is viable under strict-true branch protection. Each lane commits its own
  `cargo hakari generate` output as part of its squash-clean PR. When a sibling
  merges first, remaining lanes rebase onto advanced `main`; if the rebase
  conflicts on `workspace-hack/Cargo.toml` or `Cargo.lock`, resolve with either
  `git checkout --ours` or `--theirs` (both converge — `cargo` re-resolves the
  lockfile against the combined manifests on the next `cargo metadata` call)
  and re-run `cargo hakari generate`. Hakari emits the unified superset;
  `cargo hakari manage-deps --dry-run` reporting "no operations to perform" is
  the settlement signal. Slice-2 validated this with three parallel dep-adders
  (serde, cpal, rosc) plus one dep-free lane. The older rule ("dep-adds must
  be serialized") proved too strict. Merge queue, if ever enabled at the plan
  level, would absorb this rebase pass automatically; see the GitHub-
  operations section for the queue-activation follow-up.
- Independent ADR drafts in `docs/adr/` (one ADR per agent; the AO-only approval
  constraint still applies at merge time).
- Independent evidence-tag-producing test suites, once the crate they live in exists.
- Context gathering, CI watchers, read-only searches. Always parallelize these.

**Serial only** (single-owner, rebase-then-push; no sub-agent fan-out):

- Any edit to `Cargo.toml [workspace] members`, `commitlint.config.cjs`, `deny.toml`,
  `CODEOWNERS`, `REUSE.toml`, `.github/workflows/**`,
  `docs/26-evidence-artifact-registry.md`, or `docs/00-phase-0-placeholder-inventory.md`.
  These files are global choke points; concurrent edits produce either text-conflict
  merges or semantic conflicts the linter won't catch.
- Any PR whose merge-state is a prerequisite for another PR's CI to be meaningful —
  e.g. the PR #4 → #5 → #6 re-harden sequence (each PR needed the previous branch-
  protection transition), or the PR-A workspace-members scaffold before parallel
  PR-B..E per-crate implementations.
- Anything touching branch-protection required contexts, squash vs merge-commit policy,
  or GitHub repo settings (fork policy, default branch, signed-commit enforcement).

**Spawning rule:** if a bundle touches _any_ file on the serial list, do that work
first in the main context, land it, then fan out sub-agents for the parallel follow-
ups. Never let a sub-agent own both a serial-list touch and a parallel-safe subtree in
the same PR — split the bundle.

**Branch hygiene after sub-agent fan-out:** each sub-agent rebases onto latest `main`
before push, not onto a sibling sub-agent's branch. Sibling branches are only visible
via origin after their respective merges; local stacking produces dependency chains
that the squash-merge workflow silently breaks.

## Adjudicating briefs against the tree

Briefs describing concrete file paths, test tiers, or slice scope sometimes conflict
with in-source invariants or normative docs. Default posture: **the tree wins**, same
as the "When this file is wrong" rule at the bottom of this doc. Three heuristics,
surfaced from the PR #47 `EV-COMPAT-FIXTURE-COVERAGE` scope-correction:

- **Crate-private visibility is a slice-boundary signal, not friction to route around.**
  `pub(crate)` on a schema type whose module-level prose documents a slice-N widening
  trigger means the briefer cannot route around it with a `serde_json` dev-dep,
  `#[doc(hidden)]` test constructors, or struct-literal fixtures in a separate
  integration-test crate. The visibility invariant IS the slice scope declaration.
  Example: PR #50's `engine/scene/src/schema.rs` module-level prose makes this
  load-bearing explicit — "every field is `pub` (the slice-2 `pub(crate)` visibility
  was the slice-boundary signal this slice realizes)" — confirming the slice-2
  visibility was the boundary marker that the slice-3a deserializer honored when it
  opened the schema. A brief proposing external construction in slice-2 was the
  weaker document and the work correctly relocated in-crate at T1.
- **Docstring invariants in schema / contract files are normative.** Module-level
  prose in files whose job is to declare a contract (`engine/*/src/schema.rs`,
  `docs/26`, `docs/34`, `docs/43`) is the slice boundary encoded in-source. When a
  brief conflicts with an in-source invariant that cites ADRs or slice scope, the
  brief is the weaker document.
- **Re-derive test tier from the registry `Generation Source` column, not the brief.**
  A brief's `T<n>` label is inherited data; the canonical axis is `docs/26` row
  `Generation Source` (e.g. row 23's "Scene import harness + golden corpus fixtures"
  as the T2 target for `EV-SCENE-IMPORT`) read against `docs/43` §15.5 T-tier
  definitions. A tier-mislabel produces a test module that can't satisfy its row — a
  rollup inconsistency `evidence-tag-lint` surfaces much later than the PR that
  introduced it.

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

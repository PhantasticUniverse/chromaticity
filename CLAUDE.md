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
the normative toolkit decision. Phase 1 entered April 2026 and EXIT was achieved at
W12 (commit `e818752`); Phase 2 EXIT was achieved 2026-05-01 at commit `d8e92db`;
**Phase 3 EXIT was achieved 2026-05-01 at commit `953fd74`** per the Phase-3 EXIT
simulation closing sign-off in `docs/EV-GATE-MODEL-AUDIT-checklist.md` — seven of the eight
Phase-3 EXIT evidence IDs (`EV-AUDIO-TRACE` re-validated cross-tag, `EV-AUDIO-ROUTING-MATRIX`,
`EV-STATE-REPLAY`, `EV-STATE-MIGRATION`, `EV-PRESET-PLAYLIST`, `EV-SHOW-BUNDLE-RECOVERY`,
`EV-PERF-BUDGETS`) transitioned `Planned` → `Pass` against six per-row anchor run ids
(25204946481, 25203436497, 25207466995, 25205805621, 25208208038, 25208934213) plus the
Phase-2 cross-tag for row 30; the eighth row `EV-COLOR-MANAGEMENT-SRGB` (row 38) is
explicitly deferred per AO direction sequenced behind a future render-HUD-diagnostics PR.
Phase-3 narrowing decisions: row 33 forward-migration only (rollback Phase-4+); row 34
existing PresetStore + PlaylistStore semantics (preset-bank durability + scene-author
preset capture + playlist reordering Phase-4+); row 35 bundle-format spec only (recovery /
rollback semantics Phase-4+); row 36 observable-subsystem proxy benchmarks (true frame-loop
budget surface deferred until ADR-001/ADR-006 slice-3 lands the scheduler-tick path; Path B
nightly-only criterion harness in `engine/runtime/benches/budgets.rs` per the
~$20/month-vs-~$70-80/month AO-approved cost posture). Phase-4a Wave 1a partial closure
landed 2026-05-01 in a 5-PR session (slice-3 PR 1 wave-PR `#321` introducing the
foundational `Scheduler` + `Scheduler::tick` per ADR-001/006 with new evidence ID
`EV-SCHEDULER-TICK` (pending Pass-anchor in the slice-3 closing docs PR after PRs 2-6 of
the slice-3 wave land); plus Wave 1a feat PRs `#322` `#323` `#324` `#325`; plus this
docs(repo) closing PR): 5 of 11 Phase-4a EXIT rows at `Pass` under AO-approved narrowings —
`EV-INTEGRATION-MATRIX` (OSC-routing-matrix-only narrowing — MIDI / camera / Syphon /
audio-uniform-IPC arms deferred to Wave 1b), `EV-HOTKEY-PARITY-CONFORMANCE` +
`EV-HOTKEY-COMPLETENESS` + `EV-HOTKEY-CONFLICT` (full scope), `EV-NET-HARDENING` (full
scope), `EV-HEALTH-METRICS-LOOPBACK-ONLY` (full scope per `docs/31` Phase-1 baseline
catalog), and `EV-ENTITLEMENT-ENFORCEMENT` + `EV-ENTITLEMENT-OFFLINE` +
`EV-ENTITLEMENT-REVOCATION` + `EV-ENTITLEMENT-FREE-FEATURES` (4 rows bundled at MVP
watermark-only narrowing — tier gating deferred to post-MVP per `docs/11` line ~125). 6
Phase-4a EXIT rows remain `Planned` for Wave 1b (`EV-MIDI-EXTENDED` plus the four deferred
`EV-INTEGRATION-MATRIX` arms plus entitlement tier-gating beyond the MVP watermark
surface). Slice-3 scheduler-tick engineering wave closure landed 2026-05-01 in a 5-PR
sequence (`#321` foundational module + `#326` T2 corpus + `#328` observability metric-emit
wiring + `#329` audio snapshot pull wiring + `#330` render dispatch wiring + this
docs(repo) closing PR): `engine/runtime/src/scheduler/{mod,observability,audio,render}.rs`
ships the canonical scheduler-tick frame loop per ADR-001 / ADR-006 with the established
sub-module + builder + tick-step extension pattern. Row 38 (`EV-SCHEDULER-TICK`) at Pass.
Tick-step ordering: audio pull (BEFORE state mutation) → bus drain + frame advance →
render dispatch (AFTER state mutation) → observability emit (last). Existing T2 corpora
(state_replay, preset_playlist, state_migration) preserve goldens via the additive-not-
replacing design (corpora continue to use `commit_all_into` directly with `frame=0` static
state). Phase 4+ entry remains a separate AO-held milestone with its own gate set per
`docs/11-roadmap-phases.md`; Phase-3 EXIT achievement does not pre-empt the Phase-4 gate
decision. Phase-4a Wave 1b partial closure landed 2026-05-02 in a 3-PR session
(`#332` entitlement tier-gating widening + `#333` MIDI adapter and EV-MIDI-EXTENDED
corpus + this docs(repo) closing PR): row 80 `EV-MIDI-EXTENDED` Pass under new
`integrations/midi-adapter/` crate (midir 0.10 cross-platform MIDI; 17 fixtures × 100
trials covering MIDI-CONF-11..20 + 5 dual-tagged matrix-arm fixtures; MPE deferred to
V2 with diagnostic-fires-once rejection); rows 37 + 86 narrowing-clauses dropped via
`HONORED_TOKEN_CAPABILITIES` widening 1 → 13 entries (data-only; IO_CAMERA intentionally
excluded — free at MVP per ADR-009). Phase-4a EXIT row state after Wave 1a + 1b:
**11 of 11 Phase-4a rows at Pass under collective narrowings**; full conformance modulo
Wave 1c camera/Syphon/audio-uniform-IPC narrowing on row 37 + IO_CAMERA tier-gating on
row 86 (Wave 1c explicitly deferred per AO direction; estimated 5,000-10,000 LoC
multi-week scope; requires platform integration decisions + likely new ADRs). Phase-4a
EXIT is **passable under current narrowings per `docs/26` §Rules line 194**; AO can
declare at discretion. Engine and integration crates
(runtime, scene, audio, observability, render, script, entitlement, osc-adapter,
midi-adapter) carry foundational evidence at EXIT quality; parallel UI port waves
against the v0.2-beta bundle closed all three Vitest test-coverage tracks (widgets,
shell components, routed views).

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
bash scripts/adr-lint.sh             # Phase-0 grace mode currently reports 0 warnings
bash scripts/trailer-check.sh HEAD   # DCO + Profile-Impact trailers
npm run commitlint:last              # Conventional Commits check
```

The cargo target directory is shared at the workspace root `/target/`; there is no
per-crate `target/` to clean.

## Workspace layout

```
chromaticity/
├── app/
│   ├── shell/            # Tauri 2 binary (chromaticity-app crate)
│   └── ui-bridge/        # ts-rs IPC type generator (Rust → TypeScript bindings)
├── engine/               # runtime/, scene/, audio/, observability/, render/, script/, entitlement/
├── integrations/         # osc-adapter/ (more adapter crates to follow)
├── third_party/          # vendored snapshots — currently lygia/ (Lygia v1.1.4 trial vendor)
├── workspace-hack/       # cargo-hakari managed crate (at root by convention)
├── .config/hakari.toml   # cargo-hakari config; platforms mirror deny.toml graph.targets
├── src/                  # Svelte web root; may migrate under app/ui/ later
├── tests/                # workspace-level cross-cutting tests
├── dist/                 # Vite build output — .gitkeep is load-bearing
├── docs/                 # normative planning corpus + ADRs
├── scripts/              # policy / CI shell helpers
├── .github/workflows/    # rust, frontend, sbom, evidence-tag-lint, policy, labels
├── Cargo.toml            # workspace root (edition 2024, MSRV 1.85.0)
├── deny.toml             # cargo-deny advisories / licenses / bans / sources
├── lefthook.yml          # pre-commit / pre-push hooks
├── vitest.config.ts      # Vitest runner (jsdom + Svelte plugin)
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
body / footer ≤ 100 chars per line. Scope is mandatory on `feat` / `fix` / `perf` /
`refactor` per `docs/43` §4.3 (enforced by the `scope-required-for-semver-types`
plugin rule in `commitlint.config.cjs`); optional on `docs` / `test` / `build` / `ci` /
`chore` / `revert`.

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
- When a rune is needed in a `.ts` module (e.g., reactive state in a store), use the
  `.svelte.ts` extension; ESLint has a parser override for this glob (per
  `feedback_eslint_svelte_dot_ts_parser.md`).

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
- `chromaticity-planning/` — read-only design + planning references (`.gitignored`,
  source tooling must not parse them); the `ui-predev-bundle*/` subtrees are active port
  references for UI/UX work — see "UI design references" below.
- `LICENSES/` — canonical third-party license text; preserve upstream formatting.
- `docs/adr/*` and `docs/29-cleanroom-legal-policy-baseline.md` — non-delegable; AO-only
  approval.
- Anything under `**/*.synScene` or `Contents/Resources/release/`.

## Load-bearing files that look optional

- `dist/.gitkeep` — Tauri's `generate_context!()` macro expands at `cargo check` time and
  reads `frontendDist` via `build.rs`. A missing `dist/` triggers a compile-time panic.
  `.gitignore` tracks only the sentinel and ignores everything else inside. **Note:**
  `npm run build` (vite build) clobbers the sentinel when emitting fresh artifacts;
  restore with `git checkout -- dist/.gitkeep` before staging, or you commit the
  deletion and break `cargo check` on fresh clones. `npm run dev` does not clobber.
- `app/shell/icons/icon.png` — Tauri config requires a window icon even when
  `bundle.active = false`. The 83-byte transparent placeholder is intentional.
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
- **`gh pr edit --title` and `gh pr edit --body-file` may silently no-op** due to a
  GraphQL Projects-classic deprecation error hitting the underlying PR-edit mutation —
  both commands return zero plus a warning but the title / body does not change. Do
  not trust the zero exit; verify with `gh pr view <N> --json title` (or `--json body`)
  after the edit. Workaround uses the REST PATCH endpoint:
  `gh api -X PATCH repos/OWNER/REPO/pulls/<N> -f title='<new title>'` for the title,
  and `gh api -X PATCH repos/OWNER/REPO/pulls/<N> --field body=@/tmp/pr-body.md` for
  the body.
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
  - **`trailer-check` has three firing points, not two.** The PR-event job in
    `.github/workflows/policy.yml` (a) range-walks branch commits AND (b)
    reconstructs the synthesised squash body from `PR_TITLE + "\n\n" + PR_BODY`
    and pipes it through `scripts/trailer-check.sh --stdin`. The (b) step is
    the one that catches a PR body that bypassed `.github/pull_request_template.md`
    and omits Profile-Impact — branch-commit trailers DO NOT survive squash-
    merge, so (a) alone will pass on a bad-body PR. The third firing point is
    the `push`-to-`main` backstop on HEAD; a red status there indicates either
    a parser regression or a PR that bypassed (b), and is remediated with a
    forward-looking fix commit (branch protection disallows history rewrite of
    `main`). PR #83 / merge `09a067d` motivated adding step (b).
- **`gh pr update-branch` breaks DCO and `trailer-check`.** The command asks GitHub to
  update the PR branch from its base, which creates a SERVER-side merge commit signed
  only by GitHub (`GitHub <noreply@github.com>`, no `Signed-off-by`, no
  `Profile-Impact`). That commit fails the `dco` and `trailer-check` required contexts
  on the next CI run and blocks merge. When a laggard PR goes out of date under
  `strict: true` branch protection, use `git rebase origin/main` in a local worktree
  followed by `git push --force-with-lease=<branch>:<old-sha>`; never the GitHub web
  button, never `gh pr update-branch`. Applies equally to the "Update branch" button
  in the PR UI.
- **Architecture-labeled PRs require `ADR:` + `Evidence:` trailers; solo-merge PRs
  require `Succession-Waiver: T1`/`T2`.** Two required branch-protection contexts on
  `main` enforce this: `architecture-trailer-check` (gated on the `architecture`
  label, validates the trailers exist plus reference real
  `docs/adr/ADR-NNN-*.md` files plus valid registry EV-IDs) and
  `succession-waiver-check` (validates `Succession-Waiver: T1`/`T2` format
  whenever the trailer is present). The PR_TITLE plus PR_BODY are reconstructed
  by the workflow into the synthesized squash-merge body and piped to
  `scripts/architecture-trailer-check.sh` / `scripts/succession-waiver-check.sh` —
  meaning the trailers must live in the **PR body**, not just on individual
  branch commits, since GitHub's squash-merge composes the merge commit from the
  PR title plus body. If your architecture PR fails the architecture-trailer-check
  required context, append the missing trailers to the PR body (via
  `gh pr edit --body-file` or the REST PATCH fallback per the `gh pr edit --title`
  gotcha above) — do NOT add them as a separate commit on the branch. Both
  checks are no-ops on PRs without the matching label / trailer, so
  non-architecture / non-solo-merge PRs are unaffected. Rollback path: a
  misbehaving context can be removed with
  `gh api -X DELETE repos/PhantasticUniverse/chromaticity/branches/main/protection/required_status_checks/contexts/<context-name>`,
  which reverts the check to advisory non-required without touching the
  workflow file.

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

## UI design references — wireframes drive UI/UX work

Two read-only assets in `chromaticity-planning/` define the canonical visual targets for
every UI/UX decision (`.gitignored` and `.prettierignored`; source tooling must not parse
them, but `Read` and review are the entire point of the bundle):

- **`chromaticity-planning/ui-predev-bundle-v0.2-beta/`** — design-surface-complete
  React/JSX bundle (stamped 2026-04-23f, marked partial: design-surface only;
  engineering-mechanics deferred to dev queue). Per-view JSX under `views/` (perform,
  library, scene, playlist, media, midi, osc, import, settings, audio, patch, onboard;
  the bundle's `scene.jsx` is the full Author port source) plus shared primitives —
  `chroma_inspector.jsx` (input controls palette, the W10 port source), `cue_curve.jsx`
  (cubic bezier transition curve, the W13 PR2 port source), `hotkeys_data.jsx`
  (canonical 57-row hotkey vocabulary with the `cap` entitlement field, mirrored in
  `src/lib/data/hotkeys.ts` for the `?`-toggle overlay),
  `empty_states.jsx` (per-view copy), `v02_supplement.jsx` (ChromaCtx + AdapterPill /
  DegradedBanner / OutputRoutingTile / WatermarkOverlay / LockedRow / CrashResumeBanner
  chrome), `extras.jsx` (ThemeSwitcher / FavslotsBar / SnapBar / ControlHotkeysHint),
  `components.jsx` (Icon / SceneStill / Preview / Spectrum primitives), `shaders.jsx`
  (GLSL shader demo patterns), `screenshot_modal.jsx`, plus `views/_data.jsx` fixture
  corpus, `styles.css` with all three themes (Pulse / Atelier / Console), `lib/` and
  `styles/` subdirs, and provenance markers (`BUNDLE-PROVENANCE-v0.2-beta.md`,
  `HANDOFF.md`, `DESIGN.md`, `FONTS.md`). The older `ui-predev-bundle/` (v0.1-alpha)
  sibling exists for historical comparison; the v0.2-beta bundle is canonical.
- **`chromaticity-planning/ui-predev-bundle/screenshots/`** — raw wireframe captures
  (timestamp-named PNGs from 2026-04-22). The W9 port pass copied them to
  `.playwright-mcp/wireframe-reference/` under descriptive filenames
  (`01-library-scene-grid`, `02-author-scene-editor`, `03-playlist-cue-table`,
  `04-media-library`, `05-mappings-midi-pads`, `06-osc-link-settings`,
  `07-import-shadertoy-isf`) plus an index `README.md` for easier reference during port
  work — that's the directory to read first, not the timestamped originals.

When scoping a UI/UX wave or PR, read `.playwright-mcp/wireframe-reference/README.md`
first, then the relevant numbered screenshot, then the corresponding bundle JSX.
Wireframe-faithful ports are the default; divergences need explicit AO approval. The
bundle JSX is portable as a structural reference — React patterns (Context, hooks)
translate to Svelte 5 idioms (stores, runes); the bezier math, hotkey vocabulary, OSC
route taxonomy, and empty-state copy are pure data and port nearly directly. Per the
auto-memory `feedback_wireframes_normative_for_ui_ux.md`, this is a load-bearing
principle, not a stylistic preference — failing to consult the wireframes leads to UX
work that needs to be redone when shell architecture lands (avoided by W12) or when
entitlement infrastructure gates components (deferred for W14+).

UI ports already in production: W6 ThemeSwitcher (3-theme cascade); W8 widget primitives
(Fader / Knob / Toggle / Swatch / ControlGroup) + HotkeyOverlay + EmptyState; W9 Library
backbone; W10 Inputs Inspector; W11 ErrorMessage primitive + Vitest scaffold + Tauri
ship-time bundling; W12 app shell (AppShell + 13-icon SidebarRail + StatusBar +
RouteStore + Perform / Library / Author / Settings views); W13 CueTransitionCurve widget

- Playlist v2 view (first un-disabled bundle-port routed view in the W12 shell). W14-W34
  waves shipped MIDI view, OSC view, Audio view + canvas Spectrum / BpmRing / BeatDots
  visualizers, Author full port (with CodeMirror integration), and the Adapter UI suite
  (AdapterPill + DegradedBanner + OutputRoutingTile + LockedRow + WatermarkOverlay +
  CrashResumeBanner) wired via the `capabilitiesStore` that replaced the bundle's
  React `ChromaCtx`. The store is currently fixture-seeded from the W33b
  `LICENSE_FIXTURE`'s capability matrix per `src/lib/stores/capabilities.svelte.ts`
  lines 19-28; the live `EntitlementBitmask` IPC swap-in is Phase-3+/4a
  `EV-ENTITLEMENT-ENFORCEMENT` scope, so chrome elements render as if a fully-
  activated license were installed until that lands. **Wireframe-faithful chrome
  ported, not engine-wired.** The Spectrum / BpmRing / BeatDots / AudioReactivity
  widgets run rAF against synthetic time-domain functions per their in-source
  "Frozen frame contract: synthetic, no IPC dispatch" comments. The producer-side
  `engine/audio::AudioUniformSnapshot` is real (slice-1 + slice-2 + slice-3a/b
  - Tier-9 + Tier-11 cross-tag landed), and `engine/render::audio_adapter`
    marshals it to GPU uniforms feature-gated under `audio-binding`, but the IPC
    bridge that would carry that producer data through Tauri to the Svelte
    visualizer surface is Phase-3+/4a `EV-INTEGRATION-MATRIX` scope. The
    `AudioReactivity` widget is similarly stubbed (band-tile values, BPM, peer-
    count, tap button — visual stubs; no IPC dispatch).

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

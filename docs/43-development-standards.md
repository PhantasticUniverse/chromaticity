# Development Standards

This document defines the repository-level standards that govern how Chromaticity source is written, committed, reviewed, versioned, released, and audited. It is the mechanical counterpart to `23-decision-ownership-matrix.md` (who decides), `26-evidence-artifact-registry.md` (what evidence is required), and `22-compatibility-versioning-policy.md` (how compatibility is guaranteed): this doc says _how those decisions, evidence artifacts, and compatibility guarantees are actually produced by day-to-day engineering work_.

## 1. Purpose

- Convert the planning corpus into enforceable repo policy before the first line of implementation code lands.
- Make clean-room provenance (`29-cleanroom-legal-policy-baseline.md`) mechanically auditable per commit, not retrospectively reconstructed.
- Make SemVer bumps, changelog entries, and compatibility-profile impact statements derive from commit metadata rather than human memory.
- Match the accountable-ownership + evidence-registry model already in place: every standard in this doc either feeds an `EV-*` artifact, satisfies an ADR, or closes a risk in `13-risk-register.md`.
- Constrain process overhead so a two-human team (Accountable Owner + Deputy per `23-decision-ownership-matrix.md`) can still ship at Phase 1 pace.

## 2. Scope

In scope:
- Commit message convention, branching, review, and signing.
- Versioning, changelog, and release-engineering conventions for every versioned artifact the product ships.
- Language-specific lint/format/toolchain minimums (Rust, TypeScript/JavaScript, GLSL, Svelte).
- Documentation authoring standard and its mapping to `adr/ADR-010-embedded-docs-generation.md`.
- ADR template (MADR 4.0.0), issue/PR labels, CODEOWNERS, security disclosure.
- License metadata (SPDX, REUSE 3.2) and SBOM policy.
- Repository naming and visibility posture through MVP.

Out of scope (intentionally deferred):
- CI pipeline topology and runner selection (release-eng work; tracked by `EV-GATE-MODEL-AUDIT` and `EV-RELEASE-ATTESTATION`).
- Specific hosted services (issue tracker, artifact registry, crash-report backend) — those are named in their respective ADRs (`adr/ADR-014-...`, `adr/ADR-016-...`).
- Any code file templates (`.github/*`, `deny.toml`, `rustfmt.toml`, `eslint.config.js`, etc.). Those are produced by the Phase-0 repo-bootstrap work package; this doc only specifies their required contents.
- Final visual design language for the non-scene UI (owned by AI UX Workcell per `23-decision-ownership-matrix.md`).

## 3. Normative References

This document is normative. Conflicts between this document and the references below are resolved in favor of the reference:

- `22-compatibility-versioning-policy.md` — compatibility profiles, schema versions, migration rules.
- `23-decision-ownership-matrix.md` — decision authority, succession tiers, waiver process.
- `26-evidence-artifact-registry.md` — evidence IDs, gate ledger, status model.
- `29-cleanroom-legal-policy-baseline.md` — clean-room authoring, brand/trademark policy, provenance.
- `adr/ADR-010-embedded-docs-generation.md` — embedded documentation source of truth and URL taxonomy.
- `adr/ADR-014-signing-notarization-auto-update.md` — binary signing, notarization, and updater keys (distinct from commit signing specified here).
- `adr/ADR-016-observability-and-telemetry-posture.md` — telemetry posture that constrains what may be emitted from source.
- `adr/ADR-018-non-scene-ui-toolkit.md` — Tauri 2 + Svelte 5 + TypeScript + CodeMirror 6 stack (governs the TypeScript/Svelte tooling rules in Sec.12).

## 4. Commit Convention

### 4.1 Format

The Chromaticity repo adopts **Conventional Commits 1.0.0** with Chromaticity-specific scopes, types, and trailers. The normative form is:

```
<type>[!](<scope>): <subject>

[body]

[trailers]
```

- Subject is imperative mood, <=72 characters, no trailing period.
- Body lines wrap at 100 characters.
- Breaking changes are marked with `!` before the colon **and** a `BREAKING CHANGE:` footer describing the break.

### 4.2 Types

Only these types are accepted; `commitlint` rejects others:

| Type | Meaning | SemVer effect |
|---|---|---|
| `feat` | New end-user-visible capability | minor |
| `fix` | Bug fix | patch |
| `perf` | Performance change with measured impact on `EV-PERF-BUDGETS` | patch (or minor if it unblocks a previously failing SLO) |
| `refactor` | Internal restructure, no behavior change | none |
| `docs` | Documentation-only change | none (unless ADR-010 coverage contract changes, then minor) |
| `test` | Test-only change | none |
| `build` | Build scripts, toolchain, workspace plumbing | none |
| `ci` | CI pipeline, runner config, gate automation | none |
| `chore` | Housekeeping with no product effect | none |
| `revert` | Revert of a prior commit; must reference commit SHA | inverse of reverted commit |

Major SemVer bumps are driven exclusively by `!` or `BREAKING CHANGE:`. No other path bumps major.

### 4.3 Scopes

Scope is mandatory for `feat`, `fix`, `perf`, and `refactor`. Scope vocabulary is fixed; `commitlint` rejects unknown scopes. New scopes require a PR editing this section (scope additions are `build` or `docs` commits, not `feat`).

| Scope | Subsystem / Workcell |
|---|---|
| `runtime` | AI Runtime Workcell (render graph, scheduler, determinism) |
| `scene` | AI Scene Workcell (schema, normalizer, migration, compatibility shims) |
| `audio` | AI Audio Workcell (analysis pipeline, uniform generation) |
| `integration` | AI Integration Workcell (OSC, MIDI, media, camera, Syphon/Spout/NDI) |
| `authoring` | In-app authoring environment (`37-authoring-environment-spec.md`) |
| `library` | Scene library and import pipeline (`38-scene-library-and-import-spec.md`) |
| `ui` | Non-scene UI shell (per `adr/ADR-018-non-scene-ui-toolkit.md`) |
| `docs` | Embedded documentation and planning-doc corpus |
| `ux` | UX contracts (`EV-UX-*`) |
| `entitlement` | Entitlement substrate (`adr/ADR-009-activation-and-entitlements.md`) |
| `observability` | Telemetry, diagnostics, Sentry posture (`adr/ADR-016-...`) |
| `release-eng` | Signing, notarization, updater, SBOM, release automation |
| `policy` | Planning documents, ADRs, risk register, evidence registry |
| `repo` | Repo-wide tooling not tied to a subsystem (`.editorconfig`, CODEOWNERS, label schema) |

### 4.4 Compatibility breaking marker

A commit that invalidates any declared `compat-syn-v1` shim, changes a documented `compat-syn-v1` or `native-chroma-v1` behavior, or removes a previously-shipped route/alias is a **compat-breaking commit**. `compat` is not a Conventional Commit type; it is a **derived classification** computed mechanically from two signals on the commit:

1. The subject line uses `!` (breaking) against one of the allowed types (`feat!`, `fix!`, `perf!`, `refactor!`).
2. The mandatory `Profile-Impact:` trailer (Sec.4.5) is one of `compat-syn-v1-major`, `compat-syn-v1-minor` when the minor rev retires a shim, `native-chroma-v1-major`, or `native-chroma-v1-minor` when the minor rev retires a behavior.

A commit satisfying both (1) and (2) with one of the above values is a **compat-breaking commit** for the purposes of `EV-COMPAT-PROFILE-DIFF`. Shorthand notation used elsewhere in this document (and in `22-compatibility-versioning-policy.md`) is `compat!`; the shorthand is exactly the derived classification, not a new Conventional Commit type. Canonical form:

```
feat(scene)!: drop legacy favslot alias on /favslot/{n}/save

BREAKING CHANGE: /favslot/{n}/save removed; use /favslots/{n}/save.
Profile-Impact: compat-syn-v1-major
ADR: ADR-005
Evidence: EV-COMPAT-PROFILE-DIFF, EV-INTEGRATION-MATRIX
```

Every compat-breaking commit on `main` triggers the profile-diff-report + migration-note requirements of `22-compatibility-versioning-policy.md` Sec.Guarantees via `EV-COMPAT-PROFILE-DIFF`. The diff generator scans the release window for commits satisfying the derived classification above; any commit marked `Profile-Impact: *-major` or `Profile-Impact: *-minor` without an accompanying migration note fails `EV-COMPAT-PROFILE-DIFF`. `Profile-Impact: none` is the default and requires no diff.

### 4.5 Mandatory trailers

Every merged commit on `main` MUST carry:

```
Signed-off-by: <Full Name> <email>
Profile-Impact: none | compat-syn-v1-minor | compat-syn-v1-major | native-chroma-v1-minor | native-chroma-v1-major
```

Every architecture-impacting commit (as defined in `23-decision-ownership-matrix.md` Sec.Mandatory Decision Rules) MUST additionally carry:

```
ADR: ADR-<NNN>
Evidence: EV-<ID>[, EV-<ID>...]
```

Commits that close a row in `13-risk-register.md` SHOULD carry:

```
Risk: R-<NN>
```

### 4.6 Enforcement

- `commitlint` validates the PR squash-merge title against Sec.4.1–4.4. Per-commit lint is advisory on feature branches (friendly for WIP), mandatory for the squash-merge commit that lands on `main`.
- `Signed-off-by` is enforced by a required-status check (`dco`) on every commit in the PR, not only the merge commit.
- `Profile-Impact` trailer is enforced by a custom check; absence blocks merge.
- `ADR:` and `Evidence:` trailers are enforced only when the PR labels include `architecture` (Sec.16).

### 4.7 Rejected alternatives

- **Gitmoji / emoji-led commits** — rejected: not machine-readable for changelog automation and pollutes clean-room provenance records.
- **Free-form commit messages + release notes hand-authored per cycle** — rejected: two-human team cannot sustain the manual bookkeeping through six phases.
- **Per-commit lint on feature branches** — rejected: slows iteration; PR-title lint captures the only commit that actually appears on `main` under squash-merge policy (Sec.7).

## 5. Versioning Axes

`22-compatibility-versioning-policy.md` already enumerates the conceptual axes. This section pins the concrete version scheme, tag format, and bump driver for each axis.

| Axis | Scheme | Tag format | Bump driver | Evidence row |
|---|---|---|---|---|
| App / shipped binary | SemVer 2.0.0 | `app/vX.Y.Z` | Conventional Commits; `feat!`/`compat!` → major | Release cumulative gate |
| `scene.json` schema | SemVer | `scene-schema/vX.Y.Z` | Additive key = minor; rename/removal = major | `EV-SCHEMA-LINT` + migration note |
| Runtime compatibility profile `compat-syn-v1` | `compat-syn-v1.<rev>` | `compat-syn-v1/rev-<N>` | Additive shim/coercion = `rev` bump; behavior change = new major profile per `22-...md` | `EV-COMPAT-PROFILE-DIFF` |
| Runtime compatibility profile `native-chroma-v1` | `native-chroma-v1.<rev>` | `native-chroma-v1/rev-<N>` | Same rule as above | `EV-COMPAT-PROFILE-DIFF` |
| Reactive symbol catalog | SemVer, pinned to docs | `symbols/vX.Y.Z` | `36-reactive-symbol-catalog.md` Sec.8 change-control | `EV-SYMBOL-CATALOG`, `EV-DOCS-VERSION-SYNC` |
| Embedded docs | SemVer, pinned to catalog | `docs/vX.Y.Z` | Equals symbol catalog version at release per ADR-010 | `EV-DOCS-VERSION-SYNC` |
| Entitlement token format | SemVer | `entitlement-token/vX.Y.Z` | Any claim add/change = minor; removal/reinterpretation = major | `EV-ENTITLEMENT-TOKEN-FORMAT` |
| OSC route schema | SemVer | `osc-schema/vX.Y.Z` | New alias = minor; removal of a `compat-syn-v1` alias = major | `EV-INTEGRATION-MATRIX` |
| MIDI surface | SemVer | `midi-surface/vX.Y.Z` | Adds/extensions minor; surface removals major | `EV-MIDI-EXTENDED` |
| `.chromaShow` bundle format | SemVer | `show-bundle/vX.Y.Z` | Additive keys minor; breaking layout major; migration path required | `EV-STATE-MIGRATION`, `EV-SHOW-BUNDLE-RECOVERY` |

### 5.1 Invariants

- Every axis above is tagged on `main` by a single accountable signer (AO, or Deputy under T1/T2 succession) using `git tag -s`. Unsigned release tags are rejected by `release.yml` (`ci` scope work package).
- App release `app/vX.Y.Z` embeds the other axes' versions in its release manifest (generated artifact, not a hand-edited file). Mismatch between embedded and tagged versions fails `EV-DOCS-VERSION-SYNC` and `EV-RELEASE-ATTESTATION`.
- Pre-1.0.0 versions (`0.Y.Z`) follow SemVer's 0.x exception: minor increments may break backward compatibility. The first post-MVP release is `app/v1.0.0`; pre-MVP preview builds use `app/v0.N.M`.
- No CalVer on any axis. The stability contract of `compat-syn-v1` requires the SemVer social contract; CalVer would silently hide breaking changes behind date-driven version numbers.

### 5.2 Build metadata

Build-metadata suffixes (`+<metadata>`) are allowed per SemVer 2.0.0 Sec.10 and are used only to disambiguate CI builds of the same release version. They do not affect precedence and MUST NOT be used to smuggle breaking changes past the bump rules above.

## 6. Changelog Policy

### 6.1 Format

Chromaticity adopts **Keep a Changelog 1.1.0**. Every versioned axis in Sec.5 maintains its own `CHANGELOG.md` in its source subtree. Sections follow Keep a Changelog headings, extended with:

- **Compatibility** — entries generated from every `compat!` commit in the window, with the original `Profile-Impact` trailer value preserved.
- **Evidence** — bullet list of `EV-*` rows whose `Last Passing Run` advanced in this release.

### 6.2 Generation

`release-please` (or `cargo-release` for Rust-only crates) renders `CHANGELOG.md` updates from the Conventional Commit log at release time. Manual edits to `CHANGELOG.md` are rejected by a pre-commit check; operator edits happen only in `release-please`'s release PR before merge.

### 6.3 `/resources/release-notes` rendering

The embedded docs page at `/resources/release-notes` (per ADR-010 Sec.URL convention) is generated from the app-axis `CHANGELOG.md` at build time. The page carries a clean-room provenance entry (`author: release-engineering-pipeline`, `reviewed_by: <AO>`, `date: <release date>`). No free-text release-note prose is authored outside `CHANGELOG.md`.

### 6.4 Deprecation notices

Any deprecation declared under the Deprecation Policy of `22-...md` MUST be published in the changelog's **Compatibility** section with: rationale, affected scenes/features, migration path, removal-eligibility criterion. Absence fails `EV-COMPAT-PROFILE-DIFF`.

## 7. Branching and Pull Request Policy

### 7.1 Trunk-based branching

- `main` is the single release-eligible branch and is always green on the required CI checks.
- Feature branches are short-lived (target <=5 working days of unmerged age) and named `<initials>/<scope>/<short-kebab-slug>` (e.g., `xb/runtime/render-graph-scaffold`). Stale branches beyond 10 days are flagged by a weekly CI report and require either rebase or closure.
- Release branches exist only from Phase 5 MVP freeze forward, named `release/<app-version-minor>` (e.g., `release/mvp-1.0`). Before Phase 5, hotfixes and release prep happen on `main`.
- No direct pushes to `main`. CI rejects unsigned or non-fast-forward pushes.

### 7.2 Merge strategy

- **Squash-merge only.** The squash-merge commit message equals the PR title + body. Conventional Commits enforcement (Sec.4) applies to the squash-merge commit. Feature-branch commit hygiene is advisory.
- No merge commits on `main`. No rebase-merge on `main` (the squash-merge policy preserves a single commit per PR and keeps `main` history linear and greppable).

### 7.3 Required checks

Every PR to `main` must pass:

- `commitlint` on PR title.
- `dco` on every feature-branch commit.
- Language-specific formatters and linters declared in Sec.12.
- `reuse lint` (SPDX/REUSE compliance, Sec.10).
- `cargo deny` (license + advisory, Sec.10).
- `evidence-tag-lint` (Sec.15) — advisory on Phase 0; required from Phase 1.
- Unit test suite for affected subtrees.
- `adr-lint` (Sec.14) when `adr/` is touched.

Phase-0 exceptions: tests and evidence-tag-lint are advisory until the first `EV-DETERMINISM` baseline run passes. The exception is logged in a Phase-0-only waiver row.

### 7.4 PR size and review cadence

- Target PR size: <=400 lines changed net of generated code. Larger PRs require a `oversized` label and an AO acknowledgement comment before merge.
- Non-architecture PRs: author + one reviewer (the other human, unless both are the same under T1/T2 succession — see Sec.7.6). Auto-merge allowed after checks green.
- Architecture-impacting PRs (per `23-...md` Sec.Mandatory Decision Rules): author + AO sign-off **and** Independent Architecture Reviewer sign-off per `23-...md` Sec.Named Independent Reviewer Roles. Auto-merge disabled.

### 7.5 PR template (content requirement, not file layout)

Every PR description includes, in this order:

1. **Summary** — one paragraph, plain prose.
2. **Linked ADR** — `ADR-NNN` or explicit `N/A — non-architectural`.
3. **Evidence impact** — `EV-*` rows whose status may change; `none` is allowed but must be explicit.
4. **Profile impact** — matches the commit trailer value from Sec.4.5.
5. **Docs impact** — whether `adr/ADR-010-embedded-docs-generation.md` coverage changes, and which `/reference|manual|guides|faq|resources` pages need regeneration.
6. **Risk** — referenced `R-*` IDs (optional but encouraged when relevant).
7. **Checklist** — SPDX headers present on new files; changelog entry will be generated; migration note attached if required.

### 7.6 Solo-merge rule under succession

Merge authority tracks `23-decision-ownership-matrix.md` Sec.Accountable Owner Succession exactly; this section codifies the repo-level mechanism, it does not extend authority.

- **T0 (both humans available)**: architecture PRs require two-human sign-off per Sec.7.4.
- **T1 (AO unreachable >72h; Deputy holds interim authority)**: the Deputy MAY solo-merge non-architecture PRs that fall inside T1's gate-unblocking scope. The Deputy MUST NOT solo-merge architecture-impacting PRs, legal/brand PRs, or any PR touching `/chromaticity-planning/` or `/adr/` — those wait for AO return or escalate to T2. Any T1 solo-merge carries the trailer `Succession-Waiver: T1` and opens a succession-log row (not a `Waived` evidence row, per Sec.7.7); the action is re-ratified by the AO within 5 business days of return-of-availability.
- **T2 (AO unreachable >14 days; Deputy becomes Acting AO)**: the Acting AO MAY solo-merge architecture-impacting PRs and scope changes (matching T2 authority in `23-...md`). The Acting AO MUST NOT solo-merge PRs that touch the legal baseline carve-out in Sec.8.1 or close any gate listed in `23-...md` Sec.T2 as requiring contracted counsel. T2 solo-merges carry the trailer `Succession-Waiver: T2`.
- **T3 (permanent succession)**: the new AO-of-record operates under T0 rules; succession trailers are no longer used.

### 7.7 Succession-log vs Waived evidence

Succession merges generate a **succession-log row** attached to the PR and referenced from the next AO-return acknowledgment commit. A succession-log row is NOT an `EV-* Status: Waived` row. A `Waived` transition still requires AO (or Acting AO at T2) approval per `23-...md` Sec.Waiver Process and cannot bypass legal/compliance gates.

## 8. Code Ownership and Review

### 8.1 CODEOWNERS subtree mapping

The repo ships a `CODEOWNERS` file mapping subtrees to the named humans who hold review authority on behalf of the workcell that owns the subtree:

| Subtree | Owning workcell | Primary reviewer | Deputy reviewer |
|---|---|---|---|
| `/engine/runtime/` | AI Runtime Workcell | Xavier Butler | Fletcher Hammond |
| `/engine/scene/` | AI Scene Workcell | Xavier Butler | Fletcher Hammond |
| `/engine/audio/` | AI Audio Workcell | Xavier Butler | Fletcher Hammond |
| `/integrations/` | AI Integration Workcell | Xavier Butler | Fletcher Hammond |
| `/app/authoring/` | AI Scene Workcell | Xavier Butler | Fletcher Hammond |
| `/app/library/` | AI Scene Workcell | Xavier Butler | Fletcher Hammond |
| `/app/ui/` | AI UX Workcell | Xavier Butler | Fletcher Hammond |
| `/docs/` | AI Docs Workcell | Xavier Butler | Fletcher Hammond |
| `/release-eng/` | Accountable Owner | Xavier Butler | Fletcher Hammond |
| `/chromaticity-planning/` | Accountable Owner | Xavier Butler | (non-delegable; AO only) |
| `/adr/` | Accountable Owner | Xavier Butler | (non-delegable; AO only) |
| `/LICENSES/`, `/REUSE.toml`, `/SECURITY.md` | Accountable Owner | Xavier Butler | (non-delegable; AO only) |

Planning docs, ADRs, and legal-surface files are AO-only: the Deputy cannot self-merge changes to them even under T1. T2 transfers most of this authority to the Acting AO per `23-...md`, with the following **legal-baseline carve-out** that remains non-delegable at T2 and may transition only at T3:

- `chromaticity-planning/29-cleanroom-legal-policy-baseline.md`
- `chromaticity-planning/31-supportability-privacy-ops-policy.md` (privacy sections only)
- `LICENSES/`, `REUSE.toml`, `SECURITY.md` at repo root
- any file matching `chromaticity-planning/**/*legal*.md` or `chromaticity-planning/**/*brand*.md`
- Brand/Trademark Review Checklist artifacts referenced by `EV-BRAND-REVIEW`, `EV-LEGAL-PROVENANCE`, or `EV-LICENSE-AUDIT`

Edits to the carve-out paths require AO approval (T0 or T3); the Acting AO at T2 may open PRs against them but may not self-merge. This matches `23-...md` Sec.T2 (legal/brand gates still require contracted counsel).

### 8.2 Independent review pairing

Independent Architecture Reviewer and Independent Release QA Reviewer roles defined in `23-...md` Sec.Named Independent Reviewer Roles use the repo's review mechanics: their sign-off is recorded as a GitHub PR review with the tag `[INDEPENDENT-ARCH-REVIEW]` or `[INDEPENDENT-RELEASE-QA]` in the review comment body. The `adr-lint` check fails if an architecture-impacting PR merges without the matching tagged review.

### 8.3 Conflict-of-interest rule

When a PR author and the CODEOWNERS-listed primary reviewer are the same person, the Deputy reviewer must perform the review. If both are unavailable, the PR waits (and opens a `succession-pending` label) rather than self-merges, except under the explicit T1/T2 solo-merge rule in Sec.7.6.

## 9. Commit and Release Signing

### 9.1 Commit signing

- **Commits on `main` MUST be cryptographically signed.** Unsigned commits fail the `signed-commits` required check.
- Accepted signing methods:
  - **Sigstore `gitsign` (preferred).** Keyless OIDC-based signing; signatures recorded to Rekor. No long-lived key material; authorship provenance is externally verifiable and survives key rotation. Matches the clean-room posture in `29-cleanroom-legal-policy-baseline.md`.
  - **GPG** (long-lived key, uploaded to GitHub). Accepted as fallback for local development without network at commit time.
  - **SSH signing** (`gpg.format = ssh`). Accepted.
- Feature-branch commits need not be signed, but the final squash-merge commit on `main` is signed either by the author or by the merge automation on the author's behalf (signed attestations preserved either way).

### 9.2 DCO sign-off

Every commit MUST carry `Signed-off-by: Full Name <email>` per the Developer Certificate of Origin 1.1 (<https://developercertificate.org/>). The `dco` check enforces this on every feature-branch commit, not only the merge commit, so that rebased/fixup commits retain sign-off.

Chromaticity does not require a separate Contributor License Agreement. DCO + clean-room provenance records (ADR-010 page-level provenance) + SPDX headers (Sec.10) together produce the provenance story `EV-LEGAL-PROVENANCE` needs.

### 9.3 Release tag signing

Release tags on every axis in Sec.5 are signed with `git tag -s` by the AO (or Deputy under T2 with `Succession-Waiver: T2` on the release commit). Unsigned release tags are rejected by `release.yml`. Tag signatures are verified by the build pipeline before `release.yml` publishes any artifact under that tag.

### 9.4 Signing domains

Chromaticity uses three distinct signing domains. Each domain has its own key material, key custody, rotation policy, and governing document. Keys are never reused across domains even when the underlying primitive (e.g., Ed25519) is the same:

1. **Source-provenance signing** (this document): Git commit signatures and release-tag signatures. Scope: authorship provenance for clean-room defense. Primitives: Sigstore `gitsign` (preferred), GPG, or SSH signing per Sec.9.1. Governing policy: this section.
2. **Binary distribution signing** (`adr/ADR-014-signing-notarization-auto-update.md`): Apple Developer ID, Windows Authenticode, Sparkle EdDSA appcast signatures. Scope: end-user binary integrity and platform notarization. Governing policy: ADR-014.
3. **Entitlement token signing** (`adr/ADR-009-activation-and-entitlements.md`): Ed25519 signatures over `entitlement-v2` tokens. Scope: watermark-on/off gating and post-MVP capability attestation. Key rotation tracked via the `kid` claim. Governing policy: ADR-009.

The three domains have non-overlapping key sets. A release pipeline commit may be signed under Domain 1, produce a binary signed under Domain 2, and include or accept a token signed under Domain 3 in the same release; the three signatures are independently verified by their respective consumers.

## 10. License, SPDX, REUSE, and SBOM

### 10.1 SPDX headers

Every source file committed to the repo carries an SPDX header in its file comment syntax:

```
SPDX-License-Identifier: <license-id>
SPDX-FileCopyrightText: <year> <copyright holder>
```

The permitted set of `<license-id>` values is fixed by `deny.toml` and tracks the allowlist in `29-cleanroom-legal-policy-baseline.md` Sec.Dependency Licensing. Adding a new license ID requires a `policy` commit touching both `deny.toml` and the clean-room policy.

Headers are enforced by `reuse lint` on every PR; a file missing a valid SPDX header fails CI.

### 10.2 REUSE 3.2 layout

- `LICENSES/` directory holds the canonical text of every license referenced by an SPDX ID used in the repo.
- `REUSE.toml` declares default copyright holders and default licenses for file globs that cannot carry inline headers (assets, binaries).
- `reuse lint` is a required PR check.

### 10.3 Third-party code and docs

- Vendored third-party code lives under `/third_party/<origin>/` with an accompanying `ATTRIBUTION.md` declaring origin URL, commit SHA, license, and clean-room review outcome.
- Third-party prose (where permitted under ADR-010) is tagged with `content-origin: third-party` front-matter per Sec.13.2. `EV-DOCS-CLEANROOM` fails CI if first-party pages carry that value or if third-party pages lack it.
- The `lygia` and `hg_sdf` alternatives called out in `29-...md` Sec.External Libraries follow the vendored-with-attribution pattern above.

### 10.4 SBOM and dependency audit

- Every release artifact ships a CycloneDX 1.5 SBOM (`cargo-cyclonedx` for Rust subtrees; `syft` for assembled app bundles).
- SBOM SHA-256 is embedded in the release attestation (Sec.11) and recorded in `EV-RELEASE-ATTESTATION` (new; Sec.19).
- `cargo deny check` runs on every PR and nightly:
  - `advisories` — fail on any unpatched `HIGH` or `CRITICAL`.
  - `licenses` — fail on anything outside the allowlist.
  - `sources` — fail on crates not sourced from `crates.io` or an allowlisted mirror.
  - `bans` — fail on explicitly banned crates (list maintained per ADR-013 networking policy).
- Any entry in `deny.toml` under `advisories.ignore` requires an inline rationale comment and is
  reviewed on a fixed cadence: at least once per quarter, and immediately after every dependency
  train that bumps Tauri, WRY, or workspace Rust minors. Review outcomes are recorded in the
  corresponding policy PR body and linked to `EV-SECURITY-DECLARATION`.
- `cargo audit` runs nightly as a belt-and-suspenders check; failures open an issue with the `security` label.

### 10.5 Dependency update policy

- Patch-only security advisories → Dependabot auto-PR → squash-merge after green checks (non-architecture path).
- Minor/major updates → Dependabot PR → human review + `cargo hakari` regeneration + regression suite before merge.
- Dependency additions require a PR body entry: rationale, license, transitive count, and `EV-LICENSE-AUDIT` impact statement.

## 11. Release Engineering

### 11.1 Release automation

- `release-please` (for the app axis) and `cargo-release` (for per-crate publishes on internal crates if any) drive version bumps and changelog generation from Conventional Commits.
- Release PRs are created by automation; a human (AO or Deputy per succession) approves and merges.
- On merge of the release PR, `release.yml` runs the binary-signing pipeline per ADR-014, emits the SBOM (Sec.10.4), and publishes the release-notes doc page per Sec.6.3.

### 11.2 Release attestation

Every release artifact published under an `app/vX.Y.Z` tag carries a signed attestation recording:

- commit SHA.
- source tree digest.
- toolchain versions.
- SBOM SHA-256.
- evidence roll-up: every `EV-*` row shipped at `Pass` for this release, with its `Last Passing Run` identifier.
- binary digests per platform (macOS notarized app, Windows signed installer).

Attestation is produced with Sigstore `cosign attest` (or `slsa-github-generator`, whichever survives the first Phase-0 release-eng evaluation) and stored in Rekor.

### 11.3 Rollback posture

Rollback of a released version is performed by publishing a new patch release that reverts the offending commit(s), not by re-signing an older artifact. Releases are immutable once attested.

## 12. Language and Tooling Standards

### 12.1 Rust

Applies to: all engine code, Tauri backend, release-eng crates.

- **Edition**: Rust 2024 (or the latest stable edition at repo init; recorded in `rust-toolchain.toml`).
- **MSRV**: pinned in `rust-toolchain.toml`. Bumping MSRV requires a `build` commit and a `CHANGELOG.md` entry under **Compatibility**.
- **Formatter**: `rustfmt` with default config; no custom rules. Formatting deviation fails CI.
- **Linter**: `cargo clippy --workspace --all-targets -- -D warnings`. `clippy::pedantic` is advisory (warn-only) to avoid drive-by noise.
- **No `unwrap()`/`expect()` in non-test code.** Enforced via `clippy::unwrap_used` and `clippy::expect_used` set to `deny` in the non-test lint profile.
- **Error model**: `thiserror` for library error types, `anyhow` only at binary edges and one level below. No string-typed errors crossing crate boundaries.
- **`unsafe`**: allowed where justified; each `unsafe` block carries a doc comment beginning `// SAFETY:` that states the invariant being upheld. `clippy::missing_safety_doc` is deny.
- **Workspace hygiene**: `cargo hakari` manages the workspace-hack crate. `cargo machete` runs nightly to flag unused dependencies.
- **Determinism**: no `HashMap`/`HashSet` with default hasher in any code path that contributes to `EV-DETERMINISM`. Use `BTreeMap`/`BTreeSet` or `ahash::AHashMap` with a pinned seed.

### 12.2 TypeScript

Applies to: Tauri frontend (per ADR-018), authoring-env frontend (per `37-...md`), docs site build scripts.

- **TypeScript** with `strict: true`, `noUncheckedIndexedAccess: true`, `exactOptionalPropertyTypes: true`. `tsc --noEmit` required in CI.
- **Formatter**: `prettier` with default config; deviation fails CI.
- **Linter**: `eslint` with `@typescript-eslint/recommended` + `eslint-plugin-svelte` + `eslint-plugin-import`. No `any` in shipped code (`@typescript-eslint/no-explicit-any` = error) except in typed-boundary shims which must carry a comment justifying the escape hatch.
- **Module resolution**: native ESM. No CommonJS in first-party code.
- **Runtime**: the non-scene UI toolkit is governed by ADR-018; this section only specifies lint/format minimums.

### 12.3 Scene-author JavaScript (QuickJS sandbox)

Applies to: sample scenes, scene-library fixtures, scaffolder output, embedded-docs code examples.

- **Language level**: ES2020 subset that QuickJS supports per `adr/ADR-012-script-sandbox-engine.md`. CI runs a QuickJS-parity check that rejects syntax QuickJS does not accept.
- **Linter**: a Chromaticity-authored `eslint-plugin-chromaticity-sandbox` enforces the host-API whitelist from the script sandbox ADR. Any reference to a disallowed global fails CI.
- **Formatter**: `prettier` with default config.
- **No build step** in shipped scene scripts. A scene's `script.js` is what QuickJS executes; no transpilation, no minification.

### 12.4 GLSL

Applies to: scene `main.glsl` files, shipped helper `glsl/` directories, import-pipeline goldens.

- **Validator**: `glslangValidator` with GLSL 4.50 target (matches ADR-011). CI runs validation against both `compat-syn-v1` and `native-chroma-v1` profiles where the scene's profile declaration applies.
- **Formatter**: `glsl_analyzer`-compatible formatting via a project `.glsl-fmt`. Deviation is advisory until a formatter is stable enough to be mandatory.
- **Naming**: reactive uniform references follow `36-reactive-symbol-catalog.md`. Any `syn_*` identifier not in the catalog fails `EV-SCHEMA-LINT`.

### 12.5 Svelte

Applies to: non-scene UI per ADR-018.

- `svelte-check --tsconfig ./tsconfig.json` required in CI.
- `eslint-plugin-svelte` with Chromaticity accessibility rule pack (contributes to `EV-ACCESSIBILITY-AA`).
- Storybook for every public component; axe-core accessibility check runs on every story.

### 12.6 Common

- **`.editorconfig`**: LF line endings, UTF-8, final newline, `insert_final_newline = true`, tab-width 4 for Rust, 2 for everything else.
- **`.gitattributes`**: LF normalization across platforms; mark `*.synScene`, `*.chroma`, `*.chromaPack`, `*.chromaShow` as `binary`-ish where appropriate so Git does not attempt diff algorithms on packaged assets; mark generated docs output and SBOM files `linguist-generated=true`.
- **Git hooks**: `lefthook` runs the fast subset (`rustfmt --check`, `prettier --check`, `commitlint`, `reuse lint` on changed files, `eslint` on changed files) pre-commit and pre-push. Hook config is checked in.
- **Pre-commit on generated files**: generated files (SBOMs, `docs-index.json`, changelog sections) are emitted under `/generated/` and excluded from lint/format hooks but included in SPDX/REUSE coverage.

## 13. Documentation Standard

### 13.1 Diátaxis mapping

The four Diátaxis modes map onto the ADR-010 URL taxonomy as follows; front-matter `doc-type` is mandatory on every page:

| Diátaxis mode | ADR-010 section | `doc-type` value | Intent |
|---|---|---|---|
| Tutorials (learning-oriented) | `/guides/*` (subset) | `tutorial` | Step-by-step, known-good path for a newcomer |
| How-to guides (task-oriented) | `/guides/*` (subset), `/manual/*`, `/faq/*` | `howto` | Task completion for a user with a goal |
| Reference (information-oriented) | `/reference/*` | `reference` | Precise, exhaustive, API-accurate |
| Explanation (understanding-oriented) | `/resources/*`, portions of `/manual/*` | `explanation` | Background, rationale, trade-offs |

A page mixing modes is an `EV-DOCS-COVERAGE` failure. Split the page.

### 13.2 Front-matter

Every doc page carries YAML front-matter with at minimum:

```yaml
doc-type: tutorial | howto | reference | explanation
author: <name>
reviewed_by: <name>
date: <YYYY-MM-DD>
version: <docs version at last review>
content-origin: first-party | third-party
source: <optional — reference catalog entry or third-party URL for reference pages>
```

`author`, `reviewed_by`, and `date` together are the **provenance triple** required by `adr/ADR-010-embedded-docs-generation.md` Sec.Source of truth. `content-origin` is a separate field classifying origin licensing (first-party Chromaticity-authored vs third-party vendored or linked) and MUST NOT be confused with the provenance triple. `author` and `reviewed_by` must not be the same person for first-party pages; this mechanizes the clean-room-review requirement from ADR-010. Violation of either the provenance-triple completeness or the `content-origin` classification fails `EV-DOCS-CLEANROOM`.

### 13.3 Style linting

- `vale` with a Chromaticity style dictionary enforces:
  - clean-room policy (banned phrases list sourced from `29-cleanroom-legal-policy-baseline.md`).
  - consistent terminology (Snap vs favslot, `compat-syn-v1` vs free-form legacy references, `.chroma` vs reference-app extensions).
  - sentence-length and passive-voice advisories (warn-only, not error).
- Vale runs on changed docs pages in every PR. Unresolved errors fail merge.

### 13.4 Reference page generation

`/reference/*` pages for symbols are generated from `36-reactive-symbol-catalog.md` via the ADR-010 pipeline. Hand-editing a generated reference page is rejected by a CI check that compares the generated artifact to the committed file.

### 13.5 Planning-doc authoring

The existing `chromaticity-planning/` planning docs predate this standard and are not required to carry Diátaxis front-matter. New planning docs SHOULD carry `doc-type: explanation` and `doc-type: reference` as appropriate. ADRs follow the MADR template in Sec.14, not the Diátaxis front-matter above.

## 14. ADR Standard

### 14.1 Template

Chromaticity adopts **MADR 4.0.0**. The canonical template lives at `adr/TEMPLATE.md` and includes:

- Title (`ADR-<NNN>: <short title>`)
- Status (`Proposed | Accepted | Rejected | Deprecated | Superseded by ADR-NNN`)
- Context
- Decision Drivers (optional)
- Considered Options
- Decision
- Consequences (Positive + Negative)
- Alternatives Considered
- Evidence (linked `EV-*` IDs)
- Traceability (back-links to planning docs, risks, and FRs/NFRs)

### 14.2 Numbering and lifecycle

- ADR IDs are zero-padded three-digit integers starting at `001`. Numbers are never reused; a rejected or superseded ADR keeps its number and `Status:` transitions accordingly.
- Superseding ADR MUST link back to the superseded ADR in `Status:` and the superseded ADR MUST be updated in the same PR to say `Superseded by ADR-NNN`.

### 14.3 `adr-lint` check

`adr-lint` runs on PRs that touch `adr/` and verifies:

- filename matches `ADR-<NNN>-<kebab-slug>.md`.
- `Status:` line is present and uses one of the allowed values.
- `Evidence:` and `Traceability:` sections exist (content may be `N/A` with justification).
- For `Superseded by`: the target ADR exists and points back.

### 14.4 ADR ownership

ADR authoring is non-delegable from the Accountable Owner per `23-...md`. ADR PRs auto-assign the AO as reviewer and block merge without AO sign-off plus Independent Architecture Reviewer sign-off.

## 15. Evidence-Tagged Testing Convention

### 15.1 Motivation

`26-evidence-artifact-registry.md` Sec.Packet Minimums requires every evidence artifact to include a run identifier and produce a traceable rollup. Tagging tests at the source eliminates a manual bookkeeping step and makes the evidence roll-up a CI output rather than a hand-assembled document.

### 15.2 Convention

Every test that contributes to an evidence row is tagged with the evidence ID in language-native metadata:

- **Rust**: `#[evidence("EV-DETERMINISM", "EV-RENDER-CONFORMANCE")]` — a trivial Chromaticity-authored attribute macro that emits a metadata entry consumed by the test harness.
- **TypeScript/JavaScript**: test-framework-specific tag, e.g., `test.concurrent("...", { evidence: ["EV-SCRIPT-CONTRACT"] }, async () => { ... })`.
- **GLSL conformance**: fixture metadata sidecar carries `evidence:` list.

### 15.3 Harness output

CI emits `evidence-runs/<commit-sha>.json` listing every `(EV-ID, test-name, status, duration, fixture-set)` tuple from the run. The evidence registry's `Last Passing Run` column is populated from this file automatically on releases.

### 15.4 Enforcement

- `evidence-tag-lint` (required from Phase 1): every test in a subtree whose work package declares evidence outputs (per `12-work-breakdown-estimates.md`) must carry at least one `#[evidence(...)]` tag, or be explicitly marked `#[evidence(none = "rationale")]`.
- Tags referencing evidence IDs not present in `26-...md` fail the lint (catches typos and forgotten registry updates).

## 16. Issue and PR Label Taxonomy

Labels are namespaced and machine-readable. Label vocabulary is fixed; adding a label requires a `policy` commit editing this section.

- `phase:0` … `phase:5`
- `workcell:runtime` | `workcell:scene` | `workcell:audio` | `workcell:integration` | `workcell:qa` | `workcell:ux` | `workcell:docs`
- `profile:compat-syn-v1` | `profile:native-chroma-v1`
- `evidence:EV-<ID>` — attached when the issue/PR changes that evidence row's status
- `risk:R-<NN>`
- `severity:S1` | `severity:S2` | `severity:S3`
- `type:bug` | `type:feat` | `type:perf` | `type:compat` | `type:docs` | `type:security` | `type:chore`
- `architecture` — triggers ADR + Independent Architecture Reviewer requirement (Sec.7.4, Sec.8.2)
- `oversized` — PR exceeds 400-line target; requires AO acknowledgement
- `legal:review-required` — touches licensing, brand, clean-room surface
- `succession-pending` — auto-applied by Sec.8.3 when no eligible reviewer is online

## 17. Security Disclosure

- `SECURITY.md` at repo root declares:
  - coordinated disclosure window: 90 days default, with extension clause.
  - reporting channels: Chromaticity-owned email + an age/PGP public key for encrypted submission.
  - scope (shipped app + release artifacts; does not cover third-party scenes or user-authored JavaScript, which are sandbox-isolated per ADR-012).
  - safe-harbor clause for good-faith reporters.
- Security issues are filed as GitHub advisories, not public issues; advisory disclosure follows the 90-day window unless the reporter requests earlier disclosure.
- Security fixes merge on `main` under the normal PR policy but carry the `type:security` label and a `CHANGELOG.md` entry. Disclosure of the specific mitigation lands with the release that contains the fix, not earlier.

## 18. Repository Naming, Visibility, and Mirror Posture

### 18.1 Name

The code repository is named `chromaticity` (all lowercase). The existing planning corpus continues to live under `chromaticity-planning/` as a subtree of that repo (no history split; preserves the full Phase 0 paper trail).

### 18.2 Visibility

The repo remains **private** through Phase 0 and Phase 1. Rationale: clean-room provenance defense (`EV-LEGAL-PROVENANCE`, `EV-CLEANROOM-POLICY-BASELINE`) is easier when the authorship history is reviewable only by the AO and Deputy until the baseline policy evidence is `Pass`. Flip to **public** no earlier than Phase 4c. The visibility flip requires an AO-signed commit touching this section and a one-line entry under **Compatibility** in the app changelog.

### 18.3 Mirror and backup

- Authoritative host: GitHub (or equivalent; final host is a release-eng decision tracked as a Phase-0 infra item).
- Nightly mirror to an AO-controlled backup (bare clone, encrypted at rest) — documents-only, no secrets.
- Secrets live in a host-native secrets store, not in the repo. `git-secrets` or `gitleaks` runs on every PR as a belt-and-suspenders check.

## 19. New Evidence IDs Introduced

This document introduces the following new evidence rows. They are to be added to `26-evidence-artifact-registry.md` in a paired `policy` PR at the same time this doc is ratified:

| New ID | Purpose | Generation Source | Owner Workcell | Gate Usage | Profile Scope |
|---|---|---|---|---|---|
| `EV-COMMIT-CONVENTION-LINT` | Validate every merged commit on `main` satisfies Sec.4 (type, scope, trailers, signing) | `commitlint` + `dco` + custom trailer lint runs | AI QA/Ops Workcell | Phase 0 exit + release cumulative gate | All profiles |
| `EV-LICENSE-HEADERS` | Validate every source file carries a valid SPDX header per Sec.10.1 | `reuse lint` in CI | AI QA/Ops Workcell | Phase 0 exit + release cumulative gate | All profiles |
| `EV-SBOM-COVERAGE` | Validate SBOM is emitted per release and every shipped dependency appears in it | `cargo-cyclonedx` + `syft` + manifest-diff check | AI QA/Ops Workcell | Release cumulative gate | All profiles |
| `EV-RELEASE-ATTESTATION` | Validate every release artifact carries a signed attestation per Sec.11.2 | `cosign attest` + Rekor verification | Xavier Butler | Release cumulative gate | All profiles |
| `EV-COMPAT-PROFILE-DIFF` | Validate every `compat!` commit produces the profile-diff report + migration note required by `22-...md` | Commit-derived profile diff generator | AI Scene Workcell | Per-release gate when any `compat!` commit is present | Parity profiles |
| `EV-EVIDENCE-TAG-COVERAGE` | Validate every test in an evidence-producing subtree carries an `#[evidence(...)]` tag per Sec.15 | `evidence-tag-lint` | AI QA/Ops Workcell | Phase 1 exit + release cumulative gate | All profiles |
| `EV-ADR-LINT` | Validate ADR directory satisfies Sec.14.3 on every PR that touches it | `adr-lint` | Xavier Butler | Phase 0 exit + continuous | All profiles |

None of these new rows invalidate any prior evidence row. They are additive and enforce the standards this doc creates.

## 20. Conflict Resolution

When a standard in this document conflicts with a rule elsewhere in the corpus, resolve in this order of precedence:

1. Legal/compliance rules in `29-cleanroom-legal-policy-baseline.md` (always wins).
2. Compatibility/versioning policy in `22-compatibility-versioning-policy.md`.
3. Decision-ownership rules in `23-decision-ownership-matrix.md`.
4. Evidence-registry rules in `26-evidence-artifact-registry.md`.
5. ADRs (most recent `Accepted` ADR wins on its topic).
6. This document.
7. Subtree-local conventions (e.g., a lint rule declared in a `README.md` under `/engine/runtime/`).

Subtree-local conventions may **tighten** but not relax a rule from this document. For example, `/engine/runtime/` may forbid a clippy lint that this document merely warns on, but may not allow `HashMap` with default hasher where Sec.12.1 forbids it.

## 21. Phase-0 Ratification Checklist

For this document to move to `Status: Accepted` and its Sec.19 evidence rows to appear in `26-...md`, the following must land in a single `policy` PR:

- This document (`43-development-standards.md`) reviewed and signed off by AO and Deputy.
- `26-evidence-artifact-registry.md` updated with the seven new rows from Sec.19 at status `Planned`.
- `23-decision-ownership-matrix.md` Sec.Decision Matrix adds a "Development standards change" row owned by the Accountable Owner with this doc as the decision packet.
- `11-roadmap-phases.md` Phase 0 exit criteria additions:
  - `EV-COMMIT-CONVENTION-LINT` at `Pass` (CI wired).
  - `EV-LICENSE-HEADERS` at `Pass`.
  - `EV-ADR-LINT` at `Pass`.
  - `EV-EVIDENCE-TAG-COVERAGE` may remain `Planned` through Phase 0 and must reach `Pass` by Phase 1 exit.
- `README.md` Document Map updated with the `43-development-standards.md` entry.

Once the above lands, the repo-bootstrap work package (creating `.editorconfig`, `rustfmt.toml`, `deny.toml`, `REUSE.toml`, `LICENSES/`, `.github/` templates, `CODEOWNERS`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `lefthook.yml`, `adr/TEMPLATE.md`, the `release-please` config, and the empty per-axis `CHANGELOG.md` files) can proceed. That work package implements this standard; it does not modify it.

## 22. Change Control

Amendments to this document are `policy`-scoped PRs authored by the AO. Material changes (adding or removing a Sec.5 versioned axis, changing the Sec.4 scope vocabulary, altering Sec.9 signing requirements, changing Sec.10 license allowlist) require:

- Independent Architecture Reviewer sign-off per `23-...md`.
- A `CHANGELOG.md` entry under **Compatibility** against the `repo` scope.
- Re-ratification checklist per Sec.21 for any new `EV-*` rows introduced.

Editorial changes (clarifying wording, fixing cross-references, adding examples) do not require Independent Reviewer sign-off and may be merged by the AO or Deputy under normal PR policy.

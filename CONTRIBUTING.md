<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Contributing to Chromaticity

Chromaticity is currently a closed two-human project (Accountable Owner and
Deputy per `docs/23-decision-ownership-matrix.md`). External pull requests
are not accepted during Phase 0 and Phase 1. This document is the internal
contribution standard and is binding on every commit that lands on `main`.

This document is a **summary** of the enforceable subset of
`docs/43-development-standards.md`. When the two conflict, the standards
doc wins.

## TL;DR

1. Use Conventional Commits 1.0.0 with a Chromaticity scope (§Commit Format).
2. Sign off every commit (`git commit -s`). DCO 1.1.
3. Cryptographically sign every commit that lands on `main` (§Signing).
4. Every PR carries a `Profile-Impact:` trailer and, for architecture-
   impacting changes, `ADR:` + `Evidence:` trailers.
5. Squash-merge only. PRs target ≤400 lines net of generated code.
6. Every source file carries an SPDX header.
7. Security issues: private via `SECURITY.md`, never public GitHub issues.

## Developer Certificate of Origin

Every commit to Chromaticity MUST include a `Signed-off-by:` trailer per the
Developer Certificate of Origin 1.1, the full text of which is at
<https://developercertificate.org/>. Signing off is a statement that you
authored the change (or have the right to submit it) under the repository
license. Use `git commit -s` to append this automatically.

Chromaticity does not require a separate Contributor License Agreement.
DCO + clean-room provenance (ADR-010 page-level provenance) + SPDX headers
together produce the provenance story `EV-LEGAL-PROVENANCE` needs.

## Commit Format

Normative form (§4.1 of `docs/43-development-standards.md`):

    <type>[!](<scope>): <subject>

    [body, wrapped at 100 cols]

    [trailers]

### Types

Accepted types (others rejected by `commitlint`):

| Type       | Meaning                                                      | SemVer  |
| ---------- | ------------------------------------------------------------ | ------- |
| `feat`     | New end-user-visible capability                              | minor   |
| `fix`      | Bug fix                                                      | patch   |
| `perf`     | Performance change with measured impact on `EV-PERF-BUDGETS` | patch   |
| `refactor` | Internal restructure, no behavior change                     | none    |
| `docs`     | Documentation-only change                                    | none    |
| `test`     | Test-only change                                             | none    |
| `build`    | Build scripts, toolchain, workspace plumbing                 | none    |
| `ci`       | CI pipeline, runner config, gate automation                  | none    |
| `chore`    | Housekeeping with no product effect                          | none    |
| `revert`   | Revert a prior commit (must reference commit SHA)            | inverse |

Major bumps are driven exclusively by `!` or `BREAKING CHANGE:` footer.

### Scopes (mandatory for feat/fix/perf/refactor)

`runtime`, `scene`, `audio`, `integration`, `authoring`, `library`, `ui`,
`docs`, `ux`, `entitlement`, `observability`, `release-eng`, `policy`,
`repo`. Adding a scope is a `policy` commit that edits §4.3 of
`docs/43-development-standards.md`.

### Mandatory trailers

Every merged commit on `main`:

    Signed-off-by: <Full Name> <email>
    Profile-Impact: none | compat-syn-v1-minor | compat-syn-v1-major | native-chroma-v1-minor | native-chroma-v1-major

Architecture-impacting commits additionally carry:

    ADR: ADR-NNN
    Evidence: EV-<ID>[, EV-<ID>...]

Commits closing a row in `docs/13-risk-register.md` SHOULD carry:

    Risk: R-NN

Succession merges under T1/T2 carry `Succession-Waiver: T1` or
`Succession-Waiver: T2` per `docs/43-development-standards.md` §7.6.

### Subject rules

- Imperative mood: "add", "fix", "remove" — not "added"/"fixed".
- ≤72 characters, no trailing period.
- Subject matches the squash-merge commit's subject because PR-title lint
  is authoritative.

### Examples

    feat(runtime): implement deterministic render-graph dispatcher

    Profile-Impact: none
    ADR: ADR-001
    Evidence: EV-DETERMINISM, EV-RENDER-CONFORMANCE
    Signed-off-by: Xavier Butler <xjbutler@gmail.com>

    feat(scene)!: drop legacy favslot alias on /favslot/{n}/save

    BREAKING CHANGE: /favslot/{n}/save removed; use /favslots/{n}/save.
    Profile-Impact: compat-syn-v1-major
    ADR: ADR-005
    Evidence: EV-COMPAT-PROFILE-DIFF, EV-INTEGRATION-MATRIX
    Signed-off-by: Xavier Butler <xjbutler@gmail.com>

## Branching

- `main` is the only release-eligible branch.
- Feature branches: `<initials>/<scope>/<short-kebab-slug>`
  (e.g. `xb/runtime/render-graph-scaffold`). ≤5 working days unmerged.
- No direct pushes to `main`.
- **Squash-merge only.** No merge commits, no rebase-merge.

## Pull Requests

Every PR description includes, in this order (see §7.5):

1. **Summary** — one paragraph, plain prose.
2. **Linked ADR** — `ADR-NNN` or explicit `N/A — non-architectural`.
3. **Evidence impact** — `EV-*` rows whose status may change; `none` allowed.
4. **Profile impact** — matches the commit trailer value.
5. **Docs impact** — whether ADR-010 coverage changes; which
   `/reference|manual|guides|faq|resources` pages need regeneration.
6. **Risk** — referenced `R-*` IDs (optional).
7. **Checklist** — SPDX headers, changelog entry, migration note if required.

Use the `.github/pull_request_template.md` template; it matches this list.

### Size

Target ≤400 lines net of generated code. Oversized PRs require the
`oversized` label and AO acknowledgement comment before merge.

### Review

- Non-architecture: author + one reviewer (auto-merge allowed).
- Architecture-impacting (`architecture` label): author + AO sign-off **and**
  Independent Architecture Reviewer sign-off with `[INDEPENDENT-ARCH-REVIEW]`
  tag in the review comment. Auto-merge disabled.

## Signing

All commits on `main` MUST be cryptographically signed (§9.1). Preferred:

    # Sigstore keyless signing (preferred)
    brew install sigstore/tap/gitsign
    git config --global gpg.x509.program gitsign
    git config --global gpg.format x509
    git config --global commit.gpgsign true
    git config --global tag.gpgsign true

GPG (long-lived key) or SSH signing are accepted fallbacks. Release tags
are signed with `git tag -s` by the Accountable Owner.

## License, SPDX, REUSE

Every source file carries an SPDX header:

    SPDX-License-Identifier: <license-id>
    SPDX-FileCopyrightText: <year> <copyright holder>

Source-code comment syntax varies; YAML/TOML/shell use `#`, Rust/TS/GLSL/
CSS use `//` or `/* */`, Markdown/HTML uses `<!-- -->`.

`reuse lint` runs in CI. Missing headers fail merge.

### Allowed license IDs

See `docs/29-cleanroom-legal-policy-baseline.md` "Allowed-License Policy
Baseline". Unknown IDs are blocked pending legal review. Adding an ID
requires a `policy` commit editing both `deny.toml` (when added) and the
clean-room policy.

## Tooling

Full detail: `docs/43-development-standards.md` §12.

- **Rust**: Rust 2024, `rustfmt`, `cargo clippy -D warnings`,
  workspace-wide `unsafe_code = "forbid"`,
  no `HashMap` default hasher in `EV-DETERMINISM` paths.
- **TypeScript**: `strict: true`, `noUncheckedIndexedAccess: true`,
  `exactOptionalPropertyTypes: true`, `prettier` default config,
  `eslint` with `@typescript-eslint/recommended`.
- **Svelte**: `svelte-check` required; `eslint-plugin-svelte`.
- **GLSL**: `glslangValidator`; reactive uniform names validated against
  `docs/36-reactive-symbol-catalog.md`.
- **Scene JS (QuickJS)**: ES2020 subset only, no build step,
  `eslint-plugin-chromaticity-sandbox` enforces host-API whitelist.

Pre-commit hooks (lefthook) run the fast subset locally. See
`lefthook.yml` for the current hook graph and command set.

## Evidence Tags

Tests that contribute to an evidence row carry a language-native tag:

- Rust: `#[evidence("EV-DETERMINISM", "EV-RENDER-CONFORMANCE")]`
- TS/JS: `test("...", { evidence: ["EV-SCRIPT-CONTRACT"] }, ...)`
- GLSL: fixture sidecar `evidence:` list.

`evidence-tag-lint` is required from Phase 1 onward.

## Security

Do not open public issues for security matters. Follow
[`SECURITY.md`](SECURITY.md).

## Questions

Direct questions to the Accountable Owner (see `CODEOWNERS`). Changes to
this document are `policy`-scoped PRs authored by the AO per
`docs/43-development-standards.md` §22.

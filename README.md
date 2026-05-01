<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Chromaticity

Chromaticity is a deterministic, audio-reactive live-visuals runtime and authoring
environment targeting macOS and Windows. This repository is **private through Phase 0
and Phase 1** per `docs/43-development-standards.md` §18.2. Flip to public is
scheduled no earlier than Phase 4c and requires AO-signed approval.

## Status

**Phase 1 EXIT achieved; Phase 2 evidence deepening in flight.** The
repository contains active application and engine code (Tauri 2 + Svelte 5 +
TypeScript + Rust 2024), with policy, evidence, and CI gates running
continuously on `main`. All six Phase 1 exit-gate evidence rows are `Pass`
per `docs/26-evidence-artifact-registry.md` (`EV-DETERMINISM`,
`EV-OBSERVABILITY`, `EV-EVIDENCE-TAG-COVERAGE`, `EV-COMPAT-FIXTURE-COVERAGE`,
`EV-SCENE-IMPORT`, `EV-SCHEMA-LINT` — sixth gate closed at commit `e818752`,
W12 close at `0d83fb5`). Phase 0 governance and clean-room baseline ADRs are
established; the late-April Phase-0 closure tail landed
`EV-DOCS-PHASE0-SEED`, `EV-ENTITLEMENT-TOKEN-FORMAT`, and
`EV-GATE-MODEL-AUDIT`, with `EV-OBSERVABILITY-INVENTORY` remaining
implementation-gated on the first release artifact and the UX-bundle and
hotkey-parity rows remaining AO-gated on the dev-team queue. Current work
advances Phase 2 evidence-row deepening across the four foundational rows
(`EV-RENDER-CONFORMANCE`, `EV-SCRIPT-CONTRACT`, `EV-AUDIO-TRACE`,
`EV-SANDBOX-SAFETY`) toward EXIT-quality coverage.

## Repository Layout

```
chromaticity/
├── README.md
├── CHANGELOG.md                 # release-please-managed release history
├── LICENSE                    # see §License below
├── LICENSES/                  # REUSE 3.2 canonical license texts
├── REUSE.toml                 # per-glob license + copyright defaults
├── CONTRIBUTING.md            # commit/branch/review conventions
├── CODE_OF_CONDUCT.md
├── SECURITY.md                # coordinated disclosure
├── CODEOWNERS                 # subtree review authority
├── Cargo.toml                 # Rust workspace root
├── Cargo.lock                 # locked Rust dependency graph
├── package.json               # frontend workspace scripts and tooling
├── .editorconfig
├── .gitattributes
├── .gitignore
├── .github/                   # PR/issue templates, labels, workflows
├── app/                       # Tauri app shell + app-facing modules
├── engine/                    # runtime, scene, audio, observability, render, script, entitlement crates
├── integrations/              # external protocol adapters (OSC, etc.)
├── scripts/                   # policy/evidence automation helpers
├── docs/                      # normative planning corpus (promoted)
│   ├── README.md              # document map
│   └── adr/                   # Architecture Decision Records (MADR 4.0.0)
├── src/                       # Svelte UI source
└── tests/                     # cross-cutting test harnesses
```

### AO-local partitions (not in version control)

Two partitions live on the Accountable Owner's and technical lead's
workstations only and are excluded from version control via `.gitignore`.
They are tracked normatively in `docs/43-development-standards.md` §18.1
and `docs/29-cleanroom-legal-policy-baseline.md` §Corpus Partitioning,
which together carry the access boundary, origin tagging, and clean-room
promotion path:

- `chromaticity-planning/` — Phase-0 planning corpus paper trail
  (review rounds, intake audits, deep-dive findings, scoring matrices,
  pre-dev UX bundles, and the `ERRATA-v0.2-beta.md` record).
- `Contents/` — reference-app materials consulted for feature-coverage
  and quality-parity verification under the clean-room SOP.

Neither partition is accessible to the dev-team clean-room working
surface. Content promoted into the shipped corpus (`docs/`, `engine/`,
`integrations/`, etc.) follows the clean-room reconstruction protocol in
`docs/29-cleanroom-legal-policy-baseline.md` and is authored
Chromaticity-originated, never copied verbatim.

## Document Map

See [`docs/README.md`](docs/README.md) for the full normative document map,
including Architecture Decision Records under [`docs/adr/`](docs/adr/).

Key entry points for new contributors:

- `docs/01-charter.md` — product mission and scope.
- `docs/03-product-requirements.md` — FRs and NFRs.
- `docs/11-roadmap-phases.md` — phased delivery.
- `docs/23-decision-ownership-matrix.md` — who decides what.
- `docs/26-evidence-artifact-registry.md` — `EV-*` evidence rows and gates.
- `docs/29-cleanroom-legal-policy-baseline.md` — clean-room SOP and legal posture.
- `docs/43-development-standards.md` — commit/branch/release/lint/CI standards.
- `docs/adr/` — every architecture decision of record.

## How to Contribute

Chromaticity is currently a closed two-human team (Accountable Owner + Deputy
per `docs/23-decision-ownership-matrix.md`). External contribution is **not
open** at this time. Internal contributors: read `CONTRIBUTING.md` end-to-end
before the first commit. In particular:

- All commits are **Conventional Commits 1.0.0** (`docs/43-development-standards.md` §4).
- All commits require `Signed-off-by` (DCO 1.1).
- All commits on `main` are cryptographically signed (Sigstore `gitsign` preferred).
- The `Profile-Impact:` trailer is mandatory on every merged commit.

## Security

Report security issues privately per [`SECURITY.md`](SECURITY.md). Do not open
public GitHub issues for security matters.

## License

Chromaticity source code is proprietary: **All Rights Reserved** under
`LicenseRef-Chromaticity-Proprietary` (see [`LICENSE`](LICENSE) and
[`LICENSES/LicenseRef-Chromaticity-Proprietary.txt`](LICENSES/LicenseRef-Chromaticity-Proprietary.txt)).

Planning and documentation under `docs/` is licensed `CC-BY-4.0` unless a
file's SPDX header states otherwise. Bundled third-party material (if and
when it ships) is governed by its own license, captured under
`/third_party/<origin>/` with an `ATTRIBUTION.md` and reflected in
`REUSE.toml`.

REUSE 3.2 compliance is enforced in CI (`reuse lint`). See
`docs/43-development-standards.md` §10 for the full license/SPDX/SBOM policy.

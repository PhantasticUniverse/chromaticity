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

**Phase 1, Phase 2, and Phase 3 EXIT all achieved.** The repository carries
active application and engine code (Tauri 2 + Svelte 5 + TypeScript +
Rust 2024) with policy, evidence, and CI gates running continuously on
`main`. Phase 1 EXIT closed at commit `e818752` (W12) with all six Phase 1
gate evidence rows `Pass` per `docs/26-evidence-artifact-registry.md`.
Phase 2 EXIT closed 2026-05-01 at commit `d8e92db` with the four
foundational rows (`EV-RENDER-CONFORMANCE`, `EV-SCRIPT-CONTRACT`,
`EV-AUDIO-TRACE`, `EV-SANDBOX-SAFETY`) `Pass` against a green workspace
nextest run. Phase 3 EXIT closed 2026-05-01 at commit `953fd74` with
seven of the eight Phase-3 gate rows `Pass` (`EV-AUDIO-TRACE`
re-validated, `EV-AUDIO-ROUTING-MATRIX`, `EV-STATE-REPLAY`,
`EV-STATE-MIGRATION`, `EV-PRESET-PLAYLIST`, `EV-SHOW-BUNDLE-RECOVERY`,
`EV-PERF-BUDGETS`); the eighth row `EV-COLOR-MANAGEMENT-SRGB` (row 38)
is explicitly deferred per AO direction sequenced behind a future
render-HUD-diagnostics PR. Phase 0 governance and clean-room baseline
ADRs remain established; Phase 4+ entry is a separate AO-held milestone
per `docs/11-roadmap-phases.md`.

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
├── engine/                    # authoring, runtime, scene, audio, observability, render, script, entitlement crates
├── integrations/              # external protocol adapters (OSC, MIDI, etc.)
├── scripts/                   # policy/evidence automation helpers
├── docs/                      # normative planning corpus (promoted)
│   ├── README.md              # document map
│   └── adr/                   # Architecture Decision Records (MADR 4.0.0)
├── src/                       # Svelte UI source
└── tests/                     # workspace-level policy fixtures (e.g. trailer-check inputs)
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
file's SPDX header states otherwise. Bundled third-party material is
governed by its own license, captured under `/third_party/<origin>/` with
an `UPSTREAM-VERSION.md` (origin URL + version stamp + drop date), the
upstream license file (e.g. `LICENSE.md`), and any upstream `README` —
all reflected in `REUSE.toml`. The current vendor drop is
`third_party/lygia/` (Lygia v1.1.4 trial, license election deferred per
`docs/13-risk-register.md` R-37).

REUSE 3.2 compliance is enforced in CI (`reuse lint`). See
`docs/43-development-standards.md` §10 for the full license/SPDX/SBOM policy.

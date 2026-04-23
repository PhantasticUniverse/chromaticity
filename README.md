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

**Phase 0 — Governance Bootstrap.** No application source has landed yet. The first
commit stream establishes the legal, provenance, and review guardrails required for
clean-room-defensible implementation. Scaffold work (Tauri 2 + Svelte 5 + Rust
workspace per `docs/adr/ADR-018-non-scene-ui-toolkit.md` and
`docs/adr/ADR-011-rendering-backend.md`) begins after Phase 0 policy gates pass.

## Repository Layout

```
chromaticity/
├── README.md
├── LICENSE                    # see §License below
├── LICENSES/                  # REUSE 3.2 canonical license texts
├── REUSE.toml                 # per-glob license + copyright defaults
├── CONTRIBUTING.md            # commit/branch/review conventions
├── CODE_OF_CONDUCT.md
├── SECURITY.md                # coordinated disclosure
├── CODEOWNERS                 # subtree review authority
├── .editorconfig
├── .gitattributes
├── .gitignore
├── .github/                   # PR/issue templates, labels, workflows
├── docs/                      # normative planning corpus (promoted)
│   ├── README.md              # document map
│   └── adr/                   # Architecture Decision Records (MADR 4.0.0)
└── chromaticity-planning/     # ARCHIVE-ONLY; gitignored; see below
```

### Archive-only planning corpus

`chromaticity-planning/` contains review rounds, intake audits, deep-dive
findings, scoring matrices, pre-dev UX bundles, and the `ERRATA-v0.2-beta.md`
record. It is **intentionally excluded from production history** via
`.gitignore` and lives on the AO/Deputy's disk for clean-room review only.
Normative content from that corpus has been promoted into `docs/` in this
commit stream; the rest is historical and does not ship.

> **Deviation note vs `docs/43-development-standards.md` §18.1.** The
> standards doc anticipated the planning corpus living in-tree as
> `chromaticity-planning/`. The kickoff plan (ratified 2026-04-23) adopted
> the split-and-archive strategy described above. A `policy`-scoped
> amendment to §18.1 will land in a subsequent PR; `chromaticity-planning/`
> remains ignored in the interim.

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

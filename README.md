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

**Phases 1 through 4a EXIT achieved; Phase 4b passable at AO discretion;
Phase 4d at 5 of 6.** The repository carries active application and
engine code (Tauri 2 + Svelte 5 + TypeScript + Rust 2024) with policy,
evidence, and CI gates running continuously on `main`. Phased rollup per
`docs/26-evidence-artifact-registry.md`:

- **Phase 1 EXIT** — closed at commit `e818752` (W12, April 2026) with
  all six gate evidence rows `Pass`.
- **Phase 2 EXIT** — closed 2026-05-01 at commit `d8e92db` with the four
  foundational rows (`EV-RENDER-CONFORMANCE`, `EV-SCRIPT-CONTRACT`,
  `EV-AUDIO-TRACE`, `EV-SANDBOX-SAFETY`) `Pass` against a green
  workspace nextest run.
- **Phase 3 EXIT** — closed 2026-05-01 at commit `953fd74` with seven of
  the eight Phase-3 gate rows `Pass`; the eighth
  (`EV-COLOR-MANAGEMENT-SRGB`) is explicitly deferred per AO direction
  sequenced behind a future render-HUD-diagnostics PR.
- **Phase 4a EXIT** — achieved 2026-05-03. Wave 1c-α-2 production audio
  bootstrap closure landed 2026-05-06; Wave 1c-β camera + Syphon
  adapter cluster closure landed 2026-05-07 under synthetic-narrowing
  (per-platform concrete bindings + hardware validation deferred as
  known follow-ups).
- **Phase 4b EXIT** — passable at AO discretion as of 2026-05-07; all
  eleven Phase-4b rows at `Pass` under collective narrowings (Wave 4b.1
  through 4b.8 inclusive).
- **Phase 4c EXIT** — remains AO-held.
- **Phase 4d** — five of six rows at `Pass`; `EV-ANALYTICS-ZERO-DEFAULT`
  calendar-gated to ~2026-05-20 closes the sixth.

V1-RC content closure (Wave 1d: in-app help corpus + bundled scene
starter library) and Phase-5 release pipeline scaffold (Wave 1e: signing
script stubs + `tauri-plugin-updater` + `release.yml` workflow) both
landed on `main` 2026-05-10 (PRs `#425`-`#430`). `EV-DOCS-CORPUS` and
`EV-LIBRARY-CORPUS-MIN` reached `Pass` (11 corpus pages bundled via
`docs-corpus/` mkdocs source tree; 12 `.chroma` scenes spanning five
feature surfaces under `engine/scene/tests/fixtures/scenes/`); since
this morning's fanout `EV-PERF-BUDGETS` also carries forward at `Pass`
under the per-bench 2x loose-floor + composite-sum 1.25 ms hard-budget
posture. The `release.yml` workflow validated end-to-end via a
synthetic-signing smoke test (run `25628685936`); production cert
procurement remains AO-gated and `EV-RELEASE-PIPELINE` stays at
`Planned` until then. Phase 0 governance and clean-room baseline ADRs
remain authoritative throughout.

Pre-alpha smoke testing on 2026-05-11 surfaced that ~60% of cross-seam
IPC was gapped despite the v0.2-beta UI port being wireframe-faithful
complete. Wave 2.1 (2026-05-11) closed the gap in a single day across
11 PRs (`#435`-`#445`) covering scene load + control dispatch + snap +
preset / playlist + Perform STANDARD/CONSOLE variant + top bar HUD +
Settings TOML persistence + MIDI device list + learn-mode capture +
OSC route inspection. Wave 2.2 R.1 (`#448`) then wired the v0.2-beta
bundle's 16 procedural Canvas2D scene renderers into the Perform /
Library / Playlist / Author preview panes; Wave 2.3 Q.1 + Q.2
(`#449` / `#450`) fixed ResizeObserver flicker + DPR-on-retina +
audio-gate widening and brought Perform's STANDARD layout to
wireframe fidelity. Wave 2.4 (2026-05-13) shipped page-by-page
wiring solidification on the Perform page across 6 PRs (`#451`-`#456`)
adding action buttons (Default / Random / Lock-all) + scene-list
click-to-load + `Command::PresetSave` engine surface + adapter-pill
liveness + media-tab filter + SnapBar slot labels with right-click
rename. Wave 2.5 (2026-05-13) followed on the Library page across 5
PRs (`#457`-`#461`) adding card SceneStill thumbnails + favorites + a
source / sort / recent filter axis + the canonical bundle sidebar
preview pane with Open-in-Author handoff + empty-state polish + a
double-click → Perform handoff. The `DESIGN.md` token contract was
reconciled against the rendered shell post-Wave-2.4 (`#462`). Wave 2.6
(Audio page) is in progress at the time of this refresh; subsequent
pages (Settings, MIDI, OSC, Sources, Playlist, Author, Patch, Import,
License, Help, Welcome) follow the same playbook.

## Repository Layout

```
chromaticity/
├── README.md
├── CHANGELOG.md                 # release-please-managed release history
├── LICENSE                      # see §License below
├── LICENSES/                    # REUSE 3.2 canonical license texts
├── REUSE.toml                   # per-glob license + copyright defaults
├── CONTRIBUTING.md              # commit/branch/review conventions
├── CODE_OF_CONDUCT.md
├── SECURITY.md                  # coordinated disclosure
├── CODEOWNERS                   # subtree review authority
├── Cargo.toml                   # Rust workspace root
├── Cargo.lock                   # locked Rust dependency graph
├── rust-toolchain.toml          # MSRV pin (1.85.0)
├── deny.toml                    # cargo-deny advisories / licenses / bans / sources
├── package.json                 # frontend workspace scripts and tooling
├── lefthook.yml                 # pre-commit / pre-push hooks
├── commitlint.config.cjs        # Conventional Commits enforcement
├── vitest.config.ts             # Vitest runner (jsdom + Svelte plugin)
├── .editorconfig
├── .gitattributes
├── .gitignore
├── .github/                     # PR/issue templates, labels, workflows
├── app/
│   ├── shell/                   # Tauri 2 binary (chromaticity-app crate)
│   └── ui-bridge/               # ts-rs IPC type generator (Rust → TypeScript bindings)
├── engine/                      # runtime, scene, audio, authoring, observability, render, script, entitlement
├── integrations/                # osc-adapter, midi-adapter, camera-adapter, syphon-adapter
├── workspace-hack/              # cargo-hakari managed workspace dep unifier
├── release-eng/                 # signing/notarization/staging-appcast scripts (stubbed pre-cert)
├── docs-corpus/                 # in-app help system mkdocs source tree
├── dist/                        # Vite build output + bundled mkdocs help pages (dist/help/)
├── third_party/                 # vendored snapshots (lygia trial vendor + Syphon-Framework)
├── scripts/                     # policy/evidence automation helpers
├── evidence-runs/               # evidence-run capture artifacts (per-row anchor bundles)
├── docs/                        # normative planning corpus (promoted)
│   ├── README.md                # document map
│   └── adr/                     # Architecture Decision Records (MADR 4.0.0)
├── src/                         # Svelte UI source
└── tests/                       # workspace-level policy fixtures (e.g. trailer-check inputs)
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
all reflected in `REUSE.toml`. Current vendor drops:

- `third_party/lygia/` — Lygia v1.1.4 trial vendor; license election
  deferred per `docs/13-risk-register.md` R-37.
- `third_party/Syphon-Framework/` — BSD-3-Clause clean-room reference
  for the macOS Syphon producer/consumer surface (`integrations/syphon-adapter`).

REUSE 3.2 compliance is enforced in CI (`reuse lint`). See
`docs/43-development-standards.md` §10 for the full license/SPDX/SBOM policy.

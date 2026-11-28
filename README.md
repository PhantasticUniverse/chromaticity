<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Chromaticity

Chromaticity is a deterministic, audio-reactive live-visuals runtime, broadcast
compositor, and authoring environment targeting macOS and Windows. This
repository is **source-visible** under `LicenseRef-Chromaticity-Proprietary`
(All Rights Reserved; see [`LICENSE`](LICENSE) and [`NOTICE.md`](NOTICE.md)).

## Prerequisites & Quickstart

### Runtime Dependencies (for YouTube Visual Loops & Live Twitch Streaming)

- **Node.js 20+** and **Rust 1.85+** (edition 2024)
- **macOS (Homebrew):**
  ```bash
  brew install ffmpeg yt-dlp
  ```
- **macOS Permissions (System Settings → Privacy & Security):**
  - **Microphone** — required for live audio reactivity, BPM/FFT analysis, and DJ mixer/audio interface capture.
  - **Camera** — required for live Webcam / Chroma Key / Luma Key layers.
  - **Screen & System Audio Recording** — required for Screen Capture layers and built-in System Audio loopback.

### Run Locally

```bash
npm install
npm run dev:doctor     # check Node, Rust, hooks, and optional CI tools
npx lefthook install   # install pre-commit / commit-msg / pre-push hooks
npm run tauri dev
```

Run fast verification gates before pushing:

```bash
npm run verify:fast
```

### Build Standalone Desktop Bundle

Requires `mkdocs` (`pip install mkdocs-material`) if rebuilding the in-app help corpus
(`resources/help/` is decoupled from Vite's `dist/` so `npm run build` never clobbers help):

```bash
npm run build:help
npx tauri build --bundles app
```

## Status

Chromaticity is in **active pre-alpha** (Tauri 2 + Svelte 5 + TypeScript + Rust 2024) with
continuous CI policy, evidence, and accessibility gates. Authoritative gate status lives in
[`docs/governance/26-evidence-artifact-registry.md`](docs/governance/26-evidence-artifact-registry.md);
historical Phase 1–4 / Wave 1–2.8 notes are archived under [`docs/archive/`](docs/archive/).

## Repository Layout

```
chromaticity/
├── README.md
├── AGENTS.md                    # canonical AI agent operating guide (CLAUDE.md -> AGENTS.md)
├── CHANGELOG.md                 # release history
├── LICENSE                      # LicenseRef-Chromaticity-Proprietary
├── LICENSES/                    # REUSE 3.2 canonical license texts
├── REUSE.toml                   # per-glob license + copyright defaults
├── CONTRIBUTING.md              # commit/branch/review conventions
├── CODE_OF_CONDUCT.md
├── SECURITY.md                  # coordinated disclosure
├── CODEOWNERS                   # subtree review authority (root-pinned by policy CI)
├── Cargo.toml / Cargo.lock      # Rust workspace root (edition 2024, MSRV 1.85.0)
├── package.json / vite.config.ts / vitest.config.ts / eslint.config.js / lefthook.yml
├── app/
│   ├── shell/                   # Tauri 2 binary (chromaticity-app crate)
│   └── ui-bridge/               # ts-rs IPC type generator (Rust -> TypeScript bindings)
├── engine/                      # runtime, scene, audio, authoring, observability, render, script, entitlement
├── integrations/                # osc-adapter, midi-adapter, camera-adapter, syphon-adapter
├── src/                         # Svelte 5 + TypeScript frontend
├── resources/
│   └── help/                    # built MkDocs help corpus bundled by Tauri (decoupled from dist/)
├── dist/                        # ephemeral Vite build output (only .gitkeep tracked)
├── docs/                        # unified documentation root (see docs/README.md)
│   ├── adr/                     # Architecture Decision Records (MADR 4.0.0)
│   ├── user/                    # Diátaxis in-app help corpus MkDocs source
│   ├── architecture/            # active engine, runtime, symbol, and design-system specs
│   ├── governance/              # policies, standards, evidence registry, checklists, evidence-runs
│   ├── research/                # non-normative comparison and sourcing memos
│   └── archive/                 # completed phase/wave snapshots
├── release-eng/                 # signing, notarization, and appcast staging scripts
├── scripts/                     # policy/evidence linters, dev-doctor, verify-fast, trailer fixtures
├── third_party/                 # vendored snapshots (cpal-0.17.1-patched, hg_sdf, lygia, Syphon-Framework)
└── workspace-hack/              # cargo-hakari managed workspace dep unifier
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

- `third_party/cpal-0.17.1-patched/` — Apache-2.0 vendored `cpal` 0.17.1 with
  macOS CoreAudio Process Tap unmuted loopback patch.
- `third_party/hg_sdf/` — MIT (elected over CC-BY-NC-4.0) Mercury Demogroup
  GLSL SDF library.
- `third_party/lygia/` — Lygia v1.1.4 trial vendor (`LicenseRef-Prosperity-3.0.0`);
  commercial license election deferred per `docs/13-risk-register.md` R-37.
- `third_party/Syphon-Framework/` — BSD-3-Clause clean-room reference
  for the macOS Syphon producer/consumer surface (`integrations/syphon-adapter`).

See [`NOTICE.md`](NOTICE.md) for full third-party attribution.

REUSE 3.2 compliance is enforced in CI (`reuse lint`). See
`docs/43-development-standards.md` §10 for the full license/SPDX/SBOM policy.

<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Chromaticity Documentation

This directory is the **normative** documentation set for Chromaticity:
the planning corpus, policy baselines, Architecture Decision Records,
and specifications that the product is built against. Everything here
is production-facing and change-controlled. Historical artifacts
(review rounds, intake audits, deep-dive findings, pre-dev UX bundles)
are archive-only and live outside the repo history per the root
`README.md`.

## Structure

```
docs/
├── README.md                     # you are here
├── 01-*.md … 51-*.md             # numbered planning and policy docs
└── adr/                          # Architecture Decision Records (MADR 4.0.0)
    ├── TEMPLATE.md
    └── ADR-001-*.md … ADR-018-*.md
```

## Reading Order for New Contributors

1. `01-charter.md` — product mission, scope, success metrics.
2. `03-product-requirements.md` — FRs and NFRs.
3. `11-roadmap-phases.md` — phased delivery plan.
4. `23-decision-ownership-matrix.md` — who decides what.
5. `43-development-standards.md` — commit / branch / release / lint / CI
   standards. Binding on every commit.
6. `adr/` — every architecture decision, in order.

## Normative Document Map

### Charter, Requirements, and Roadmap
- `01-charter.md` — product mission, scope, and success metrics.
- `02-synesthesia-contracts.md` — observed compatibility and runtime contracts.
- `03-product-requirements.md` — functional and non-functional requirements.
- `11-roadmap-phases.md` — phased delivery plan.
- `12-work-breakdown-estimates.md` — work packages, ownership, and sequence.
- `13-risk-register.md` — top risks and mitigation plans.

### Architecture and Subsystem Specs
- `04-runtime-architecture.md` — runtime topology and orchestration.
- `05-scene-system.md` — scene package, schema, compiler, lifecycle.
- `06-audio-engine.md` — audio analysis and uniform generation model.
- `07-control-state-presets.md` — control state, presets, playlist semantics.
- `08-io-integrations.md` — OSC, MIDI, camera, Syphon/Spout/NDI boundaries.
- `36-reactive-symbol-catalog.md` — normative reactive-uniform, meta-control,
  GLSL-helper, and script-host mirror catalog.
- `37-authoring-environment-spec.md` — in-app authoring environment contract.
- `38-scene-library-and-import-spec.md` — scene library + import pipeline.
- `41-data-ingestion-differentiator.md` — post-MVP real-time data ingestion
  reservation (activated by a future ADR-017).
- `42-operator-hotkey-map.md` — operator hotkey table.

### Quality, Performance, and Operations
- `09-test-strategy.md` — quality strategy and regression harness.
- `10-performance-latency-budgets.md` — measurable performance targets.
- `21-operational-slos-slis.md` — SLOs and SLIs.
- `30-mvp-support-matrix.md` — supported environment classes and benchmarks.
- `31-supportability-privacy-ops-policy.md` — support-bundle and privacy policy.
- `34-golden-corpus-definition.md` — canonical golden corpus coverage.

### Traceability, Evidence, and Governance
- `20-requirements-traceability-matrix.md` — requirement → owner → evidence.
- `23-decision-ownership-matrix.md` — accountable ownership matrix.
- `24-experiment-protocol-template.md` — standard experiment-protocol template.
- `26-evidence-artifact-registry.md` — `EV-*` evidence IDs and phase-gate rows.
- `28-implementation-contract-pack.md` — implementation contract baseline.

### Compatibility, State, and Migration
- `22-compatibility-versioning-policy.md` — compatibility profiles; versioning.
- `32-state-preset-playlist-migration-policy.md` — state / preset / playlist
  migration and versioning.

### Legal, Clean-Room, and Policy
- `29-cleanroom-legal-policy-baseline.md` — clean-room SOP; legal posture.
- `51-ux-bundle-legal-check.md` — icon and font SPDX/licensing sweep.

### Development Standards
- `43-development-standards.md` — repo-level engineering standards
  (Conventional Commits, per-axis SemVer, Keep a Changelog, trunk-based
  branching, Diátaxis docs mapping, MADR ADRs, SPDX/REUSE/SBOM, signed
  commits + DCO, release attestation, evidence-tagged tests).

### UX Contracts and Design Tokens
- `45-design-token-mapping.md` — reconciliation of UX bundle tokens to the
  ADR-018 locked slot set; Chromaticity product-tier token layer.
- `47-ui-port-plan.md` — JSX reference mock → Tauri 2 + Svelte 5 + TS +
  CodeMirror 6 port plan.
- `48-ux-bundle-open-questions-routing.md` — routing ledger for
  `HANDOFF.md` open design questions.
- `49-ux-bundle-supplement-scope.md` — v0.2-beta supplement commissioning
  brief (error / degraded / updated / support-bundle / crash-resume
  surfaces).
- `50-ux-bundle-entitlement-surface-scope.md` — watermark overlay +
  capability-gated affordance styling per ADR-009.

## Architecture Decision Records (`adr/`)

- `adr/TEMPLATE.md` — MADR 4.0.0 template. Use this for every new ADR.
- `adr/ADR-001-runtime-model.md` — deterministic single-process runtime.
- `adr/ADR-002-rendering-backend.md` — frame-graph strategy (backend
  selection superseded by ADR-011).
- `adr/ADR-003-script-runtime-sandbox.md` — script runtime sandbox shape
  (engine / budgets superseded by ADR-012).
- `adr/ADR-004-state-model.md` — control and preset state model.
- `adr/ADR-005-integration-adapter-boundary.md` — integration adapter
  boundary and fault isolation.
- `adr/ADR-006-thread-and-process-model.md` — thread and process model.
- `adr/ADR-007-scene-warm-switch-policy.md` — scene warm-switch policy.
- `adr/ADR-008-transition-and-traveler-semantics.md` — transition and
  traveler semantics.
- `adr/ADR-009-activation-and-entitlements.md` — capability-bitmask
  entitlement substrate; `entitlement-v2` token format.
- `adr/ADR-010-embedded-docs-generation.md` — embedded documentation
  generation and clean-room authoring.
- `adr/ADR-011-rendering-backend.md` — wgpu + naga + GLSL 4.50 backend.
- `adr/ADR-012-script-sandbox-engine.md` — QuickJS script sandbox engine
  and per-frame budgets.
- `adr/ADR-013-networking-defaults-and-security.md` — OSC/NDI bind
  defaults, rate limits, allowlists.
- `adr/ADR-014-signing-notarization-auto-update.md` — macOS Developer ID
  + Sparkle 2 + Windows Authenticode design.
- `adr/ADR-015-reference-app-user-data-import.md` — reference-app
  user-data import strategy.
- `adr/ADR-016-observability-and-telemetry-posture.md` — zero-default
  outbound traffic; self-hosted Sentry opt-in; loopback-only health
  metrics.
- `adr/ADR-017-real-time-external-data-ingestion.md` — slot reserved;
  not yet authored. Design begins post-MVP per doc 41.
- `adr/ADR-018-non-scene-ui-toolkit.md` — Tauri 2 + Svelte 5 + TS +
  CodeMirror 6 stack for the non-scene UI surface.

## Working Rules

- Any architecture-impacting change requires an ADR. See
  `43-development-standards.md` §14 and `adr/TEMPLATE.md`.
- Requirements updates must propagate to the roadmap and risk register.
- Performance budgets are treated as product requirements.
- Compatibility claims require a reproducible validation method
  (see `09-test-strategy.md`).
- Every doc page SHOULD carry a Diátaxis-compliant front-matter
  (`doc-type`, `author`, `reviewed_by`, `date`, `version`,
  `content-origin`) per `43-development-standards.md` §13.2. Existing
  planning docs predate this rule and are grandfathered; new docs
  MUST comply from this commit forward.

## Licensing

Files in this directory are licensed CC-BY-4.0 via `REUSE.toml` at the
repo root unless a file's individual SPDX header states otherwise.

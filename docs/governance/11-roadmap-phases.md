# Roadmap Phases

## Objective
Sequence implementation work so high-risk/high-leverage foundations are delivered first.

## Phase 0: Planning Finalization
- Complete charter, contracts, requirements, ADRs, blueprints, budgets, risks.
- Complete legal policy baseline and UX pre-build gates before runtime implementation kickoff.
- Exit criteria:
 - no critical architecture unknowns,
 - WBS ownership assigned,
 - implementation kickoff checklist approved,
 - all required Phase 0 evidence is `Pass` in `26-evidence-artifact-registry.md`, including:
   - `EV-ARCH-CONTRACT-BASELINE` (ADR-011..016 + ADR-018),
   - `EV-SYMBOL-CATALOG`,
   - `EV-UX-CONTROL-CONTRACT`, `EV-UX-FLOW-CONTRACT`, `EV-UX-HUD-CONTRACT`, `EV-UX-AUTHORING-CONTRACT`, `EV-UX-LIBRARY-CONTRACT`,
   - `EV-UX-OSC-SETTINGS-CONTRACT`, `EV-UX-MIDI-SETTINGS-CONTRACT` (introduced by `08-io-integrations.md` Sec.OSC Operator Settings Panel Contract),
   - `EV-HOTKEY-PARITY-DESIGN` (ratification),
   - `EV-ENTITLEMENT-DESIGN`, `EV-ENTITLEMENT-TOKEN-FORMAT`,
   - `EV-DOCS-PHASE0-SEED`,
   - `EV-CLEANROOM-POLICY-BASELINE`,
   - `EV-OBSERVABILITY-INVENTORY`,
   - `EV-GATE-MODEL-AUDIT`,
   - `EV-COMMIT-CONVENTION-LINT`, `EV-LICENSE-HEADERS`, `EV-ADR-LINT` (development standards gates per `43-development-standards.md`).

### Phase 0 Blocking UX Gates
All seven UX contracts below (five primary + OSC/MIDI operator-settings) are produced under **WP-0 UX Contract Authoring** (see `12-work-breakdown-estimates.md`) owned by the **AI UX Workcell** (`23-decision-ownership-matrix.md`) with Accountable Owner sign-off.

- Control interaction contract signed (`EV-UX-CONTROL-CONTRACT`).
- Scene/preset/playlist interaction contract signed (`EV-UX-FLOW-CONTRACT`).
- Mapping/diagnostics/operator HUD interaction contract signed (`EV-UX-HUD-CONTRACT`).
- Authoring-environment interaction contract signed (`EV-UX-AUTHORING-CONTRACT`) per `37-authoring-environment-spec.md`.
- Scene-library interaction contract signed (`EV-UX-LIBRARY-CONTRACT`) per `38-scene-library-and-import-spec.md`.
- Reactive symbol catalog ratified (`EV-SYMBOL-CATALOG`) per `36-reactive-symbol-catalog.md` Sec.8 change-control.
- Operator hotkey map ratified (`EV-HOTKEY-PARITY-DESIGN`) per FR-17. Runtime conformance (`EV-HOTKEY-PARITY-CONFORMANCE`, `EV-HOTKEY-COMPLETENESS`, `EV-HOTKEY-CONFLICT`) ships at Phase 4a.
- Entitlement substrate design approved (`EV-ENTITLEMENT-DESIGN`) per `adr/ADR-009-activation-and-entitlements.md` (MVP watermark-on/off scope).
- Rendering backend approved (ADR-011 merged; covered under `EV-ARCH-CONTRACT-BASELINE`).
- Script sandbox engine approved (ADR-012 merged; covered under `EV-ARCH-CONTRACT-BASELINE`).
- Networking defaults approved (ADR-013 merged; covered under `EV-ARCH-CONTRACT-BASELINE`).
- Signing/notarization/auto-update design approved (ADR-014 merged; covered under `EV-ARCH-CONTRACT-BASELINE`).
- Reference-app user-data import strategy approved (ADR-015 merged; covered under `EV-ARCH-CONTRACT-BASELINE`).
- Non-scene UI toolkit decision approved (ADR-018 merged; covered under `EV-ARCH-CONTRACT-BASELINE`).
- Embedded docs Phase-0 seed pages authored and reviewed (`EV-DOCS-PHASE0-SEED`) per WP-9.
- Development standards ratified and wired (`43-development-standards.md`): `EV-COMMIT-CONVENTION-LINT`, `EV-LICENSE-HEADERS`, `EV-ADR-LINT` pass on the initial `main`. `EV-EVIDENCE-TAG-COVERAGE` remains `Planned` through Phase 0 and must reach `Pass` by Phase 1 exit.
- Runtime implementation branches that would invalidate pending UX contracts or the symbol catalog are blocked until all gates are approved.

#### UX Gate Definition of Done

Each UX gate passes only when all deliverables for that gate are present, reviewed, and approved.

**Shared deliverable minimums (all UX gates):**
- wireframes covering all primary and secondary states,
- interaction states including default, hover/focus, active, disabled, loading, empty, error, and recovery,
- degraded-mode and offline/unavailable-input states,
- keyboard and OSC/MIDI parity notes where the UI surface mirrors control-plane routes,
- accessibility design notes per `03-product-requirements.md` NFR-11 (WCAG 2.2 AA): contrast targets, focus indicators, minimum target size, keyboard paths, screen-reader labels for controls, and reduced-motion rules for non-scene chrome. Implementation conformance remains Phase 4b (`EV-ACCESSIBILITY-AA`); Phase 0 deliverables are design artifacts that give Phase 4b something to conform to,
- acceptance checklist with owner (Accountable Owner) and independent reviewer per `23-decision-ownership-matrix.md`,
- artifact location recorded in `26-evidence-artifact-registry.md` per Evidence Packet Minimums.

**`EV-UX-CONTROL-CONTRACT` (control interaction):**
- per-control-type interaction spec covering `slider`, `slider smooth`, `slider speed`, `knob`, `knob smooth`, `knob speed`, `toggle`, `toggle smooth`, `bang`, `bang smooth`, `bang counter`, `bang counter smooth`, `xy`, `xy smooth`, `xy speed`, `color`, `color smooth`, `dropdown`, `dropdown smooth`, and traveler variants (`slider traveler`, `knob traveler`, `xy traveler`),
- lock, default, randomize, and group-action gestures,
- soft-takeover/pickup visualization when a mapped input diverges from current value,
- error and out-of-range feedback behavior.

**`EV-UX-FLOW-CONTRACT` (scene/preset/playlist):**
- scene browse and launch flow including gold/Snap-0 default behavior (Snap 0 is the "gold" default preset; `/favslots/0` on the compat OSC surface),
- preset apply across channels (`scene`, `meta`, `media`) including partial apply,
- Snap slot save/apply/clear flow,
- playlist transport flow including `position` (with negative indices), `next`/`previous`, `play`/`toggleplay`, `select`,
- transition feedback UI tied to warm-switch states from `28-implementation-contract-pack.md` CP-03.

**`EV-UX-HUD-CONTRACT` (mapping/diagnostics/operator HUD):**
- MIDI-learn and OSC-mapping UX,
- adapter health states per `08-io-integrations.md` (`Disconnected`, `Connecting`, `Connected`, `Error`, `Recovering`),
- reconnect/backoff indicators and operator-actionable remediation copy,
- performance HUD with frame budget, script cost, pass cost, audio health, adapter health,
- degraded-mode banner and fallback-scene indicator,
- support bundle export entry point per `31-supportability-privacy-ops-policy.md`,
- **Settings → Hotkeys read-only operator surface**: renders every row of `42-operator-hotkey-map.md` Sec.1-10 in the operator's current layout. MVP is read-only (no rebinding UI per `42` Sec.Scope); the contract includes the empty-state, layout-switch affordance, conflict-indicator visualization (Sec.Conflict and priority rules), and the V2 "rebinding coming" affordance copy so Phase 4a conformance evidence (`EV-HOTKEY-COMPLETENESS`) has a visual ground truth to diff against.

## Phase 1: Runtime Foundation
- Build canonical state core and command bus.
- Implement scene loader, schema validator, canonical model normalizer.
- Implement base render graph scaffold and single-pass execution.
- Implement diagnostics infrastructure.
- Exit criteria:
 - `EV-SCENE-IMPORT` passes for baseline scene set.
 - `EV-DETERMINISM` baseline replay passes.
 - `EV-OBSERVABILITY` baseline telemetry schema is emitted and validated.
 - `EV-EVIDENCE-TAG-COVERAGE` passes: every test in an evidence-producing subtree carries the evidence tag required by `43-development-standards.md` Sec.15.

## Phase 2: Compatibility Core
- Multipass execution and feedback contract.
- Script runtime sandbox with lifecycle/event APIs.
- Core audio uniform generation pipeline.
- Compatibility mode for known legacy scene schema/control variants.
- Exit criteria:
 - representative golden corpus subset passes end-to-end (`EV-RENDER-CONFORMANCE`).
 - `EV-SCRIPT-CONTRACT` passes without critical failures.
 - no Severity 1 issues open for script/runtime compatibility.

## Phase 3: State and Performance Workflows
- Preset store and scene startup preset logic.
- Playlist orchestration and transition semantics.
- Signed show bundle export/import and rollback snapshot flow.
- Performance instrumentation and budget enforcement controls.
- System-audio ingestion and channel-routing conformance for supported profiles.
- Exit criteria:
 - `EV-PRESET-PLAYLIST` passes for parity scenarios.
 - `EV-PERF-BUDGETS` meets target thresholds in baseline profiles.
 - `EV-STATE-REPLAY` passes deterministic state transition checks.
 - `EV-STATE-MIGRATION` passes migration and rollback conformance tests.
 - `EV-SHOW-BUNDLE-RECOVERY` passes bundle integrity and rollback recovery tests.
 - `EV-AUDIO-ROUTING-MATRIX` passes required platform/device routing scenarios.
 - `EV-COLOR-MANAGEMENT-SRGB` passes V1 sRGB conformance on declared display profiles per NFR-12.

## Phase 4: Integrations, Authoring, Docs, and Hardening
Phase 4 is split into four orthogonal sub-phases. Each sub-phase has its own exit evidence set. A stall in one sub-phase does not block others from closing.

### Phase 4a: Integrations and Entitlement
- OSC/MIDI adapters and route compatibility (`OSC-CONF-01..20`, `MIDI-CONF-01..20`).
- Camera/media input adapters and Media Folders workflow.
- External output adapter abstraction (Syphon/Spout). NDI is post-MVP stub.
- Entitlement substrate integrated at adapter boundaries (MVP watermark-on/off only; tier gating hooks wired but dormant).
- Operator hotkey map bound application-wide.
- Network-hardening defaults per `adr/ADR-013-networking-defaults-and-security.md`.
- Health-metrics exposure scoped loopback-only per `adr/ADR-016-observability-and-telemetry-posture.md`.
- Exit criteria:
 - `EV-INTEGRATION-MATRIX` passes for required adapters/routes.
 - automatic reconnect criteria for supported adapters are met.
 - `EV-HOTKEY-PARITY-CONFORMANCE`, `EV-HOTKEY-COMPLETENESS`, `EV-HOTKEY-CONFLICT` pass.
 - `EV-MIDI-EXTENDED` passes.
 - `EV-NET-HARDENING` passes.
 - `EV-ENTITLEMENT-ENFORCEMENT`, `EV-ENTITLEMENT-OFFLINE`, `EV-ENTITLEMENT-REVOCATION`, `EV-ENTITLEMENT-FREE-FEATURES` pass (MVP watermark scope).
 - `EV-HEALTH-METRICS-LOOPBACK-ONLY` passes.

### Phase 4b: Authoring, Library, and Import
- In-app scene authoring environment (editor, hot reload, diagnostics, scaffolder, import/export).
- Scene library (browse, search, filter, thumbnails, drag-to-playlist).
- Shadertoy import (3+ golden fixtures); ISF reject-with-diagnostic path.
- Reference-app user-data import (presets, MIDI mappings, playlists, Snap slots, hotkey remaps, OSC mappings, settings) per `adr/ADR-015-reference-app-user-data-import.md`.
- Non-scene UI accessibility conformance per NFR-11.
- Exit criteria:
 - `EV-AUTHORING-LIVE-RELOAD`, `EV-AUTHORING-DIAGNOSTICS`, `EV-AUTHORING-SCAFFOLDER` pass.
 - `EV-AUTHORING-UX-LATENCY` passes.
 - `EV-LIBRARY-DISCOVERY`, `EV-LIBRARY-THUMBNAIL`, `EV-LIBRARY-BUNDLE-LAYOUT` pass.
 - `EV-IMPORT-SHADERTOY` passes; `EV-IMPORT-ISF-REJECT` confirms diagnostic path.
 - `EV-IMPORT-REFERENCE-SHOW` passes for every artifact class declared in ADR-015 (scenes, presets, playlists, Snap slots, MIDI maps, OSC mappings, hotkey remaps, settings snapshot).
 - `EV-ACCESSIBILITY-AA` passes on non-scene UI per NFR-11.

### Phase 4c: Embedded Documentation
- Full embedded documentation coverage per `adr/ADR-010-embedded-docs-generation.md`.
- Docs version-sync CI enforced.
- Exit criteria:
 - `EV-DOCS-CLEANROOM` passes on all pages shipped to date.
 - `EV-DOCS-VERSION-SYNC` passes (docs version matches symbol catalog version).
 - `EV-DOCS-UX-LATENCY` passes (hover-docs p95 <= 80 ms, symbol-search p95 <= 150 ms).
 - Note: `EV-DOCS-COVERAGE` is release-cumulative, not phase-local.

### Phase 4d: Hardening
- Resilience, soak, crash-recovery, and shader-cache validation.
- Multi-GPU selection policy conformance per `adr/ADR-011-rendering-backend.md` §7.
- Default-off outbound traffic audit and crash-payload redaction.
- Exit criteria:
 - `EV-SOAK-CHAOS` meets stability thresholds.
 - `EV-CRASH-RESUME` validates mid-show crash -> show-continuation with declared state-restoration acceptance.
 - `EV-SHADER-CACHE` validates cold vs warm shader-compile budgets.
 - `EV-GPU-SELECTION` validates multi-GPU pinning and fallback.
 - `EV-CRASH-PAYLOAD-REDACTION` confirms Sentry payload excludes `main.glsl`, `script.js`, media buffers, and free-text fields.
 - `EV-ANALYTICS-ZERO-DEFAULT` confirms zero outbound traffic on cold-start + idle with all toggles off.

## Phase 5: MVP Freeze and Launch Prep
- Regression stabilization and bug burn-down.
- Documentation and migration notes.
- Release candidate validation.
- Formal legal audit review before freeze finalization.
- Exit criteria:
 - all V1 bets meet acceptance targets from `18-recommended-bets-v1-v2.md`.
 - no unresolved P0 issues in planning quality review.
 - post-MVP backlog and owner accountability finalized.
 - cumulative release gate is `Pass` in `26-evidence-artifact-registry.md`.
 - legal release evidence (`EV-LEGAL-PROVENANCE`, `EV-LICENSE-AUDIT`, `EV-BRAND-REVIEW`) is approved.
 - supportability/privacy readiness evidence (`EV-SUPPORT-OPS-READINESS`) is approved.

## Milestone Checks (All Phases)
- Update ADRs on architecture-impacting changes.
- Keep risk register and budgets synchronized with observed data.
- Re-score parity matrix after each phase.
- Enforce accountable sign-off using `23-decision-ownership-matrix.md`.
- Validate gate evidence status from `26-evidence-artifact-registry.md` only.

## Gate Rule
- A phase is complete only when all required evidence IDs for that phase are `Pass` in `26-evidence-artifact-registry.md`.
- Evidence from other documents is informational unless mapped through the registry.
- Any critical SLO miss in `21-operational-slos-slis.md` blocks advancement until mitigated or explicitly waived by the Accountable Owner per `23-decision-ownership-matrix.md`.
- Release gate is cumulative: all shipped requirement-linked evidence rows must be `Pass`, not only phase-local release artifacts.

## Deep-Dive Opportunity Mapping
- **Phase 1 priority bets**
 - Compatibility loader + normalization warnings (`17-differentiation-opportunity-matrix.md`: O05).
 - Scene preflight/lint (`17-differentiation-opportunity-matrix.md`: O06).
 - Runtime operator HUD baseline (`17-differentiation-opportunity-matrix.md`: O03).
 - Integration contract foundations (versioned OSC schema + adapter health primitives) (`17-differentiation-opportunity-matrix.md`: O01, O02).
- **Phase 2 priority bets**
 - Script sandbox + watchdog (`17-differentiation-opportunity-matrix.md`: O04).
 - Lifecycle contract formalization (`17-differentiation-opportunity-matrix.md`: O09).
 - Operational reconnect hardening and route alias implementation (`17-differentiation-opportunity-matrix.md`: O01, O02).
- **Phase 3 priority bets**
 - Signed show bundles + rollback (`17-differentiation-opportunity-matrix.md`: O08).
 - Audio routing diagnostics (`17-differentiation-opportunity-matrix.md`: O07).
- **Phase 4a priority bets**
 - Multi-slot and advanced input/output integration capabilities (`17-differentiation-opportunity-matrix.md`: O11).
 - Route alias and reconnect hardening closure.
- **Phase 4b priority bets**
 - Authoring-environment live-reload polish and scaffolder UX.
 - Scene library filter/search performance.
- **Phase 4c priority bets**
 - Embedded docs coverage for every public symbol.
- **Phase 5 and post-MVP**
 - Adaptive quality ladder and advanced differentiation capabilities (`17-differentiation-opportunity-matrix.md`: O10, O12, O13, O14).
 - ISF scene import (deferred V2 per `38-scene-library-and-import-spec.md` Sec.6).
 - Redefinable hotkey map UI (V2 extension of FR-17).
 - Auto-updater, code signing, and notarization pipeline (release-engineering deferrals).

## Cross-Cutting Dependencies
- **Entitlement substrate** (`adr/ADR-009-activation-and-entitlements.md`) ships at MVP with watermark-on/off scope only. Design-level approval (`EV-ENTITLEMENT-DESIGN`) is a Phase 0 gate; integration (`EV-ENTITLEMENT-ENFORCEMENT`) is a Phase 4a exit criterion and tests only the watermark transitions (token absent/present/expired/revoked) at MVP. Post-MVP tier rollout (NDI, tiered OSC, `_Reactivity`, `_Audio_Speed`, custom-resolution) reuses the same adapter-boundary hooks wired in Phase 4a without requiring architecture changes; it is tracked as a post-MVP work package (see `R-18` in `13-risk-register.md`).
- **Reactive symbol catalog** (`36-reactive-symbol-catalog.md`) must be ratified in Phase 0 because Phase 1 scene loader, Phase 2 script sandbox, and Phase 4 authoring autocomplete all consume it as source of truth.
- **Rendering backend** (`adr/ADR-011-rendering-backend.md`) locks wgpu + naga and must be approved as part of Phase 0 (`EV-ARCH-CONTRACT-BASELINE` references it). Phase 1 render graph implementation starts against this backend.
- **Script sandbox engine** (`adr/ADR-012-script-sandbox-engine.md`) locks QuickJS and per-frame budgets. Phase 2 `EV-SCRIPT-CONTRACT` + `EV-SANDBOX-SAFETY` depend on these numerics.
- **Network security defaults** (`adr/ADR-013-networking-defaults-and-security.md`) govern OSC/NDI bind scope and rate limits from Phase 1 onward. `EV-NET-HARDENING` is a Phase 4a exit criterion.
- **Signing/notarization/auto-update design** (`adr/ADR-014-signing-notarization-auto-update.md`) is approved at Phase 0. Pipeline implementation is Phase 5; MVP freeze cannot complete without it.

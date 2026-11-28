# Work Breakdown and Sequencing (Planning Baseline)

This document defines implementation order, dependencies, and ownership signals.

## Effort Scale (Relative, Non-temporal)
- S: localized change with limited cross-subsystem coupling.
- M: moderate coupling and multiple interfaces touched.
- L: broad subsystem work with significant integration points.
- XL: foundational work affecting many subsystems and validation paths.

## Work Packages

### WP-0 UX Contract Authoring
- Wireframes, interaction states (default/hover/focus/active/disabled/loading/empty/error/recovery), degraded/offline states, keyboard + OSC/MIDI parity notes, and accessibility design notes (WCAG 2.2 AA per NFR-11: contrast targets, focus indicators, target size, screen-reader labels, reduced-motion rules) for every UX surface that is a Phase-0 blocker.
- Deliverables (seven UX contract packets):
 - `EV-UX-CONTROL-CONTRACT` packet (per-control-type interaction spec for all control types in `02-synesthesia-contracts.md` Sec.2.2 + traveler variants, including shared settings chrome and design-token usage per `adr/ADR-018-non-scene-ui-toolkit.md`),
 - `EV-UX-FLOW-CONTRACT` packet (scene/preset/playlist/Snap/transition),
 - `EV-UX-HUD-CONTRACT` packet (mapping, adapter-health, performance HUD, degraded-mode, support-bundle entry point, Settings → Hotkeys read-only table per `42-operator-hotkey-map.md`),
 - `EV-UX-AUTHORING-CONTRACT` packet (editor, live-reload feedback, diagnostics, scaffolder),
 - `EV-UX-LIBRARY-CONTRACT` packet (browse, search, filter, thumbnail fallback chain, drag-to-playlist),
 - `EV-UX-OSC-SETTINGS-CONTRACT` packet (OSC Operator Settings Panel per `08-io-integrations.md` Sec.OSC Operator Settings Panel Contract: row spec, persistence, accessibility conformance, mapping-tooltip behavior),
 - `EV-UX-MIDI-SETTINGS-CONTRACT` packet (MIDI Operator Settings Panel per `08-io-integrations.md`: device list + allowlist, per-scene mapping list, learn indicator, external-BPM-sync, extended-surface opt-ins MIDI-CONF-16..20, persistence, accessibility).
- Effort class: XL (spans seven gate packets).
- Dependencies: `36-reactive-symbol-catalog.md` ratified, `28-implementation-contract-pack.md` CP-03/CP-05 stable, `11-roadmap-phases.md` Phase-0 UX Gate DoD, `08-io-integrations.md` OSC Operator Settings Panel Contract stable, `42-operator-hotkey-map.md` ratified, `adr/ADR-018-non-scene-ui-toolkit.md` accepted.
- Owner: AI UX Workcell with Accountable Owner sign-off.

### WP-1 Scene System
- Schema design + validator + compatibility normalizer.
- Deliverables:
 - parser APIs,
 - canonical scene model,
 - diagnostics taxonomy.
- Owner: AI Scene Workcell with Accountable Owner sign-off.
- Inputs: `02-synesthesia-contracts.md`, `05-scene-system.md`, `28-implementation-contract-pack.md` (CP-01, CP-03..CP-09, CP-12), `36-reactive-symbol-catalog.md` ratified, ADR-001, ADR-002, ADR-011, ADR-012.
- Outputs (Evidence IDs): `EV-SCENE-IMPORT`, `EV-SCHEMA-LINT`, `EV-DIAGNOSTIC-QUALITY` (scene-load subset).
- Blocking deps: WP-0 (for authoring-scaffold interaction contract where applicable); ADR-011 approved; `EV-SYMBOL-CATALOG` ratified.
- Effort class: XL.

### WP-2 Render Graph Runtime
- Pass graph execution, resource descriptors, feedback contract.
- Deliverables:
 - pass scheduler,
 - target allocator,
 - pass telemetry.
- Owner: AI Runtime Workcell with Accountable Owner sign-off.
- Inputs: `04-runtime-architecture.md`, `28-implementation-contract-pack.md` CP-10, ADR-001, ADR-006, ADR-007, ADR-011 (wgpu/naga pinning), Sec.16 pass-sizing algebra.
- Outputs (Evidence IDs): `EV-RENDER-CONFORMANCE`, `EV-PERF-BUDGETS` (render subset), `EV-GPU-SELECTION`, `EV-SHADER-CACHE`.
- Blocking deps: WP-1 (scene model); ADR-011 merged.
- Effort class: XL.

### WP-3 Script Runtime Sandbox
- Lifecycle hooks, host API, event registration, fault containment.
- Deliverables:
 - sandbox wrapper,
 - host API contract,
 - runtime guards.
- Owner: AI Runtime Workcell with Accountable Owner sign-off.
- Inputs: `05-scene-system.md` script-runtime section, ADR-003, ADR-012 (QuickJS + per-frame budgets), `28-implementation-contract-pack.md` CP-07.
- Outputs (Evidence IDs): `EV-SCRIPT-CONTRACT`, `EV-SANDBOX-SAFETY`, `EV-DETERMINISM` (script subset).
- Blocking deps: WP-1, WP-2; ADR-012 merged.
- Effort class: L to XL.

### WP-4 Audio Engine
- Input capture, analysis pipeline, reactive uniform publisher.
- Deliverables:
 - uniform families,
 - data texture streams,
 - fallback behavior.
- Owner: AI Audio Workcell with Accountable Owner sign-off.
- Inputs: `06-audio-engine.md` §7 normative spec, `28-implementation-contract-pack.md` CP-02 + CP-02a snapshot handoff, `36-reactive-symbol-catalog.md` Sec.2 + Sec.2.10.
- Outputs (Evidence IDs): `EV-AUDIO-TRACE`, `EV-AUDIO-ROUTING-MATRIX`.
- Blocking deps: WP-1 (control surface); `36` Sec.2 ratified.
- Effort class: XL.

### WP-5 State, Presets, Playlists
- Canonical store, commands/events, serialization and migration.
- Deliverables:
 - state schemas,
 - persistence adapters,
 - playlist transport semantics.
- Owner: AI Scene Workcell with Accountable Owner sign-off.
- Inputs: `07-control-state-presets.md`, ADR-004, ADR-008 transitions/travelers, `28-implementation-contract-pack.md` CP-05, CP-11, `32-state-preset-playlist-migration-policy.md`.
- Outputs (Evidence IDs): `EV-STATE-REPLAY`, `EV-PRESET-PLAYLIST`, `EV-STATE-MIGRATION`, `EV-SHOW-BUNDLE-RECOVERY`.
- Blocking deps: WP-1 (control model); ADR-004 merged.
- Effort class: L to XL.

### WP-6 Integration Adapters
- OSC, MIDI, camera/media, output adapters.
- Deliverables:
 - adapter interfaces,
 - translation mappings,
 - health/recovery model.
- Owner: AI Integration Workcell with Accountable Owner sign-off.
- Inputs: `08-io-integrations.md` (including OSC Operator Settings Panel Contract + Alias Table + MIDI-CONF-01..20), ADR-005, ADR-009 entitlement surface, ADR-013 networking defaults, `21-operational-slos-slis.md` SLO-04, `42-operator-hotkey-map.md`.
- Outputs (Evidence IDs): `EV-INTEGRATION-MATRIX`, `EV-MIDI-EXTENDED`, `EV-NET-HARDENING`, `EV-HOTKEY-PARITY-CONFORMANCE`, `EV-HOTKEY-COMPLETENESS`, `EV-HOTKEY-CONFLICT`, `EV-HEALTH-METRICS-LOOPBACK-ONLY`.
- Blocking deps: WP-5 (state model for OSC/MIDI mapping targets); ADR-005/013/014 merged; `EV-UX-OSC-SETTINGS-CONTRACT` + `EV-UX-MIDI-SETTINGS-CONTRACT` delivered from WP-0.
- Effort class: XL.

### WP-7 Test and Regression Harness
- Golden scene harness, fixture runners, trace/frame diff reporting, fuzz, chaos, crash-resume drill.
- Deliverables:
 - headless runner,
 - test corpus tooling,
 - benchmark suite,
 - fuzz driver for `scene.json`/`main.glsl`/`script.js`/OSC/MIDI packet/`.synScene` import/`.chromaPack` extractor,
 - chaos driver for network partition, device unplug, disk-full, thermal throttle, GPU reset.
- Owner: AI QA/Ops Workcell with Accountable Owner sign-off.
- Inputs: `09-test-strategy.md`, `10-performance-latency-budgets.md`, `34-golden-corpus-definition.md`, `21-operational-slos-slis.md`.
- Outputs (Evidence IDs): `EV-DETERMINISM`, `EV-SOAK-CHAOS`, `EV-CRASH-RESUME`, and the fuzz/chaos sub-evidence rows referenced by those IDs.
- Blocking deps: WP-1..WP-4 stable for fixture compilation; WP-5 for state-replay; WP-6 for integration chaos.
- Effort class: L to XL.

### WP-8 Observability and Performance Tooling
- Runtime metrics, latency instrumentation, budget dashboards, crash-payload redaction.
- Deliverables:
 - instrumentation points,
 - benchmark reports,
 - budget alerting hooks,
 - Sentry payload redactor.
- Owner: AI QA/Ops Workcell with Accountable Owner sign-off.
- Inputs: `10-performance-latency-budgets.md`, `21-operational-slos-slis.md`, ADR-016 observability posture, NFR-4/NFR-8a.
- Outputs (Evidence IDs): `EV-OBSERVABILITY`, `EV-OBSERVABILITY-INVENTORY`, `EV-PERF-BUDGETS`, `EV-CRASH-PAYLOAD-REDACTION`, `EV-ANALYTICS-ZERO-DEFAULT`, `EV-ATS-NO-THIRD-PARTY-EXCEPTIONS`.
- Blocking deps: WP-2 (render instrumentation), WP-4 (audio instrumentation), WP-6 (adapter health signal).
- Effort class: L.

### WP-9 Embedded Docs Authoring
- Original, clean-room authoring of every documentation page shipped under `<AppBundle>/Resources/docs/`.
- Phase-0 seed deliverables (required for `EV-DOCS-PHASE0-SEED` gate): Introduction / Quickstart, Scene JSON reference, Standard Uniforms, at minimum one tutorial (First Scene).
- Full deliverables (required for release cumulative gate `EV-DOCS-COVERAGE`): Audio Uniforms, Meta Controls, GLSL Helpers, Script Host API, External Libraries, Control Hotkeys, Operator Hotkey Map, OSC route reference, MIDI mapping guide, FAQ (audio/video/settings/troubleshooting/license-entitlement), Tutorials (Shadertoy conversion, livestream, presets-Snaps-playlists, authoring events with JavaScript).
- Provenance per page (author, reviewed_by, date) per `adr/ADR-010-embedded-docs-generation.md` and `29-cleanroom-legal-policy-baseline.md`; paraphrase-only rewrites of reference text are disallowed.
- Effort class: XL (content authoring is critical-path for release gate; throughput is capped by clean-room review).
- Dependencies: `36-reactive-symbol-catalog.md` ratified; `adr/ADR-010-embedded-docs-generation.md` pipeline established.
- Owner: AI Docs Workcell with Accountable Owner sign-off for clean-room review.

## Suggested Staffing Pattern
- AI UX Workcell lane: WP-0.
- AI Runtime Workcell lane: WP-1, WP-2, WP-3.
- AI Audio + Integration Workcell lane: WP-4, WP-6.
- AI Scene/State Workcell lane: WP-5.
- AI QA/Ops Workcell lane: WP-7, WP-8.
- AI Docs Workcell lane: WP-9.
- Accountable Owner lane: ADR/gate/waiver approvals, scope arbitration, legal gate closure.

## Approval Throughput Constraints
- Workcell evidence packets must be pre-reviewed before Accountable Owner sign-off.
- Independent checklist reviewer must be assigned for architecture and release gates.
- Approval SLA targets follow `23-decision-ownership-matrix.md` and must be tracked in phase status reviews.

## Critical Path (Preliminary)
WP-0 (Phase 0) -> WP-1 -> WP-2 -> (WP-3 + WP-4) -> WP-5 -> WP-6 -> WP-7 -> MVP validation. WP-9 (docs) runs in parallel from Phase 0 onward and converges with WP-6/WP-7 at release.

## Sequencing Follow-up Needed
- Confirm dependency edges after implementation spikes:
 - multipass prototype spike,
 - script sandbox overhead spike,
 - audio latency spike.
- Re-rank work-package order if risk profile changes.

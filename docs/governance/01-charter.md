# Chromaticity Project Charter

## Vision
Chromaticity is a live-performance visual instrument that matches or exceeds the incumbent Synesthesia app in capability, reliability, and creative flexibility. Public-facing comparative claims are evidence-backed per the Brand/Trademark Review Checklist in `29-cleanroom-legal-policy-baseline.md`.

## Mission
Deliver a deterministic, low-latency, scene-driven visual runtime with strong backward compatibility for reference-app authoring workflows (the `compat-syn-v1` profile in `22-compatibility-versioning-policy.md`) while improving developer ergonomics and operational robustness.

## Product Principles
- Performance first: live use is the primary context.
- Deterministic behavior: identical inputs produce predictable outputs.
- Contract stability: scene/runtime APIs are treated as long-lived contracts.
- Safe extensibility: integrations are adapter-based and isolated.
- Observability by default: measurable latency, cost, and failure behavior.

## Scope
### In Scope (Foundational)
- Scene runtime architecture and schema compatibility strategy.
- Audio analysis pipeline + reactive uniforms contract.
- Script runtime contract for scene-level logic (`setup`, `update`, events).
- Controls/state/presets/playlists model.
- Integration boundaries for OSC, MIDI, media, camera, syphon/spout equivalents.
- Test/performance planning and execution roadmap.
- **Product-surface features required for authoring and operating parity**: in-app scene authoring environment (`37-authoring-environment-spec.md`), scene library and import pipelines (`38-scene-library-and-import-spec.md`), operator hotkey map, and embedded documentation (`adr/ADR-010-embedded-docs-generation.md`). These are product features, not UI polish.
- **Entitlement enforcement substrate (mechanism only)** for watermark-on/off gating (`adr/ADR-009-activation-and-entitlements.md`). The mechanism is in scope; tiered feature rollout (Advanced/NDI/Developer/Education editions) is post-MVP per `30-mvp-support-matrix.md`. This removes commerce-backend dependency from the MVP freeze.
- **MVP edition scope: Core-only** (one edition preset). Watermark overlays when no token is present; watermark is removed by any valid entitlement token. Post-MVP rollout of additional capability presets (marketed as editions) is tracked as a scheduled follow-on and remains under the same `adr/ADR-009-activation-and-entitlements.md` capability-bitmask substrate.

### Out of Scope (Pre-build Planning Phase)
- Final visual design language (colors, type, iconography) for the UI; the authoring/library/hotkey **contracts** are in scope but their visual realization lives in design deliverables.
- Post-MVP capability-preset rollout (marketed as Advanced, NDI I/O, Developer, Education editions per `adr/ADR-009-activation-and-entitlements.md` and `30-mvp-support-matrix.md`) and the commerce backend required to issue their tokens. MVP entitlement posture is watermark-on/watermark-off only, which is satisfied by statically-issued tokens for closed beta or self-signed Accountable-Owner-issued tokens; no commerce backend is required to ship MVP.
- Production deployment and installer tooling execution (notarization/signing/auto-updater **design** is now Phase-0 per `adr/ADR-014-signing-notarization-auto-update.md`; **implementation** is Phase 5).
- Commercial/licensing **model** design (pricing, SKU structure, commerce backend).

## Success Metrics
### Product Metrics
- Parity scorecard defined for all critical reference-app capabilities (per `02-synesthesia-contracts.md`).
- 100% of required runtime contracts documented with acceptance criteria.
- MVP boundaries explicitly documented and approved.

### Engineering Metrics
- First sprint can start with no unresolved architecture blockers.
- Each subsystem has defined interfaces, ownership, and failure modes.
- Performance budgets defined for frame time, latency, and startup behavior.

### Risk Metrics
- Top 10 technical risks documented with mitigation and trigger conditions.
- All high risks assigned owner + early validation activity.

## Users and Primary Use Cases
- Live VJs and performers controlling visuals in real time.
- Scene authors creating GLSL + script-based reactive visuals.
- Technical artists integrating external controllers via OSC/MIDI.

## Constraints
- Must support low-jitter operation within the MVP support envelope defined in `30-mvp-support-matrix.md`.
- Must maintain the `compat-syn-v1` compatibility profile for reference-app scene behaviors where feasible.
- Must tolerate faulty scenes/scripts without crashing entire runtime.

## Strategic Bets
- A contract-first architecture will reduce long-term maintenance cost.
- Golden-scene regression testing will enable rapid safe iteration.
- Adapter-based integrations will contain platform/protocol complexity.

## Decision Gates
- Gate 1: Requirements + ADR baseline approved.
- Gate 2: Subsystem blueprints reviewed against budgets.
- Gate 3: Test/performance strategy accepted.
- Gate 4: Roadmap and WBS staffed for implementation kickoff.

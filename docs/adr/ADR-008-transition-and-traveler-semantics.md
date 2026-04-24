# ADR-008: Transition Hook and Traveler Semantics

## Status
Accepted

## Context
Traveler controls and transition hook timing were open compatibility questions and a major source of implementation ambiguity.

## Decision
- Traveler controls are normalized path-time drivers with optional speed modulation.
- Compatibility mode clamps traveler values outside `[0.0, 1.0]`; strict mode rejects out-of-range payloads.
- `transition()` hook executes once per transition boundary after transition variable binding and before first target-scene render pass.
- Scenes without `transition()` follow standard switch flow without compatibility penalty.

## Consequences
### Positive
- Removes ambiguity for script/runtime behavior.
- Preserves compatibility while enforcing deterministic transition timing.
- Provides explicit conformance points for regression harness.

### Negative
- Requires additional transition contract testing coverage.

## Alternatives Considered
- Per-pass transition hook execution (rejected: nondeterminism and script overhead).
- Ignoring traveler outliers (rejected: compatibility failures on legacy scenes).

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision. The "transition() executes once per boundary, after binding, before first target-scene render pass" decision is the architectural precondition for the script-and-transition evidence set; per-pass execution would have made `EV-SCRIPT-CONTRACT` and `EV-RENDER-CONFORMANCE` non-deterministic on transition boundaries, and ignoring traveler outliers would have failed `EV-COMPAT-FIXTURE-COVERAGE` on the legacy-scene corpus.

- `EV-SCRIPT-CONTRACT` — the `transition()` hook timing, lifecycle position, and execution-cardinality (once per boundary) are part of the script contract this row tests; refined by `adr/ADR-003-script-runtime-sandbox.md` (sandbox shape) and `adr/ADR-012-script-sandbox-engine.md` (engine choice + budget numerics).
- `EV-COMPAT-FIXTURE-COVERAGE` — the compat-mode-clamps / strict-mode-rejects taxonomy on out-of-range traveler payloads is a CP-01-shaped coercion-vs-rejection rule; the slice-2 T1 cr-01 branch coverage at `engine/scene` (`docs/26` row 25 + PR `#47`) tests the same coercion taxonomy class.
- `EV-RENDER-CONFORMANCE` — transition timing affects render output; deterministic hook position relative to render passes is what makes transition fixtures repeatable in the conformance corpus.
- `EV-SCHEMA-LINT` — out-of-range traveler payloads in compat mode emit lint warnings (per `05-scene-system.md` Sec.5 warning taxonomy realized in `engine/scene/src/lint.rs`); the warning-class taxonomy pin at slice-2 (`docs/26` row 24, T1 graduated by `#49`) covers the class this ADR's compat-mode behavior produces.
- `EV-DETERMINISM` — deterministic transition timing is part of the runtime determinism precondition; per-pass hook execution would have introduced cross-run variance the determinism replay corpus (`docs/26` row 37) explicitly forecloses.
- `EV-AUDIO-TRACE` — transition-time audio sampling cadence is bounded by this ADR's timing decision; the audio-snapshot publication occurs at known points relative to transition boundaries, not interleaved with hook execution.

## Traceability

- `05-scene-system.md` Sec.Transition Contract — the canonical transition-hook shape this ADR locks; warning-class taxonomy in Sec.5 is what `engine/scene/src/lint.rs` implements.
- `02-synesthesia-contracts.md` Sec.Traveler Controls — normalized path-time driver contract this ADR refines into the `[0.0, 1.0]` clamp / reject behavior.
- `28-implementation-contract-pack.md` CP-01 (missing-`TYPE` coercion fallback, same-shaped coercion taxonomy as the traveler clamp) and CP-07 (script API parity matrix; `transition()` is enumerated there).
- `adr/ADR-007-scene-warm-switch-policy.md` — owns the four-phase lifecycle this hook fires within (after binding, before first target-scene render); the two ADRs are co-required.
- `adr/ADR-003-script-runtime-sandbox.md` — script runtime architectural sandbox the hook executes inside (superseded on engine choice by `adr/ADR-012-script-sandbox-engine.md`).
- `adr/ADR-001-runtime-model.md` — `ScriptRuntime` ownership of the hook execution.
- `adr/ADR-006-thread-and-process-model.md` — hook executes on a known thread relative to the scheduler; per-thread affinity is part of the determinism precondition.
- `13-risk-register.md` R-01 (compatibility gaps in scene schema variants) — the compat-mode clamp + strict-mode reject is part of the closure path; R-02 (script runtime behavioral drift) — explicit hook timing + cardinality is the closure path for transition-related drift.
- `03-product-requirements.md` FR-3 (Rendering Runtime); FR-4 (Script Runtime); NFR-2 (Determinism).

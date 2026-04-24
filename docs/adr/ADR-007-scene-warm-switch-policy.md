# ADR-007: Scene Warm-Switch Policy

## Status
Accepted

## Context
Warm-switch behavior was previously underspecified, creating risk of inconsistent scene transition implementation.

## Decision
- Implement four-phase warm-switch lifecycle:
  1. `PrepareNext`,
  2. `PrimeNext`,
  3. `CrossfadeOrHardCut`,
  4. `CommitNext`.
- Keep current scene active until `PrepareNext` and `PrimeNext` succeed.
- On preparation failure, abort switch and preserve current output with diagnostics.
- Default warm-switch timeout is 500 ms; timeout degrades to hard-cut fallback with warning.

## Consequences
### Positive
- Preserves continuity under transition failures.
- Defines deterministic transition state boundaries.
- Enables testable warm-switch behavior.

### Negative
- Adds transition orchestration complexity.
- Requires explicit resource lifecycle ownership.

## Alternatives Considered
- Immediate scene replacement without priming (rejected: continuity risk).
- Full dual-scene parallel execution for all switches (rejected for v1 cost/complexity).

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision. The four-phase lifecycle (`PrepareNext` / `PrimeNext` / `CrossfadeOrHardCut` / `CommitNext`) plus the 500 ms timeout-degrades-to-hard-cut fallback is the architectural precondition for the scene-transition evidence set; an immediate-replacement alternative would have removed the preparation-failure abort path several rows below depend on.

- `EV-RENDER-CONFORMANCE` — warm-switch behavior is part of the conformance corpus; deterministic transition state boundaries declared here are what the corpus's transition fixtures test against.
- `EV-SCENE-IMPORT` — scene transitions through the canonical model; the warm-switch is the runtime path that the import harness exercises end-to-end (parse → load → prepare → prime → commit). Phase 1 row depends on this lifecycle being deterministic.
- `EV-AUTHORING-LIVE-RELOAD` — hot-reload uses the warm-switch to preserve preset / control state during edit; without `PrepareNext` + `PrimeNext` the editor would tear down running scenes on every reload.
- `EV-CRASH-RESUME` — preparation-failure abort + diagnostic preservation is the row's continuity model; the "preserve current output with diagnostics" decision here is what makes mid-show transition failures recoverable to the operator.
- `EV-DETERMINISM` — deterministic transition state boundaries; the four-phase lifecycle is what makes transition replay reproducible at the same wallclock points across runs.
- `EV-PERF-BUDGETS` — 500 ms warm-switch timeout is a per-transition budget declared in `10-performance-latency-budgets.md` and validated against this ADR's policy.

## Traceability

- `05-scene-system.md` Sec.Scene Lifecycle + Sec.Loader Modes — the scene lifecycle the warm-switch policy implements.
- `28-implementation-contract-pack.md` CP-04 (typed command schema; the four warm-switch phases dispatch as typed commands) and CP-07 (script API parity matrix; warm-switch interacts with `setup` / `transition` hooks).
- `adr/ADR-001-runtime-model.md` — `SceneRuntime` owns the warm-switch lifecycle.
- `adr/ADR-002-rendering-backend.md` — frame graph the warm-switch invalidates and re-primes (with backend selection rolled forward to ADR-011).
- `adr/ADR-008-transition-and-traveler-semantics.md` — `transition()` hook fires once per boundary _within_ this lifecycle (after binding, before first target-scene render pass); the two ADRs are co-required.
- `adr/ADR-004-state-model.md` — preset / control state preserved across transitions per the canonical state model.
- `04-runtime-architecture.md` — runtime architecture this lifecycle operates within.
- `13-risk-register.md` R-13 (authoring contract ambiguity, docs vs runtime behavior) — explicit lifecycle phase boundaries are the closure path; transition contract is testable at each boundary.
- `03-product-requirements.md` FR-3 (Rendering Runtime); FR-13 (Scene Authoring Environment); NFR-1 (Reliability); NFR-2 (Determinism).

# ADR-006: Runtime Thread and Process Model

## Status
Accepted

## Context
Runtime architecture required explicit process/thread decisions to avoid implementation drift and state race conditions.

## Decision
- Adopt single-process runtime with dedicated subsystem threads:
 - render/scheduler thread,
 - audio analysis thread,
 - integration IO thread pool,
 - state commit thread.
- Durable state mutation is restricted to `StateCore` on the state commit thread.
- Subsystems communicate through typed commands/events and immutable frame snapshots.

## Consequences
### Positive
- Deterministic ordering with reduced race-condition surface.
- Clear observability boundaries for latency attribution.
- Compatible with adapter isolation and command validation model.

### Negative
- Requires strict thread-affinity and queue-pressure controls.
- Higher up-front complexity than ad hoc shared mutation.

## Alternatives Considered
- Fully single-threaded runtime (rejected: IO/audio contention risk).
- Multi-process architecture (rejected for v1 operational complexity).

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision. The single-process / dedicated-subsystem-thread / `StateCore`-on-commit-thread decomposition is the architectural precondition for the determinism / performance / observability evidence set; a fully single-threaded runtime would have made `EV-AUDIO-TRACE` and `EV-PERF-BUDGETS` unreachable under contention, and a multi-process architecture would have shifted `EV-CRASH-RESUME` and `EV-OBSERVABILITY` onto IPC-mediated semantics this ADR explicitly forecloses for v1.

- `EV-DETERMINISM` — single-writer `StateCore` on the state commit thread is the precondition the determinism replay corpus depends on; if multiple threads could write `StateCore` directly, the typed-command total-order validated by `engine/runtime/src/command_bus.rs` would not be sufficient. The slice-2 Pass on `docs/26` `EV-INTEGRATION-MATRIX` is what this ADR's threading decision realizes.
- `EV-PERF-BUDGETS` — per-thread latency attribution is what `10-performance-latency-budgets.md` measures; the per-subsystem thread allocation here is the source of per-thread budget rows (render thread frame budget, audio analysis thread cadence, integration IO thread queue latency).
- `EV-RENDER-CONFORMANCE` — render/scheduler thread owns the frame graph; thread affinity is what prevents drift between scene-script frame-state reads and frame-graph commits the conformance suite measures.
- `EV-AUDIO-TRACE` — audio analysis thread is a dedicated thread by this ADR; the row's fixture-trace comparison depends on the audio-snapshot publication being on a known cadence rather than serialized behind unrelated runtime work.
- `EV-OBSERVABILITY` — clear observability boundaries for latency attribution per thread; the per-thread metric attribution validated at `EV-SCHEDULER-TICK` (`engine/observability/src/catalog.rs` Pass at slice-2) presupposes the per-thread allocation declared here.
- `EV-CRASH-RESUME` — single-process posture is the row's recovery model; per-class state restoration scope (active scene, preset values, Snap slots, playlist position, OSC/MIDI bindings) maps to subsystems whose threads + queues are owned by this ADR.
- `EV-STATE-REPLAY` — `StateCore` on the state commit thread = single-writer = canonical event log = replay determinism. The threading half of `adr/ADR-004-state-model.md`'s typed-command-validated-before-commit posture.
- `EV-INTEGRATION-MATRIX` — integration IO thread pool isolates protocol IO from the render and audio threads; the conformance matrix relies on adapters not being able to delay frame scheduling. The post-MVP data-adapter thread pool is a co-change with `adr/ADR-005-integration-adapter-boundary.md` Sec.Post-MVP Extension Points.

## Traceability

- `04-runtime-architecture.md` — primary planning doc for the runtime architecture this ADR's threading decomposition implements.
- `06-audio-engine.md` — audio analysis thread cadence + publication contract.
- `08-io-integrations.md` — integration IO thread pool work (per-protocol adapter ownership).
- `10-performance-latency-budgets.md` — per-thread latency budgets the row `EV-PERF-BUDGETS` measures.
- `28-implementation-contract-pack.md` CP-04 (typed command schema dispatched on the commit thread), CP-08 (cross-thread snapshot contract).
- `adr/ADR-001-runtime-model.md` — runtime decomposition this ADR allocates threads for.
- `adr/ADR-004-state-model.md` — `StateCore` on the state commit thread; this ADR is the threading half.
- `adr/ADR-005-integration-adapter-boundary.md` — adapter boundary owns the integration IO thread pool work; the post-MVP data-adapter thread pool is a co-change.
- `adr/ADR-016-observability-and-telemetry-posture.md` — per-thread metric and trace attribution the observability posture publishes.
- `13-risk-register.md` R-06 (state race conditions across input sources) — single-writer `StateCore` is the closure path; R-04 (audio latency and jitter instability) — dedicated audio analysis thread is the closure path; R-09 (architecture drift without decision logging) — this ADR is the locking decision for the threading model.
- `03-product-requirements.md` NFR-2 (Determinism), NFR-3 (Performance), NFR-4 (Observability), NFR-1 (Reliability).

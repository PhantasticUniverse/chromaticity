# ADR-001: Core Runtime Model

## Status
Accepted (planning baseline)

## Context
Chromaticity must operate as a live-performance system with strict timing behavior, predictable state transitions, and strong fault containment across scenes, scripts, and integrations.

## Decision
Adopt a deterministic frame-orchestrated runtime with explicit subsystem boundaries:
- Scheduler orchestrates frame ticks.
- SceneRuntime owns scene lifecycle and frame graph.
- AudioRuntime publishes sampled reactive state.
- ScriptRuntime executes scene script hooks with controlled host API.
- IntegrationHub adapts external protocols to internal commands/events.

Use a single canonical internal state model as the only source of truth for:
- active scene,
- control values,
- meta controls,
- preset/playlist state,
- integration mappings.

## Consequences
### Positive
- Strong debuggability and reproducibility.
- Clear ownership boundaries and easier testing.
- Predictable event ordering.

### Negative
- Requires disciplined API design across subsystems.
- Early planning effort is higher than ad hoc integration.

## Alternatives Considered
- Event-only architecture without central canonical state (rejected: harder determinism).
- Plugin-first decentralized runtimes (rejected for v1 complexity/risk).

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision. The frame-orchestrated runtime + single-canonical-state-model decision is the architectural precondition for the determinism-class evidence set; an event-only or plugin-first alternative would have shifted every replay-determinism, render-conformance, and state-replay row's closure path off the runtime baseline this ADR locks.

- `EV-DETERMINISM` — frame-orchestrated dispatcher + canonical state is the precondition the determinism replay corpus measures. The `engine/runtime/src/command_bus.rs` typed-command + commit-thread implementation that Pass'd the row at slice-2 (`docs/26` `EV-INTEGRATION-MATRIX`) is the direct realization of this ADR's "single canonical internal state model" decision.
- `EV-RENDER-CONFORMANCE` — `SceneRuntime` ownership of scene lifecycle and frame graph is the boundary the render-conformance suite tests against; the multipass + feedback parity behavior measured by the row depends on the SceneRuntime owning the frame graph rather than scripts owning rendering directly.
- `EV-SCRIPT-CONTRACT` — `ScriptRuntime` lifecycle hooks + controlled host API is the contract the row tests; subsequently refined by `adr/ADR-003-script-runtime-sandbox.md` (architectural sandbox) and `adr/ADR-012-script-sandbox-engine.md` (engine choice + budgets), but the lifecycle separation comes from this ADR.
- `EV-AUDIO-TRACE` — `AudioRuntime` publishing sampled reactive state on a known cadence is what the audio-trace fixture comparison harness compares against; without the runtime separation this row would not have a deterministic publication-point to sample.
- `EV-STATE-REPLAY` — single canonical state + typed commands is the precondition; `adr/ADR-004-state-model.md` refines into the per-store decomposition this row tests, but the canonical-state decision originates here.
- `EV-INTEGRATION-MATRIX` — `IntegrationHub` as the canonical adapter sink is what the conformance matrix asserts; refined by `adr/ADR-005-integration-adapter-boundary.md`.
- `EV-PERF-BUDGETS` — frame-orchestrated tick is the budget unit `10-performance-latency-budgets.md` measures against.
- `EV-OBSERVABILITY` — explicit subsystem boundaries are observable; the per-subsystem metric attribution validated by `EV-SCHEDULER-TICK` (`engine/observability/src/catalog.rs`) presupposes the subsystem decomposition declared here.

## Traceability

- `04-runtime-architecture.md` — primary planning doc; the canonical decomposition (Scheduler / SceneRuntime / AudioRuntime / ScriptRuntime / IntegrationHub) lives there.
- `05-scene-system.md` — SceneRuntime ownership of scene lifecycle and frame graph.
- `06-audio-engine.md` — AudioRuntime publication contract.
- `07-control-state-presets.md` — canonical state model the runtime hosts.
- `08-io-integrations.md` — IntegrationHub contract.
- `28-implementation-contract-pack.md` CP-01..CP-12 — runtime-facing contracts the subsystem boundaries enforce.
- `adr/ADR-002-rendering-backend.md` — frame-graph strategy inside SceneRuntime (superseded on backend selection by `adr/ADR-011-rendering-backend.md`).
- `adr/ADR-003-script-runtime-sandbox.md` — ScriptRuntime sandbox shape (superseded on engine choice by `adr/ADR-012-script-sandbox-engine.md`).
- `adr/ADR-004-state-model.md` — canonical state decomposition the runtime hosts.
- `adr/ADR-006-thread-and-process-model.md` — thread allocation that hosts the subsystems declared here.
- `adr/ADR-005-integration-adapter-boundary.md` — IntegrationHub adapter-boundary contract.
- `13-risk-register.md` R-09 (architecture drift without decision logging) — this ADR is the locking decision for the runtime decomposition; absence of an ADR-001 was the residual risk surface R-09 was authored against.
- `03-product-requirements.md` NFR-2 (Determinism) — the runtime-architectural precondition; NFR-1 (Reliability) — fault containment across subsystems; NFR-3 (Performance) — frame-orchestrated tick is the latency unit.

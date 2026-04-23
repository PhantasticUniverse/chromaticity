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

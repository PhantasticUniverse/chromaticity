# ADR-002: Rendering Backend and Frame Graph Strategy

## Status
Superseded by ADR-011

The frame-graph strategy and pass-level semantics in this ADR remain the planning baseline and are cited throughout the corpus; the backend-selection portion (e.g., choice of graphics API, shader toolchain) is superseded by `ADR-011-rendering-backend.md` (wgpu + naga + GLSL 4.50). Content below is retained for traceability; Sec. on backend selection MUST be read as historical and deferred to ADR-011 for normative decisions.

## Context
The reference-app's shipped scenes (`../Contents/Resources/release/*.synScene`) heavily rely on multipass rendering, feedback textures, configurable pass precision, and media sampling helpers. Chromaticity must preserve these behaviors under `compat-syn-v1` while enforcing performance budgets.

## Decision
Use an explicit frame-graph renderer with:
- deterministic pass ordering,
- per-pass resource descriptors (size, format, wrap, filter),
- explicit final-frame feedback resource,
- pass invalidation and hot-reload-safe recompile pipeline.

Represent scene rendering contracts in an engine-neutral internal interface so the backend implementation can evolve without changing scene semantics.

## Consequences
### Positive
- Clear and testable pass behavior.
- Easier performance instrumentation per pass.
- Better compatibility for multipass-heavy scenes.

### Negative
- More upfront engineering than immediate shader execution.

## Alternatives Considered
- Implicit pass execution from shader conditionals only (rejected: weak observability).
- Hardcoded fixed pipeline (rejected: incompatible with scene diversity).

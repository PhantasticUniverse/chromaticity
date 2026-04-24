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

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision. The frame-graph strategy and pass-level semantics here remain normative even though backend selection is superseded by `adr/ADR-011-rendering-backend.md`; the rows below depend on the engine-neutral frame-graph contract this ADR established, with backend-specific evidence rolled forward into ADR-011's coverage.

- `EV-RENDER-CONFORMANCE` — multipass + feedback behavior is the row's direct subject; the frame-graph strategy declared here (deterministic pass ordering, per-pass resource descriptors, explicit feedback resource) is the architectural precondition. Backend-specific conformance details (wgpu adapter selection, GLSL 4.50 shader toolchain) are rolled forward to ADR-011.
- `EV-COMPAT-FIXTURE-COVERAGE` — CP-01 missing-`TYPE` coercion fallback fixture coverage uses the frame-graph contract; without per-pass resource descriptors the coercion-fallback branches (slider, toggle, color, xy, strict-reject) would not be testable on a per-pass basis. The slice-2 T1 cr-01 branch coverage at `engine/scene` (`docs/26` row 25 + PR `#47`) tests against this ADR's contract.
- `EV-PERF-BUDGETS` — per-pass performance instrumentation is what the frame-graph enables; budgets are declared per-pass in `10-performance-latency-budgets.md` because this ADR makes the per-pass boundary first-class.
- `EV-SHADER-CACHE` — the hot-reload-safe recompile pipeline this ADR mandates is what the shader cache wraps; cold vs warm compile budgets are measurable because the per-pass shader is a cache key.
- `EV-IMPORT-SHADERTOY` — multipass Shadertoy imports lower into the frame-graph defined here; row's golden-fixture conversion is to this contract.
- `EV-GPU-SELECTION` — deterministic multi-GPU adapter selection is declared in `adr/ADR-011-rendering-backend.md` Sec.7, but the per-pass invalidation pipeline this ADR mandates is what makes adapter switches recoverable without scene reconstruction.
- `EV-COLOR-MANAGEMENT-SRGB` / `EV-COLOR-MANAGEMENT-HDR` — output color-space behavior is delegated to ADR-011 (V1 sRGB) and the post-MVP HDR amendment of ADR-011 Sec.8; the frame-graph here is colour-space-neutral and remains the host for either pipeline.

The backend-selection coverage portion of this ADR is rolled forward into the `EV-ARCH-CONTRACT-BASELINE` enumeration via ADR-011 (per `docs/26` row 22, ADR-011 is the gating ADR; ADR-002 is not enumerated because the superseder carries the gating responsibility).

## Traceability

- `02-synesthesia-contracts.md` Sec.15-16 (multipass + feedback) + Sec.18 (`#include` resolution contract).
- `05-scene-system.md` Sec.Canonical Internal Scene Model — scene rendering contracts the frame graph realizes.
- `28-implementation-contract-pack.md` CP-10 (Pass-Sizing Algebra: `RGBA32F` default + `RGBA16F` fallback) — the per-pass resource descriptor format this ADR mandates.
- `adr/ADR-001-runtime-model.md` — `SceneRuntime` ownership of the frame graph.
- `adr/ADR-011-rendering-backend.md` — supersedes the backend-selection portion of this ADR (wgpu + naga + GLSL 4.50). Bidirectional supersession link declared on both files (this ADR's Status; ADR-011 lines 116, 127).
- `adr/ADR-007-scene-warm-switch-policy.md` — the four-phase warm-switch lifecycle that operates on the frame graph declared here.
- `13-risk-register.md` R-03 (performance budget overruns in multipass scenes) — per-pass instrumentation is the closure path, owned forward by `EV-PERF-BUDGETS`.
- `03-product-requirements.md` FR-3 (Rendering Runtime); NFR-2 (Determinism); NFR-3 (Performance).

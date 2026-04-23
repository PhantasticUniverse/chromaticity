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

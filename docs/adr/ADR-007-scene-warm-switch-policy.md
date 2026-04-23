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

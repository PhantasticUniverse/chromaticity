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

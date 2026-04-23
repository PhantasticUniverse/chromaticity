# ADR-003: Script Runtime Sandbox and Host API

## Status
Superseded by ADR-012

The sandbox shape, host-API posture, and lifecycle semantics in this ADR remain the planning baseline. Engine choice (e.g., QuickJS vs alternatives) and per-frame budget numerics (p99 execution budget, hard watchdog) are superseded by `ADR-012-script-sandbox-engine.md`. Content below is retained for traceability; Sec. on engine selection and budget values MUST be read as historical and deferred to ADR-012 for normative decisions.

## Context
Scene scripts are essential for behavior in many of the reference-app's bundled scenes (observed in `../Contents/Resources/release/*.synScene/script.js`). Chromaticity must support script lifecycle, event APIs, and control updates under `compat-syn-v1` while protecting runtime stability.

## Decision
Implement script execution in a constrained sandbox with:
- explicit lifecycle hooks (`setup`, `update`, transition hook),
- allowlisted host API (`setUniform`, control updates, event registration, optional pixel readback helpers),
- deterministic host call ordering relative to frame scheduler,
- runtime guards (timeout, memory and error containment),
- compatibility shims for deprecated patterns where required.

## Consequences
### Positive
- Strong fault isolation.
- Safer extension surface.
- Easier migration support.

### Negative
- Sandbox constraints may require edge-case compatibility work.

## Alternatives Considered
- Full unrestricted embedded JS runtime (rejected: stability and security risk).
- No script support in MVP (rejected: parity failure).

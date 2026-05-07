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

## Evidence

`EV-*` IDs whose status is produced, advanced, or blocked by this decision. The sandbox shape (allowlisted host API, lifecycle hooks, deterministic host call ordering, runtime guards) remains normative even though engine choice and budget numerics are superseded by `adr/ADR-012-script-sandbox-engine.md`; the rows below depend on the sandbox-architectural contract this ADR locks, with engine-specific evidence rolled forward into ADR-012's coverage.

- `EV-SCRIPT-CONTRACT` — script lifecycle hooks (`setup`, `update`, `transition`) and the allowlisted host API (`setUniform`, control updates, event registration, optional pixel readback helpers) are the row's direct subjects; the contract this row tests originates here, with engine-specific behavior validated by ADR-012's engine choice.
- `EV-SANDBOX-SAFETY` — script isolation boundaries are this row's direct subject. The runtime guards declared here (timeout, memory, error containment) are what the sandbox security suite validates; budget numerics (p99 execution budget, hard watchdog timing) are delegated to ADR-012.
- `EV-DETERMINISM` — deterministic host call ordering relative to the frame scheduler is part of the runtime determinism precondition; the determinism replay corpus (`docs/26` `EV-INTEGRATION-MATRIX`) depends on script host-call dispatch being reproducible across runs. JIT vs interpreted engine choice is delegated to ADR-012 because JIT compilers are a known source of cross-run variance.
- `EV-AUTHORING-DIAGNOSTICS` — script error-containment is part of the inline/console diagnostic mapping the row tests; the runtime-guards posture here is what makes script errors recoverable to actionable diagnostics rather than runtime crashes.
- `EV-CRASH-RESUME` — fault isolation of script crashes (per the `## Positive` consequences) is what keeps script-induced failures within their containment boundary rather than escalating to host-process crashes; the row's recovery model presupposes this isolation.

The engine-choice + budgets coverage portion of this ADR is rolled forward into the `EV-ARCH-CONTRACT-BASELINE` enumeration via ADR-012 (per the `EV-ARCH-CONTRACT-BASELINE` row in `docs/26`, ADR-012 is the gating ADR; ADR-003 is not enumerated because the superseder carries the gating responsibility).

## Traceability

- `02-synesthesia-contracts.md` Sec.Script API parity contract — the script API parity surface this ADR's allowlist realizes.
- `04-runtime-architecture.md` — `ScriptRuntime` placement within the runtime decomposition.
- `05-scene-system.md` Sec.Scene Lifecycle — script lifecycle hook placement relative to scene lifecycle.
- `28-implementation-contract-pack.md` CP-07 (script API parity matrix) — the row-level allowlist this ADR enforces.
- `adr/ADR-001-runtime-model.md` — `ScriptRuntime` ownership and runtime decomposition.
- `adr/ADR-012-script-sandbox-engine.md` — supersedes the engine-choice and budget-numerics portion of this ADR (QuickJS engine selection + p99 execution budget + hard watchdog timing). Bidirectional supersession link declared on both files (this ADR's Status; ADR-012 line 88).
- `adr/ADR-006-thread-and-process-model.md` — thread placement of script execution; deterministic host-call ordering is enforced by this ADR's threading decision.
- `adr/ADR-008-transition-and-traveler-semantics.md` — `transition()` hook timing + cardinality is enforced by ADR-008 against the lifecycle-hook contract this ADR declares.
- `adr/ADR-013-networking-defaults-and-security.md` — network surface that the script sandbox host-API allowlist excludes.
- `13-risk-register.md` R-02 (script runtime behavioral drift) — runtime guards + allowlisted host API is the closure path; explicit lifecycle hook semantics + sandbox boundaries prevent the drift R-02 was authored against.
- `03-product-requirements.md` FR-4 (Script Runtime); NFR-2 (Determinism); NFR-6 (Security and Isolation).

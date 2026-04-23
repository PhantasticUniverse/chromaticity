# ADR-012: Script Sandbox Engine and Budgets

## Status
Accepted

## Context
`ADR-003-script-runtime-sandbox.md` specifies the architectural sandbox (allowlisted host API, lifecycle hooks, fault isolation). `28-implementation-contract-pack.md` CP-07 specifies the script API parity matrix. Neither picks a concrete JavaScript engine or locks the numeric budgets that the watchdog enforces. Without these, `R-02` (script behavioral drift), `EV-SANDBOX-SAFETY`, and `EV-SCRIPT-CONTRACT` cannot be closed with confidence. The engine choice also affects NFR-2 (determinism) because JIT compilers are a well-known source of cross-run variance.

## Decision

### 1. Engine: **QuickJS** (bellard/quickjs)

- ECMAScript 2020 compliant, interpreter-only (no JIT), ~600 KB compiled binary size.
- Deterministic across runs and platforms for a given input program.
- Embedded in Rust via `quickjs-ng` or `rquickjs` bindings; matches `ADR-011-rendering-backend.md` Rust ecosystem.
- BSD-2-Clause license; compatible with `29-cleanroom-legal-policy-baseline.md` allowed-license policy.

### 2. Per-frame budgets

| Budget dimension | Value | Notes |
|---|---:|---|
| `update(dt)` CPU time target (p99) | 1.75 ms | matches frame-budget allocation in `10-performance-latency-budgets.md` |
| `update(dt)` hard watchdog timeout | 4.00 ms | preempts script, frame continues with last-known state |
| `setup()` hard timeout | 500 ms | on scene load; timeout marks scene degraded |
| `transition()` hard timeout | 250 ms | per transition boundary |
| Event-handler hard timeout (per call) | 0.5 ms | invoked from `onChange`/`onOffToOn`/etc. |

### 3. Memory budgets
- Per-scene heap cap: 16 MiB (QuickJS `JS_SetMemoryLimit`).
- Per-scene stack cap: 256 KiB.
- String interning is enabled; authors referencing the same control names across many event handlers do not balloon the heap.
- Heap snapshot on approach-to-cap (>= 80% used) emits `script-heap-warning` diagnostic.

### 4. Watchdog behavior
- Timeout on `update(dt)` -> preempt script, frame advances with previous uniform state, emit `script-update-timeout` diagnostic, increment `consecutive_timeouts` counter.
- 2 consecutive timeouts -> scene is marked `Degraded`; operator HUD surfaces this; output pipeline unaffected.
- 3 consecutive timeouts -> scene is reset via `PrepareNext`/`PrimeNext` per CP-03 using current controls; script state is re-seeded at rebuild boundary per `37-authoring-environment-spec.md` Sec.5.
- Timeout on `setup()` -> scene enters `Error` lifecycle state; fallback (previous scene or black frame per NFR-1); operator can retry.
- Timeout on event handler -> handler is unregistered for the remainder of the frame; diagnostic emitted; subsequent frames re-enable it.

### 5. Determinism rules
- QuickJS interpreter is deterministic given identical inputs and RNG seed.
- `Math.random()` is shimmed with a seeded PRNG (xoroshiro128+) reseeded per scene load; seed is part of the scene's deterministic replay metadata (`EV-DETERMINISM`).
- `Date.now()` is shimmed with a frame-locked wall-clock snapshot taken at frame start; all `Date.now()` calls within a single `update(dt)` return the same value.
- `performance.now()` is shimmed similarly, returning a monotonic frame-start timestamp.
- Host-API callbacks into JS are invoked in deterministic ordering index per CP-07.

### 6. Security boundaries (NFR-6)
- QuickJS runs in its own isolate per scene. No cross-scene memory access.
- Host-API allowlist enforced at binding time; unbound globals throw on access rather than silently no-op.
- No network/IO/filesystem globals exposed; `fetch`, `XMLHttpRequest`, `require`, `import` are not bound.
- `chromaticity.entitled(token)` is read-only; writing to entitlement state from script is not possible because the mirror is behind a frozen proxy.

### 7. Hot-reload semantics
- On `script.js` change (per `37-authoring-environment-spec.md` Sec.3.3), the entire QuickJS isolate is discarded and a new one created. Script-local state is reset; control values and render state are preserved.
- Per-scene PRNG seed is reset deterministically at rebuild (authors can rely on identical first-frame behavior across saves).

### 8. Compat-mode shims (`compat-syn-v1`)
- `inputs[name]` and `uniforms[name]` proxies are implemented as a single frozen handler invoking `getControl`/`setControl`. First access per scene load emits `deprecated-script-api` warning.
- `print(...)` and `console.log(...)` are shimmed to write to the diagnostics console per `37-authoring-environment-spec.md` Sec.3.4.

## Consequences

### Positive
- Determinism across runs and platforms - `EV-DETERMINISM` and `EV-SCRIPT-CONTRACT` can assert bit-identical state hashes.
- Small footprint (~600 KB) vs V8 (20+ MB) keeps app bundle reasonable.
- No JIT -> no surprise de-optimization spikes during live shows.
- BSD-2 license is commercially clean.

### Negative
- Interpreter is ~3-10x slower than V8 JIT for hot loops. The 1.75 ms p99 target is still achievable for typical scene scripts, but scripts that do heavy per-frame math must be GLSL-preferred.
- QuickJS ecosystem is smaller than V8; fewer stack traces / debug tools off-the-shelf.

## Alternatives Considered

- **V8 isolate (via `rusty_v8`)**: rejected. Non-deterministic JIT, 20+ MB footprint, complex build dependency, license compatible but operational overhead significant.
- **Duktape**: rejected. ES5.1 only; author UX regression (no `let`/`const`, no arrow functions, no template literals is visible degradation vs modern JS).
- **Rhino (JVM)**: rejected. Adds a JVM dependency to the app; out of character for a Rust/native desktop runtime.
- **Custom-built DSL**: rejected. Script compatibility with reference scenes depends on JavaScript; a DSL would break FR-4 and ATF-04 migration.

## Evidence
- `EV-SANDBOX-SAFETY`: validates isolate boundary (no cross-scene mem access), host-API allowlist enforcement, entitlement-state immutability.
- `EV-SCRIPT-CONTRACT`: validates CP-07 surface behaves identically across strict/compat profiles.
- `EV-DETERMINISM`: validates identical RNG/Date/state hashes across runs.
- `EV-PERF-BUDGETS`: validates p99 `update(dt)` time within 1.75 ms under synthetic script load.

## Traceability
- `ADR-003-script-runtime-sandbox.md` (supersedes engine-choice and budgets portion)
- `28-implementation-contract-pack.md` CP-07
- `10-performance-latency-budgets.md` frame-budget allocation
- `13-risk-register.md` R-02, R-17
- `26-evidence-artifact-registry.md` `EV-SANDBOX-SAFETY`, `EV-SCRIPT-CONTRACT`, `EV-DETERMINISM`, `EV-PERF-BUDGETS`

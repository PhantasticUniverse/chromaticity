// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine script.
//!
//! Phase-2 slice 1 mounts the rquickjs Rust binding to QuickJS (per
//! ADR-012) and produces the foundational evidence harness toward
//! `EV-SCRIPT-CONTRACT`. The crate's surface is intentionally minimal:
//! a sandboxed [`ScriptRuntime`] that compiles source, runs it
//! synchronously, returns a structured [`ScriptValue`], or surfaces
//! a structured [`ScriptError`].
//!
//! Slice progression:
//!
//! - **Slice 1** — sandbox-default toolchain (no I/O / FS /
//!   native modules), [`ScriptRuntime::execute`] entry point, T1 / T2
//!   contract evidence. The default sandbox posture comes for free
//!   from the rquickjs default feature set: no `loader` feature
//!   means no module resolver is mounted, no `parallel` /
//!   `futures` features means no async / threading surface, and no
//!   custom global bindings are installed. `globalThis.fs`,
//!   `globalThis.process`, `globalThis.window` are all `undefined`
//!   by construction, and `import('foo')` rejects at parse time as
//!   ECMAScript-module syntax outside an `eval`-mode script.
//! - **Slice 2 (this PR)** — watchdog timeouts (1 ms p99 soft / 4 ms
//!   hard per ADR-012 §2) via
//!   `rquickjs::Runtime::set_interrupt_handler`. The
//!   [`ScriptRuntime::with_watchdog`] builder method mounts a
//!   per-execute interrupt handler against the shared rquickjs
//!   runtime; the soft cap is observed via [`watchdog::WatchdogObservation`]
//!   without aborting, the hard cap aborts the script and the
//!   caller receives [`ScriptError::TimeoutExceeded`] populated
//!   with the deadline-derived budget context. `EV-SANDBOX-SAFETY`
//!   evidence opens; `EV-SCRIPT-CONTRACT` widens with the watchdog
//!   path. The slice-1 `ScriptRuntime::new()` constructor stays
//!   watchdog-free for slice-1 callers' continuity.
//! - **Slice 3** — memory cap (16 MiB heap per ADR-012 §3) via
//!   `rquickjs::Runtime::set_memory_limit`. `EV-SANDBOX-SAFETY`
//!   widens.
//! - **Slice 4** — determinism shims: seeded `Math.random()`
//!   (xoroshiro128+), frame-locked `Date.now()` and
//!   `performance.now()` per ADR-012 §5. `EV-SCRIPT-CONTRACT`
//!   widens; `EV-DETERMINISM` opens for the script lane.
//! - **Slice 5+** — command-bus integration (`engine/runtime`
//!   mount), reactive-symbol-catalog binding (docs/36 v1.1.0),
//!   scene-author surface (control bindings, event handlers,
//!   preset-recall hooks per ADR-003).
//!
//! Produces evidence for `EV-SCRIPT-CONTRACT`:
//!
//! - T1 entry-point drift pins in
//!   [`runtime::execute_drift_pins`] — slice-1 sandbox-default
//!   posture, parse-error / runtime-error categorisation, plus
//!   slice-2 watchdog drift pins (interrupt-handler install,
//!   hard-cap → [`ScriptError::TimeoutExceeded`] lowering,
//!   observation recording, builder-level config validation).
//! - T2 deterministic contract corpus in
//!   `engine/script/tests/contract.rs` — function-purity, array /
//!   object round-trip, and per-runtime isolation.
//!
//! Produces evidence for `EV-SANDBOX-SAFETY` (opened by slice 2):
//!
//! - T1 deadline arithmetic + watchdog config validation pins in
//!   [`watchdog::deadline_pins`] — wall-clock elapsed accounting,
//!   inverted-pair / zero-hard rejection, soft-cap signal flag.
//! - T1 watchdog integration drift pins in
//!   [`runtime::execute_drift_pins`] — interrupt-handler install /
//!   detach behaviour against the live rquickjs runtime.
//! - T2 timeout corpus in `engine/script/tests/sandbox.rs` —
//!   tight-CPU loop, recursion explosion, busy `Date.now()`
//!   polling, multi-runtime watchdog independence.
//!
//! Per `docs/26-evidence-artifact-registry.md` rows 29 / 42
//! Phase-2-exit definitions, full conformance — host-API parity
//! matrix, lifecycle hooks, event ordering, full sandbox-isolation
//! audit — requires successive slices. Both rows stay `Planned` /
//! `pending:artifact` while the harness path tightens. Registry
//! updates after slice 2 land on a separate small follow-up commit
//! (this PR is `engine/script` only and does not touch
//! `docs/26-evidence-artifact-registry.md` or
//! `docs/13-risk-register.md`).
//!
//! Governing ADRs:
//!
//! - ADR-003 — script runtime sandbox
//!   (`docs/adr/ADR-003-script-runtime-sandbox.md`).
//! - ADR-012 — script sandbox engine and budgets
//!   (`docs/adr/ADR-012-script-sandbox-engine.md`).

mod error;
mod runtime;
mod value;
pub mod watchdog;

pub use error::ScriptError;
pub use runtime::ScriptRuntime;
pub use value::ScriptValue;
pub use watchdog::{Deadline, WatchdogConfig, WatchdogConfigError, WatchdogObservation};

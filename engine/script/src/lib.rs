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
//! - **Slice 1 (this PR)** — sandbox-default toolchain (no I/O / FS /
//!   native modules), [`ScriptRuntime::execute`] entry point, T1 / T2
//!   contract evidence. The default sandbox posture comes for free
//!   from the rquickjs default feature set: no `loader` feature
//!   means no module resolver is mounted, no `parallel` /
//!   `futures` features means no async / threading surface, and no
//!   custom global bindings are installed. `globalThis.fs`,
//!   `globalThis.process`, `globalThis.window` are all `undefined`
//!   by construction, and `import('foo')` rejects at parse time as
//!   ECMAScript-module syntax outside an `eval`-mode script.
//! - **Slice 2** — watchdog timeouts (1.75 ms p99 / 4 ms hard per
//!   ADR-012 §2) via `rquickjs::Runtime::set_interrupt_handler`.
//!   `EV-SANDBOX-SAFETY` evidence opens.
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
//!   posture and parse-error / runtime-error categorisation.
//! - T2 deterministic contract corpus in
//!   `engine/script/tests/contract.rs` — function-purity, array /
//!   object round-trip, and per-runtime isolation.
//!
//! Per `docs/26-evidence-artifact-registry.md` row 29's Phase-2-exit
//! definition, full `EV-SCRIPT-CONTRACT` conformance — host-API
//! parity matrix, `setup`/`update`/`transition` lifecycle hooks,
//! event-handler ordering — requires successive slices; the row's
//! status stays `Planned` while the harness path tightens. The
//! registry update after slice 1 lands on a separate small
//! follow-up commit (this PR is `engine/script` only and does not
//! touch the registry).
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

pub use error::ScriptError;
pub use runtime::ScriptRuntime;
pub use value::ScriptValue;

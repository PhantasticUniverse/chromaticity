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
//! - **Slice 3 (this PR)** — memory cap (16 MiB heap per ADR-012
//!   §3) via `rquickjs::Runtime::set_memory_limit`. The
//!   [`ScriptRuntime::with_memory_limit`] builder method mounts
//!   the cap on every per-execute bracketed scope; allocations
//!   that would cross the limit raise QuickJS's uncatchable
//!   `InternalError: out of memory` and the caller receives
//!   [`ScriptError::MemoryCapExceeded`] populated with the
//!   limit-derived budget context. The post-execute
//!   [`memory_cap::MemoryUsageSnapshot`] is exposed via
//!   [`ScriptRuntime::last_memory_snapshot`] for in-process
//!   inspection of `malloc_size` at install vs uninstall.
//!   `EV-SANDBOX-SAFETY` widens with the memory-cap path; the
//!   slice-2 watchdog continues to compose alongside the
//!   memory cap on a single-runtime builder chain.
//! - **Slice 4 (this PR)** — determinism shims: seeded
//!   `Math.random()` (xoroshiro128+, expanded from a caller-supplied
//!   `u64` seed via splitmix64) and frame-locked `Date.now()` (a
//!   caller-injected [`determinism::Clock`] trait object,
//!   defaulting to no shim — slice-1 / slice-2 / slice-3 callers
//!   continue to see the QuickJS wall-clock built-ins). The
//!   [`ScriptRuntime::with_random_seed`] and
//!   [`ScriptRuntime::with_clock`] builder methods compose with
//!   the slice-2 watchdog and slice-3 memory cap, install the
//!   shims on the rquickjs [`rquickjs::Ctx::globals`] object once
//!   at builder time, and surface install failures as
//!   [`ScriptError::DeterminismInstall`]. `EV-SCRIPT-CONTRACT`
//!   widens with the determinism path. `EV-DETERMINISM` opens
//!   for the script lane in a later slice once the recorded-seed
//!   replay corpus lands.
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
//!   observation recording, builder-level config validation),
//!   plus slice-4 determinism drift pins (default `Math.random`
//!   without seed still works, default `Date.now` without clock
//!   still works, seeded `Math.random` advances per call,
//!   frame-locked `Date.now` returns the injected tick exactly).
//! - T1 PRNG / clock data-shape pins in
//!   [`determinism::determinism_pins`] — splitmix64 permutation
//!   contract, xoroshiro128+ same-seed-same-sequence contract,
//!   `[0,1)` projection contract, [`determinism::FixedClock`] /
//!   [`determinism::MonotonicClock`] built-in surface.
//! - T2 deterministic contract corpus in
//!   `engine/script/tests/contract.rs` — function-purity, array /
//!   object round-trip, and per-runtime isolation; widened by
//!   `engine/script/tests/determinism.rs` (slice 4) with the
//!   cross-runtime seed-determinism, seed-isolation,
//!   PRNG-state-isolation, and frame-locked-clock corpus.
//!
//! Produces evidence for `EV-SANDBOX-SAFETY` (opened by slice 2,
//! widened by slice 3):
//!
//! - T1 deadline arithmetic + watchdog config validation pins in
//!   [`watchdog::deadline_pins`] — wall-clock elapsed accounting,
//!   inverted-pair / zero-hard rejection, soft-cap signal flag.
//! - T1 watchdog integration drift pins in
//!   [`runtime::execute_drift_pins`] — interrupt-handler install /
//!   detach behaviour against the live rquickjs runtime.
//! - T1 memory-cap config validation + snapshot accounting pins
//!   in [`memory_cap::cap_pins`] — phase-2 default acceptance,
//!   zero-limit rejection, install / uninstall snapshot
//!   advancement, OOM-error identifier match.
//! - T1 memory-cap integration drift pins in
//!   [`runtime::execute_drift_pins`] — `set_memory_limit` install
//!   / uninstall, OOM → MemoryCapExceeded lowering, builder
//!   chaining with the watchdog.
//! - T2 timeout corpus in `engine/script/tests/sandbox.rs` —
//!   tight-CPU loop, recursion explosion, busy `Date.now()`
//!   polling, multi-runtime watchdog independence (slice 2);
//!   plus large-string + recursive-array allocation pressure and
//!   per-runtime memory cap isolation (slice 3).
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

pub mod determinism;
mod error;
pub mod memory_cap;
mod runtime;
mod value;
pub mod watchdog;

pub use determinism::{Clock, FixedClock, MonotonicClock, RandomSeedConfig};
pub use error::ScriptError;
pub use memory_cap::{MemoryCapConfig, MemoryUsageSnapshot};
pub use runtime::ScriptRuntime;
pub use value::ScriptValue;
pub use watchdog::{Deadline, WatchdogConfig, WatchdogConfigError, WatchdogObservation};

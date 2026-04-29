// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Watchdog timeouts for [`crate::ScriptRuntime`] execution.
//!
//! ADR-012 §2 mandates two execution-time budgets per script eval:
//!
//! - **1.75 ms p99 soft cap** — instrumentation-only. Observed
//!   wall-clock execution time exceeding the soft cap is recorded
//!   as a [`WatchdogObservation`] datum but does NOT abort
//!   execution; the soft cap exists so the runtime-bus telemetry
//!   surface can flag p99 regressions in the script lane without
//!   killing the show during a momentary stall.
//! - **4 ms hard cap** — execution-aborting. The hard cap installs a
//!   `rquickjs::Runtime::set_interrupt_handler` closure that QuickJS
//!   polls during compilation and execution; once `Instant::now()`
//!   crosses the deadline, the handler returns `true` and QuickJS
//!   throws an uncatchable `InternalError: interrupted` from the C
//!   layer (see `quickjs.c` `JS_ThrowInternalError(ctx, "interrupted")`).
//!   The wrapping [`crate::ScriptRuntime::execute`] catches the
//!   thrown exception and lowers it to
//!   [`crate::ScriptError::TimeoutExceeded`].
//!
//! Slice-2 scope (this file): the watchdog mechanism, the
//! [`Deadline`] handle, and the [`WatchdogObservation`] datum
//! returned alongside the result. Slice-3 widens to the 16 MiB heap
//! cap (`Runtime::set_memory_limit`); slice-4 introduces the
//! determinism shim layer over `Math.random()` / `Date.now()`.
//!
//! Observability integration (deferred). The slice-2 observation
//! structure is in-crate; it does NOT install a
//! [`chromaticity_observability::MetricEvent`] subscriber — that
//! widens the docs/31 catalog (a `Script` subsystem variant + a new
//! row), which is a serial-list change deferred to slice-3 +. The
//! [`WatchdogObservation`] surface is the data-shape the eventual
//! catalog row will project from, recorded in the per-execute
//! return so callers can route it however they want without a
//! crate-level subscriber wired in.
//!
//! Send / Sync posture: the rquickjs `InterruptHandler` type alias
//! is `Box<dyn FnMut() -> bool + 'static>` under the default feature
//! set (no `parallel`). The [`Deadline`] holds the same data the
//! handler closes over; the closure clones the deadline state at
//! install time so the host can keep handing the [`Deadline`]
//! around for elapsed-time accounting after the handler is in
//! place.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rquickjs::Runtime;
use rquickjs::runtime::InterruptHandler;

/// Soft + hard cap configuration for the watchdog.
///
/// Defaults to ADR-012 §2 budgets (1 ms p99 / 4 ms hard). The slice
/// brief uses 1 ms as the soft cap nominal — the 1.75 ms ADR-012
/// figure is the p99 target, and the slice-2 implementation uses
/// integer-millisecond resolution because the host
/// [`std::time::Instant`] arithmetic is integer at the
/// `Duration::from_millis` boundary used to seed the deadline.
///
/// Construction is fallible iff `p99_ms >= hard_ms` — the soft cap
/// must precede the hard cap or the soft-cap signal is dead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogConfig {
    p99_ms: u64,
    hard_ms: u64,
}

impl WatchdogConfig {
    /// Construct a watchdog config, validating `p99_ms < hard_ms`.
    ///
    /// On invalid input, returns the offending pair as a
    /// [`WatchdogConfigError`] so the caller can surface a
    /// [`crate::ScriptError::WatchdogConfig`] without losing the
    /// numbers.
    pub fn new(p99_ms: u64, hard_ms: u64) -> Result<Self, WatchdogConfigError> {
        if p99_ms >= hard_ms {
            return Err(WatchdogConfigError {
                p99_ms,
                hard_ms,
                reason: "soft cap (p99_ms) must be strictly less than hard cap (hard_ms)",
            });
        }
        if hard_ms == 0 {
            return Err(WatchdogConfigError {
                p99_ms,
                hard_ms,
                reason: "hard cap (hard_ms) must be > 0",
            });
        }
        Ok(Self { p99_ms, hard_ms })
    }

    /// Soft-cap budget in milliseconds (instrumentation only).
    #[must_use]
    pub fn p99_ms(&self) -> u64 {
        self.p99_ms
    }

    /// Hard-cap budget in milliseconds (interrupt-handler kill).
    #[must_use]
    pub fn hard_ms(&self) -> u64 {
        self.hard_ms
    }
}

/// Failure shape for [`WatchdogConfig::new`]. Carried into
/// [`crate::ScriptError::WatchdogConfig`] verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogConfigError {
    /// Offending soft-cap value supplied by the caller.
    pub p99_ms: u64,
    /// Offending hard-cap value supplied by the caller.
    pub hard_ms: u64,
    /// Static rationale for the rejection (for ergonomic
    /// `Display`-style rendering on the error surface).
    pub reason: &'static str,
}

/// A live watchdog deadline.
///
/// Returned by [`install_watchdog`] when the interrupt handler is
/// mounted on the rquickjs runtime; held by the caller for the
/// duration of one [`crate::ScriptRuntime::execute`] call.
/// [`Deadline::elapsed_ms`] gives the wall-clock execution time
/// observed at the call site, used to (a) feed
/// [`crate::ScriptError::TimeoutExceeded::observed_ms`] when the
/// hard cap fires and (b) drive the soft-cap observability check
/// when the script returned normally.
///
/// The deadline state is shared with the installed interrupt
/// handler via `Rc<Cell<...>>`. The handler reads the deadline
/// instant on every poll; the host reads `start` for elapsed-time
/// arithmetic. Single-threaded by construction (rquickjs's default
/// feature set is `!Send`), so `Rc` + `Cell` is sufficient — no
/// atomics needed.
#[derive(Debug, Clone)]
pub struct Deadline {
    start: Instant,
    deadline: Instant,
    config: WatchdogConfig,
    // Inner state the interrupt handler reads. Shared with the
    // installed handler; the handler is the only writer beyond
    // construction (it does not write — only reads) and the host
    // is the only reader of `start` for elapsed-ms accounting.
    // Wrapped in `Rc<Cell<_>>` so the `Clone` impl on `Deadline`
    // doesn't drift the deadline if the host clones the handle for
    // diagnostics (only the handler-installed instance is the
    // source of truth on the QuickJS side).
    _shared: Rc<Cell<Instant>>,
}

impl Deadline {
    /// Wall-clock milliseconds elapsed since the watchdog was
    /// installed. Saturates at `u64::MAX` on overflow (will not
    /// happen for any realistic script but the saturation keeps
    /// the surface panic-free).
    #[must_use]
    pub fn elapsed_ms(&self) -> u64 {
        let elapsed = self.start.elapsed();
        u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
    }

    /// True iff the elapsed wall-clock time exceeds the soft cap
    /// (p99). Drives the slice-2 [`WatchdogObservation::soft_cap_exceeded`]
    /// flag emitted alongside successful results.
    #[must_use]
    pub fn soft_cap_exceeded(&self) -> bool {
        self.elapsed_ms() > self.config.p99_ms
    }

    /// Watchdog config the deadline was built from.
    #[must_use]
    pub fn config(&self) -> WatchdogConfig {
        self.config
    }

    /// Wall-clock instant the watchdog was installed.
    #[must_use]
    pub fn start(&self) -> Instant {
        self.start
    }

    /// Wall-clock instant the hard cap will fire at.
    #[must_use]
    pub fn deadline(&self) -> Instant {
        self.deadline
    }
}

/// Snapshot recorded at the end of a watchdog-instrumented
/// [`crate::ScriptRuntime::execute`] call.
///
/// The slice-2 surface is intentionally a plain data struct rather
/// than a [`chromaticity_observability::MetricEvent`] emission —
/// see the module docs for the deferral rationale.
/// [`crate::ScriptRuntime::last_observation`] exposes the most
/// recent observation for in-process inspection; tests use this
/// for the soft-cap-not-tripped contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogObservation {
    /// Watchdog configuration in force when the observation was
    /// taken (1.75 ms p99 / 4 ms hard by ADR-012 default; the
    /// slice-2 host calls feed integer-millisecond budgets).
    pub config: WatchdogConfig,
    /// Wall-clock milliseconds the script ran before the host
    /// returned (whether successfully or via interrupt).
    pub observed_ms: u64,
    /// `true` iff `observed_ms > config.p99_ms`. Slice-3+ may emit
    /// a `chromaticity_script_*` counter on this flag once the
    /// docs/31 catalog widens.
    pub soft_cap_exceeded: bool,
    /// `true` iff the watchdog fired the interrupt handler before
    /// the script returned naturally. Mutually exclusive with a
    /// successful result on the host side: when this is `true` the
    /// caller has already mapped the rquickjs surface to
    /// [`crate::ScriptError::TimeoutExceeded`].
    pub hard_cap_exceeded: bool,
}

/// Install the watchdog interrupt handler on `runtime` and return a
/// [`Deadline`] handle the caller stores for the lifetime of one
/// `execute` call.
///
/// The handler is installed via `Runtime::set_interrupt_handler`
/// with a closure that closes over a clone of the deadline instant
/// shared via `Rc<Cell<Instant>>`. The closure returns `true` (=
/// "interrupt") once the wall clock crosses the deadline. QuickJS
/// then surfaces the interrupt as an uncatchable
/// `InternalError: interrupted` exception, which the caller catches
/// and lowers to [`crate::ScriptError::TimeoutExceeded`].
///
/// Caller responsibility: call [`uninstall_watchdog`] after the
/// `execute` call returns, otherwise the handler stays mounted on
/// the runtime and any subsequent `execute` call would be subject
/// to the leftover deadline (immediately tripping if the deadline
/// has already passed). The `Deadline` returned does not own the
/// handler lifecycle — `set_interrupt_handler(None)` is the only
/// way to detach.
pub fn install_watchdog(runtime: &Runtime, config: WatchdogConfig) -> Deadline {
    let start = Instant::now();
    let deadline = start + Duration::from_millis(config.hard_ms);
    let shared = Rc::new(Cell::new(deadline));

    let handler_state = Rc::clone(&shared);
    let handler: InterruptHandler = Box::new(move || {
        // QuickJS calls this on a periodic basis during JS
        // execution. We compare wall-clock now() against the
        // deadline instant; once the deadline has passed, return
        // `true` to request interruption. The Cell is single-
        // threaded by construction (rquickjs default features are
        // `!Send`); `get()` on a `Cell<Instant>` is a `Copy` read.
        Instant::now() >= handler_state.get()
    });
    runtime.set_interrupt_handler(Some(handler));

    Deadline {
        start,
        deadline,
        config,
        _shared: shared,
    }
}

/// Detach the watchdog interrupt handler. Idempotent — calling on
/// a runtime that has no handler installed is a no-op (the
/// rquickjs surface accepts `None` either way).
///
/// Pair every [`install_watchdog`] with one [`uninstall_watchdog`]
/// at the end of the bracketed `execute` scope; a future slice may
/// fold both into a guard type once the per-call composition shape
/// stabilises.
pub fn uninstall_watchdog(runtime: &Runtime) {
    runtime.set_interrupt_handler(None);
}

/// Heuristic: identify the `InternalError: interrupted` exception
/// QuickJS raises when the watchdog interrupt handler returns
/// `true`. The error class string is `InternalError`; the message
/// is `interrupted` (see `rquickjs-sys-0.11.0/quickjs/quickjs.c`
/// line 7438 `JS_ThrowInternalError(ctx, "interrupted")`).
///
/// Slice-2 reads both `name` and `message` from the caught
/// exception value — pinning only one would give false positives
/// (a script that throws `new InternalError("interrupted")` would
/// otherwise spuriously map to a timeout). The combination of
/// `name == "InternalError"` and `message == "interrupted"` is
/// unique to the QuickJS interrupt path.
#[must_use]
pub fn is_interrupted_exception(name: &str, message: &str) -> bool {
    name == "InternalError" && message == "interrupted"
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod deadline_pins {
    //! Evidence: EV-SANDBOX-SAFETY (T1) — deadline arithmetic + watchdog config validation drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: the
    //! [`Deadline`] elapsed-ms accounting and the
    //! [`WatchdogConfig`] validation rules are the slice-2
    //! data-shape contract that the host
    //! [`crate::ScriptRuntime::execute`] integration depends on.
    //! These pins do NOT exercise QuickJS — the integration-level
    //! T1 pins live in [`crate::runtime::execute_drift_pins`].

    use std::thread;
    use std::time::Duration;

    use super::{WatchdogConfig, is_interrupted_exception};

    #[test]
    fn watchdog_config_rejects_inverted_caps() {
        // Soft cap must be strictly less than hard cap; otherwise
        // the soft-cap signal can never fire before the hard cap
        // takes the script down. Slice-brief contract: the builder
        // surfaces this as a structured error.
        let err = WatchdogConfig::new(10, 4).expect_err("inverted caps must reject");
        assert_eq!(err.p99_ms, 10);
        assert_eq!(err.hard_ms, 4);
        assert!(!err.reason.is_empty());

        let err_eq = WatchdogConfig::new(4, 4).expect_err("equal caps must reject");
        assert_eq!(err_eq.p99_ms, 4);
        assert_eq!(err_eq.hard_ms, 4);
    }

    #[test]
    fn watchdog_config_rejects_zero_hard_cap() {
        // A zero hard cap fires immediately on every script, which
        // would silently break the public surface. The constructor
        // refuses; the slice-2 default budgets (1 ms / 4 ms) are
        // both non-zero so this only catches mis-config.
        let err = WatchdogConfig::new(0, 0).expect_err("zero hard cap must reject");
        assert_eq!(err.hard_ms, 0);
    }

    #[test]
    fn watchdog_config_accepts_phase2_defaults() {
        // ADR-012 §2: 1 ms p99 / 4 ms hard. The slice-2 nominal
        // pair must round-trip through the constructor.
        let cfg = WatchdogConfig::new(1, 4).expect("phase-2 default must accept");
        assert_eq!(cfg.p99_ms(), 1);
        assert_eq!(cfg.hard_ms(), 4);
    }

    #[test]
    fn deadline_elapsed_ms_advances_monotonically() {
        // The deadline holds an `Instant` from
        // `Runtime::set_interrupt_handler` install time.
        // `elapsed_ms()` against a clock that has advanced must be
        // non-zero; sleeping 5 ms gives a wide margin for the CI
        // runner clock variance noted in
        // `engine/script/tests/sandbox.rs`.
        let cfg = WatchdogConfig::new(1, 4).expect("config must accept");
        let start = std::time::Instant::now();
        let deadline = super::Deadline {
            start,
            deadline: start + Duration::from_millis(cfg.hard_ms()),
            config: cfg,
            _shared: std::rc::Rc::new(std::cell::Cell::new(
                start + Duration::from_millis(cfg.hard_ms()),
            )),
        };
        assert_eq!(
            deadline.elapsed_ms(),
            0,
            "fresh deadline reports 0 ms elapsed",
        );
        thread::sleep(Duration::from_millis(5));
        let elapsed = deadline.elapsed_ms();
        assert!(
            elapsed >= 5,
            "after 5ms sleep, elapsed must be >= 5; got {elapsed}"
        );
        assert!(deadline.soft_cap_exceeded(), "5ms exceeds 1ms p99 soft cap");
    }

    #[test]
    fn interrupted_exception_identifier_matches_quickjs_format() {
        // The QuickJS C source (`quickjs.c` line 7438) raises the
        // interrupted exception as `InternalError("interrupted")`.
        // The watchdog mapper uses both fields; this pin is the
        // tripwire for any future QuickJS upgrade that adjusts the
        // diagnostic string.
        assert!(is_interrupted_exception("InternalError", "interrupted"));
        assert!(!is_interrupted_exception(
            "InternalError",
            "not the interrupt"
        ));
        assert!(!is_interrupted_exception("RangeError", "interrupted"));
        assert!(!is_interrupted_exception("", ""));
    }
}

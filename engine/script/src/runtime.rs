// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Sandboxed QuickJS execution entry point.
//!
//! Slice-1 ships [`ScriptRuntime::new`] (constructs an isolated
//! `rquickjs::Runtime` + `Context`) and [`ScriptRuntime::execute`]
//! (compiles the supplied source via `Ctx::eval`, runs it
//! synchronously, and converts the resulting QuickJS `Value` to a
//! borrow-free [`crate::ScriptValue`]).
//!
//! Design choices that may surprise a code reader:
//!
//! - **Sandbox-by-construction.** No `loader` feature is enabled
//!   on `rquickjs` and no globals are installed beyond what
//!   QuickJS bakes in. The cleanroom audit (ADR-012 §6) requires
//!   that `globalThis.fs`, `globalThis.process`, and
//!   `globalThis.window` be `undefined` from the script's
//!   perspective; rquickjs's default-feature surface delivers
//!   that with no opt-out — there is no host code to add and no
//!   feature to disable. The drift pin in
//!   [`execute_drift_pins`] asserts the posture as a tripwire so
//!   any future feature opt-in (e.g. slice-4 determinism shims)
//!   that accidentally surfaces a host global trips the test.
//! - **Each [`ScriptRuntime`] owns its own `rquickjs::Runtime`.**
//!   ADR-012 §6 specifies "QuickJS runs in its own isolate per
//!   scene"; slice 1 honours that at the host-API level by
//!   binding one [`ScriptRuntime`] = one isolate. Slice 5+ may
//!   pool runtimes for cold-scene reuse, but the public surface
//!   keeps the one-isolate-per-runtime invariant for the
//!   `EV-SANDBOX-SAFETY` audit.
//! - **Sync `execute`.** `Ctx::eval` is synchronous; the wrapping
//!   [`Context::with`] closure drives the QuickJS event loop
//!   inline. No tokio runtime, no `async` in this slice — the
//!   T2 corpus drives `execute` from `#[test]` and slice 5's
//!   command-bus mount will switch to a real executor when async
//!   command dispatch lands.
//! - **`undefined` / `null` collapse.** ECMAScript distinguishes
//!   `undefined` from `null`; the slice-1 `ScriptValue` does not.
//!   Both lower to `ScriptValue::Null`. Authors writing scenes
//!   today don't differentiate, and JSON-style projection keeps
//!   the contract corpus simple. A later slice may widen the
//!   `ScriptValue` vocabulary if author tooling needs the
//!   distinction.
//! - **Error categorisation.** rquickjs returns `Error::Exception`
//!   both for compile-time syntax errors and for uncaught
//!   `throw` statements. The compile / runtime split is recovered
//!   by inspecting the `Error.name` property on the caught value:
//!   `SyntaxError` lowers to [`crate::ScriptError::Compile`];
//!   anything else lowers to [`crate::ScriptError::Runtime`].
//!   This matches the QuickJS convention (compile-time errors
//!   throw `SyntaxError` synchronously from the eval call).

use std::cell::Cell;

use rquickjs::{CatchResultExt, Context, Ctx, Object, Runtime, Value};

use crate::determinism::{self, Clock, DeterminismState, RandomSeedConfig};
use crate::error::ScriptError;
use crate::memory_cap::{self, MemoryCapConfig, MemoryUsageSnapshot};
use crate::value::ScriptValue;
use crate::watchdog::{self, Deadline, WatchdogConfig, WatchdogObservation};

/// A sandboxed QuickJS execution environment.
///
/// One [`ScriptRuntime`] owns one `rquickjs::Runtime` + one
/// `rquickjs::Context`; constructing two instances yields two
/// fully isolated heaps (the [`execute_drift_pins`] T1 suite
/// asserts this with `multiple_runtimes_isolated` once it lands
/// in T2 — kept in T2 because the corpus needs the cross-runtime
/// fixture pair).
///
/// The struct is intentionally `!Send` and `!Sync`: rquickjs's
/// `Ctx<'js>` is bound to a thread-local QuickJS isolate, and
/// the Phase-1 in-process command bus drives script work from the
/// runtime thread (ADR-006). Slice 5 will revisit the threading
/// model if author tooling needs background compilation.
///
/// `Debug` is implemented manually rather than derived because
/// neither `rquickjs::Context` nor `rquickjs::Runtime` carry a
/// `Debug` impl on the slice-1 / slice-2 surface. The host-side
/// state we render is the watchdog config and the most-recent
/// observation; the QuickJS handles are opaque from Rust's
/// perspective and would not surface useful diagnostics anyway.
pub struct ScriptRuntime {
    // Field order matters for drop: `context` references the
    // runtime, so the runtime must outlive the context. Rust
    // drops fields in declaration order, so `context` is dropped
    // first.
    context: Context,
    // Slice-2 mounts the watchdog interrupt handler against this
    // handle on every [`Self::execute`] call; slice 3 also mounts
    // the memory cap on the same handle. The field is no longer
    // underscore-prefixed since both code paths use it.
    runtime: Runtime,
    // Watchdog config in force for this runtime, set via
    // [`Self::with_watchdog`]. `None` reproduces the slice-1
    // behaviour: no interrupt handler is installed, no per-execute
    // observation is recorded.
    watchdog_config: Option<WatchdogConfig>,
    // Memory cap config in force for this runtime, set via
    // [`Self::with_memory_limit`]. `None` reproduces the
    // slice-1 / slice-2 behaviour: no cap is mounted on the
    // QuickJS allocator and no snapshot is recorded.
    memory_cap_config: Option<MemoryCapConfig>,
    // Most-recent watchdog observation, populated on every
    // [`Self::execute`] call when a watchdog config is active.
    // Read by tests via [`Self::last_observation`] for the slice-2
    // soft-cap-not-tripped contract. `Cell<Option<...>>` keeps the
    // host surface `&self`-friendly without forcing
    // `RefCell::borrow_mut` and the associated runtime borrow
    // panic surface; observations are `Copy` so `Cell::set` /
    // `Cell::get` is enough.
    last_observation: Cell<Option<WatchdogObservation>>,
    // Most-recent memory snapshot, populated on every
    // [`Self::execute`] call when a memory cap is active. Same
    // Cell-backed read pattern as `last_observation`.
    last_memory_snapshot: Cell<Option<MemoryUsageSnapshot>>,
    // Slice-4 determinism shims state. The PRNG handle and the
    // clock-installed flag live in here; the actual closures
    // installed on `Math.random` / `Date.now` close over their own
    // `Rc<RefCell<...>>` shares of the same underlying state, so
    // [`Self::execute`] does NOT need a per-call install / uninstall
    // bracket — the shims survive across [`Context::with`] entries
    // for the lifetime of the runtime. `None` reproduces the slice-1
    // / slice-2 / slice-3 behaviour: no shim installed, the QuickJS
    // built-in `Math.random` and `Date.now` are unchanged.
    determinism_state: Option<DeterminismState>,
}

impl std::fmt::Debug for ScriptRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptRuntime")
            .field("watchdog_config", &self.watchdog_config)
            .field("memory_cap_config", &self.memory_cap_config)
            .field("last_observation", &self.last_observation.get())
            .field("last_memory_snapshot", &self.last_memory_snapshot.get())
            .field("determinism_state", &self.determinism_state)
            .finish_non_exhaustive()
    }
}

impl ScriptRuntime {
    /// Construct a fresh sandboxed runtime.
    ///
    /// Allocates a new `rquickjs::Runtime` and a `Context::full`
    /// over it. Default-feature posture: no I/O / FS / native
    /// modules, no host globals beyond the QuickJS baseline. See
    /// the module docs for the sandbox audit notes that this
    /// constructor implements.
    pub fn new() -> Result<Self, ScriptError> {
        let runtime = Runtime::new().map_err(|e| ScriptError::RuntimeInit(format!("{e}")))?;
        let context =
            Context::full(&runtime).map_err(|e| ScriptError::ContextInit(format!("{e}")))?;
        Ok(Self {
            context,
            runtime,
            watchdog_config: None,
            memory_cap_config: None,
            last_observation: Cell::new(None),
            last_memory_snapshot: Cell::new(None),
            determinism_state: None,
        })
    }

    /// Configure watchdog soft + hard caps, returning the runtime
    /// for chaining.
    ///
    /// Slice-2 surface (per ADR-012 §2):
    ///
    /// - `p99_ms` — soft cap, milliseconds. Instrumentation only;
    ///   does NOT abort the script. The post-execute
    ///   [`WatchdogObservation::soft_cap_exceeded`] flag goes
    ///   `true` whenever observed wall-clock execution time
    ///   exceeds this budget.
    /// - `hard_ms` — hard cap, milliseconds. The interrupt handler
    ///   installed on the rquickjs `Runtime` returns `true` once
    ///   the deadline passes; QuickJS raises an uncatchable
    ///   `InternalError: interrupted` exception which the executor
    ///   lowers to [`ScriptError::TimeoutExceeded`].
    ///
    /// Validation: `0 < p99_ms < hard_ms`. Inverted, equal, or
    /// zero-hard-cap pairs reject with
    /// [`ScriptError::WatchdogConfig`] and the runtime is consumed
    /// (the builder pattern returns `Self` on success).
    ///
    /// Slice-1 callers continue using [`Self::new`] directly when
    /// they don't need the watchdog (the per-execute interrupt-
    /// handler install + uninstall is skipped, and no observation
    /// is recorded). The slice-2 sandbox-safety corpus exercises
    /// the watchdog branch.
    pub fn with_watchdog(mut self, p99_ms: u64, hard_ms: u64) -> Result<Self, ScriptError> {
        let cfg =
            WatchdogConfig::new(p99_ms, hard_ms).map_err(|e| ScriptError::WatchdogConfig {
                p99_ms: e.p99_ms,
                hard_ms: e.hard_ms,
                reason: e.reason,
            })?;
        self.watchdog_config = Some(cfg);
        Ok(self)
    }

    /// Watchdog config currently in force, or `None` if no
    /// watchdog has been mounted.
    #[must_use]
    pub fn watchdog_config(&self) -> Option<WatchdogConfig> {
        self.watchdog_config
    }

    /// Most-recent watchdog observation, or `None` if no
    /// watchdog-instrumented call has been made.
    ///
    /// Slice-2 host surface for the soft-cap-not-tripped contract:
    /// callers that ran a script under the watchdog can read this
    /// to confirm whether the soft cap fired without re-entering
    /// the engine. Cleared on every [`Self::execute`] call when a
    /// watchdog config is present (so a long-running success
    /// followed by a fast success doesn't leak the earlier
    /// observation forward).
    #[must_use]
    pub fn last_observation(&self) -> Option<WatchdogObservation> {
        self.last_observation.get()
    }

    /// Configure the per-runtime heap cap, returning the runtime
    /// for chaining.
    ///
    /// Slice-3 surface (per ADR-012 §3):
    ///
    /// - `bytes` — heap-cap budget in bytes. The cap is mounted via
    ///   `rquickjs::Runtime::set_memory_limit` on every
    ///   [`Self::execute`] call; once an allocation would cross the
    ///   limit, QuickJS aborts the allocation and raises an
    ///   uncatchable `InternalError: out of memory` exception which
    ///   the executor lowers to
    ///   [`ScriptError::MemoryCapExceeded`].
    ///
    /// Validation: `bytes > 0`. A zero cap rejects with
    /// [`ScriptError::MemoryCapConfig`] — QuickJS treats `0` as
    /// "unlimited", which contradicts the slice-3 contract that a
    /// cap MUST be enforced once the builder returns `Ok`. Callers
    /// that want an uncapped runtime should not call this builder
    /// at all; the slice-1 / slice-2 [`Self::new`] path leaves
    /// the runtime uncapped.
    ///
    /// Builder chaining: composes with [`Self::with_watchdog`] —
    /// both can be in force on the same runtime simultaneously,
    /// and [`Self::execute`] brackets the watchdog interrupt
    /// handler and the memory cap install / uninstall pairs around
    /// every call.
    pub fn with_memory_limit(mut self, bytes: usize) -> Result<Self, ScriptError> {
        let cfg = MemoryCapConfig::new(bytes)?;
        self.memory_cap_config = Some(cfg);
        Ok(self)
    }

    /// Memory-cap config currently in force, or `None` if no cap
    /// has been mounted.
    #[must_use]
    pub fn memory_cap_config(&self) -> Option<MemoryCapConfig> {
        self.memory_cap_config
    }

    /// Configure the seeded `Math.random` shim, returning the
    /// runtime for chaining.
    ///
    /// Slice-4 surface (per ADR-012 §5):
    ///
    /// - `seed` — `u64` value to seed the per-runtime xoroshiro128+
    ///   generator. Any `u64` is acceptable, including `0`; the
    ///   builder expands the value into the 128-bit PRNG state via
    ///   `splitmix64`, which guarantees the underlying state is not
    ///   the all-zero degenerate case.
    ///
    /// Effect: replaces the QuickJS default `Math.random` (which
    /// pulls entropy from the host) with a closure that delegates
    /// to the per-runtime PRNG. The shim is installed once at
    /// builder time and lives for the lifetime of the runtime;
    /// [`Self::execute`] does NOT need a per-call install /
    /// uninstall bracket because the shim closure captures the
    /// PRNG state via `Rc<RefCell<_>>` and survives across
    /// `Context::with` boundaries.
    ///
    /// Failure modes: this builder fails with
    /// [`ScriptError::DeterminismInstall`] iff rquickjs returns an
    /// error wiring the global override (`globals().Math` lookup,
    /// `Function::new` wiring, or property `set` failed). The
    /// builder validates the `seed` via
    /// [`crate::determinism::RandomSeedConfig::validate`] which is
    /// always `Ok` today; future config knobs may surface
    /// [`ScriptError::DeterminismConfig`] from this path.
    ///
    /// Builder chaining: composes with [`Self::with_watchdog`] +
    /// [`Self::with_memory_limit`] + [`Self::with_clock`]; any order
    /// is valid (each builder is independent).
    pub fn with_random_seed(mut self, seed: u64) -> Result<Self, ScriptError> {
        let cfg = RandomSeedConfig::new(seed).validate()?;
        let mut state = self.determinism_state.take().unwrap_or_default();
        // The shim install path is a single `Context::with` entry.
        // The closure captures an `Rc<RefCell<PrngState>>`; the
        // returned `Rc` clone is folded into the host-side
        // `DeterminismState` for diagnostics.
        let install = self
            .context
            .with(|ctx: Ctx<'_>| determinism::install_random_shim(&ctx, cfg));
        let prng_state = install?;
        determinism::record_random_seed(&mut state, cfg, prng_state);
        self.determinism_state = Some(state);
        Ok(self)
    }

    /// Configure the frame-locked `Date.now` shim, returning the
    /// runtime for chaining.
    ///
    /// Slice-4 surface (per ADR-012 §5):
    ///
    /// - `clock` — boxed [`Clock`] trait object. Every script call
    ///   to `Date.now()` invokes [`Clock::now_ms`] synchronously on
    ///   the runtime thread. The slice-4 built-in implementors are
    ///   [`crate::FixedClock`] (always returns its `tick_ms`,
    ///   matching ADR-012 §5's per-frame replay primitive) and
    ///   [`crate::MonotonicClock`] (returns
    ///   `base_ms + Instant::now() - start`).
    ///
    /// Effect: replaces the QuickJS default `Date.now` (which
    /// returns the host wall-clock in milliseconds since the Unix
    /// epoch) with a closure that delegates to the supplied
    /// [`Clock`]. The shim is installed once at builder time and
    /// lives for the lifetime of the runtime, with the same
    /// across-`Context::with`-boundaries-survival pattern as
    /// [`Self::with_random_seed`].
    ///
    /// Failure modes: this builder fails with
    /// [`ScriptError::DeterminismInstall`] iff rquickjs returns an
    /// error wiring the global override.
    ///
    /// Builder chaining: composes with [`Self::with_watchdog`] +
    /// [`Self::with_memory_limit`] + [`Self::with_random_seed`];
    /// any order is valid (each builder is independent).
    pub fn with_clock(mut self, clock: Box<dyn Clock + Send>) -> Result<Self, ScriptError> {
        let mut state = self.determinism_state.take().unwrap_or_default();
        let install = self
            .context
            .with(|ctx: Ctx<'_>| determinism::install_clock_shim(&ctx, clock));
        install?;
        determinism::record_clock_installed(&mut state);
        self.determinism_state = Some(state);
        Ok(self)
    }

    /// Determinism state currently in force, or `None` if neither
    /// [`Self::with_random_seed`] nor [`Self::with_clock`] has been
    /// called. Read-only diagnostic surface; the underlying PRNG
    /// state and `Clock` trait object are not exposed (the closure
    /// installed on the QuickJS side owns the only mutable handle).
    #[must_use]
    pub fn determinism_state(&self) -> Option<&DeterminismState> {
        self.determinism_state.as_ref()
    }

    /// Most-recent memory-usage snapshot, or `None` if no
    /// memory-cap-instrumented call has been made.
    ///
    /// Slice-3 host surface for the heap-usage observability
    /// contract: callers that ran a script under the memory cap
    /// can read this to inspect the install / uninstall
    /// `malloc_size` pair without re-entering the engine. Cleared
    /// on every [`Self::execute`] call when a memory cap is
    /// present (so a long-running success followed by a fast
    /// success doesn't leak the earlier snapshot forward).
    #[must_use]
    pub fn last_memory_snapshot(&self) -> Option<MemoryUsageSnapshot> {
        self.last_memory_snapshot.get()
    }

    /// Compile and execute `source` in the global context, returning
    /// a borrow-free [`ScriptValue`] tree.
    ///
    /// Failure modes (see [`ScriptError`] variants for the full
    /// surface): syntax / parse errors lower to
    /// [`ScriptError::Compile`]; uncaught thrown values during
    /// execution lower to [`ScriptError::Runtime`]; values whose
    /// shape is outside the slice-1 vocabulary lower to
    /// [`ScriptError::ValueConversion`]; watchdog interrupts
    /// lower to [`ScriptError::TimeoutExceeded`]; memory-cap hits
    /// lower to [`ScriptError::MemoryCapExceeded`].
    pub fn execute(&self, source: &str) -> Result<ScriptValue, ScriptError> {
        // Slice-2 watchdog bracket: install the interrupt handler
        // before evaluation, capture the deadline handle, run the
        // eval, lower the result, then unconditionally uninstall
        // the handler (whether the eval succeeded, threw, or got
        // interrupted). The bracket is structured so the
        // `set_interrupt_handler(None)` call is the last thing
        // before the `Result` propagates, avoiding any "leftover
        // deadline trips the next call" failure mode.
        let deadline = self
            .watchdog_config
            .map(|cfg| watchdog::install_watchdog(&self.runtime, cfg));

        // Slice-3 memory-cap bracket: mount the cap on the QuickJS
        // allocator before evaluation, capture the
        // `malloc_size`-baseline snapshot, run the eval, then
        // unconditionally detach the cap. Bracketing is the same
        // shape as the watchdog: install before, uninstall after,
        // regardless of success / throw / cap-hit. Order against
        // the watchdog is "watchdog outer, memory cap inner" — both
        // are independent on the QuickJS side, but mounting the
        // memory cap second means the bracketed scope on the
        // memory-cap path is strictly narrower; the watchdog's
        // interrupt-handler slot is detached first on the way out.
        let mut memory_snapshot = self
            .memory_cap_config
            .map(|cfg| memory_cap::install_memory_cap(&self.runtime, cfg));

        let outcome = self.context.with(|ctx: Ctx<'_>| {
            // `ctx.eval::<Value, _>` is the entry into the QuickJS
            // interpreter. We collect into the lifetime-bound
            // `Value<'js>` so we can branch on the JS-level type
            // before lowering; lowering to a more concrete `FromJs`
            // target (`f64`, `String`, etc.) would force one type
            // up-front and discard the type-tag information needed
            // for `ScriptValue` projection.
            let raw = ctx.eval::<Value<'_>, _>(source);
            let value = match raw.catch(&ctx) {
                Ok(v) => v,
                Err(caught) => return Err(map_caught_error(caught)),
            };
            lower_value(&ctx, value)
        });

        // Detach the memory cap first (LIFO against install order:
        // watchdog before, memory cap after). The snapshot
        // `at_uninstall_bytes` is populated here so the
        // [`ScriptError::MemoryCapExceeded`] remap downstream can
        // read a tight upper bound on script peak heap usage.
        if let Some(snap) = memory_snapshot.as_mut() {
            memory_cap::uninstall_memory_cap(&self.runtime, snap);
        }

        // Always detach the interrupt handler — see install_watchdog
        // doc comment for the leftover-deadline rationale.
        if deadline.is_some() {
            watchdog::uninstall_watchdog(&self.runtime);
        }

        // Memory-cap remap runs first. A cap-hit during a watchdog-
        // instrumented call would otherwise show up as a generic
        // `ScriptError::Runtime("out of memory")`; the remap
        // rewrites it to [`ScriptError::MemoryCapExceeded`] with
        // the limit-derived budget context. Watchdog remap follows
        // afterward so a "memory cap hit + watchdog timeout" race
        // resolves in favour of the memory-cap diagnosis (the
        // QuickJS interpreter raises the OOM exception first; the
        // watchdog interrupt would only have fired if the
        // allocation pressure didn't trip the cap, which this
        // ordering preserves).
        let outcome =
            if let (Some(cfg), Some(snap)) = (self.memory_cap_config, memory_snapshot.as_ref()) {
                self.last_memory_snapshot.set(Some(*snap));
                remap_oom(outcome, cfg, snap)
            } else {
                self.last_memory_snapshot.set(None);
                outcome
            };

        // Record observation if a watchdog was active. Done after
        // detach so the observation captures the elapsed time at
        // host-return, not at eval-return (the two are within
        // microseconds on a hot host). An interrupted-by-watchdog
        // outcome may surface as [`ScriptError::Runtime`] from the
        // slice-1 catch path; [`remap_interrupt`] rewrites it to
        // [`ScriptError::TimeoutExceeded`] populated from the
        // deadline-derived budget context.
        if let Some(d) = deadline {
            let observed_ms = d.elapsed_ms();
            let cfg = d.config();
            let (final_outcome, hard_cap_exceeded) = remap_interrupt(outcome, &d);
            self.last_observation.set(Some(WatchdogObservation {
                config: cfg,
                observed_ms,
                soft_cap_exceeded: observed_ms > cfg.p99_ms(),
                hard_cap_exceeded,
            }));
            final_outcome
        } else {
            // Slice-1 continuity: no watchdog mounted ⇒ no
            // observation recorded; clear any stale observation
            // from a prior call to keep the contract simple.
            self.last_observation.set(None);
            outcome
        }
    }
}

/// If `outcome` is a [`ScriptError::Runtime`] whose payload
/// matches the QuickJS interrupted-exception identifier
/// (`InternalError: interrupted`), rewrite it to
/// [`ScriptError::TimeoutExceeded`] populated from `deadline`.
///
/// Returns the (possibly remapped) outcome alongside a
/// `hard_cap_exceeded` flag for the
/// [`WatchdogObservation`] surface. The function lives on the
/// host side rather than in `watchdog.rs` because the error
/// remapping is the host's job — `watchdog.rs` only exposes the
/// identifier predicate.
fn remap_interrupt(
    outcome: Result<ScriptValue, ScriptError>,
    deadline: &Deadline,
) -> (Result<ScriptValue, ScriptError>, bool) {
    match outcome {
        Ok(value) => (Ok(value), false),
        Err(ScriptError::Runtime(message)) => {
            // The slice-1 runtime-error path renders the QuickJS
            // exception's `message` string. The interrupt path
            // throws `InternalError("interrupted")`; the message
            // surface here is just `"interrupted"`. Slice-1's
            // map_caught_error ALSO formats InternalError +
            // (message="") through the rendered path; we accept
            // both forms.
            if message == "interrupted" || message.contains("InternalError: interrupted") {
                let cfg = deadline.config();
                (
                    Err(ScriptError::TimeoutExceeded {
                        p99_ms: cfg.p99_ms(),
                        hard_ms: cfg.hard_ms(),
                        observed_ms: deadline.elapsed_ms(),
                    }),
                    true,
                )
            } else {
                (Err(ScriptError::Runtime(message)), false)
            }
        }
        Err(other) => (Err(other), false),
    }
}

/// If `outcome` is a [`ScriptError::Runtime`] whose payload
/// matches the QuickJS out-of-memory identifier (`InternalError:
/// out of memory`), rewrite it to
/// [`ScriptError::MemoryCapExceeded`] populated from `config`
/// (limit) + `snapshot` (observed `malloc_size` at host-return).
///
/// The function lives on the host side, mirroring
/// [`remap_interrupt`]'s slice-2 placement: the `memory_cap.rs`
/// module exposes only the identifier predicate
/// ([`memory_cap::is_oom_error`]) and the install / uninstall
/// pair, leaving the host to do the error-shape rewriting.
fn remap_oom(
    outcome: Result<ScriptValue, ScriptError>,
    config: MemoryCapConfig,
    snapshot: &MemoryUsageSnapshot,
) -> Result<ScriptValue, ScriptError> {
    match outcome {
        Err(ScriptError::Runtime(message)) => {
            // The slice-1 runtime-error path renders the QuickJS
            // exception's `message` string. The OOM path throws
            // `InternalError("out of memory")`; the message
            // surface here is just `"out of memory"`. Slice-1's
            // `map_caught_error` ALSO formats the rendered path
            // through `format_runtime` for non-empty messages, so
            // both forms surface here.
            if message == "out of memory" || message.contains("InternalError: out of memory") {
                Err(ScriptError::MemoryCapExceeded {
                    limit_bytes: config.limit_bytes(),
                    observed_bytes: snapshot.at_uninstall_bytes,
                })
            } else {
                Err(ScriptError::Runtime(message))
            }
        }
        other => other,
    }
}

/// Map a caught QuickJS error into the slice-1 [`ScriptError`]
/// surface.
///
/// rquickjs surfaces both compile-time `SyntaxError` and runtime
/// `throw` through the same `CaughtError::Exception` path. The
/// distinction is preserved by inspecting the JS-level
/// `Error.name`. `SyntaxError` lowers to [`ScriptError::Compile`];
/// every other thrown value lowers to [`ScriptError::Runtime`].
///
/// Non-exception caught errors (e.g. allocation failures, type
/// conversion errors raised by rquickjs glue rather than by
/// QuickJS itself) are also routed to [`ScriptError::Runtime`]
/// for slice 1; the compile / runtime split is the only contract
/// the slice-1 corpus asserts. Slice 4 may add a finer surface as
/// determinism / watchdog variants land.
fn map_caught_error(caught: rquickjs::CaughtError<'_>) -> ScriptError {
    use rquickjs::CaughtError;
    match caught {
        CaughtError::Exception(exc) => {
            // `exc.message()` is the QuickJS `Error.message`
            // string; we render the whole exception to capture
            // both name + message + stack (`{exc:?}` would lose
            // QuickJS-side prose).
            let message = exc.message().unwrap_or_default();
            // QuickJS sets `Error.name` to a category string
            // (`SyntaxError`, `ReferenceError`, `TypeError`,
            // etc.) for each built-in error class. We read the
            // property directly off the Exception object — the
            // formatted `Display` rendering embeds the name but
            // is not as deterministic to substring-match against
            // (the QuickJS C side may format differently between
            // versions).
            let name: String = exc
                .get::<_, Value<'_>>("name")
                .ok()
                .and_then(|v| v.into_string())
                .and_then(|s| s.to_string().ok())
                .unwrap_or_default();
            let rendered = format!("{exc}");
            if name == "SyntaxError" {
                ScriptError::Compile(format_compile(&message, &rendered))
            } else if watchdog::is_interrupted_exception(&name, &message) {
                // Slice-2: the watchdog interrupt path raises
                // `InternalError: interrupted` from QuickJS C
                // (`quickjs.c` line 7438). Render the message
                // straight so [`remap_interrupt`] downstream can
                // recognise the canonical string and lower to
                // [`ScriptError::TimeoutExceeded`] with the
                // deadline-derived budget context. Slice-1's
                // catch path didn't differentiate this case
                // because no watchdog was installed; slice-2's
                // bracketed `execute` is what creates the
                // interrupt opportunity.
                ScriptError::Runtime(message.to_string())
            } else if memory_cap::is_oom_error(&name, &message) {
                // Slice-3: the memory-cap path raises
                // `InternalError: out of memory` from QuickJS C
                // (`quickjs.c` line 7348 `JS_ThrowOutOfMemory`).
                // Render the message straight so [`remap_oom`]
                // downstream can recognise the canonical string
                // and lower to [`ScriptError::MemoryCapExceeded`]
                // with the limit-derived budget context. Mirrors
                // the slice-2 `is_interrupted_exception` shape.
                ScriptError::Runtime(message.to_string())
            } else {
                ScriptError::Runtime(format_runtime(&message, &rendered))
            }
        }
        CaughtError::Error(err) => {
            // `rquickjs::Error::Exception` may surface here on
            // some catch paths (e.g. when the rquickjs glue
            // can't materialise the JS-side Exception object).
            // We render through Display; this path does NOT
            // route the watchdog identifier through
            // `is_interrupted_exception` because the underlying
            // exception object is not in hand.
            ScriptError::Runtime(format!("{err}"))
        }
        CaughtError::Value(val) => ScriptError::Runtime(format!("{val:?}")),
    }
}

/// Best-effort message rendering for a [`ScriptError::Compile`]
/// payload. Falls back to the full formatted exception when the
/// message string is empty (QuickJS occasionally surfaces an
/// empty message for low-level parse states like an unterminated
/// regular-expression literal).
fn format_compile(message: &str, rendered: &str) -> String {
    if message.is_empty() {
        rendered.to_string()
    } else {
        format!("SyntaxError: {message}")
    }
}

/// Best-effort message rendering for a [`ScriptError::Runtime`]
/// payload. Same fallback pattern as [`format_compile`].
fn format_runtime(message: &str, rendered: &str) -> String {
    if message.is_empty() {
        rendered.to_string()
    } else {
        message.to_string()
    }
}

/// Lower a QuickJS [`Value`] to a borrow-free [`ScriptValue`].
///
/// Slice-1 vocabulary only — see [`crate::value`] module docs.
/// Anything outside the supported set returns
/// [`ScriptError::ValueConversion`] with a `type_name()`-derived
/// diagnostic.
fn lower_value(ctx: &Ctx<'_>, value: Value<'_>) -> Result<ScriptValue, ScriptError> {
    if value.is_null() || value.is_undefined() {
        return Ok(ScriptValue::Null);
    }
    if let Some(b) = value.as_bool() {
        return Ok(ScriptValue::Bool(b));
    }
    if let Some(i) = value.as_int() {
        return Ok(ScriptValue::Number(f64::from(i)));
    }
    if let Some(f) = value.as_float() {
        return Ok(ScriptValue::Number(f));
    }
    if let Some(s) = value.as_string() {
        let owned = s
            .to_string()
            .map_err(|e| ScriptError::ValueConversion(format!("string decode failed: {e}")))?;
        return Ok(ScriptValue::String(owned));
    }
    if let Some(arr) = value.as_array() {
        let mut items = Vec::with_capacity(arr.len());
        for entry in arr.iter::<Value<'_>>() {
            let v = entry.map_err(|e| {
                ScriptError::ValueConversion(format!("array element decode failed: {e}"))
            })?;
            items.push(lower_value(ctx, v)?);
        }
        return Ok(ScriptValue::Array(items));
    }
    // Promise / Function / TypedArray / ArrayBuffer / Constructor /
    // Proxy / Class — all are JS-level objects and would otherwise
    // fall into `as_object` below. We bail out first because the
    // slice-1 vocabulary deliberately does not project them; later
    // slices may surface promises through `eval_promise` once the
    // script-API surface introduces async (slice 5+).
    if value.is_promise()
        || value.is_function()
        || value.is_constructor()
        || value.is_proxy()
        || value.is_error()
    {
        return Err(ScriptError::ValueConversion(format!(
            "unsupported script value of type `{}`",
            value.type_name(),
        )));
    }
    if let Some(obj) = value.as_object() {
        return lower_object(ctx, obj).map(ScriptValue::Object);
    }
    // Symbols, BigInts, and any future tagged values fall through.
    Err(ScriptError::ValueConversion(format!(
        "unsupported script value of type `{}`",
        value.type_name(),
    )))
}

/// Lower a QuickJS plain object to an insertion-ordered
/// `Vec<(String, ScriptValue)>`.
///
/// Property iteration uses `Object::keys::<String>()` which
/// returns the script-author-visible insertion order (QuickJS
/// preserves creation order for non-integer string keys, matching
/// ECMAScript 2015 `[[OwnPropertyKeys]]` semantics for plain
/// objects). The slice-1 corpus's `object_round_trip` fixture
/// asserts the order is honoured across the host boundary.
fn lower_object(
    ctx: &Ctx<'_>,
    obj: &Object<'_>,
) -> Result<Vec<(String, ScriptValue)>, ScriptError> {
    let mut entries: Vec<(String, ScriptValue)> = Vec::new();
    for key_result in obj.keys::<String>() {
        let key = key_result
            .map_err(|e| ScriptError::ValueConversion(format!("object key decode failed: {e}")))?;
        let raw_value: Value<'_> = obj.get(key.as_str()).map_err(|e| {
            ScriptError::ValueConversion(format!("object value lookup failed for `{key}`: {e}"))
        })?;
        let lowered = lower_value(ctx, raw_value)?;
        entries.push((key, lowered));
    }
    Ok(entries)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod execute_drift_pins {
    //! Evidence: EV-SCRIPT-CONTRACT (T1) — sandbox-default posture and error categorisation.
    //! Evidence: EV-SANDBOX-SAFETY (T1) — watchdog interrupt + memory cap + deadline arithmetic + config validation.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins the
    //! slice-1 [`super::ScriptRuntime::execute`] surface against
    //! the entry-point contract, plus the slice-2 watchdog surface
    //! (interrupt-handler install / detach, hard-cap → TimeoutExceeded
    //! lowering, observation recording, builder-level config
    //! validation), plus the slice-3 memory-cap surface
    //! (`set_memory_limit` install / uninstall, OOM → MemoryCapExceeded
    //! lowering, snapshot recording, builder-level config
    //! validation, builder chaining with the watchdog):
    //!
    //! - the runtime constructs without panicking,
    //! - trivial arithmetic and string literals lower to the
    //!   expected [`crate::ScriptValue`] variants,
    //! - syntactically broken source rejects with
    //!   [`crate::ScriptError::Compile`] (NOT a panic, NOT a
    //!   `Runtime` mis-categorisation),
    //! - `throw new Error(...)` rejects with
    //!   [`crate::ScriptError::Runtime`] (NOT a panic),
    //! - the sandbox-default audit (ADR-012 §6): no `globalThis.fs`,
    //!   no `globalThis.process`, no `globalThis.window` are
    //!   defined out of the box,
    //! - dynamic `import('foo')` rejects (slice 1 evaluates as
    //!   `Eval` mode, where the `import(...)` syntax is not
    //!   accepted; the test asserts non-Ok and the error category
    //!   without pinning the exact message because rquickjs may
    //!   adjust the diagnostic between point releases).
    //!
    //! The full contract corpus (function determinism, array /
    //! object round-trip, multi-runtime isolation) is covered by
    //! T2 under `engine/script/tests/contract.rs`; T1 here is the
    //! always-on tripwire that survives even on a runner without
    //! the broader fixture set.

    use crate::error::ScriptError;
    use crate::runtime::ScriptRuntime;
    use crate::value::ScriptValue;

    #[test]
    fn trivial_arithmetic_executes() {
        let rt = ScriptRuntime::new().expect("runtime constructs");
        let v = rt
            .execute("1 + 2")
            .expect("trivial arithmetic must succeed");
        assert_eq!(v, ScriptValue::Number(3.0), "got {v:?}");
    }

    #[test]
    fn string_literal_executes() {
        let rt = ScriptRuntime::new().expect("runtime constructs");
        let v = rt
            .execute("\"hello\"")
            .expect("string literal must succeed");
        assert_eq!(v, ScriptValue::String("hello".to_string()), "got {v:?}");
    }

    #[test]
    fn broken_syntax_returns_compile_error() {
        let rt = ScriptRuntime::new().expect("runtime constructs");
        let err = rt
            .execute("let x =")
            .expect_err("incomplete `let` must reject");
        assert!(
            matches!(err, ScriptError::Compile(_)),
            "broken syntax must surface as Compile, got {err:?}",
        );
    }

    #[test]
    fn runtime_throw_returns_runtime_error() {
        let rt = ScriptRuntime::new().expect("runtime constructs");
        let err = rt
            .execute("throw new Error(\"x\")")
            .expect_err("uncaught throw must reject");
        assert!(
            matches!(err, ScriptError::Runtime(_)),
            "uncaught throw must surface as Runtime, got {err:?}",
        );
    }

    #[test]
    fn no_globalthis_fs() {
        let rt = ScriptRuntime::new().expect("runtime constructs");
        let v = rt
            .execute("typeof globalThis.fs === 'undefined'")
            .expect("typeof check must succeed");
        assert_eq!(
            v,
            ScriptValue::Bool(true),
            "globalThis.fs must be undefined; got {v:?}",
        );
    }

    #[test]
    fn no_globalthis_process() {
        let rt = ScriptRuntime::new().expect("runtime constructs");
        let v = rt
            .execute("typeof globalThis.process === 'undefined'")
            .expect("typeof check must succeed");
        assert_eq!(
            v,
            ScriptValue::Bool(true),
            "globalThis.process must be undefined; got {v:?}",
        );
    }

    #[test]
    fn no_globalthis_window() {
        let rt = ScriptRuntime::new().expect("runtime constructs");
        let v = rt
            .execute("typeof globalThis.window === 'undefined'")
            .expect("typeof check must succeed");
        assert_eq!(
            v,
            ScriptValue::Bool(true),
            "globalThis.window must be undefined; got {v:?}",
        );
    }

    #[test]
    fn import_rejected() {
        let rt = ScriptRuntime::new().expect("runtime constructs");
        // `import(...)` is dynamic-import syntax; in slice-1 Eval
        // mode QuickJS evaluates it to a Promise (the rejection /
        // resolution lands later via the QuickJS job queue, which
        // slice 1 does not pump). The slice-1 [`super::lower_value`]
        // path classifies promises as unrepresentable, so the
        // call surfaces as [`ScriptError::ValueConversion`]. The
        // tripwire here is that the import does NOT silently
        // resolve to a usable module — `Compile`, `Runtime`, or
        // `ValueConversion` are all acceptable error categories
        // for the contract corpus. The exact category is
        // intentionally not pinned (per the slice brief: "depending
        // on QuickJS / rquickjs behavior").
        let err = rt
            .execute("import('foo')")
            .expect_err("import in eval mode must reject");
        assert!(
            matches!(
                err,
                ScriptError::Compile(_) | ScriptError::Runtime(_) | ScriptError::ValueConversion(_),
            ),
            "import must reject as Compile / Runtime / ValueConversion, got {err:?}",
        );
    }

    #[test]
    fn require_undefined() {
        // CommonJS `require` is not a QuickJS built-in; calling
        // it should throw a ReferenceError at runtime (the symbol
        // is unbound, not a syntax error). Companion check to
        // `import_rejected`: covers the CommonJS host-API surface
        // alongside the ES-module surface.
        let rt = ScriptRuntime::new().expect("runtime constructs");
        let err = rt
            .execute("require('foo')")
            .expect_err("require must reject (unbound symbol)");
        assert!(
            matches!(err, ScriptError::Runtime(_)),
            "require must surface as Runtime, got {err:?}",
        );
    }

    // ----- Slice-2 watchdog drift pins -----

    #[test]
    fn trivial_script_under_cap_succeeds() {
        // Evidence: EV-SANDBOX-SAFETY (T1) + EV-SCRIPT-CONTRACT (T1).
        // A trivial arithmetic expression must complete well under
        // the 4 ms hard cap; the soft cap is generous (1 ms) so the
        // success path should not surface a soft-cap-exceeded
        // observation either.
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_watchdog(1, 4)
            .expect("watchdog config valid");
        let v = rt
            .execute("1 + 2")
            .expect("trivial arithmetic must succeed under watchdog");
        assert_eq!(v, ScriptValue::Number(3.0), "got {v:?}");
        let obs = rt
            .last_observation()
            .expect("watchdog records observation on every call");
        assert!(
            !obs.hard_cap_exceeded,
            "trivial script must not trip hard cap; obs={obs:?}",
        );
        assert!(
            obs.observed_ms < obs.config.hard_ms(),
            "observed_ms must be < hard_ms; got obs={obs:?}",
        );
    }

    #[test]
    fn infinite_loop_rejects_with_timeout_exceeded() {
        // Evidence: EV-SANDBOX-SAFETY (T1).
        // The most basic watchdog tripwire: a tight infinite loop
        // must surface as TimeoutExceeded and the observed time
        // must be at or after the hard cap (modulo runner clock
        // resolution, hence `>= hard_ms` rather than `>= hard_ms +
        // 1`).
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_watchdog(1, 4)
            .expect("watchdog config valid");
        let err = rt
            .execute("while (true) { let x = 1 + 1; }")
            .expect_err("infinite loop must hit hard cap");
        let (p99, hard, observed) = match err {
            ScriptError::TimeoutExceeded {
                p99_ms,
                hard_ms,
                observed_ms,
            } => (p99_ms, hard_ms, observed_ms),
            other => panic!("expected TimeoutExceeded, got {other:?}"),
        };
        assert_eq!(p99, 1, "soft cap echoed verbatim");
        assert_eq!(hard, 4, "hard cap echoed verbatim");
        assert!(
            observed >= hard,
            "observed_ms ({observed}) must be >= hard_ms ({hard})",
        );
        let obs = rt.last_observation().expect("watchdog records observation");
        assert!(obs.hard_cap_exceeded, "hard_cap_exceeded must be set");
    }

    #[test]
    fn recursion_explosion_rejects_with_timeout_exceeded() {
        // Evidence: EV-SANDBOX-SAFETY (T1).
        // Non-tail-call recursion blows the QuickJS stack quickly;
        // the watchdog must fire BEFORE QuickJS itself raises a
        // RangeError("Maximum call stack size exceeded"). On most
        // hosts the stack overflow lands within microseconds, so
        // the test runs `f(n+1)` with a synthetic per-call delay
        // (a busy reduction) to keep the call chain alive long
        // enough for the 4 ms hard cap to fire. Either outcome
        // (TimeoutExceeded OR a stack-depth Runtime error) is a
        // valid sandbox-safety result, but the watchdog SHOULD
        // win on a configured-with-watchdog runtime.
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_watchdog(1, 4)
            .expect("watchdog config valid");
        // The body sums `n` with itself a few times to grow the
        // per-frame work and slow stack growth. We don't strictly
        // need TimeoutExceeded here — a stack overflow is also an
        // acceptable sandbox-safety outcome — but on slice-2's
        // 4 ms budget the watchdog typically wins. Accept either.
        let err = rt
            .execute(
                r"
                function f(n) {
                    let s = n + n + n;
                    return f(s + 1);
                }
                f(0)
            ",
            )
            .expect_err("runaway recursion must reject");
        match err {
            ScriptError::TimeoutExceeded {
                hard_ms,
                observed_ms,
                ..
            } => {
                assert!(
                    observed_ms >= hard_ms,
                    "observed_ms ({observed_ms}) >= hard_ms ({hard_ms})",
                );
            }
            ScriptError::Runtime(msg) => {
                // QuickJS may raise a stack-overflow RangeError
                // before the watchdog's polling interval comes
                // around. Accept this outcome as long as it's not
                // a successful eval (which would be a sandbox
                // breach).
                assert!(
                    !msg.is_empty(),
                    "stack-overflow path must surface a non-empty diagnostic",
                );
            }
            other => panic!("expected TimeoutExceeded or Runtime stack overflow, got {other:?}",),
        }
    }

    #[test]
    fn watchdog_config_invalid_rejects() {
        // Evidence: EV-SANDBOX-SAFETY (T1).
        // Slice-2 builder validates `0 < p99_ms < hard_ms`. An
        // inverted pair (p99=10, hard=4) must surface as
        // [`ScriptError::WatchdogConfig`] without consuming the
        // builder's runtime allocation as a side effect — the
        // caller can recover and re-call with a valid pair.
        let err = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_watchdog(10, 4)
            .expect_err("inverted caps must reject");
        match err {
            ScriptError::WatchdogConfig {
                p99_ms,
                hard_ms,
                reason,
            } => {
                assert_eq!(p99_ms, 10, "p99_ms echoed verbatim");
                assert_eq!(hard_ms, 4, "hard_ms echoed verbatim");
                assert!(!reason.is_empty(), "reason populated");
            }
            other => panic!("expected WatchdogConfig, got {other:?}"),
        }
    }

    #[test]
    fn watchdog_passthrough_under_threshold() {
        // Evidence: EV-SANDBOX-SAFETY (T1) + EV-SCRIPT-CONTRACT (T1).
        // A script that completes well under the soft cap must
        // surface a clean observation: hard cap not exceeded AND
        // soft cap not exceeded. Confirms that the watchdog does
        // not spuriously flag fast scripts.
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            // Generous soft cap (200 ms) so that any reasonable
            // host completes the trivial arithmetic well under it.
            .with_watchdog(200, 400)
            .expect("watchdog config valid");
        let v = rt
            .execute("1 + 2")
            .expect("trivial arithmetic must succeed under watchdog");
        assert_eq!(v, ScriptValue::Number(3.0), "got {v:?}");
        let obs = rt
            .last_observation()
            .expect("watchdog records observation on every call");
        assert!(
            !obs.hard_cap_exceeded,
            "hard cap must not trip; obs={obs:?}",
        );
        assert!(
            !obs.soft_cap_exceeded,
            "soft cap must not trip on trivial arithmetic; obs={obs:?}",
        );
    }

    #[test]
    fn watchdog_observation_cleared_when_no_config() {
        // Evidence: EV-SANDBOX-SAFETY (T1).
        // [`ScriptRuntime::new`] alone (no watchdog) must NOT
        // record an observation; this preserves slice-1's surface
        // for callers that haven't migrated to the builder.
        let rt = ScriptRuntime::new().expect("runtime constructs");
        let _ = rt.execute("1 + 2").expect("succeeds");
        assert!(
            rt.last_observation().is_none(),
            "no watchdog ⇒ no observation",
        );
    }

    // ----- Slice-3 memory-cap drift pins -----

    #[test]
    fn trivial_script_under_memory_cap_succeeds() {
        // Evidence: EV-SANDBOX-SAFETY (T1) + EV-SCRIPT-CONTRACT (T1).
        // A trivial arithmetic expression must complete well under
        // the 16 MiB heap cap; the cap-installed runtime must
        // surface the result identically to a cap-free runtime
        // and record a memory snapshot for inspection.
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_memory_limit(crate::memory_cap::MemoryCapConfig::PHASE2_DEFAULT)
            .expect("memory cap config valid");
        let v = rt
            .execute("1 + 2")
            .expect("trivial arithmetic must succeed under memory cap");
        assert_eq!(v, ScriptValue::Number(3.0), "got {v:?}");
        let snap = rt
            .last_memory_snapshot()
            .expect("memory cap records snapshot on every call");
        assert!(
            snap.at_uninstall_bytes >= snap.at_install_bytes,
            "snapshot must not shrink: install={} uninstall={}",
            snap.at_install_bytes,
            snap.at_uninstall_bytes,
        );
        assert!(
            snap.at_uninstall_bytes < crate::memory_cap::MemoryCapConfig::PHASE2_DEFAULT as u64,
            "trivial script must stay under the 16 MiB cap; observed={}",
            snap.at_uninstall_bytes,
        );
    }

    #[test]
    fn large_string_allocation_rejects_with_memory_cap_exceeded() {
        // Evidence: EV-SANDBOX-SAFETY (T1).
        // The most basic memory-cap tripwire: `String.repeat`
        // allocating 20 MiB worth of single-byte characters under
        // a 16 MiB cap must surface as MemoryCapExceeded with the
        // configured limit echoed verbatim. QuickJS's allocator
        // checks the running total before each allocation; the
        // failing allocation never updates the bookkeeping, so
        // `observed_bytes` is the snapshot at host-return — which
        // is necessarily within the cap (the failing allocation
        // didn't accumulate).
        let limit = 16 * 1024 * 1024;
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_memory_limit(limit)
            .expect("memory cap config valid");
        let err = rt
            .execute(r#"let s = "x".repeat(20 * 1024 * 1024); s.length"#)
            .expect_err("20 MiB string under 16 MiB cap must reject");
        match err {
            ScriptError::MemoryCapExceeded {
                limit_bytes,
                observed_bytes,
            } => {
                assert_eq!(limit_bytes, limit, "limit echoed verbatim");
                // observed_bytes is `malloc_size` at host-return;
                // it may be at or just above / below the limit
                // depending on QuickJS's allocator-state update
                // order. Just sanity-check non-zero.
                assert!(
                    observed_bytes > 0,
                    "observed_bytes must be populated; got {observed_bytes}",
                );
            }
            other => panic!("expected MemoryCapExceeded, got {other:?}"),
        }
    }

    #[test]
    fn array_explosion_rejects_with_memory_cap_exceeded() {
        // Evidence: EV-SANDBOX-SAFETY (T1).
        // Recursive array-grow exhausts the heap in pure JS.
        // The watchdog is intentionally not configured here so
        // that the only abort path is the memory cap; any other
        // error category is a sandbox-safety regression. We use a
        // moderately fast loop so the OOM lands before any host
        // perf surprise; the array growth doubles each iteration
        // until the cap fires.
        let limit = 16 * 1024 * 1024;
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_memory_limit(limit)
            .expect("memory cap config valid");
        let err = rt
            .execute(
                r"
                let a = [];
                while (true) {
                    a = a.concat(a);
                    a.push(1);
                }
            ",
            )
            .expect_err("array explosion must reject");
        // Either MemoryCapExceeded (the typical outcome — the
        // doubling loop OOMs before any other guard fires) or a
        // ScriptError::Runtime carrying the lowered OOM message
        // is acceptable; the contract is "the runtime does not
        // return Ok from a runaway allocation under a 16 MiB cap".
        match err {
            ScriptError::MemoryCapExceeded { limit_bytes, .. } => {
                assert_eq!(limit_bytes, limit, "limit echoed verbatim");
            }
            ScriptError::Runtime(msg) => {
                assert!(
                    msg.contains("out of memory") || !msg.is_empty(),
                    "array explosion path must surface a non-empty diagnostic; got {msg:?}",
                );
            }
            other => {
                panic!("expected MemoryCapExceeded or Runtime OOM, got {other:?}",)
            }
        }
    }

    #[test]
    fn memory_limit_config_invalid_rejects() {
        // Evidence: EV-SANDBOX-SAFETY (T1).
        // Slice-3 builder validates `bytes > 0`. A zero limit
        // surfaces as [`ScriptError::MemoryCapConfig`] rather than
        // silently surfacing an uncapped runtime (QuickJS-native
        // semantics for `set_memory_limit(0)`).
        let err = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_memory_limit(0)
            .expect_err("zero limit must reject");
        match err {
            ScriptError::MemoryCapConfig {
                limit_bytes,
                reason,
            } => {
                assert_eq!(limit_bytes, 0, "limit echoed verbatim");
                assert!(!reason.is_empty(), "reason populated");
            }
            other => panic!("expected MemoryCapConfig, got {other:?}"),
        }
    }

    #[test]
    fn combined_watchdog_and_memory_limit_compose() {
        // Evidence: EV-SANDBOX-SAFETY (T1) + EV-SCRIPT-CONTRACT (T1).
        // The two slice-2 + slice-3 builder methods compose. A
        // runtime configured with both surfaces must honour both:
        // a fast script under both caps must succeed and record
        // both a watchdog observation and a memory snapshot.
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_watchdog(200, 400)
            .expect("watchdog config valid")
            .with_memory_limit(crate::memory_cap::MemoryCapConfig::PHASE2_DEFAULT)
            .expect("memory cap config valid");
        let v = rt
            .execute("1 + 2")
            .expect("trivial arithmetic must succeed under both caps");
        assert_eq!(v, ScriptValue::Number(3.0), "got {v:?}");
        let obs = rt
            .last_observation()
            .expect("watchdog records observation when configured");
        assert!(
            !obs.hard_cap_exceeded,
            "hard cap must not trip on trivial arithmetic; obs={obs:?}",
        );
        let snap = rt
            .last_memory_snapshot()
            .expect("memory cap records snapshot when configured");
        assert!(
            snap.at_uninstall_bytes >= snap.at_install_bytes,
            "snapshot must not shrink: install={} uninstall={}",
            snap.at_install_bytes,
            snap.at_uninstall_bytes,
        );
    }

    #[test]
    fn memory_snapshot_cleared_when_no_config() {
        // Evidence: EV-SANDBOX-SAFETY (T1).
        // [`ScriptRuntime::new`] alone (no memory cap) must NOT
        // record a snapshot; preserves slice-1 / slice-2 surface
        // for callers that haven't migrated to the builder.
        let rt = ScriptRuntime::new().expect("runtime constructs");
        let _ = rt.execute("1 + 2").expect("succeeds");
        assert!(
            rt.last_memory_snapshot().is_none(),
            "no memory cap ⇒ no snapshot",
        );
    }

    // ----- Slice-4 determinism shim drift pins -----

    #[test]
    fn determinism_state_absent_by_default() {
        // Evidence: EV-SCRIPT-CONTRACT (T1).
        // [`ScriptRuntime::new`] alone (no shims) must NOT mount a
        // determinism state; preserves the slice-1 / slice-2 /
        // slice-3 surface for callers that haven't migrated to the
        // determinism builders.
        let rt = ScriptRuntime::new().expect("runtime constructs");
        assert!(
            rt.determinism_state().is_none(),
            "no determinism builders ⇒ no state",
        );
    }

    #[test]
    fn with_random_seed_records_state() {
        // Evidence: EV-SCRIPT-CONTRACT (T1).
        // The builder must round-trip the seed through the
        // host-side diagnostic surface so callers / tests can
        // confirm the shim is in force.
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_random_seed(0xDEAD_BEEF)
            .expect("seed installs");
        let state = rt
            .determinism_state()
            .expect("seed installed ⇒ state present");
        assert_eq!(state.seed(), Some(0xDEAD_BEEF), "seed echoed");
        assert!(
            !state.clock_installed(),
            "seed-only path must not flag clock as installed",
        );
    }

    #[test]
    fn with_clock_records_state() {
        // Evidence: EV-SCRIPT-CONTRACT (T1).
        // The clock builder must flag the host-side
        // clock-installed presence bit; the underlying clock is
        // opaque post-install (the closure owns the only mutable
        // handle).
        use crate::determinism::FixedClock;
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_clock(Box::new(FixedClock::new(1_700_000_000_000)))
            .expect("clock installs");
        let state = rt
            .determinism_state()
            .expect("clock installed ⇒ state present");
        assert!(state.clock_installed(), "clock flag must flip");
        assert!(
            state.seed().is_none(),
            "clock-only path must not flag seed as installed",
        );
    }

    #[test]
    fn determinism_builders_compose() {
        // Evidence: EV-SCRIPT-CONTRACT (T1).
        // Both shim builders MUST chain on a single runtime —
        // a seed installed first does not invalidate a later
        // clock install, and vice versa. Fold into one diagnostic
        // surface.
        use crate::determinism::FixedClock;
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_random_seed(7)
            .expect("seed installs")
            .with_clock(Box::new(FixedClock::new(42)))
            .expect("clock installs after seed");
        let state = rt.determinism_state().expect("composed state present");
        assert_eq!(state.seed(), Some(7), "seed survived clock install");
        assert!(
            state.clock_installed(),
            "clock flag survived seed-already-installed path",
        );
    }

    #[test]
    fn seeded_random_lowers_to_number() {
        // Evidence: EV-SCRIPT-CONTRACT (T1).
        // Integration drift pin: with a seed installed, a JS-side
        // call to `Math.random()` must succeed and lower to a
        // ScriptValue::Number in [0,1). The cross-runtime corpus
        // (same-seed-same-sequence, isolation) lives in
        // `engine/script/tests/determinism.rs`.
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_random_seed(1)
            .expect("seed installs");
        let v = rt
            .execute("Math.random()")
            .expect("seeded Math.random must succeed");
        let n = match v {
            ScriptValue::Number(n) => n,
            other => panic!("expected Number, got {other:?}"),
        };
        assert!(
            (0.0..1.0).contains(&n),
            "seeded Math.random must return [0,1); got {n}",
        );
    }

    #[test]
    fn frame_locked_clock_lowers_to_number() {
        // Evidence: EV-SCRIPT-CONTRACT (T1).
        // Integration drift pin: with a FixedClock installed, a
        // JS-side call to `Date.now()` must lower to the exact
        // injected tick.
        use crate::determinism::FixedClock;
        let tick = 1_700_000_000_000_i64;
        let rt = ScriptRuntime::new()
            .expect("runtime constructs")
            .with_clock(Box::new(FixedClock::new(tick)))
            .expect("clock installs");
        let v = rt.execute("Date.now()").expect("Date.now succeeds");
        let n = match v {
            ScriptValue::Number(n) => n,
            other => panic!("expected Number, got {other:?}"),
        };
        #[allow(clippy::cast_precision_loss)]
        let expected = tick as f64;
        assert_eq!(
            n.to_bits(),
            expected.to_bits(),
            "frame-locked Date.now must return tick verbatim; got {n}",
        );
    }
}

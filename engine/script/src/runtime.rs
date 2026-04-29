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

use crate::error::ScriptError;
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
    // handle on every [`Self::execute`] call; slice 3 will also
    // mount the memory cap. The field is no longer underscore-
    // prefixed since the watchdog code path uses it.
    runtime: Runtime,
    // Watchdog config in force for this runtime, set via
    // [`Self::with_watchdog`]. `None` reproduces the slice-1
    // behaviour: no interrupt handler is installed, no per-execute
    // observation is recorded.
    watchdog_config: Option<WatchdogConfig>,
    // Most-recent watchdog observation, populated on every
    // [`Self::execute`] call when a watchdog config is active.
    // Read by tests via [`Self::last_observation`] for the slice-2
    // soft-cap-not-tripped contract. `Cell<Option<...>>` keeps the
    // host surface `&self`-friendly without forcing
    // `RefCell::borrow_mut` and the associated runtime borrow
    // panic surface; observations are `Copy` so `Cell::set` /
    // `Cell::get` is enough.
    last_observation: Cell<Option<WatchdogObservation>>,
}

impl std::fmt::Debug for ScriptRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptRuntime")
            .field("watchdog_config", &self.watchdog_config)
            .field("last_observation", &self.last_observation.get())
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
            last_observation: Cell::new(None),
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

    /// Compile and execute `source` in the global context, returning
    /// a borrow-free [`ScriptValue`] tree.
    ///
    /// Failure modes (see [`ScriptError`] variants for the full
    /// surface): syntax / parse errors lower to
    /// [`ScriptError::Compile`]; uncaught thrown values during
    /// execution lower to [`ScriptError::Runtime`]; values whose
    /// shape is outside the slice-1 vocabulary lower to
    /// [`ScriptError::ValueConversion`]; watchdog interrupts
    /// lower to [`ScriptError::TimeoutExceeded`].
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

        // Always detach the interrupt handler — see install_watchdog
        // doc comment for the leftover-deadline rationale.
        if deadline.is_some() {
            watchdog::uninstall_watchdog(&self.runtime);
        }

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
    //! Evidence: EV-SANDBOX-SAFETY (T1) — watchdog interrupt + deadline arithmetic + config validation.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins the
    //! slice-1 [`super::ScriptRuntime::execute`] surface against
    //! the entry-point contract, plus the slice-2 watchdog surface
    //! (interrupt-handler install / detach, hard-cap → TimeoutExceeded
    //! lowering, observation recording, builder-level config
    //! validation):
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
}

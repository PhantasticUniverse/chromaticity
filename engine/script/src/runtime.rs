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

use rquickjs::{CatchResultExt, Context, Ctx, Object, Runtime, Value};

use crate::error::ScriptError;
use crate::value::ScriptValue;

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
pub struct ScriptRuntime {
    // Field order matters for drop: `context` references the
    // runtime, so the runtime must outlive the context. Rust
    // drops fields in declaration order, so `context` is dropped
    // first.
    context: Context,
    // Held but unused on the public surface — required so the
    // QuickJS isolate stays alive for as long as the context
    // does. `_runtime` is the slice-1 storage point; slice 2 will
    // mount the watchdog interrupt handler against the same
    // handle and slice 3 will mount the memory cap.
    _runtime: Runtime,
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
            _runtime: runtime,
        })
    }

    /// Compile and execute `source` in the global context, returning
    /// a borrow-free [`ScriptValue`] tree.
    ///
    /// Failure modes (see [`ScriptError`] variants for the full
    /// surface): syntax / parse errors lower to
    /// [`ScriptError::Compile`]; uncaught thrown values during
    /// execution lower to [`ScriptError::Runtime`]; values whose
    /// shape is outside the slice-1 vocabulary lower to
    /// [`ScriptError::ValueConversion`].
    pub fn execute(&self, source: &str) -> Result<ScriptValue, ScriptError> {
        self.context.with(|ctx: Ctx<'_>| {
            // `ctx.eval::<Value, _>` is the slice-1 entry into the
            // QuickJS interpreter. We collect into the lifetime-bound
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
        })
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
            } else {
                ScriptError::Runtime(format_runtime(&message, &rendered))
            }
        }
        CaughtError::Error(err) => ScriptError::Runtime(format!("{err}")),
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
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins the
    //! slice-1 [`super::ScriptRuntime::execute`] surface against
    //! the entry-point contract:
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
}

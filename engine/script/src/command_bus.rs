// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Command-bus integration surface for [`crate::ScriptRuntime`].
//!
//! Slice 5.1 mounts the foundational producer-side bridge so a
//! script running under [`crate::ScriptRuntime::execute`] can emit
//! typed [`ScriptCommand`] values to a host-supplied sink. The sink
//! is expressed as a [`ScriptCommandSender`] trait object so the
//! engine/runtime command bus, a test buffer, or any other
//! transport can be plugged in without engine/script taking a
//! direct dep on `chromaticity-runtime` — the trait is the seam.
//! Slice 5.2 will land the typed reactive-symbol-catalog binding
//! (`docs/36` v1.3.0 `syn_*` names → typed runtime command
//! variants); slice 5.1 keeps the JS-visible surface deliberately
//! narrow so the cross-crate plumbing can settle first.
//!
//! Slice-5.1 surface (this file):
//!
//! - [`ScriptCommand`] — `#[non_exhaustive]` enum with one variant
//!   today, [`ScriptCommand::ReactiveWrite { name, value }`]. The
//!   variant carries a free-form name string + a borrow-free
//!   [`crate::ScriptValue`] payload so the JS-side surface does not
//!   need to know about runtime command types. Slice 5.2 may add
//!   typed variants alongside this one (e.g.
//!   `Bpm { value: f32 }`); the catch-all stays for forward
//!   compat.
//! - [`ScriptCommandError`] — `#[non_exhaustive]` enum with the two
//!   slice-5.1 failure modes a sender can report
//!   ([`ScriptCommandError::Disconnected`] /
//!   [`ScriptCommandError::Backpressure`]). The shape mirrors
//!   `chromaticity_runtime::CommandBusError` so the engine/runtime
//!   adapter slice 5.2 lands can do a one-liner conversion.
//! - [`ScriptCommandSender`] — `Send + Sync` trait the host
//!   implements. Object-safe so callers store
//!   `Arc<dyn ScriptCommandSender>` on the [`crate::ScriptRuntime`]
//!   without a generic parameter on the runtime.
//! - JS-side global `__chromaticityEmit(name, value)` installed via
//!   [`install_command_emit_shim`] once at builder time. The
//!   underscore prefix marks it as a host-private bridge — slice
//!   5.2 will install the script-author-facing `syn_*` Proxy /
//!   accessor surface that calls into this private bridge under
//!   the hood. Naming the bridge `__chromaticityEmit` (not e.g.
//!   `chromaticity.emit`) keeps it out of the most likely
//!   namespace collisions with author-supplied globals.
//!
//! Threading model (`Send + Sync` audit):
//!
//! [`ScriptCommandSender`] is `Send + Sync` because the trait
//! object is stored on [`crate::ScriptRuntime`] inside an `Arc` and
//! is closed over by the JS-side closure. The closure itself runs
//! on the runtime thread (rquickjs default features are `!Send`),
//! so the actual call into [`ScriptCommandSender::send`] always
//! happens single-threaded on the host's runtime thread — but the
//! `Arc` lets the host clone the sender and hand it to the
//! engine/runtime command bus from any thread, which is the seam
//! the slice-5.2 adapter will ride.
//!
//! The closure does NOT need [`rquickjs::function::MutFn`]: the
//! [`ScriptCommandSender::send`] signature takes `&self`, so the
//! closure body is non-mutating from Rust's perspective. Mirrors
//! the slice-3 memory-cap install path which also avoids `MutFn`
//! when the closure has no per-call mutable state.
//!
//! Failure-mode lowering on the JS side:
//!
//! When [`ScriptCommandSender::send`] returns a [`ScriptCommandError`],
//! the JS-side closure throws an `Error` whose `message` carries
//! the canonical category string (`"chromaticity command bus
//! disconnected"` / `"chromaticity command bus backpressure"`).
//! This integrates with the slice-1 [`crate::ScriptError::Runtime`]
//! lowering through [`crate::runtime::ScriptRuntime::execute`]'s
//! existing catch path and the
//! [`crate::runtime::remap_command_bus_error`] post-execute remap
//! that lifts the canonical messages back to the typed
//! [`crate::ScriptError::CommandBusDisconnected`] /
//! [`crate::ScriptError::CommandBusBackpressure`] variants — so
//! the host caller receives a typed error without parsing
//! free-form prose, and a script `try { __chromaticityEmit(...) }
//! catch (e) { ... }` block that swallows the error sees the
//! canonical message verbatim.
//!
//! Governing ADRs:
//!
//! - ADR-001 — runtime model (commands are the inter-thread seam).
//! - ADR-006 — thread and process model (script code runs on the
//!   runtime thread, command bus producers can be on any thread).
//! - ADR-012 — script sandbox engine and budgets.

use std::sync::Arc;

use rquickjs::{Ctx, Function, Value};

use crate::error::ScriptError;
use crate::reactive_symbol::ReactiveSymbol;
use crate::value::ScriptValue;

/// Canonical message string surfaced through the JS-side `Error`
/// thrown when [`ScriptCommandSender::send`] reports
/// [`ScriptCommandError::Disconnected`].
///
/// Crate-private and kept colocated with both the install-path
/// closure and the runtime remap so a future change to the
/// canonical string only needs one edit.
pub(crate) const DISCONNECTED_MESSAGE: &str = "chromaticity command bus disconnected";

/// Canonical message string surfaced through the JS-side `Error`
/// thrown when [`ScriptCommandSender::send`] reports
/// [`ScriptCommandError::Backpressure`]. See [`DISCONNECTED_MESSAGE`]
/// for the locality rationale.
pub(crate) const BACKPRESSURE_MESSAGE: &str = "chromaticity command bus backpressure";

/// Typed message a script emits via the producer-side bridge.
///
/// Slice-5.1 shipped the foundational [`Self::ReactiveWrite`]
/// variant carrying a free-form `name` + borrow-free
/// [`ScriptValue`]. Slice-5.2 widens with the typed
/// [`Self::TypedReactiveWrite`] variant carrying a
/// [`ReactiveSymbol`] resolved against the
/// `docs/36-reactive-symbol-catalog.md` v1.3.0 catalog; the
/// catch-all [`Self::ReactiveWrite`] variant stays for forward
/// compat — a host may keep accepting unknown names (a script
/// emits `__chromaticityEmit('experimental_name', v)`) and a
/// host that wants to drop them just discards the routed
/// rejection.
///
/// `#[non_exhaustive]` keeps slice-5.3+ widening additive on the
/// public surface (e.g. `BeatTick`, `PresetRecall`, etc.).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ScriptCommand {
    /// Free-form reactive-symbol write. `name` is the JS-side
    /// argument verbatim; `value` is the lowered [`ScriptValue`]
    /// (Null / Bool / Number / String / Array / Object). The
    /// slice-5.1 variant; slice-5.2 routes resolvable names into
    /// [`Self::TypedReactiveWrite`] via [`Self::route`], leaving
    /// this variant as the catch-all for unresolved names.
    ReactiveWrite {
        /// Symbol name supplied by the script.
        name: String,
        /// Lowered [`ScriptValue`] payload.
        value: ScriptValue,
    },
    /// Typed reactive-symbol write. `symbol` is the catalog-
    /// resolved [`ReactiveSymbol`]; `value` has been validated
    /// against [`ReactiveSymbol::expected_kind`] at construction
    /// time. Produced by [`Self::route`] when the slice-5.1
    /// string-keyed variant resolves and validates against the
    /// `docs/36` v1.3.0 catalog.
    TypedReactiveWrite {
        /// Catalog-resolved typed symbol.
        symbol: ReactiveSymbol,
        /// Lowered [`ScriptValue`] payload, conforming to
        /// [`ReactiveSymbol::expected_kind`].
        value: ScriptValue,
    },
}

impl ScriptCommand {
    /// Convert the slice-5.1 string-keyed
    /// [`Self::ReactiveWrite { name, value }`] variant into the
    /// slice-5.2 typed [`Self::TypedReactiveWrite { symbol, value
    /// }`] variant when the name resolves against the
    /// `docs/36-reactive-symbol-catalog.md` v1.3.0 catalog AND
    /// the carrier value's shape conforms to the symbol's
    /// [`ReactiveValueKind`].
    ///
    /// Behaviour matrix:
    ///
    /// - Self is [`Self::ReactiveWrite { name, value }`] AND
    ///   [`ReactiveSymbol::from_name`] returns `Some(sym)` AND
    ///   [`ReactiveValueKind::validate`] passes:
    ///   `Ok(Self::TypedReactiveWrite { symbol: sym, value })`.
    /// - Self is [`Self::ReactiveWrite { name, value }`] AND
    ///   [`ReactiveSymbol::from_name`] returns `None`:
    ///   `Err(ScriptError::ReactiveSymbolUnknown { name })`. The
    ///   caller decides the policy — drop the write, escalate as
    ///   an error, or fall back to the catch-all
    ///   [`Self::ReactiveWrite`] by reconstructing it (the
    ///   slice-5.2 routing path is opt-in).
    /// - Self is [`Self::ReactiveWrite { name, value }`] AND the
    ///   name resolves but the value's shape mismatches:
    ///   `Err(ScriptError::ReactiveValueKindMismatch { ... })`.
    /// - Self is already [`Self::TypedReactiveWrite { .. }`]:
    ///   passed through unchanged. Idempotent on the typed
    ///   variant; routing is producer-side and runs once.
    ///
    /// The function consumes `self` so the routed variant's
    /// `value` payload is moved into the result without an extra
    /// clone — slice 5.1 corpus assumes the `Vec<ScriptCommand>`
    /// receiver clones once at sink-time.
    pub fn route(self) -> Result<Self, ScriptError> {
        match self {
            Self::ReactiveWrite { name, value } => match ReactiveSymbol::from_name(&name) {
                Some(symbol) => match symbol.expected_kind().validate(&value) {
                    Ok(()) => Ok(Self::TypedReactiveWrite { symbol, value }),
                    Err(detail) => Err(ScriptError::ReactiveValueKindMismatch {
                        symbol_name: symbol.name(),
                        expected: symbol.expected_kind(),
                        detail,
                    }),
                },
                None => Err(ScriptError::ReactiveSymbolUnknown { name }),
            },
            already @ Self::TypedReactiveWrite { .. } => Ok(already),
        }
    }
}

/// Errors a [`ScriptCommandSender::send`] call may report.
///
/// Shape mirrors `chromaticity_runtime::CommandBusError` so the
/// slice-5.2 adapter that bridges this trait onto
/// `chromaticity_runtime::CommandBus` is a one-liner. Both
/// surfaces are `#[non_exhaustive]` so future variants stay
/// additive.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScriptCommandError {
    /// The sink has been dropped and no longer accepts commands.
    Disconnected,
    /// The sink is at capacity and shed the command without
    /// enqueuing it. The producer caller decides whether to drop,
    /// retry, or fail; slice 5.1 does not embed a per-caller
    /// policy.
    Backpressure,
}

impl ScriptCommandError {
    /// Canonical message string surfaced through the JS-side
    /// `Error` thrown on this variant. Crate-private to keep the
    /// install-path closure and the runtime remap in lock-step.
    pub(crate) fn canonical_message(&self) -> &'static str {
        match self {
            Self::Disconnected => DISCONNECTED_MESSAGE,
            Self::Backpressure => BACKPRESSURE_MESSAGE,
        }
    }
}

/// Producer-side seam for [`ScriptCommand`] values emitted by a
/// script.
///
/// Implementors: typically the host wraps a
/// `chromaticity_runtime::CommandBus` (slice-5.2 adapter), a
/// `crossbeam_channel::Sender<ScriptCommand>`, a `Vec<ScriptCommand>`
/// behind a `Mutex` for tests, or any other transport. The trait
/// is object-safe so [`crate::ScriptRuntime`] can store
/// `Arc<dyn ScriptCommandSender>` without a generic parameter on
/// the runtime — keeping the `with_*` builder chain monomorphic.
///
/// `Send + Sync` is required because the host may clone the `Arc`
/// across threads (e.g. hand one clone to the script thread for
/// the JS-side closure and a sibling clone to the engine/runtime
/// commit thread). The actual `send` call always happens on the
/// runtime thread per ADR-006; the trait bounds let the
/// engine/runtime adapter publish from any thread.
pub trait ScriptCommandSender: Send + Sync {
    /// Enqueue `command` onto the host-side sink. Returns
    /// `Ok(())` on success and a typed [`ScriptCommandError`] on
    /// disconnection / backpressure.
    fn send(&self, command: ScriptCommand) -> Result<(), ScriptCommandError>;
}

/// Install the slice-5.1 `__chromaticityEmit` global on
/// `ctx.globals()`, closing over `sender` so JS-side calls hand
/// the lowered [`ScriptCommand`] to the host-supplied sink.
///
/// JS-side signature: `__chromaticityEmit(name: string, value:
/// any) -> undefined`. Argument shape:
///
/// - `name`: must be a string. Anything else (`number`, `null`,
///   `undefined`, omitted) raises a `TypeError` on the JS side.
/// - `value`: any value the slice-1 [`crate::value`] vocabulary
///   accepts (Null / Bool / Number / String / Array / Object).
///   Symbols / functions / promises raise a `TypeError` on the JS
///   side via the existing
///   [`crate::runtime::lower_value`] error path.
///
/// Failure-mode lowering: when the host-side
/// [`ScriptCommandSender::send`] returns a
/// [`ScriptCommandError`], the closure throws a JS-side `Error`
/// whose message carries the canonical category string. The
/// runtime's post-execute remap re-lifts the canonical strings to
/// typed [`crate::ScriptError::CommandBusDisconnected`] /
/// [`crate::ScriptError::CommandBusBackpressure`] variants when
/// the script does NOT swallow the throw with a `try`/`catch`.
///
/// Returns a [`ScriptError::CommandBusInstall`] if any of the
/// rquickjs operations fail (function construction, property
/// `set`).
///
/// Crate-private: callers route through the
/// [`crate::ScriptRuntime::with_command_sender`] builder, which
/// folds the install into the runtime's
/// `command_sender: Option<Arc<dyn ScriptCommandSender>>` field.
pub(crate) fn install_command_emit_shim(
    ctx: &Ctx<'_>,
    sender: Arc<dyn ScriptCommandSender>,
) -> Result<(), ScriptError> {
    // Capture the `Arc<dyn ...>` by clone into the closure so the
    // runtime keeps its own strong reference for diagnostics
    // (`ScriptRuntime::command_sender()`) while the closure owns
    // a sibling. The shim closure runs on the runtime thread so
    // single-threaded `Arc::clone` sees no contention.
    let sender_for_closure = Arc::clone(&sender);

    // The closure is `Fn` (not `FnMut`) — `ScriptCommandSender::send`
    // takes `&self`, no per-call mutable state lives inside the
    // closure. rquickjs `IntoJsFunc` is implemented blanket for
    // `Fn(...) -> R where R: IntoJs`; `rquickjs::Result<()>` lowers
    // through the `IntoJs for StdResult<T, E> where Error: From<E>`
    // blanket impl so a thrown error propagates as a JS-side
    // exception while `Ok(())` lowers to `undefined`.
    //
    // Argument signature: (Ctx, name: Value, value: Value) ->
    // rquickjs::Result<()>. We accept generic `Value<'_>` slots
    // and decode in-body so the JS-side error surface
    // (`TypeError: name must be a string`) goes through our own
    // `throw_*` calls rather than rquickjs's auto-convert
    // diagnostics.
    let func = Function::new(
        ctx.clone(),
        move |ctx: Ctx<'_>, name: Value<'_>, value: Value<'_>| -> rquickjs::Result<()> {
            emit_into_sender(&ctx, &sender_for_closure, name, value)
        },
    )
    .map_err(|e| ScriptError::CommandBusInstall {
        reason: format!("Function::new(__chromaticityEmit) failed: {e}"),
    })?;

    ctx.globals()
        .set("__chromaticityEmit", func)
        .map_err(|e| ScriptError::CommandBusInstall {
            reason: format!("globals().set(\"__chromaticityEmit\") failed: {e}"),
        })?;

    Ok(())
}

/// Closure body for the JS-side `__chromaticityEmit(name, value)`
/// call.
///
/// Decodes the JS-side arguments, lowers the `value` to a
/// [`ScriptValue`] using the slice-1 vocabulary, hands the
/// resulting [`ScriptCommand`] to the host sender, and lifts any
/// [`ScriptCommandError`] back into a JS-side `Error` so the
/// script can `try`/`catch` it OR the runtime's post-execute
/// remap can recognise the canonical message.
///
/// Argument errors raise a `TypeError` on the JS side. Value-
/// conversion errors raise a `TypeError` carrying the underlying
/// [`ScriptError::ValueConversion`] diagnostic.
fn emit_into_sender(
    ctx: &Ctx<'_>,
    sender: &Arc<dyn ScriptCommandSender>,
    name: Value<'_>,
    value: Value<'_>,
) -> rquickjs::Result<()> {
    // Argument 0: name. Must be a JS string. `as_string()` returns
    // `Some` for the JS string primitive AND for boxed `String`
    // objects via QuickJS's auto-unboxing, which mirrors the
    // ECMAScript spec for `typeof "foo" === "string"` semantics.
    let name_owned: String = match name.as_string() {
        Some(s) => s.to_string()?,
        None => {
            return Err(rquickjs::Exception::throw_type(
                ctx,
                "__chromaticityEmit: argument 0 (name) must be a string",
            ));
        }
    };

    // Argument 1: value. Lower through the slice-1 vocabulary;
    // anything outside the supported set surfaces as a
    // `TypeError` rather than a successful command emit. Reuses
    // [`crate::runtime::lower_value`] (re-exported through the
    // crate root for this purpose) so the slice-1 contract corpus
    // governs the lowering shape.
    let lowered = match crate::runtime::lower_value_for_command_bus(ctx, value) {
        Ok(v) => v,
        Err(e) => {
            return Err(rquickjs::Exception::throw_type(
                ctx,
                &format!("__chromaticityEmit: value lowering failed: {e}"),
            ));
        }
    };

    let command = ScriptCommand::ReactiveWrite {
        name: name_owned,
        value: lowered,
    };

    match sender.send(command) {
        Ok(()) => Ok(()),
        Err(err) => {
            // Throw a regular JS `Error` carrying the canonical
            // message string. The runtime's post-execute remap
            // recognises both canonical strings and lifts them
            // back to typed [`crate::ScriptError`] variants when
            // the throw escapes the script's own `try`/`catch`.
            // A regular `Error` (not `TypeError`) is correct
            // here: the failure is a runtime-environment
            // condition (queue full / disconnected), not a
            // type mismatch.
            Err(rquickjs::Exception::throw_message(
                ctx,
                err.canonical_message(),
            ))
        }
    }
}

/// Test-only sender helpers re-exported into the rest of the
/// crate (drift pins in `crate::runtime::execute_drift_pins`) and
/// kept colocated with the production [`ScriptCommandSender`]
/// trait for proximity. The module is `#[cfg(test)]`-gated so the
/// helpers don't ship in non-test builds.
#[cfg(test)]
pub(crate) mod test_helpers {
    use super::{ScriptCommand, ScriptCommandError, ScriptCommandSender};
    use std::sync::Mutex;

    /// Test-only sender that buffers commands into a
    /// `Mutex<Vec<...>>`. Used by both the T1 pins below and the
    /// integration drift pins in
    /// [`crate::runtime::execute_drift_pins`].
    pub(crate) struct BufferingSender {
        commands: Mutex<Vec<ScriptCommand>>,
    }

    impl BufferingSender {
        pub(crate) fn new() -> Self {
            Self {
                commands: Mutex::new(Vec::new()),
            }
        }

        #[cfg_attr(test, allow(clippy::unwrap_used))]
        pub(crate) fn drain(&self) -> Vec<ScriptCommand> {
            std::mem::take(&mut self.commands.lock().unwrap())
        }
    }

    impl ScriptCommandSender for BufferingSender {
        #[cfg_attr(test, allow(clippy::unwrap_used))]
        fn send(&self, command: ScriptCommand) -> Result<(), ScriptCommandError> {
            self.commands.lock().unwrap().push(command);
            Ok(())
        }
    }

    /// Test-only sender that always reports
    /// [`ScriptCommandError::Disconnected`]. Used by the
    /// integration drift pins to exercise the canonical-message
    /// remap path.
    pub(crate) struct DisconnectedSender;
    impl ScriptCommandSender for DisconnectedSender {
        fn send(&self, _command: ScriptCommand) -> Result<(), ScriptCommandError> {
            Err(ScriptCommandError::Disconnected)
        }
    }

    /// Test-only sender that always reports
    /// [`ScriptCommandError::Backpressure`].
    pub(crate) struct BackpressureSender;
    impl ScriptCommandSender for BackpressureSender {
        fn send(&self, _command: ScriptCommand) -> Result<(), ScriptCommandError> {
            Err(ScriptCommandError::Backpressure)
        }
    }
}

#[cfg(test)]
pub(crate) use test_helpers::{BackpressureSender, BufferingSender, DisconnectedSender};

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod command_bus_pins {
    //! Evidence: EV-SCRIPT-CONTRACT (T1) — slice-5.1 command-bus
    //! data-shape contract.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins the
    //! [`ScriptCommand`] / [`ScriptCommandError`] /
    //! [`ScriptCommandSender`] data-shape contract that the host
    //! [`crate::ScriptRuntime::with_command_sender`] integration
    //! depends on. Integration-level T1 pins (builder composition,
    //! JS-side `__chromaticityEmit` call → sender receive)
    //! live in [`crate::runtime::execute_drift_pins`]; the
    //! end-to-end T2 corpus lives in
    //! `engine/script/tests/command_bus.rs`.
    //!
    //! These pins do NOT exercise QuickJS — they only assert the
    //! Rust-side surface (error shape, sender trait Send+Sync,
    //! command construction, canonical-message wiring). The
    //! QuickJS install path is exercised by the integration drift
    //! pins.

    use super::test_helpers::{BackpressureSender, BufferingSender, DisconnectedSender};
    use super::{
        BACKPRESSURE_MESSAGE, DISCONNECTED_MESSAGE, ScriptCommand, ScriptCommandError,
        ScriptCommandSender,
    };
    use crate::value::ScriptValue;
    use std::sync::Arc;

    #[test]
    fn buffering_sender_round_trips_a_command() {
        let s = BufferingSender::new();
        let cmd = ScriptCommand::ReactiveWrite {
            name: "chroma_BPM".to_string(),
            value: ScriptValue::Number(120.0),
        };
        s.send(cmd.clone()).expect("buffering sender accepts");
        let drained = s.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0], cmd);
    }

    #[test]
    fn disconnected_sender_returns_disconnected_error() {
        let s = DisconnectedSender;
        let err = s
            .send(ScriptCommand::ReactiveWrite {
                name: "x".to_string(),
                value: ScriptValue::Null,
            })
            .expect_err("disconnected sender must reject");
        assert_eq!(err, ScriptCommandError::Disconnected);
    }

    #[test]
    fn backpressure_sender_returns_backpressure_error() {
        let s = BackpressureSender;
        let err = s
            .send(ScriptCommand::ReactiveWrite {
                name: "x".to_string(),
                value: ScriptValue::Null,
            })
            .expect_err("backpressure sender must reject");
        assert_eq!(err, ScriptCommandError::Backpressure);
    }

    #[test]
    fn canonical_messages_are_distinct_and_stable() {
        // The runtime post-execute remap recognises the canonical
        // strings. If a future edit changes either string, BOTH
        // the install-path closure and the remap have to update
        // in lock-step; this pin protects against the partial-
        // edit case where only one site is updated.
        assert_ne!(DISCONNECTED_MESSAGE, BACKPRESSURE_MESSAGE);
        assert_eq!(
            ScriptCommandError::Disconnected.canonical_message(),
            DISCONNECTED_MESSAGE,
        );
        assert_eq!(
            ScriptCommandError::Backpressure.canonical_message(),
            BACKPRESSURE_MESSAGE,
        );
    }

    #[test]
    fn sender_trait_object_is_send_sync() {
        // Compile-time pin: `Arc<dyn ScriptCommandSender>` MUST be
        // `Send + Sync` so the host can clone it across threads.
        // This trips at compile time if the trait bounds drift.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Arc<dyn ScriptCommandSender>>();
    }

    #[test]
    fn script_command_is_clone_eq() {
        // `Clone` + `PartialEq` are part of the slice-5.1
        // contract: the integration tests assert on collected
        // command vectors.
        let a = ScriptCommand::ReactiveWrite {
            name: "n".to_string(),
            value: ScriptValue::Bool(true),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}

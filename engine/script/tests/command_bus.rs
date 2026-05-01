// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-SCRIPT-CONTRACT (T2) — slice-5.1 command-bus producer-side bridge corpus.
//!
//! Widens `docs/26-evidence-artifact-registry.md` row 29
//! (`EV-SCRIPT-CONTRACT`) past the slice-1..slice-4 conformance set
//! with the slice-5.1 producer-side bridge. T2 per
//! `docs/43-development-standards.md` §15.5: corpus conformance
//! over a fixture set with stable golden assertions.
//!
//! Slice-5.1 corpus per ADR-001 (runtime model) + ADR-006 (thread
//! and process model):
//!
//! - `js_emit_round_trips_through_buffering_sender` — a JS-side
//!   call to `__chromaticityEmit('name', value)` produces exactly
//!   one [`chromaticity_script::ScriptCommand::ReactiveWrite`] on
//!   the host buffering sender; the name is preserved verbatim and
//!   the value is the lowered slice-1 [`ScriptValue`].
//! - `value_vocabulary_round_trips` — the slice-1 value vocabulary
//!   (Null, Bool, Number, String, Array, Object) round-trips
//!   through the bridge unchanged. Asserts the closure's
//!   value-lowering path delegates correctly to the slice-1
//!   `lower_value` shape.
//! - `multiple_emits_preserve_order` — N consecutive
//!   `__chromaticityEmit` calls produce N commands in the order
//!   the script issued them. Confirms the sender side honours
//!   per-script ordering even though the trait is `Send + Sync`.
//! - `script_caught_disconnected_throw` — a script that catches
//!   the host-disconnected throw observes the canonical message
//!   string verbatim on `e.message`; the Rust-side `execute`
//!   returns the script's caught-result.
//! - `script_caught_backpressure_throw` — same shape as the
//!   disconnected case for the Backpressure canonical message.
//! - `script_uncaught_disconnected_lifts_to_typed_error` — a
//!   script that does NOT catch the host-disconnected throw
//!   surfaces as
//!   [`chromaticity_script::ScriptError::CommandBusDisconnected`]
//!   on the Rust side via the post-execute canonical-message
//!   remap.
//! - `script_uncaught_backpressure_lifts_to_typed_error` — same
//!   shape for the Backpressure canonical message.
//! - `multiple_runtimes_isolated_senders` — two
//!   [`chromaticity_script::ScriptRuntime`] instances each with
//!   their own sender do NOT cross-contaminate; emits on runtime
//!   A do not appear on runtime B's sender. Mirrors the
//!   slice-1 isolate-per-runtime contract for this surface.
//! - `unrepresentable_value_rejected` — passing an unsupported
//!   value category (e.g. a function) as the second argument
//!   raises a JS-side TypeError; the sender does NOT receive a
//!   command. Asserts the closure's value-lowering rejects the
//!   slice-1 unsupported set the same way `execute` itself does.
//!
//! The slice-5.2 typed reactive-symbol-catalog binding (`docs/36`
//! v1.3.0 `syn_*` names → typed [`ScriptCommand`] variants) is
//! deferred. The slice-5.1 surface intentionally accepts every
//! `name` string verbatim; slice-5.2 will widen
//! [`ScriptCommand`] with typed variants alongside the catch-all
//! [`ScriptCommand::ReactiveWrite`] and tighten the JS-side
//! surface (likely a Proxy or property-getter / setter for `syn_*`
//! that calls the slice-5.1 `__chromaticityEmit` bridge under the
//! hood).
//!
//! Anchors:
//! - `docs/adr/ADR-001-runtime-model.md` — runtime model.
//! - `docs/adr/ADR-006-thread-and-process-model.md` — thread + process model.
//! - `docs/adr/ADR-012-script-sandbox-engine.md` — engine choice + budgets.
//! - `docs/26-evidence-artifact-registry.md` row 29 — `EV-SCRIPT-CONTRACT`.
//! - `docs/36-reactive-symbol-catalog.md` — reactive symbol catalog (slice 5.2 binding target).
//! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::sync::{Arc, Mutex};

use chromaticity_script::{
    ScriptCommand, ScriptCommandError, ScriptCommandSender, ScriptError, ScriptRuntime, ScriptValue,
};

/// External (out-of-crate) buffering sender for the T2 corpus.
///
/// Re-implemented here rather than re-using the crate-private
/// `BufferingSender` so the T2 corpus exercises only the public
/// surface (`Arc<dyn ScriptCommandSender>` is the slice-5.1
/// boundary contract). Identical shape to the in-crate test helper.
struct BufferingSender {
    commands: Mutex<Vec<ScriptCommand>>,
}

impl BufferingSender {
    fn new() -> Self {
        Self {
            commands: Mutex::new(Vec::new()),
        }
    }

    fn drain(&self) -> Vec<ScriptCommand> {
        std::mem::take(&mut self.commands.lock().unwrap())
    }
}

impl ScriptCommandSender for BufferingSender {
    fn send(&self, command: ScriptCommand) -> Result<(), ScriptCommandError> {
        self.commands.lock().unwrap().push(command);
        Ok(())
    }
}

/// External sender that always reports
/// [`ScriptCommandError::Disconnected`].
struct DisconnectedSender;
impl ScriptCommandSender for DisconnectedSender {
    fn send(&self, _command: ScriptCommand) -> Result<(), ScriptCommandError> {
        Err(ScriptCommandError::Disconnected)
    }
}

/// External sender that always reports
/// [`ScriptCommandError::Backpressure`].
struct BackpressureSender;
impl ScriptCommandSender for BackpressureSender {
    fn send(&self, _command: ScriptCommand) -> Result<(), ScriptCommandError> {
        Err(ScriptCommandError::Backpressure)
    }
}

#[test]
fn js_emit_round_trips_through_buffering_sender() {
    // The simplest happy path: one JS-side emit produces one
    // host-side ScriptCommand::ReactiveWrite. Pins the
    // foundational round-trip contract.
    let sender = Arc::new(BufferingSender::new());
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(sender.clone())
        .expect("sender installs");
    let _ = rt
        .execute("__chromaticityEmit('syn_BPM', 120)")
        .expect("emit must succeed");
    let drained = sender.drain();
    assert_eq!(drained.len(), 1, "exactly one emit; got {drained:?}");
    let ScriptCommand::ReactiveWrite { name, value } = &drained[0] else {
        panic!("expected ReactiveWrite; got {:?}", drained[0]);
    };
    assert_eq!(name, "syn_BPM");
    assert_eq!(value, &ScriptValue::Number(120.0));
}

#[test]
fn value_vocabulary_round_trips() {
    // The slice-1 value vocabulary (Null/Bool/Number/String/
    // Array/Object) MUST round-trip through the bridge. Six
    // consecutive emits, one per category. Order assertion is
    // covered separately by `multiple_emits_preserve_order`; this
    // test pins the value-shape projection.
    let sender = Arc::new(BufferingSender::new());
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(sender.clone())
        .expect("sender installs");
    let _ = rt
        .execute(
            r"
            __chromaticityEmit('null_case', null);
            __chromaticityEmit('bool_case', true);
            __chromaticityEmit('number_case', 3.5);
            __chromaticityEmit('string_case', 'hello');
            __chromaticityEmit('array_case', [1, 2, 3]);
            __chromaticityEmit('object_case', { a: 1, b: 2 });
        ",
        )
        .expect("six emits must succeed");
    let drained = sender.drain();
    assert_eq!(drained.len(), 6, "six emits; got {drained:?}");

    let expected: Vec<(&str, ScriptValue)> = vec![
        ("null_case", ScriptValue::Null),
        ("bool_case", ScriptValue::Bool(true)),
        // 3.5 (not π) so clippy's approx_constant lint doesn't
        // mistake it for std::f64::consts::PI; any exact f64
        // value works for the round-trip pin.
        ("number_case", ScriptValue::Number(3.5)),
        ("string_case", ScriptValue::String("hello".to_string())),
        (
            "array_case",
            ScriptValue::Array(vec![
                ScriptValue::Number(1.0),
                ScriptValue::Number(2.0),
                ScriptValue::Number(3.0),
            ]),
        ),
        (
            "object_case",
            ScriptValue::Object(vec![
                ("a".to_string(), ScriptValue::Number(1.0)),
                ("b".to_string(), ScriptValue::Number(2.0)),
            ]),
        ),
    ];

    for (i, (expected_name, expected_value)) in expected.iter().enumerate() {
        let ScriptCommand::ReactiveWrite { name, value } = &drained[i] else {
            panic!("emit {i}: expected ReactiveWrite; got {:?}", drained[i]);
        };
        assert_eq!(name, expected_name, "emit {i} name mismatch");
        assert_eq!(
            value, expected_value,
            "emit {i} value mismatch; got {value:?}",
        );
    }
}

#[test]
fn multiple_emits_preserve_order() {
    // 16 consecutive emits with monotonically increasing values
    // MUST appear on the sender in the same order. The
    // `BufferingSender::Mutex<Vec<...>>` shape preserves push
    // order, so this pins the closure's call-into-sender ordering
    // matches the JS-side execution order.
    let sender = Arc::new(BufferingSender::new());
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(sender.clone())
        .expect("sender installs");
    let _ = rt
        .execute(
            r"
            for (let i = 0; i < 16; i++) {
                __chromaticityEmit('seq', i);
            }
        ",
        )
        .expect("16 emits must succeed");
    let drained = sender.drain();
    assert_eq!(drained.len(), 16);
    for (i, cmd) in drained.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let expected = i as f64;
        let ScriptCommand::ReactiveWrite { name, value } = cmd else {
            panic!("emit {i}: expected ReactiveWrite; got {cmd:?}");
        };
        assert_eq!(name, "seq");
        assert_eq!(value, &ScriptValue::Number(expected));
    }
}

#[test]
fn script_caught_disconnected_throw() {
    // A script that catches the host-disconnected throw observes
    // the canonical message via `e.message`. The Rust-side
    // `execute` then returns the script's caught-result (a string),
    // not an error. The post-execute canonical-message remap MUST
    // NOT misfire on a caught throw.
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(Arc::new(DisconnectedSender))
        .expect("sender installs");
    let v = rt
        .execute(
            r"
            let caught = '';
            try { __chromaticityEmit('x', 1); }
            catch (e) { caught = e.message; }
            caught
        ",
        )
        .expect("script must succeed (caught throw)");
    assert_eq!(
        v,
        ScriptValue::String("chromaticity command bus disconnected".to_string()),
        "caught e.message must be canonical disconnected string; got {v:?}",
    );
}

#[test]
fn script_caught_backpressure_throw() {
    // Mirror of `script_caught_disconnected_throw` for the
    // Backpressure canonical message.
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(Arc::new(BackpressureSender))
        .expect("sender installs");
    let v = rt
        .execute(
            r"
            let caught = '';
            try { __chromaticityEmit('x', 1); }
            catch (e) { caught = e.message; }
            caught
        ",
        )
        .expect("script must succeed (caught throw)");
    assert_eq!(
        v,
        ScriptValue::String("chromaticity command bus backpressure".to_string()),
        "caught e.message must be canonical backpressure string; got {v:?}",
    );
}

#[test]
fn script_uncaught_disconnected_lifts_to_typed_error() {
    // The post-execute canonical-message remap MUST lift an
    // uncaught canonical-message throw to the typed
    // ScriptError::CommandBusDisconnected variant on the
    // Rust-side surface, so the host doesn't have to parse
    // free-form prose.
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(Arc::new(DisconnectedSender))
        .expect("sender installs");
    let err = rt
        .execute("__chromaticityEmit('x', 1)")
        .expect_err("uncaught disconnected throw must reject");
    assert!(
        matches!(err, ScriptError::CommandBusDisconnected),
        "expected CommandBusDisconnected, got {err:?}",
    );
}

#[test]
fn script_uncaught_backpressure_lifts_to_typed_error() {
    // Mirror of
    // `script_uncaught_disconnected_lifts_to_typed_error` for
    // Backpressure.
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(Arc::new(BackpressureSender))
        .expect("sender installs");
    let err = rt
        .execute("__chromaticityEmit('x', 1)")
        .expect_err("uncaught backpressure throw must reject");
    assert!(
        matches!(err, ScriptError::CommandBusBackpressure),
        "expected CommandBusBackpressure, got {err:?}",
    );
}

#[test]
fn multiple_runtimes_isolated_senders() {
    // ADR-012 §6 isolate-per-runtime: two ScriptRuntime instances
    // each with their own sender MUST NOT cross-contaminate.
    // Pinning this for the slice-5.1 surface confirms the
    // `__chromaticityEmit` global is a per-runtime install (the
    // closure captures the per-runtime sender Arc), not a
    // process-global.
    let sender_a = Arc::new(BufferingSender::new());
    let sender_b = Arc::new(BufferingSender::new());
    let rt_a = ScriptRuntime::new()
        .expect("runtime A constructs")
        .with_command_sender(sender_a.clone())
        .expect("sender A installs");
    let rt_b = ScriptRuntime::new()
        .expect("runtime B constructs")
        .with_command_sender(sender_b.clone())
        .expect("sender B installs");
    let _ = rt_a
        .execute("__chromaticityEmit('A_only', 1)")
        .expect("A emit must succeed");
    let _ = rt_b
        .execute("__chromaticityEmit('B_only', 2)")
        .expect("B emit must succeed");

    let drained_a = sender_a.drain();
    let drained_b = sender_b.drain();
    assert_eq!(drained_a.len(), 1, "sender A must see exactly its own emit");
    assert_eq!(drained_b.len(), 1, "sender B must see exactly its own emit");
    let ScriptCommand::ReactiveWrite { name: name_a, .. } = &drained_a[0] else {
        panic!("expected ReactiveWrite on A; got {:?}", drained_a[0]);
    };
    assert_eq!(name_a, "A_only");
    let ScriptCommand::ReactiveWrite { name: name_b, .. } = &drained_b[0] else {
        panic!("expected ReactiveWrite on B; got {:?}", drained_b[0]);
    };
    assert_eq!(name_b, "B_only");
}

#[test]
fn unrepresentable_value_rejected() {
    // The closure's value-lowering MUST reject the slice-1
    // unsupported set (Symbol, BigInt, Function, Promise, etc.)
    // the same way `execute` itself does. Passing a function as
    // the value argument raises a JS-side TypeError; the sender
    // does NOT receive a command.
    let sender = Arc::new(BufferingSender::new());
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(sender.clone())
        .expect("sender installs");
    let err = rt
        .execute("__chromaticityEmit('fn_value', function() { return 1; })")
        .expect_err("unrepresentable value must reject");
    assert!(
        matches!(err, ScriptError::Runtime(_)),
        "unrepresentable value must surface as Runtime (TypeError), got {err:?}",
    );
    assert!(
        sender.drain().is_empty(),
        "rejected emit must not enqueue a command",
    );
}

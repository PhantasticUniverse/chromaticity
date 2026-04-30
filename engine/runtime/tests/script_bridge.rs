// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: (c) 2026 Xavier Butler

//! Evidence: EV-SCRIPT-CONTRACT (T2) — slice-5.3 engine/runtime
//! consumer-side adapter corpus.
//!
//! Widens `docs/26-evidence-artifact-registry.md` row 29
//! (`EV-SCRIPT-CONTRACT`) past the slice 5.1 + 5.2 producer-side
//! evidence with the slice-5.3 engine/runtime adapter that bridges
//! [`chromaticity_script::ScriptCommandSender`] onto a runtime-owned
//! bounded `crossbeam-channel` consumer.
//!
//! T2 per `docs/43-development-standards.md` §15.5: end-to-end
//! corpus exercising the adapter across the producer / consumer
//! seam. The fixtures wire a real
//! [`chromaticity_script::ScriptRuntime`] via the slice-5.1
//! `with_command_sender` builder against an
//! [`chromaticity_runtime::ScriptCommandRouter`] and assert the
//! [`chromaticity_runtime::ScriptBridgeReceiver`] sees the expected
//! [`chromaticity_script::ScriptCommand`] sequence.
//!
//! Slice-5.3 corpus per ADR-001 (runtime model) + ADR-006 (thread
//! and process model) + ADR-012 (script sandbox engine):
//!
//! - [`script_emit_lands_on_runtime_drain`] — a single JS-side
//!   `__chromaticityEmit` flows through the router and lands on
//!   the runtime drain unchanged.
//! - [`drain_observes_script_emit_order`] — multiple JS-side emits
//!   land in script-issued FIFO order.
//! - [`typed_route_lands_on_runtime_drain`] — the slice-5.2
//!   `ScriptCommand::route` typed conversion composes with the
//!   slice-5.3 bridge: a string-keyed emit can be routed to a
//!   typed `TypedReactiveWrite` and forwarded onto the drain.
//! - [`disconnected_drain_lifts_to_typed_script_error`] — dropping
//!   the runtime drain mid-script causes the next JS-side emit to
//!   raise `ScriptError::CommandBusDisconnected` via the slice-5.1
//!   canonical-message remap.
//! - [`backpressure_lifts_to_typed_script_error`] — filling a
//!   bounded bridge causes the next JS-side emit to raise
//!   `ScriptError::CommandBusBackpressure` via the same remap.
//! - [`empty_drain_distinguishes_alive_and_disconnected`] — the
//!   receiver-side `try_recv` distinguishes alive-but-empty from
//!   the dropped-router state.
//! - [`script_caught_disconnected_throw`] — a script that catches
//!   the host-disconnected throw observes the canonical message
//!   verbatim on `e.message`; the Rust-side `execute` returns
//!   the script's caught result, NOT a `ScriptError`.
//!
//! Anchors:
//! - `docs/adr/ADR-001-runtime-model.md` — runtime model.
//! - `docs/adr/ADR-003-script-runtime-sandbox.md` — script sandbox
//!   architectural baseline (engine choice + budgets superseded by
//!   ADR-012).
//! - `docs/adr/ADR-006-thread-and-process-model.md` — thread + process model.
//! - `docs/adr/ADR-012-script-sandbox-engine.md` — engine choice + budgets.
//! - `docs/26-evidence-artifact-registry.md` row 29 — `EV-SCRIPT-CONTRACT`.
//! - `docs/36-reactive-symbol-catalog.md` — reactive symbol catalog.
//! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
#![cfg(feature = "script-bridge")]

use std::sync::Arc;

use chromaticity_runtime::{ScriptBridgeError, script_bridge};
use chromaticity_script::{ReactiveSymbol, ScriptCommand, ScriptError, ScriptRuntime, ScriptValue};

/// Standard bridge capacity for fixtures that don't intentionally
/// stress the bound. Mirrors the slice-2
/// `chromaticity_runtime::CrossbeamChannel::CAPACITY` so the
/// adapter stays calibrated against the runtime bus's depth.
const TEST_CAPACITY: usize = 64;

#[test]
fn script_emit_lands_on_runtime_drain() {
    let (router, drain) = script_bridge(TEST_CAPACITY);
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(Arc::new(router))
        .expect("router installs as sender");
    let _ = rt
        .execute("__chromaticityEmit('syn_BPM', 120)")
        .expect("emit must succeed");
    let observed = drain.drain();
    assert_eq!(observed.len(), 1);
    let ScriptCommand::ReactiveWrite { name, value } = &observed[0] else {
        panic!("expected ReactiveWrite; got {:?}", observed[0]);
    };
    assert_eq!(name, "syn_BPM");
    assert_eq!(value, &ScriptValue::Number(120.0));
}

#[test]
fn drain_observes_script_emit_order() {
    // Five consecutive emits. The drain MUST observe them in the
    // order the script issued them — crossbeam_channel's MPSC FIFO
    // guarantee carries through.
    let (router, drain) = script_bridge(TEST_CAPACITY);
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(Arc::new(router))
        .expect("router installs");
    let _ = rt
        .execute(
            r"
            __chromaticityEmit('a', 0);
            __chromaticityEmit('b', 1);
            __chromaticityEmit('c', 2);
            __chromaticityEmit('d', 3);
            __chromaticityEmit('e', 4);
        ",
        )
        .expect("five emits must succeed");
    let observed = drain.drain();
    let names: Vec<String> = observed
        .into_iter()
        .map(|c| match c {
            ScriptCommand::ReactiveWrite { name, .. } => name,
            ScriptCommand::TypedReactiveWrite { symbol, .. } => symbol.name().to_string(),
            // `ScriptCommand` is `#[non_exhaustive]`; future
            // additive variants would surface as a stable
            // placeholder string so the order pin keeps
            // structural meaning.
            _ => "<unknown>".to_string(),
        })
        .collect();
    assert_eq!(names, vec!["a", "b", "c", "d", "e"]);
}

#[test]
fn typed_route_lands_on_runtime_drain() {
    // Slice-5.2 `ScriptCommand::route` composes with slice-5.3 by
    // sitting on the consumer side of the bridge. The script emits
    // a slice-5.1 string-keyed `ReactiveWrite`; the bridge forwards
    // it; the host applies `route()` and observes a typed
    // `TypedReactiveWrite { symbol: ReactiveSymbol::Bpm, .. }`.
    // This pin documents the supported composition order.
    let (router, drain) = script_bridge(TEST_CAPACITY);
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(Arc::new(router))
        .expect("router installs");
    let _ = rt
        .execute("__chromaticityEmit('syn_BPM', 120)")
        .expect("emit must succeed");
    let observed = drain.drain();
    assert_eq!(observed.len(), 1);
    // The bridge is value-transparent: it forwards the string-keyed
    // `ReactiveWrite` as-is; the consumer applies `route()`.
    let routed = observed[0].clone().route().expect("syn_BPM resolves");
    match routed {
        ScriptCommand::TypedReactiveWrite { symbol, value } => {
            assert_eq!(symbol, ReactiveSymbol::Bpm);
            assert_eq!(value, ScriptValue::Number(120.0));
        }
        // `ScriptCommand` is `#[non_exhaustive]`; the wildcard arm
        // covers any future additive variant on top of the slice
        // 5.1 + 5.2 set.
        other => panic!("expected TypedReactiveWrite; got {other:?}"),
    }
}

#[test]
fn disconnected_drain_lifts_to_typed_script_error() {
    // Dropping the runtime drain MUST lift to
    // `ScriptError::CommandBusDisconnected` via slice-5.1's
    // post-execute canonical-message remap. This is the end-to-end
    // pin tying the slice-5.3 adapter's error mapping
    // (`TrySendError::Disconnected` → `ScriptCommandError::Disconnected`)
    // through the slice-5.1 closure-side throw and the post-execute
    // remap.
    let (router, drain) = script_bridge(TEST_CAPACITY);
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(Arc::new(router))
        .expect("router installs");
    drop(drain);
    let err = rt
        .execute("__chromaticityEmit('syn_BPM', 120)")
        .expect_err("script-uncaught throw must lift to typed error");
    assert!(
        matches!(err, ScriptError::CommandBusDisconnected),
        "expected CommandBusDisconnected; got {err:?}"
    );
}

#[test]
fn backpressure_lifts_to_typed_script_error() {
    // Fill the bridge to capacity, then issue one more emit. The
    // last emit MUST surface as `ScriptError::CommandBusBackpressure`
    // via the same remap chain. Capacity = 2 keeps the test cheap;
    // the DoS-surface invariant is the same regardless of bound.
    const CAP: usize = 2;
    let (router, _drain) = script_bridge(CAP);
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(Arc::new(router))
        .expect("router installs");
    let err = rt
        .execute(
            r"
            __chromaticityEmit('a', 0);
            __chromaticityEmit('b', 1);
            __chromaticityEmit('c', 2);
        ",
        )
        .expect_err("third emit (over cap=2) must lift to typed error");
    assert!(
        matches!(err, ScriptError::CommandBusBackpressure),
        "expected CommandBusBackpressure; got {err:?}"
    );
}

#[test]
fn empty_drain_distinguishes_alive_and_disconnected() {
    // The receiver-side `try_recv` must distinguish empty from
    // disconnected. The slice-5.1 canonical-message remap is one
    // axis; the consumer-side error type is the other. Both sides
    // need to see distinct shapes for the host's drain loop to
    // decide whether to retry or shut down.
    let (router, drain) = script_bridge(TEST_CAPACITY);
    // Alive + empty: Ok(None).
    assert_eq!(drain.try_recv(), Ok(None));
    drop(router);
    // Dropped + empty: Disconnected.
    assert_eq!(drain.try_recv(), Err(ScriptBridgeError::Disconnected));
}

#[test]
fn script_caught_disconnected_throw() {
    // A script that catches the host-disconnected throw observes
    // the canonical message verbatim on `e.message`; the Rust-side
    // `execute` returns the script's caught result (NOT a
    // ScriptError). This pin asserts the bridge composes correctly
    // with slice-5.1's "throw escapes if uncaught, surfaces as
    // value if caught" contract.
    let (router, drain) = script_bridge(TEST_CAPACITY);
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_command_sender(Arc::new(router))
        .expect("router installs");
    drop(drain);
    let result = rt
        .execute(
            r"
            try {
              __chromaticityEmit('x', 1);
              'unreachable'
            } catch (e) {
              e.message
            }
            ",
        )
        .expect("caught throw returns script value, not ScriptError");
    assert_eq!(
        result,
        ScriptValue::String("chromaticity command bus disconnected".to_string()),
    );
}

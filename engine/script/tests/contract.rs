// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-SCRIPT-CONTRACT (T2) — slice-1 contract corpus over the QuickJS scaffold.
//!
//! Advances `docs/26-evidence-artifact-registry.md` row 29
//! (`EV-SCRIPT-CONTRACT`) past `pending:artifact` toward Phase-2
//! exit. T2 per `docs/43-development-standards.md` §15.5: corpus
//! conformance over a fixture set with stable golden assertions.
//!
//! Slice-1 corpus (this file): the four foundational fixtures
//! enumerated in the slice brief —
//!
//! - `pure_function_deterministic` — same source, same output
//!   across two `execute` calls on a single runtime; pure
//!   ECMAScript reductions are deterministic. The fib(10) case
//!   is the slice-1 sentinel; widening to a determinism corpus
//!   per ADR-012 §5 (seeded RNG, frame-locked Date.now, host-API
//!   call-ordering index) is slice-4 scope.
//! - `array_round_trip` — `[1, 2, 3].map(x => x * 2)` lowers to
//!   a [`chromaticity_script::ScriptValue::Array`] preserving
//!   index order and numeric value.
//! - `object_round_trip` — `{ name: 'chromaticity', version: 1 }`
//!   lowers to a [`chromaticity_script::ScriptValue::Object`]
//!   preserving insertion order and key-value association.
//! - `multiple_runtimes_isolated` — mutating a global on one
//!   [`chromaticity_script::ScriptRuntime`] does not affect a
//!   sibling runtime; ADR-012 §6 isolate-per-runtime contract.
//!
//! Slice-1 corpus is deliberately compact — full
//! `EV-SCRIPT-CONTRACT` Phase-2-exit definition (host-API parity
//! matrix from CP-07, lifecycle hooks, event ordering) requires
//! successive slices. The row stays `Planned` in `docs/26` while
//! the harness path tightens; the row's `Generation Source` cell
//! cites this file as the starting point in the registry update
//! follow-up.
//!
//! Anchors:
//! - `docs/adr/ADR-003-script-runtime-sandbox.md` — script runtime sandbox.
//! - `docs/adr/ADR-012-script-sandbox-engine.md` — engine choice + budgets.
//! - `docs/26-evidence-artifact-registry.md` row 29 — `EV-SCRIPT-CONTRACT`.
//! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_script::{ScriptRuntime, ScriptValue};

#[test]
fn pure_function_deterministic() {
    // Recursive fib is a pure reduction over integer arithmetic.
    // Two evaluations on the same runtime must return the same
    // value (intra-runtime determinism); this is the precondition
    // for the slice-4+ inter-run determinism corpus that
    // `EV-DETERMINISM` will cover with seeded RNG / frame-locked
    // clock shims (ADR-012 §5).
    let source = r"
        (function fib(n) {
            return n < 2 ? n : fib(n - 1) + fib(n - 2);
        })(10)
    ";
    let rt = ScriptRuntime::new().expect("runtime constructs");
    let v1 = rt.execute(source).expect("fib(10) must succeed");
    let v2 = rt
        .execute(source)
        .expect("fib(10) must succeed (second pass)");
    assert_eq!(
        v1,
        ScriptValue::Number(55.0),
        "fib(10) must be 55; got {v1:?}"
    );
    assert_eq!(v1, v2, "second execution diverged: {v1:?} vs {v2:?}");
}

#[test]
fn array_round_trip() {
    let rt = ScriptRuntime::new().expect("runtime constructs");
    let v = rt
        .execute("[1, 2, 3].map(x => x * 2)")
        .expect("array map must succeed");
    let expected = ScriptValue::Array(vec![
        ScriptValue::Number(2.0),
        ScriptValue::Number(4.0),
        ScriptValue::Number(6.0),
    ]);
    assert_eq!(v, expected, "got {v:?}");
}

#[test]
fn object_round_trip() {
    let rt = ScriptRuntime::new().expect("runtime constructs");
    // Parenthesise: a bare `{ ... }` at statement-position parses
    // as a block, not an object literal. `({...})` forces the
    // expression form QuickJS evaluates as the eval result.
    let v = rt
        .execute("({ name: 'chromaticity', version: 1 })")
        .expect("object literal must succeed");
    // Insertion order is part of the contract — ECMAScript 2015
    // `[[OwnPropertyKeys]]` preserves creation order for plain
    // string keys, and `lower_object` projects through that
    // ordering. The test pins the order to `[name, version]`
    // matching the source.
    let expected = ScriptValue::Object(vec![
        (
            "name".to_string(),
            ScriptValue::String("chromaticity".to_string()),
        ),
        ("version".to_string(), ScriptValue::Number(1.0)),
    ]);
    assert_eq!(v, expected, "got {v:?}");
}

#[test]
fn multiple_runtimes_isolated() {
    // Two runtimes share no QuickJS heap state. Setting a global
    // on one must NOT leak to the other. ADR-012 §6: "QuickJS
    // runs in its own isolate per scene. No cross-scene memory
    // access."
    let rt_a = ScriptRuntime::new().expect("runtime A constructs");
    let rt_b = ScriptRuntime::new().expect("runtime B constructs");

    let a_set = rt_a
        .execute("globalThis.crossSceneTaint = 42; globalThis.crossSceneTaint")
        .expect("rt_a set must succeed");
    assert_eq!(a_set, ScriptValue::Number(42.0), "got {a_set:?}");

    let b_observed = rt_b
        .execute("typeof globalThis.crossSceneTaint")
        .expect("rt_b observation must succeed");
    assert_eq!(
        b_observed,
        ScriptValue::String("undefined".to_string()),
        "rt_b saw rt_a's global; got {b_observed:?}",
    );
}

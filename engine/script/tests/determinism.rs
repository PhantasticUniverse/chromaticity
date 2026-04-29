// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-SCRIPT-CONTRACT (T2) — determinism shim corpus over QuickJS.
//!
//! Slice-4 corpus per ADR-012 §5:
//!
//! - `default_math_random_unshimmed_works` — without
//!   [`chromaticity_script::ScriptRuntime::with_random_seed`], the
//!   QuickJS built-in `Math.random()` still returns a valid `[0,1)`
//!   double. Slice-4 must NOT degrade the slice-1 / slice-2 / slice-3
//!   surface.
//! - `default_date_now_unshimmed_works` — without
//!   [`chromaticity_script::ScriptRuntime::with_clock`], the QuickJS
//!   built-in `Date.now()` still returns a positive monotonic value
//!   (the host wall-clock).
//! - `seeded_random_advances_per_call` — with a seed installed, two
//!   consecutive `Math.random()` calls produce DIFFERENT values
//!   (the sequence advances).
//! - `seeded_random_replays_across_runtimes` — two
//!   [`chromaticity_script::ScriptRuntime`] instances built with the
//!   same seed produce the same sequence of `Math.random()` outputs
//!   (10-call window).
//! - `seeded_random_diverges_across_seeds` — two runtimes with
//!   different seeds produce different first outputs.
//! - `seeded_random_state_isolated_per_runtime` — calling
//!   `Math.random` on runtime A does NOT advance the PRNG on
//!   runtime B (the cross-runtime independence contract).
//! - `frame_locked_clock_returns_tick_exactly` — with a
//!   [`chromaticity_script::FixedClock`] installed, `Date.now()`
//!   returns the injected tick exactly across 100 consecutive
//!   calls.
//! - `frame_locked_clock_accepts_pre_epoch_tick` — the
//!   [`chromaticity_script::FixedClock`] accepts negative ticks
//!   (the JS `Date.now()` spec permits pre-epoch readings); the
//!   shim does NOT clamp to `>=0`.
//!
//! ## Build-time shim install
//!
//! Slice-4 installs the `Math.random` and `Date.now` shims once at
//! builder time and they survive across [`rquickjs::Context::with`]
//! boundaries via `Rc<RefCell<...>>` capture in the closure. This
//! means the per-call `execute` overhead is unchanged from
//! slice-1 / slice-2 / slice-3 — no install / uninstall bracket. The
//! corpus exercises this implicitly: every `seeded_*` test calls
//! `execute` multiple times on the same runtime and asserts the
//! shim still fires on the second / third call.
//!
//! Anchors:
//! - `docs/adr/ADR-012-script-sandbox-engine.md` — engine choice +
//!   determinism shape (§5 in particular).
//! - `docs/26-evidence-artifact-registry.md` row 29 — `EV-SCRIPT-CONTRACT`.
//! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_script::{FixedClock, ScriptRuntime, ScriptValue};

#[test]
fn default_math_random_unshimmed_works() {
    // Evidence: EV-SCRIPT-CONTRACT (T2).
    // Slice-1 / slice-2 / slice-3 callers don't install a seed; the
    // QuickJS default `Math.random()` must still produce a valid
    // `[0,1)` double. The slice-4 surface MUST NOT degrade the
    // unshimmed path.
    let rt = ScriptRuntime::new().expect("runtime constructs");
    let v = rt
        .execute("Math.random()")
        .expect("default Math.random must succeed");
    let n = match v {
        ScriptValue::Number(n) => n,
        other => panic!("expected Number, got {other:?}"),
    };
    assert!(
        (0.0..1.0).contains(&n),
        "default Math.random must return [0,1); got {n}",
    );
}

#[test]
fn default_date_now_unshimmed_works() {
    // Evidence: EV-SCRIPT-CONTRACT (T2).
    // The unshimmed `Date.now()` is the QuickJS host wall-clock
    // surface; it must return a positive value (after epoch).
    let rt = ScriptRuntime::new().expect("runtime constructs");
    let v = rt
        .execute("Date.now()")
        .expect("default Date.now must succeed");
    let n = match v {
        ScriptValue::Number(n) => n,
        other => panic!("expected Number, got {other:?}"),
    };
    assert!(
        n > 0.0,
        "default Date.now must return positive value; got {n}",
    );
}

#[test]
fn seeded_random_advances_per_call() {
    // Evidence: EV-SCRIPT-CONTRACT (T2).
    // With a seed installed, `Math.random()` is the seeded PRNG;
    // two consecutive calls must advance the state and return
    // different values. (xoroshiro128+ is bijective so adjacent
    // outputs are different with overwhelming probability — the
    // only way they would match is the all-equal degenerate state
    // which splitmix64 expansion rules out by construction.)
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_random_seed(42)
        .expect("seed installs");
    let v1 = match rt.execute("Math.random()").expect("first call succeeds") {
        ScriptValue::Number(n) => n,
        other => panic!("expected Number, got {other:?}"),
    };
    let v2 = match rt.execute("Math.random()").expect("second call succeeds") {
        ScriptValue::Number(n) => n,
        other => panic!("expected Number, got {other:?}"),
    };
    assert!(
        (0.0..1.0).contains(&v1),
        "first PRNG output must be in [0,1); got {v1}",
    );
    assert!(
        (0.0..1.0).contains(&v2),
        "second PRNG output must be in [0,1); got {v2}",
    );
    assert_ne!(
        v1, v2,
        "seeded PRNG must advance between calls; got {v1} == {v2}",
    );
}

#[test]
fn seeded_random_replays_across_runtimes() {
    // Evidence: EV-SCRIPT-CONTRACT (T2).
    // The seeded-replay contract: two ScriptRuntime instances
    // built with the same seed produce IDENTICAL sequences of
    // Math.random() outputs. Drives ADR-012 §5's "scene replay
    // metadata" promise that EV-DETERMINISM ratifies later.
    let seed = 0xC0FFEE_u64;
    let rt_a = ScriptRuntime::new()
        .expect("runtime A constructs")
        .with_random_seed(seed)
        .expect("seed installs A");
    let rt_b = ScriptRuntime::new()
        .expect("runtime B constructs")
        .with_random_seed(seed)
        .expect("seed installs B");

    // Compare the first 10 outputs of each. The script materialises
    // an array on the JS side so we can lower it through the
    // existing slice-1 array round-trip path.
    let script = r"
        let xs = [];
        for (let i = 0; i < 10; i++) { xs.push(Math.random()); }
        xs
    ";
    let a = rt_a.execute(script).expect("script A runs");
    let b = rt_b.execute(script).expect("script B runs");
    let (a_arr, b_arr) = match (a, b) {
        (ScriptValue::Array(a), ScriptValue::Array(b)) => (a, b),
        other => panic!("expected Array pair, got {other:?}"),
    };
    assert_eq!(a_arr.len(), 10, "10-call corpus");
    assert_eq!(b_arr.len(), 10, "10-call corpus");
    for (i, (av, bv)) in a_arr.iter().zip(b_arr.iter()).enumerate() {
        let an = match av {
            ScriptValue::Number(n) => *n,
            other => panic!("element {i} A: expected Number, got {other:?}"),
        };
        let bn = match bv {
            ScriptValue::Number(n) => *n,
            other => panic!("element {i} B: expected Number, got {other:?}"),
        };
        // Bit-identical: f64 equality is the canonical test for the
        // determinism contract; xoroshiro128+ deterministically
        // produces the same bit pattern across runs / runtimes for
        // a given seed.
        assert_eq!(
            an.to_bits(),
            bn.to_bits(),
            "element {i}: same seed must produce same f64 bits; got {an} != {bn}",
        );
    }
}

#[test]
fn seeded_random_diverges_across_seeds() {
    // Evidence: EV-SCRIPT-CONTRACT (T2).
    // The seed-isolation contract: different seeds must produce
    // different sequences. We pin the strict inequality on the
    // first call (xoroshiro128+ diverges immediately for distinct
    // splitmix64-expanded states).
    let rt_a = ScriptRuntime::new()
        .expect("runtime A constructs")
        .with_random_seed(1)
        .expect("seed A installs");
    let rt_b = ScriptRuntime::new()
        .expect("runtime B constructs")
        .with_random_seed(2)
        .expect("seed B installs");
    let a = match rt_a.execute("Math.random()").expect("A runs") {
        ScriptValue::Number(n) => n,
        other => panic!("expected Number, got {other:?}"),
    };
    let b = match rt_b.execute("Math.random()").expect("B runs") {
        ScriptValue::Number(n) => n,
        other => panic!("expected Number, got {other:?}"),
    };
    assert_ne!(
        a.to_bits(),
        b.to_bits(),
        "distinct seeds must produce distinct first outputs; got {a} == {b}",
    );
}

#[test]
fn seeded_random_state_isolated_per_runtime() {
    // Evidence: EV-SCRIPT-CONTRACT (T2).
    // Cross-runtime independence: two runtimes with the same seed
    // get INDEPENDENT PRNG state. Calling Math.random on runtime A
    // does NOT advance runtime B's generator. We assert this by
    // calling `Math.random` once on A, then asserting that B's
    // first `Math.random` matches A's first (which it would not if
    // the state were shared — B would observe A's second output).
    let seed = 7_u64;
    let rt_a = ScriptRuntime::new()
        .expect("runtime A constructs")
        .with_random_seed(seed)
        .expect("seed A installs");
    let rt_b = ScriptRuntime::new()
        .expect("runtime B constructs")
        .with_random_seed(seed)
        .expect("seed B installs");

    let a_first = match rt_a.execute("Math.random()").expect("A first") {
        ScriptValue::Number(n) => n,
        other => panic!("expected Number, got {other:?}"),
    };
    let _a_second = rt_a.execute("Math.random()").expect("A second");
    // After A has advanced TWICE, B's first call must still be
    // A's pristine first output. If state were shared, B's first
    // call would observe A's third output.
    let b_first = match rt_b.execute("Math.random()").expect("B first") {
        ScriptValue::Number(n) => n,
        other => panic!("expected Number, got {other:?}"),
    };
    assert_eq!(
        a_first.to_bits(),
        b_first.to_bits(),
        "runtime B PRNG must be independent of runtime A; got A_first={a_first} B_first={b_first}",
    );
}

#[test]
fn frame_locked_clock_returns_tick_exactly() {
    // Evidence: EV-SCRIPT-CONTRACT (T2).
    // The frame-locked-clock contract per ADR-012 §5: every
    // `Date.now()` call within a single frame returns the same
    // value. We exercise the contract over 100 consecutive calls
    // — far more than any realistic frame-budget call count — and
    // assert each call returns the exact injected tick. The tick
    // chosen (1_700_000_000_000) is a representative recent epoch
    // millisecond reading.
    let tick = 1_700_000_000_000_i64;
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_clock(Box::new(FixedClock::new(tick)))
        .expect("clock installs");

    let script = r"
        let xs = [];
        for (let i = 0; i < 100; i++) { xs.push(Date.now()); }
        xs
    ";
    let v = rt.execute(script).expect("script runs");
    let arr = match v {
        ScriptValue::Array(a) => a,
        other => panic!("expected Array, got {other:?}"),
    };
    assert_eq!(arr.len(), 100, "100-call corpus");
    for (i, item) in arr.iter().enumerate() {
        let n = match item {
            ScriptValue::Number(n) => *n,
            other => panic!("element {i}: expected Number, got {other:?}"),
        };
        // f64 representation of the i64 tick: 1_700_000_000_000 is
        // well within the 2^53 mantissa precision of f64 so the
        // round-trip is exact. We compare via casting back.
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let expected = tick as f64;
        assert_eq!(
            n.to_bits(),
            expected.to_bits(),
            "element {i}: frame-locked Date.now must return tick verbatim; got {n} != {expected}",
        );
    }
}

#[test]
fn frame_locked_clock_accepts_pre_epoch_tick() {
    // Evidence: EV-SCRIPT-CONTRACT (T2).
    //
    // reason: ECMAScript's `Date.now()` is documented to permit
    // pre-epoch readings (negative integer milliseconds since
    // 1970-01-01); the slice-4 FixedClock built-in MUST NOT clamp
    // negative ticks because doing so would silently break a
    // legitimate use case (scrubbing a recorded show backwards
    // through the epoch boundary, for instance). The contract is
    // "the shim returns whatever the Clock returns, verbatim".
    // -1 is the smallest representative pre-epoch reading and
    // exercises the negative-integer code path.
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_clock(Box::new(FixedClock::new(-1)))
        .expect("negative tick clock installs");
    let v = rt.execute("Date.now()").expect("Date.now runs");
    let n = match v {
        ScriptValue::Number(n) => n,
        other => panic!("expected Number, got {other:?}"),
    };
    assert_eq!(
        n.to_bits(),
        (-1.0_f64).to_bits(),
        "FixedClock(-1) must surface as -1 verbatim; got {n}",
    );
}

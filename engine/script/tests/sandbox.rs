// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-SANDBOX-SAFETY (T2) — watchdog timeout corpus over QuickJS.
//! Evidence: EV-SCRIPT-CONTRACT (T2) — host-side script-contract evidence for the watchdog surface.
//!
//! Opens `docs/26-evidence-artifact-registry.md` row 42
//! (`EV-SANDBOX-SAFETY`) past `pending:artifact`. T2 per
//! `docs/43-development-standards.md` §15.5: corpus conformance
//! over a fixture set with stable golden assertions.
//!
//! Slice-2 corpus (this file) per ADR-012 §2:
//!
//! - `tight_cpu_loop_times_out` — a CPU-bound integer-bumping loop
//!   must trip the 4 ms hard cap and surface as
//!   [`chromaticity_script::ScriptError::TimeoutExceeded`].
//! - `recursion_explosion_times_out` — a self-recursive function
//!   either trips the watchdog or hits the QuickJS stack-depth
//!   guard; either is acceptable sandbox-safety behaviour.
//! - `busy_polling_loop_times_out` — `Date.now()`-based spin loop
//!   trips the watchdog (Date.now's determinism is slice-4 scope;
//!   slice-2 just observes that wall-clock-polling JS code can't
//!   evade the interrupt handler).
//! - `multiple_runtimes_independent_watchdogs` — a watchdog mounted
//!   on runtime A does NOT spuriously trip a watchdog-free runtime
//!   B; each runtime owns its own interrupt-handler slot per
//!   ADR-012 §6 isolate-per-runtime.
//!
//! ## CI runner clock variance
//!
//! Watchdog observability is inherently wall-clock-coupled and
//! shared CI runners exhibit 50–100 ms scheduling jitter. The
//! slice-2 corpus uses a generous tolerance — `observed_ms` between
//! `hard_ms` and `hard_ms + 200 ms` is considered conformant. Tests
//! that need the soft-cap signal use even wider budgets to keep
//! the contract orthogonal to the runner's perf curve.
//!
//! Anchors:
//! - `docs/adr/ADR-012-script-sandbox-engine.md` — engine choice + budgets.
//! - `docs/26-evidence-artifact-registry.md` row 42 — `EV-SANDBOX-SAFETY`.
//! - `docs/26-evidence-artifact-registry.md` row 29 — `EV-SCRIPT-CONTRACT`.
//! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_script::{ScriptError, ScriptRuntime, ScriptValue};

/// Slice-2 default budget pair from ADR-012 §2 (1 ms p99 / 4 ms hard).
const P99_MS: u64 = 1;
const HARD_MS: u64 = 4;
/// Generous CI-runner tolerance per the file-level docs above.
const RUNNER_TOLERANCE_MS: u64 = 200;

#[test]
fn tight_cpu_loop_times_out() {
    // CPU-bound integer-bumping loop; the bumps are dead code from
    // QuickJS's perspective but the interpreter still has to step
    // through them, giving the interrupt handler a poll point.
    // 1e8 iterations is far more than the 4 ms hard cap allows;
    // the watchdog must fire long before the script returns.
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_watchdog(P99_MS, HARD_MS)
        .expect("watchdog config valid");
    let err = rt
        .execute("let x = 0; while (x < 1e10) { x = x + 1; }")
        .expect_err("tight CPU loop must trip hard cap");
    let (hard_ms, observed_ms) = match err {
        ScriptError::TimeoutExceeded {
            hard_ms,
            observed_ms,
            ..
        } => (hard_ms, observed_ms),
        other => panic!("expected TimeoutExceeded, got {other:?}"),
    };
    assert!(
        observed_ms >= hard_ms,
        "observed_ms ({observed_ms}) must be >= hard_ms ({hard_ms})",
    );
    assert!(
        observed_ms < hard_ms + RUNNER_TOLERANCE_MS,
        "observed_ms ({observed_ms}) must be within hard_ms + {RUNNER_TOLERANCE_MS} ms tolerance \
         (hard_ms={hard_ms}); a wider gap suggests the interrupt handler is not polling",
    );
    let obs = rt.last_observation().expect("watchdog records observation");
    assert!(obs.hard_cap_exceeded, "hard_cap_exceeded must be set");
}

#[test]
fn recursion_explosion_times_out() {
    // Mirror of the T1 drift pin in the corpus surface. Either
    // TimeoutExceeded or a stack-overflow Runtime is an acceptable
    // sandbox-safety outcome; the contract is "the runtime does
    // not return Ok from a runaway recursion".
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_watchdog(P99_MS, HARD_MS)
        .expect("watchdog config valid");
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
            // QuickJS RangeError("Maximum call stack size exceeded")
            // path; sandbox boundary held even if the watchdog
            // didn't get to fire first.
            assert!(
                !msg.is_empty(),
                "stack-overflow path must surface a non-empty diagnostic",
            );
        }
        other => panic!("expected TimeoutExceeded or Runtime stack overflow, got {other:?}",),
    }
}

#[test]
fn busy_polling_loop_times_out() {
    // `Date.now()` is a QuickJS built-in; the determinism shim
    // (slice 4) is not yet installed, so this is just wall-clock
    // polling. The watchdog fires regardless because
    // `Date.now() - start < 1000` keeps the JS interpreter inside
    // the QuickJS stepper, where the interrupt handler polls.
    let rt = ScriptRuntime::new()
        .expect("runtime constructs")
        .with_watchdog(P99_MS, HARD_MS)
        .expect("watchdog config valid");
    let err = rt
        .execute(
            r"
            let start = Date.now();
            while (Date.now() - start < 1000) {
                let _ = 1;
            }
        ",
        )
        .expect_err("busy-polling loop must trip hard cap");
    let observed_ms = match err {
        ScriptError::TimeoutExceeded {
            hard_ms,
            observed_ms,
            ..
        } => {
            assert!(
                observed_ms >= hard_ms,
                "observed_ms ({observed_ms}) >= hard_ms ({hard_ms})",
            );
            observed_ms
        }
        other => panic!("expected TimeoutExceeded, got {other:?}"),
    };
    assert!(
        observed_ms < HARD_MS + RUNNER_TOLERANCE_MS,
        "observed_ms ({observed_ms}) must be within hard_ms + {RUNNER_TOLERANCE_MS} ms tolerance",
    );
}

#[test]
fn multiple_runtimes_independent_watchdogs() {
    // Two runtimes — one with a watchdog, one without. Mutating
    // / running on either must not affect the other's
    // interrupt-handler slot. ADR-012 §6: "QuickJS runs in its
    // own isolate per scene. No cross-scene memory access."
    let rt_guarded = ScriptRuntime::new()
        .expect("guarded runtime constructs")
        .with_watchdog(P99_MS, HARD_MS)
        .expect("watchdog config valid");
    let rt_unguarded = ScriptRuntime::new().expect("unguarded runtime constructs");

    // First, prove the guarded runtime trips its own watchdog.
    let err = rt_guarded
        .execute("while (true) { let _ = 1; }")
        .expect_err("guarded runtime must trip");
    assert!(
        matches!(err, ScriptError::TimeoutExceeded { .. }),
        "guarded runtime must surface TimeoutExceeded, got {err:?}",
    );

    // Then, prove the unguarded runtime is NOT spuriously
    // interrupted on a fast script. If the watchdog leaked across
    // runtimes, this would either trip immediately or surface a
    // weird error. A trivial expression must succeed cleanly.
    let v = rt_unguarded
        .execute("21 * 2")
        .expect("unguarded runtime must succeed");
    assert_eq!(v, ScriptValue::Number(42.0), "got {v:?}");
    assert!(
        rt_unguarded.last_observation().is_none(),
        "unguarded runtime must not record any watchdog observation",
    );

    // Also re-run a fast script on the guarded runtime to confirm
    // the watchdog detached cleanly after the previous trip — a
    // leftover deadline from the timed-out call would
    // immediately fail this fresh call.
    let v2 = rt_guarded
        .execute("3 + 4")
        .expect("guarded runtime must succeed after a prior trip");
    assert_eq!(v2, ScriptValue::Number(7.0), "got {v2:?}");
}

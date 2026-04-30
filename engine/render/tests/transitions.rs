// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-RENDER-CONFORMANCE (T2) — slice-5b.1 warm-switch
//! lifecycle corpus.
//!
//! Advances `docs/26-evidence-artifact-registry.md` row 28
//! (`EV-RENDER-CONFORMANCE`) by exercising the
//! [`chromaticity_render::TransitionOrchestrator`] state machine
//! end-to-end against the four-phase ADR-007 lifecycle
//! (`PrepareNext` / `PrimeNext` / `CrossfadeOrHardCut` /
//! `CommitNext`). Slice-5b.1's T2 corpus is GPU-FREE — the
//! orchestrator is a pure-data state machine and does not touch
//! wgpu (see `engine/render/src/transitions.rs` module-level doc:
//! "The orchestrator is intentionally GPU-DEVICE-FREE in
//! slice-5b.1"); the actual visual transition composition + GPU
//! resource preallocation are slice-5b.2 scope.
//!
//! The fixtures here pin lifecycle invariants that span MULTIPLE
//! state advances (multi-step happy-path scene swap, abort
//! preserves current scene, repeated-cycle determinism), the
//! complement of the T1 single-step pins inline in
//! `transitions::transitions_pins`. The T1 / T2 split mirrors the
//! pattern established by slice-2's `multipass::multipass_drift_pins`
//! T1 + `engine/render/tests/multipass.rs` T2.
//!
//! T2 per `docs/43-development-standards.md` §15.5: lifecycle-
//! corpus invariants for the transition state machine. Because
//! slice-5b.1 stops short of GPU rendering, the corpus assertions
//! are state-machine invariants rather than byte-hash readbacks; a
//! T2 byte-hash corpus over the rendered crossfade output joins in
//! slice-5b.2 alongside the actual frame-composition wiring.
//!
//! Anchors:
//! - `engine/render/src/transitions.rs` — orchestrator implementation.
//! - `docs/adr/ADR-007-scene-warm-switch-policy.md` — Accepted
//!   four-phase lifecycle and the "preserve current output with
//!   diagnostics" + "default 500 ms timeout" decisions.
//! - `docs/26-evidence-artifact-registry.md` row 28 —
//!   `EV-RENDER-CONFORMANCE`.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_render::{
    MultipassSpec, PassDef, TransitionError, TransitionOrchestrator, TransitionState,
};

/// Build the smallest spec the orchestrator's shape gate accepts.
/// The fragment shader is the trivial UV-passthrough; we never
/// reach a GPU at slice-5b.1, so any shape-valid spec works.
fn spec_with_label(label: &str) -> MultipassSpec {
    MultipassSpec {
        passes: vec![PassDef {
            label: Some(label.to_string()),
            fragment_glsl: "#version 450\nlayout(location=0) in vec2 uv;\nlayout(location=0) out vec4 fragColor;\nvoid main(){fragColor=vec4(uv,0.0,1.0);}\n".to_string(),
            ..PassDef::default()
        }],
        target_size: (16, 16),
        frame_count: 1,
    }
}

#[test]
fn lifecycle_swaps_current_to_next_at_commit() {
    // T2 invariant: the cued scene becomes current AT commit, not
    // before. The ADR-007 "keep current scene active until
    // PrepareNext + PrimeNext succeed" decision is what makes this
    // the slice-5b.1 contract — the live output stays the original
    // scene through PrepareNext, PrimeNext, AND CrossfadeOrHardCut
    // (slice-5b.2 will composite the two scenes during the third
    // phase; slice-5b.1's contract is just that current_spec()
    // returns the original through all three).
    let mut orch = TransitionOrchestrator::new(spec_with_label("scene-A"));
    assert_eq!(
        orch.current_spec().passes[0].label.as_deref(),
        Some("scene-A")
    );

    orch.cue_next(spec_with_label("scene-B")).unwrap();
    assert_eq!(
        orch.current_spec().passes[0].label.as_deref(),
        Some("scene-A")
    );
    assert_eq!(
        orch.next_spec().unwrap().passes[0].label.as_deref(),
        Some("scene-B")
    );

    orch.mark_primed().unwrap();
    assert_eq!(
        orch.current_spec().passes[0].label.as_deref(),
        Some("scene-A")
    );

    orch.begin_crossfade().unwrap();
    assert_eq!(
        orch.current_spec().passes[0].label.as_deref(),
        Some("scene-A")
    );

    orch.commit_next().unwrap();
    assert_eq!(
        orch.current_spec().passes[0].label.as_deref(),
        Some("scene-B")
    );
    assert!(orch.next_spec().is_none());
    assert_eq!(orch.current_state(), TransitionState::Idle);
}

#[test]
fn repeated_lifecycle_cycles_are_deterministic() {
    // T2 invariant: running multiple sequential transitions
    // produces the expected scene-name walk. Pins the slice-5b.1
    // contract that the orchestrator is fully reusable across
    // multiple transitions without internal-state drift.
    let mut orch = TransitionOrchestrator::new(spec_with_label("scene-0"));
    for i in 1..=4u32 {
        let next_label = format!("scene-{i}");
        orch.cue_next(spec_with_label(&next_label)).unwrap();
        orch.mark_primed().unwrap();
        orch.begin_crossfade().unwrap();
        orch.commit_next().unwrap();
        assert_eq!(orch.current_state(), TransitionState::Idle);
        assert_eq!(
            orch.current_spec().passes[0].label.as_deref(),
            Some(next_label.as_str()),
        );
        assert!(orch.next_spec().is_none());
    }
}

#[test]
fn abort_preserves_current_scene_at_every_in_flight_phase() {
    // T2 invariant: the ADR-007 "preserve current output with
    // diagnostics" decision means abort from any in-flight phase
    // leaves current unchanged and clears the cued next. The T1
    // pin covers the same shape via a parameterised loop; this
    // T2 pin walks the exhaustive set explicitly so a future
    // refactor that forgets one phase is loud.
    let phases = [
        ("PrepareNext", TransitionState::PrepareNext),
        ("PrimeNext", TransitionState::PrimeNext),
        ("CrossfadeOrHardCut", TransitionState::CrossfadeOrHardCut),
    ];
    for (name, target_phase) in phases {
        let mut orch = TransitionOrchestrator::new(spec_with_label("preserved"));
        orch.cue_next(spec_with_label("aborted")).unwrap();
        if target_phase == TransitionState::PrimeNext
            || target_phase == TransitionState::CrossfadeOrHardCut
        {
            orch.mark_primed().unwrap();
        }
        if target_phase == TransitionState::CrossfadeOrHardCut {
            orch.begin_crossfade().unwrap();
        }
        assert_eq!(
            orch.current_state(),
            target_phase,
            "setup for phase {name} did not land in {name}",
        );

        orch.abort().unwrap_or_else(|err| {
            panic!("abort from {name} must succeed, got {err:?}");
        });

        assert_eq!(orch.current_state(), TransitionState::Idle);
        assert!(orch.next_spec().is_none());
        assert_eq!(
            orch.current_spec().passes[0].label.as_deref(),
            Some("preserved"),
            "abort from {name} dropped the current scene",
        );
    }
}

#[test]
fn cue_next_during_in_flight_does_not_clobber() {
    // T2 invariant: a re-cue while a transition is in flight is
    // rejected without disturbing the existing in-flight transition.
    // This is the slice-5b.1 contract — slice-5b.2 may add a
    // re-cue-replaces-pending mode under the timeout-degrades-to-
    // hard-cut path, but slice-5b.1 keeps the first cue authoritative.
    let mut orch = TransitionOrchestrator::new(spec_with_label("initial"));
    orch.cue_next(spec_with_label("first-cue")).unwrap();
    assert_eq!(orch.current_state(), TransitionState::PrepareNext);

    let err = orch
        .cue_next(spec_with_label("second-cue"))
        .expect_err("re-cue while in-flight must reject; the first cue stays authoritative");
    match err {
        TransitionError::InvalidStateTransition { from, attempted } => {
            assert_eq!(from, TransitionState::PrepareNext);
            assert_eq!(attempted, "cue_next");
        }
        other => panic!("expected InvalidStateTransition, got {other:?}"),
    }
    // First cue still in flight; second cue dropped on the floor.
    assert_eq!(orch.current_state(), TransitionState::PrepareNext);
    assert_eq!(
        orch.next_spec().unwrap().passes[0].label.as_deref(),
        Some("first-cue"),
    );
}

#[test]
fn abort_then_recue_resumes_normal_lifecycle() {
    // T2 invariant: after an abort the orchestrator is reusable —
    // a fresh cue_next succeeds and walks through the lifecycle to
    // commit. Pins that abort fully cleans up state rather than
    // leaving residue that would block the next transition.
    let mut orch = TransitionOrchestrator::new(spec_with_label("initial"));
    orch.cue_next(spec_with_label("aborted-cue")).unwrap();
    orch.mark_primed().unwrap();
    // Abort from PrimeNext.
    orch.abort().unwrap();
    assert_eq!(orch.current_state(), TransitionState::Idle);

    // Now cue a fresh transition — must succeed and reach commit.
    orch.cue_next(spec_with_label("fresh-cue")).unwrap();
    orch.mark_primed().unwrap();
    orch.begin_crossfade().unwrap();
    orch.commit_next().unwrap();
    assert_eq!(orch.current_state(), TransitionState::Idle);
    assert_eq!(
        orch.current_spec().passes[0].label.as_deref(),
        Some("fresh-cue"),
    );
}

#[test]
fn rejected_cue_leaves_orchestrator_unchanged() {
    // T2 invariant: a cue_next call that fails the shape gate does
    // NOT advance state. Pinning this via a separate test (versus
    // the T1 single-shot pins) catches regressions where a future
    // refactor advances state before validating.
    let mut orch = TransitionOrchestrator::new(spec_with_label("initial"));

    // Bad spec 1: empty pass list.
    let bad_empty = MultipassSpec {
        passes: vec![],
        target_size: (16, 16),
        frame_count: 1,
    };
    assert!(orch.cue_next(bad_empty).is_err());
    assert_eq!(orch.current_state(), TransitionState::Idle);
    assert!(orch.next_spec().is_none());

    // Bad spec 2: zero target size.
    let mut bad_size = spec_with_label("bad-size");
    bad_size.target_size = (16, 0);
    assert!(orch.cue_next(bad_size).is_err());
    assert_eq!(orch.current_state(), TransitionState::Idle);
    assert!(orch.next_spec().is_none());

    // Now a good cue must still succeed — the rejected attempts
    // did not poison state.
    orch.cue_next(spec_with_label("good-cue")).unwrap();
    assert_eq!(orch.current_state(), TransitionState::PrepareNext);
}

#[test]
fn out_of_order_advances_reject_with_observed_from_state() {
    // T2 invariant: every illegal advance reports the OBSERVED state
    // it was rejected from. This is the diagnostic contract — a
    // log-grep on the `from` field tells operators where in the
    // lifecycle the rejection happened.
    let mut orch = TransitionOrchestrator::new(spec_with_label("initial"));

    // mark_primed from Idle.
    let err = orch
        .mark_primed()
        .expect_err("mark_primed from Idle rejects");
    if let TransitionError::InvalidStateTransition { from, attempted } = err {
        assert_eq!(from, TransitionState::Idle);
        assert_eq!(attempted, "mark_primed");
    } else {
        panic!("expected InvalidStateTransition");
    }

    // begin_crossfade from Idle.
    let err = orch
        .begin_crossfade()
        .expect_err("begin_crossfade from Idle rejects");
    if let TransitionError::InvalidStateTransition { from, attempted } = err {
        assert_eq!(from, TransitionState::Idle);
        assert_eq!(attempted, "begin_crossfade");
    } else {
        panic!("expected InvalidStateTransition");
    }

    // commit_next from Idle.
    let err = orch
        .commit_next()
        .expect_err("commit_next from Idle rejects");
    if let TransitionError::InvalidStateTransition { from, attempted } = err {
        assert_eq!(from, TransitionState::Idle);
        assert_eq!(attempted, "commit_next");
    } else {
        panic!("expected InvalidStateTransition");
    }

    // Move to PrepareNext, then the same set rejects with that
    // `from` state.
    orch.cue_next(spec_with_label("next")).unwrap();
    let err = orch
        .begin_crossfade()
        .expect_err("begin_crossfade from PrepareNext rejects");
    if let TransitionError::InvalidStateTransition { from, attempted } = err {
        assert_eq!(from, TransitionState::PrepareNext);
        assert_eq!(attempted, "begin_crossfade");
    } else {
        panic!("expected InvalidStateTransition");
    }
}

#[test]
fn next_spec_visibility_matches_lifecycle_phase() {
    // T2 invariant: `next_spec()` is `Some` exactly during the
    // three in-flight phases (PrepareNext / PrimeNext /
    // CrossfadeOrHardCut), and `None` in Idle. The doc-comment on
    // `TransitionOrchestrator::next_spec` declares this; the test
    // pins it so a future refactor that exposes next_spec() too
    // early (or fails to clear it on commit) is loud.
    let mut orch = TransitionOrchestrator::new(spec_with_label("initial"));
    assert!(orch.next_spec().is_none(), "Idle must hide next_spec");

    orch.cue_next(spec_with_label("next")).unwrap();
    assert!(
        orch.next_spec().is_some(),
        "PrepareNext must expose next_spec",
    );

    orch.mark_primed().unwrap();
    assert!(
        orch.next_spec().is_some(),
        "PrimeNext must expose next_spec",
    );

    orch.begin_crossfade().unwrap();
    assert!(
        orch.next_spec().is_some(),
        "CrossfadeOrHardCut must expose next_spec",
    );

    orch.commit_next().unwrap();
    assert!(
        orch.next_spec().is_none(),
        "post-commit Idle must hide next_spec",
    );
}

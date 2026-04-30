// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-2 EV-RENDER-CONFORMANCE slice-5b.1 transition-orchestration
//! lifecycle (ADR-007 warm-switch policy first half).
//!
//! # ADR-007 four-phase warm-switch
//!
//! `docs/adr/ADR-007-scene-warm-switch-policy.md` (Accepted) defines
//! the normative four-phase lifecycle:
//!
//! 1. `PrepareNext` — begin preparing the next scene's resources;
//!    current scene stays the live output.
//! 2. `PrimeNext` — preparation has completed and the next scene's
//!    resources are PRIMED (textures allocated, pipeline cache
//!    warmed, first-frame latency hidden); current scene still owns
//!    output. The ADR's "keep current scene active until `PrepareNext`
//!    and `PrimeNext` succeed" invariant lives here.
//! 3. `CrossfadeOrHardCut` — the two scenes execute together long
//!    enough for the visual transition (crossfade composite or
//!    instantaneous hard-cut). The actual frame composition is
//!    slice-5b.2 scope.
//! 4. `CommitNext` — the next scene takes over as the live scene;
//!    the prior scene's resources are released. The transition
//!    itself completes and the orchestrator returns to a quiescent
//!    state owning ONE scene.
//!
//! The orchestrator additionally exposes [`TransitionState::Idle`] —
//! a scene is live, no transition is pending. `Idle` is NOT one of
//! the four ADR phases; it is the well-defined "between transitions"
//! state, which the four-phase lifecycle is silent about because the
//! ADR is scoped to the lifecycle of a single transition. Without an
//! explicit `Idle`, callers would have to model "no transition
//! pending" as `Option<TransitionState>` at every accessor — the
//! enum collapses that ergonomically. The `#[non_exhaustive]`
//! attribute keeps the surface free for slice-5b.2 to add additional
//! states (e.g., `Aborted` if the ADR's "preparation failure → abort"
//! path needs an explicit terminal state, currently modeled by
//! `prepare_next` returning the orchestrator unchanged with the
//! error surfaced).
//!
//! # Slice-5b.1 vs slice-5b.2 split
//!
//! Slice-5b.1 (this module) lands the LIFECYCLE STATE MACHINE plus
//! the SPEC-LEVEL preallocation invariants:
//!
//! - The `TransitionState` enum and its legal transition matrix.
//! - The [`TransitionOrchestrator`] driver type owning the current
//!   and (optionally) next [`crate::multipass::MultipassSpec`].
//! - Spec-level validation of the cued scene before phase advance:
//!   non-empty pass list, non-zero target size, non-zero frame
//!   count. The full structural validation (forward-reference,
//!   feedback-alias, float-target adapter capability) lives inside
//!   [`crate::multipass::compile_and_render_multipass`] and runs at
//!   the ACTUAL render call; the orchestrator's preallocation gate
//!   is the lighter SHAPE check that catches obviously-bad cued
//!   specs before the renderer is reached.
//! - Resource-preallocation HOOKS — the orchestrator records the
//!   intent to preallocate but does not own a wgpu device handle
//!   (see "Threading + GPU ownership" below). Slice-5b.2 wires the
//!   actual GPU resource allocation in.
//!
//! Slice-5b.2 (deferred) lands the CROSSFADE COMPOSITION:
//!
//! - Per-frame blending of the two scenes' outputs during
//!   [`TransitionState::CrossfadeOrHardCut`].
//! - The 500 ms ADR-007 default timeout + hard-cut fallback path.
//! - Wgpu device / texture preallocation actually populating the
//!   `next_spec`'s GPU handles.
//!
//! # Threading + GPU ownership
//!
//! The orchestrator is intentionally GPU-DEVICE-FREE in slice-5b.1.
//! It is a pure-data state machine: every public method takes
//! `&mut self` and runs synchronously without touching wgpu. This
//! mirrors the "render scheduler stays scene-agnostic" invariant
//! preserved in `multipass.rs` (the scheduler accepts plain-data
//! [`crate::multipass::MultipassSpec`] and acquires its own device
//! per call; cf. `multipass.rs` lines 14-26 and 431-456).
//!
//! Consequences for slice-5b.1:
//!
//! - The orchestrator can be unit-tested on headless CI without a
//!   usable wgpu adapter — the same property
//!   [`crate::multipass::compile_and_render_multipass`]'s structural
//!   validation gates achieve via the "structural validation
//!   intentionally runs BEFORE wgpu adapter acquisition" pattern.
//! - Callers (the runtime crate) drive both the orchestrator AND
//!   the per-frame scheduler invocation. The orchestrator says
//!   WHICH spec is current; the runtime calls
//!   [`crate::compile_and_render_multipass`] with that spec.
//! - Slice-5b.2 will add a new entry point that takes an
//!   `&wgpu::Device` and the orchestrator's prepared specs, and
//!   composes the two-scene crossfade. That call site is the
//!   natural home for actual GPU texture preallocation; slice-5b.1
//!   keeps the door open by making the state machine event-driven
//!   (no internal `tick`) so the slice-5b.2 driver can interleave
//!   GPU work between phase advances on its own thread.
//!
//! # Determinism
//!
//! All state advances are explicit method calls; there is no
//! wall-clock dependency. ADR-007's 500 ms timeout is a slice-5b.2
//! concern (the timeout decision belongs with the
//! `CrossfadeOrHardCut` frame loop that owns wall-clock progress).
//!
//! # Slice-5a continuity
//!
//! Transitions choose WHICH [`crate::multipass::MultipassSpec`] is
//! current; the audio binding adapter
//! ([`crate::audio_adapter::uniform_input_from_audio_snapshot`])
//! continues to feed [`crate::multipass::UniformInput`] into the
//! chosen spec's uniforms unchanged. Slice-5b.1 does NOT widen the
//! audio surface.
//!
//! # Evidence anchors
//!
//! - `docs/26-evidence-artifact-registry.md` row 28 —
//!   `EV-RENDER-CONFORMANCE`.
//! - `docs/adr/ADR-007-scene-warm-switch-policy.md` — Accepted four-
//!   phase lifecycle.
//! - `docs/13-risk-register.md` — risk row covering transition
//!   correctness.

use std::fmt;

use thiserror::Error;

use crate::multipass::MultipassSpec;

/// Warm-switch lifecycle state per ADR-007.
///
/// `#[non_exhaustive]` because slice-5b.2 may add states (e.g.
/// `Aborted` for an explicit terminal-failure state, or a
/// `TimedOut` variant for the 500 ms hard-cut fallback) — keeping
/// the enum non-exhaustive avoids a compat break on consumers that
/// `match` over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TransitionState {
    /// No transition is in flight. The orchestrator owns ONE scene
    /// (the current scene); rendering uses [`Self::current_spec`].
    /// `Idle` is NOT one of the four ADR-007 phases; it represents
    /// "between transitions" — see the module-level doc.
    Idle,
    /// ADR-007 phase 1 — preparation begins. The current scene
    /// continues to render; the next scene's spec has been validated
    /// and accepted. Resource preallocation is conceptually in
    /// flight (slice-5b.2 wires the GPU side).
    PrepareNext,
    /// ADR-007 phase 2 — preparation completed; the next scene is
    /// PRIMED and ready to take over. Per the ADR, "keep current
    /// scene active until `PrepareNext` and `PrimeNext` succeed" —
    /// the live output is still the current scene at this phase.
    PrimeNext,
    /// ADR-007 phase 3 — the visual transition is in flight. Both
    /// scenes are live; the actual crossfade or hard-cut composition
    /// is slice-5b.2 scope. Slice-5b.1 advances through this state
    /// without rendering.
    CrossfadeOrHardCut,
    /// ADR-007 phase 4 — the next scene takes over as the live
    /// scene. After advancing through this state via
    /// [`TransitionOrchestrator::commit_next`], the orchestrator
    /// returns to [`Self::Idle`] with the prior next-scene as the
    /// new current-scene; the prior current-scene's resources are
    /// released.
    CommitNext,
}

impl TransitionState {
    /// Human-readable label for diagnostics. Stable across slices —
    /// see `TransitionState::label()` drift pin in
    /// `transitions::transitions_pins`.
    pub fn label(self) -> &'static str {
        match self {
            TransitionState::Idle => "Idle",
            TransitionState::PrepareNext => "PrepareNext",
            TransitionState::PrimeNext => "PrimeNext",
            TransitionState::CrossfadeOrHardCut => "CrossfadeOrHardCut",
            TransitionState::CommitNext => "CommitNext",
        }
    }
}

impl fmt::Display for TransitionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Error surface for the warm-switch orchestrator.
///
/// `#[non_exhaustive]` so slice-5b.2 can add variants (e.g.
/// `TransitionTimedOut` for the 500 ms hard-cut fallback) without
/// a major-version bump.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TransitionError {
    /// A phase-advance API was called from a state where it is not
    /// legal per the ADR-007 lifecycle. The two fields capture the
    /// observed `from` state and the human-readable label of the
    /// attempted advance for triage.
    #[error(
        "transition state machine: cannot advance via {attempted} from state {from} \
         (legal predecessors are documented on TransitionOrchestrator's per-method docs)"
    )]
    InvalidStateTransition {
        /// State the orchestrator was in when the call was made.
        from: TransitionState,
        /// Name of the API method whose advance was attempted.
        attempted: &'static str,
    },

    /// The cued [`MultipassSpec`] failed the spec-level shape gate
    /// the orchestrator runs before accepting it for preallocation.
    /// Full structural validation (forward references, feedback
    /// aliasing, adapter capability) lives inside
    /// [`crate::multipass::compile_and_render_multipass`] and fires
    /// at render time; this gate catches obviously-bad cued specs
    /// before the renderer is reached.
    #[error("transition cued spec rejected: {reason}")]
    SpecMismatch {
        /// Diagnostic explanation.
        reason: String,
    },

    /// Slice-5b.1 reserves this variant for the slice-5b.2
    /// resource-preallocation step (GPU texture / pipeline cache
    /// allocation). The variant is declared in slice-5b.1 so the
    /// public error surface does not break-change when slice-5b.2
    /// wires it. No code path returns this variant in slice-5b.1.
    #[error("transition resource preallocation failed: {reason}")]
    ResourcePreallocFailed {
        /// Diagnostic explanation from the GPU side.
        reason: String,
    },
}

/// Driver type for the ADR-007 warm-switch lifecycle.
///
/// # Lifecycle
///
/// The legal phase-advance graph (slice-5b.1):
///
/// ```text
///   Idle ──cue_next──▶ PrepareNext
///   PrepareNext ──mark_primed──▶ PrimeNext
///   PrimeNext ──begin_crossfade──▶ CrossfadeOrHardCut
///   CrossfadeOrHardCut ──commit_next──▶ CommitNext
///   CommitNext ──(implicit on commit_next return)──▶ Idle
/// ```
///
/// `commit_next` performs the phase-3-to-phase-4 advance AND the
/// implicit `CommitNext → Idle` settling in one call. The
/// orchestrator never observably rests in `CommitNext` — that state
/// is reachable only mid-`commit_next` and is exposed to the public
/// state-accessor for completeness so tests can pin the phase
/// vocabulary against ADR-007 verbatim.
///
/// # Invariants
///
/// - `current_spec()` ALWAYS returns a `&MultipassSpec`. The
///   orchestrator never has zero scenes; `new()` consumes one and
///   `commit_next` swaps before releasing the prior current.
/// - `next_spec()` returns `Some` only in `PrepareNext` / `PrimeNext`
///   / `CrossfadeOrHardCut`; in `Idle` and (post-`commit_next`)
///   `CommitNext → Idle`, it returns `None`.
/// - On any error from a phase-advance method the state remains
///   unchanged — the API never leaves the orchestrator in a
///   half-advanced state.
///
/// # Slice scope
///
/// Slice-5b.1 stops short of the actual crossfade composition. The
/// `begin_crossfade` and `commit_next` advance methods accept the
/// state transition but do NOT call the renderer; that is
/// slice-5b.2 scope.
#[derive(Debug)]
pub struct TransitionOrchestrator {
    state: TransitionState,
    current: MultipassSpec,
    next: Option<MultipassSpec>,
}

impl TransitionOrchestrator {
    /// Construct an orchestrator with `initial` as the live scene.
    /// Initial state is [`TransitionState::Idle`].
    pub fn new(initial: MultipassSpec) -> Self {
        Self {
            state: TransitionState::Idle,
            current: initial,
            next: None,
        }
    }

    /// Current state.
    pub fn current_state(&self) -> TransitionState {
        self.state
    }

    /// Currently-live scene's spec. Always `Some` semantically — the
    /// orchestrator never has zero scenes.
    pub fn current_spec(&self) -> &MultipassSpec {
        &self.current
    }

    /// Cued next scene's spec, when one is in flight. Returns `None`
    /// in [`TransitionState::Idle`] and after [`Self::commit_next`]
    /// has settled.
    pub fn next_spec(&self) -> Option<&MultipassSpec> {
        self.next.as_ref()
    }

    /// Cue a next scene and advance `Idle → PrepareNext`.
    ///
    /// # Errors
    ///
    /// - [`TransitionError::InvalidStateTransition`] if not in
    ///   `Idle`.
    /// - [`TransitionError::SpecMismatch`] if the cued spec fails
    ///   the orchestrator's shape gate (empty pass list, zero
    ///   `frame_count`, zero target size). Full structural
    ///   validation runs at render time inside
    ///   [`crate::multipass::compile_and_render_multipass`].
    pub fn cue_next(&mut self, next: MultipassSpec) -> Result<(), TransitionError> {
        if self.state != TransitionState::Idle {
            return Err(TransitionError::InvalidStateTransition {
                from: self.state,
                attempted: "cue_next",
            });
        }
        Self::validate_cued_spec(&next)?;
        self.next = Some(next);
        self.state = TransitionState::PrepareNext;
        Ok(())
    }

    /// Advance `PrepareNext → PrimeNext` (preparation has completed
    /// and resources are primed).
    ///
    /// # Errors
    ///
    /// - [`TransitionError::InvalidStateTransition`] if not in
    ///   `PrepareNext`.
    pub fn mark_primed(&mut self) -> Result<(), TransitionError> {
        if self.state != TransitionState::PrepareNext {
            return Err(TransitionError::InvalidStateTransition {
                from: self.state,
                attempted: "mark_primed",
            });
        }
        self.state = TransitionState::PrimeNext;
        Ok(())
    }

    /// Advance `PrimeNext → CrossfadeOrHardCut`.
    ///
    /// # Errors
    ///
    /// - [`TransitionError::InvalidStateTransition`] if not in
    ///   `PrimeNext`.
    pub fn begin_crossfade(&mut self) -> Result<(), TransitionError> {
        if self.state != TransitionState::PrimeNext {
            return Err(TransitionError::InvalidStateTransition {
                from: self.state,
                attempted: "begin_crossfade",
            });
        }
        self.state = TransitionState::CrossfadeOrHardCut;
        Ok(())
    }

    /// Advance `CrossfadeOrHardCut → CommitNext → Idle` and swap the
    /// next scene to current. The prior current scene is dropped on
    /// return (its `MultipassSpec` is plain data; slice-5b.2 will
    /// also free associated GPU handles here).
    ///
    /// # Errors
    ///
    /// - [`TransitionError::InvalidStateTransition`] if not in
    ///   `CrossfadeOrHardCut`.
    pub fn commit_next(&mut self) -> Result<(), TransitionError> {
        if self.state != TransitionState::CrossfadeOrHardCut {
            return Err(TransitionError::InvalidStateTransition {
                from: self.state,
                attempted: "commit_next",
            });
        }
        // Per the doc-comment on the lifecycle graph, the
        // orchestrator never observably rests in `CommitNext`; the
        // phase exists in the enum so tests can pin the phase
        // vocabulary verbatim, but commit_next advances through it
        // and settles in `Idle`. We do not separately observable-set
        // `CommitNext` — the API contract is "commit_next advances
        // CrossfadeOrHardCut → Idle".
        let next = self.next.take().ok_or(TransitionError::SpecMismatch {
            reason:
                "commit_next reached without a cued next spec (orchestrator invariant violated)"
                    .to_string(),
        })?;
        self.current = next;
        self.state = TransitionState::Idle;
        Ok(())
    }

    /// Abort an in-flight transition and return the orchestrator to
    /// `Idle`, dropping the cued next scene. Honors the ADR-007
    /// "preparation failure abort, preserve current output with
    /// diagnostics" decision: callers route preparation failures
    /// here rather than corrupting the live output.
    ///
    /// Legal from `PrepareNext`, `PrimeNext`, or
    /// `CrossfadeOrHardCut`. From `Idle` this is a no-op (already
    /// quiescent); from any other state it returns
    /// [`TransitionError::InvalidStateTransition`].
    ///
    /// # Errors
    ///
    /// - [`TransitionError::InvalidStateTransition`] if called from
    ///   an unexpected state. `Idle` is treated as legal-no-op.
    pub fn abort(&mut self) -> Result<(), TransitionError> {
        match self.state {
            TransitionState::Idle => Ok(()),
            TransitionState::PrepareNext
            | TransitionState::PrimeNext
            | TransitionState::CrossfadeOrHardCut => {
                self.next = None;
                self.state = TransitionState::Idle;
                Ok(())
            }
            // CommitNext is unreachable as a resting state (see
            // commit_next doc) but if a future slice-5b.2 path sets
            // it, abort from CommitNext is rejected — by then the
            // swap has happened and aborting would un-do a
            // committed-to scene change.
            TransitionState::CommitNext => Err(TransitionError::InvalidStateTransition {
                from: self.state,
                attempted: "abort",
            }),
        }
    }

    /// Spec-level shape gate. Catches obviously-bad cued specs
    /// before they reach the renderer's full structural
    /// validation. The full validation (forward-reference,
    /// feedback-alias, float-target adapter capability) is the
    /// renderer's responsibility; this gate is the lighter
    /// preflight.
    fn validate_cued_spec(spec: &MultipassSpec) -> Result<(), TransitionError> {
        if spec.passes.is_empty() {
            return Err(TransitionError::SpecMismatch {
                reason: "cued spec has empty pass list".to_string(),
            });
        }
        if spec.frame_count == 0 {
            return Err(TransitionError::SpecMismatch {
                reason: "cued spec frame_count is zero".to_string(),
            });
        }
        if spec.target_size.0 == 0 || spec.target_size.1 == 0 {
            return Err(TransitionError::SpecMismatch {
                reason: format!(
                    "cued spec target_size {:?} invalid (width and height must both be > 0)",
                    spec.target_size
                ),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod transitions_pins {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — slice-5b.1 transition
    //! orchestration drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: state-machine
    //! invariant pins that fire WITHOUT a usable wgpu adapter (the
    //! T2 corpus in `engine/render/tests/transitions.rs` covers
    //! lifecycle end-to-end including invariant-cross-multiple-
    //! phases pins).
    //!
    //! Pinned invariants:
    //!
    //! - `new()` starts in `Idle` with the supplied spec as current
    //!   and `next_spec() == None`.
    //! - The legal phase-advance graph (`Idle → PrepareNext →
    //!   PrimeNext → CrossfadeOrHardCut → Idle` via `commit_next`)
    //!   accepts every legal advance and rejects every illegal
    //!   one with `InvalidStateTransition`.
    //! - The cued-spec shape gate rejects empty pass lists,
    //!   `frame_count == 0`, and zero target size with
    //!   `SpecMismatch`.
    //! - On a rejected `cue_next` call the state remains `Idle`
    //!   and `next_spec()` remains `None`.
    //! - On `commit_next` the next scene becomes current and the
    //!   prior current is dropped; `next_spec()` returns `None`.
    //! - `abort()` is a no-op from `Idle`, and from in-flight
    //!   phases it returns to `Idle` and clears the cued spec.
    //! - `TransitionState::label()` is stable for diagnostics.

    use super::{TransitionError, TransitionOrchestrator, TransitionState};
    use crate::multipass::{MultipassSpec, PassDef};

    /// Smallest valid spec the orchestrator's shape gate accepts.
    /// Real shader compilation is a renderer concern.
    fn trivial_spec(label: &str) -> MultipassSpec {
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
    fn new_starts_idle_with_current_and_no_next() {
        let orch = TransitionOrchestrator::new(trivial_spec("initial"));
        assert_eq!(orch.current_state(), TransitionState::Idle);
        assert_eq!(
            orch.current_spec().passes[0].label.as_deref(),
            Some("initial")
        );
        assert!(orch.next_spec().is_none());
    }

    #[test]
    fn full_legal_lifecycle_advances_idle_to_idle() {
        let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
        orch.cue_next(trivial_spec("next"))
            .expect("Idle → PrepareNext is legal");
        assert_eq!(orch.current_state(), TransitionState::PrepareNext);
        assert_eq!(
            orch.next_spec().unwrap().passes[0].label.as_deref(),
            Some("next")
        );
        // current still points at the original scene per the ADR
        // "keep current scene active until PrepareNext + PrimeNext
        // succeed" invariant.
        assert_eq!(
            orch.current_spec().passes[0].label.as_deref(),
            Some("initial")
        );

        orch.mark_primed()
            .expect("PrepareNext → PrimeNext is legal");
        assert_eq!(orch.current_state(), TransitionState::PrimeNext);
        // Live output STILL the original scene at PrimeNext.
        assert_eq!(
            orch.current_spec().passes[0].label.as_deref(),
            Some("initial")
        );

        orch.begin_crossfade()
            .expect("PrimeNext → CrossfadeOrHardCut is legal");
        assert_eq!(orch.current_state(), TransitionState::CrossfadeOrHardCut);

        orch.commit_next()
            .expect("CrossfadeOrHardCut → Idle (via CommitNext) is legal");
        assert_eq!(orch.current_state(), TransitionState::Idle);
        // Now current is the previously-cued scene.
        assert_eq!(orch.current_spec().passes[0].label.as_deref(), Some("next"));
        // No next pending after commit.
        assert!(orch.next_spec().is_none());
    }

    #[test]
    fn cue_next_rejects_outside_idle() {
        let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
        orch.cue_next(trivial_spec("first")).unwrap();
        // Now in PrepareNext — re-cueing is illegal in slice-5b.1.
        let err = orch
            .cue_next(trivial_spec("second"))
            .expect_err("cue_next outside Idle must reject");
        match err {
            TransitionError::InvalidStateTransition { from, attempted } => {
                assert_eq!(from, TransitionState::PrepareNext);
                assert_eq!(attempted, "cue_next");
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
        // State unchanged; the original cue is still in flight.
        assert_eq!(orch.current_state(), TransitionState::PrepareNext);
        assert_eq!(
            orch.next_spec().unwrap().passes[0].label.as_deref(),
            Some("first")
        );
    }

    #[test]
    fn mark_primed_rejects_outside_prepare_next() {
        let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
        let err = orch
            .mark_primed()
            .expect_err("mark_primed from Idle must reject");
        match err {
            TransitionError::InvalidStateTransition { from, attempted } => {
                assert_eq!(from, TransitionState::Idle);
                assert_eq!(attempted, "mark_primed");
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    #[test]
    fn begin_crossfade_rejects_outside_prime_next() {
        let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
        orch.cue_next(trivial_spec("next")).unwrap();
        // begin_crossfade from PrepareNext — must reject.
        let err = orch
            .begin_crossfade()
            .expect_err("begin_crossfade from PrepareNext must reject");
        match err {
            TransitionError::InvalidStateTransition { from, attempted } => {
                assert_eq!(from, TransitionState::PrepareNext);
                assert_eq!(attempted, "begin_crossfade");
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    #[test]
    fn commit_next_rejects_outside_crossfade() {
        let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
        orch.cue_next(trivial_spec("next")).unwrap();
        orch.mark_primed().unwrap();
        // commit_next from PrimeNext — must reject.
        let err = orch
            .commit_next()
            .expect_err("commit_next from PrimeNext must reject");
        match err {
            TransitionError::InvalidStateTransition { from, attempted } => {
                assert_eq!(from, TransitionState::PrimeNext);
                assert_eq!(attempted, "commit_next");
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    #[test]
    fn cue_next_rejects_empty_pass_list() {
        let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
        let bad = MultipassSpec {
            passes: vec![],
            target_size: (16, 16),
            frame_count: 1,
        };
        let err = orch.cue_next(bad).expect_err("empty pass list must reject");
        match err {
            TransitionError::SpecMismatch { reason } => {
                assert!(reason.contains("empty pass list"), "reason: {reason}");
            }
            other => panic!("expected SpecMismatch, got {other:?}"),
        }
        // State and next remain Idle / None on rejection.
        assert_eq!(orch.current_state(), TransitionState::Idle);
        assert!(orch.next_spec().is_none());
    }

    #[test]
    fn cue_next_rejects_zero_frame_count() {
        let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
        let mut bad = trivial_spec("bad");
        bad.frame_count = 0;
        let err = orch
            .cue_next(bad)
            .expect_err("frame_count == 0 must reject");
        match err {
            TransitionError::SpecMismatch { reason } => {
                assert!(reason.contains("frame_count is zero"), "reason: {reason}");
            }
            other => panic!("expected SpecMismatch, got {other:?}"),
        }
    }

    #[test]
    fn cue_next_rejects_zero_target_size() {
        let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
        let mut bad = trivial_spec("bad");
        bad.target_size = (0, 16);
        let err = orch
            .cue_next(bad)
            .expect_err("zero target width must reject");
        match err {
            TransitionError::SpecMismatch { reason } => {
                assert!(
                    reason.contains("target_size") && reason.contains("invalid"),
                    "reason: {reason}",
                );
            }
            other => panic!("expected SpecMismatch, got {other:?}"),
        }
    }

    #[test]
    fn abort_from_idle_is_noop() {
        let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
        orch.abort().expect("abort from Idle is a legal no-op");
        assert_eq!(orch.current_state(), TransitionState::Idle);
    }

    #[test]
    fn abort_from_in_flight_returns_to_idle_and_clears_next() {
        // Abort from each in-flight phase: PrepareNext, PrimeNext,
        // CrossfadeOrHardCut. The current scene must be preserved
        // per the ADR-007 "preserve current output with diagnostics"
        // decision.
        for advance_to in [
            TransitionState::PrepareNext,
            TransitionState::PrimeNext,
            TransitionState::CrossfadeOrHardCut,
        ] {
            let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
            orch.cue_next(trivial_spec("next")).unwrap();
            if advance_to == TransitionState::PrimeNext
                || advance_to == TransitionState::CrossfadeOrHardCut
            {
                orch.mark_primed().unwrap();
            }
            if advance_to == TransitionState::CrossfadeOrHardCut {
                orch.begin_crossfade().unwrap();
            }
            assert_eq!(orch.current_state(), advance_to);
            orch.abort().expect("abort from in-flight phase is legal");
            assert_eq!(orch.current_state(), TransitionState::Idle);
            assert!(orch.next_spec().is_none());
            // Current scene preserved.
            assert_eq!(
                orch.current_spec().passes[0].label.as_deref(),
                Some("initial")
            );
        }
    }

    #[test]
    fn transition_state_labels_are_stable() {
        // Drift pin against accidental enum-label rename. The
        // labels are part of the diagnostic surface; renaming them
        // would silently shift error-message strings and break
        // log-grep.
        assert_eq!(TransitionState::Idle.label(), "Idle");
        assert_eq!(TransitionState::PrepareNext.label(), "PrepareNext");
        assert_eq!(TransitionState::PrimeNext.label(), "PrimeNext");
        assert_eq!(
            TransitionState::CrossfadeOrHardCut.label(),
            "CrossfadeOrHardCut"
        );
        assert_eq!(TransitionState::CommitNext.label(), "CommitNext");
    }
}

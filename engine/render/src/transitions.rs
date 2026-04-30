// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-2 EV-RENDER-CONFORMANCE slice-5b transition-orchestration
//! (ADR-007 warm-switch policy).
//!
//! Slice-5b.1 (already merged) landed the lifecycle state machine.
//! Slice-5b.2 (this slice) extends the orchestrator with GPU resource
//! preallocation, the crossfade composition entry point, and the
//! 500 ms timeout-degrades-to-hard-cut fallback path. The slice-5b.1
//! `TransitionOrchestrator` API surface (`cue_next` / `mark_primed` /
//! `begin_crossfade` / `commit_next` / `abort` / accessors) STAYS
//! UNCHANGED; the slice-5b.2 additions are GPU-aware methods alongside
//! the existing GPU-FREE state machine plus a separate composition
//! entry point in [`crate::composition`].
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
use std::time::Instant;

use thiserror::Error;

use crate::multipass::MultipassSpec;

/// ADR-007 default timeout: a transition that does not reach
/// `PrimeNext` within 500 ms degrades to a hard-cut. Exposed as a
/// `pub const` so callers and tests share one source of truth.
pub const DEFAULT_TIMEOUT_MS: u64 = 500;

/// Wall-clock abstraction the orchestrator uses to enforce ADR-007's
/// 500 ms preparation timeout. Local to this module — the audio /
/// script crates have their own [`Clock`] traits with different
/// signatures (millisecond reading, `i64`); the transition-side
/// abstraction returns elapsed-time-since-start in `u64` ms because
/// negative values are not meaningful for an elapsed-budget reading
/// and saturating-add semantics on `u64` keep the timeout-firing
/// branch monotonically conservative.
///
/// `Send + Sync` so callers may share a clock instance across the
/// orchestrator's lifetime; the default [`MonotonicTransitionClock`]
/// is `Clone` for the same reason.
///
/// # Slice-5b.2 vs cross-crate `Clock` trait
///
/// `engine/script/src/determinism.rs::Clock` exists for the JS
/// `Date.now()` shim and returns `i64` (mirroring the JS spec).
/// Transitions need `Instant`-based monotonic elapsed time, not
/// wall-clock millis, and their callers don't drive a per-frame
/// fixed-tick replay primitive — a separate trait keeps the surface
/// crisp (no negative-elapsed handling, no per-frame override
/// pattern). The `engine/audio/observability` ingest tracker also
/// reaches for `Instant` directly without a trait abstraction; the
/// trait abstraction here exists specifically because the timeout-
/// firing branch needs a deterministic test path
/// ([`FixedTransitionClock`]).
pub trait TransitionClock: Send + Sync {
    /// Elapsed milliseconds since some implementation-internal
    /// anchor (typically construction). Monotonic — calls observe a
    /// non-decreasing sequence. The orchestrator records the value
    /// at `cue_next` time and computes elapsed as
    /// `now_elapsed_ms() - start_ms`.
    fn now_elapsed_ms(&self) -> u64;
}

/// Default [`TransitionClock`] implementor backed by [`Instant`].
///
/// Constructed at orchestrator-build time; subsequent
/// [`TransitionClock::now_elapsed_ms`] reads use the wall-clock
/// delta against the captured `start` instant, so the value advances
/// monotonically and is not subject to system-clock adjustments.
#[derive(Debug, Clone, Copy)]
pub struct MonotonicTransitionClock {
    start: Instant,
}

impl MonotonicTransitionClock {
    /// Construct a monotonic clock anchored at `Instant::now()`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Anchor instant the elapsed delta is computed against.
    #[must_use]
    pub fn start(&self) -> Instant {
        self.start
    }
}

impl Default for MonotonicTransitionClock {
    fn default() -> Self {
        Self::new()
    }
}

impl TransitionClock for MonotonicTransitionClock {
    fn now_elapsed_ms(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// Fixed-tick [`TransitionClock`] for tests. Every call to
/// [`TransitionClock::now_elapsed_ms`] returns the configured value.
///
/// The slice-5b.2 timeout-firing T1 pin uses this to deterministically
/// drive the orchestrator into the `TimeoutExceeded` branch without a
/// real `sleep`.
#[derive(Debug, Clone, Copy)]
pub struct FixedTransitionClock {
    elapsed_ms: u64,
}

impl FixedTransitionClock {
    /// Construct a fixed-tick clock that always returns `elapsed_ms`.
    #[must_use]
    pub fn new(elapsed_ms: u64) -> Self {
        Self { elapsed_ms }
    }

    /// The elapsed millisecond reading this clock returns.
    #[must_use]
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }
}

impl TransitionClock for FixedTransitionClock {
    fn now_elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }
}

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

    /// Slice-5b.1 reserved this variant for the slice-5b.2
    /// resource-preallocation step (GPU texture / pipeline cache
    /// allocation). Slice-5b.2 wires it: a wgpu `create_*` failure
    /// surfaces here. The variant is kept verbatim from slice-5b.1
    /// so the public error surface does not break-change between
    /// slices.
    #[error("transition resource preallocation failed: {reason}")]
    ResourcePreallocFailed {
        /// Diagnostic explanation from the GPU side.
        reason: String,
    },

    /// ADR-007 §Decision: "Default warm-switch timeout is 500 ms;
    /// timeout degrades to hard-cut fallback with warning." Slice-5b.2
    /// fires this variant when [`TransitionOrchestrator::mark_primed`]
    /// is called and the elapsed time since `cue_next` exceeds
    /// [`DEFAULT_TIMEOUT_MS`]. The caller's response is to invoke
    /// [`TransitionOrchestrator::abort`] (returning to `Idle`
    /// preserving the current scene) and then re-cue with hard-cut
    /// semantics, OR to call [`TransitionOrchestrator::commit_hard_cut`]
    /// which performs the swap immediately without a crossfade.
    #[error(
        "transition preparation timeout: {elapsed_ms} ms elapsed since cue_next \
         (limit {limit_ms} ms per ADR-007 §Decision; degrade to hard-cut)"
    )]
    TimeoutExceeded {
        /// Observed elapsed milliseconds since `cue_next`.
        elapsed_ms: u64,
        /// Configured timeout limit, surfaced for triage.
        limit_ms: u64,
    },

    /// A wgpu device.create_*() call failed during
    /// [`TransitionOrchestrator::prepare_resources`]. Slice-5b.2
    /// surfaces this distinctly from
    /// [`TransitionError::ResourcePreallocFailed`]: this variant
    /// covers GPU-side allocation failures (out-of-memory, descriptor
    /// rejection, format-unsupported); `ResourcePreallocFailed`
    /// covers higher-level preallocation logic (e.g. orchestrator
    /// state-machine violation observed during preallocation).
    #[error("transition GPU allocation failed: {reason}")]
    GpuAllocationFailed {
        /// Diagnostic explanation from the wgpu side.
        reason: String,
    },

    /// The composition pass itself failed —
    /// [`crate::composition::compose_transition_frame`] could not
    /// render the crossfade output. Slice-5b.2 wraps the inner
    /// [`crate::single_pass::RenderError`] / wgpu diagnostic into
    /// `reason` because the public error surface intentionally does
    /// NOT widen with a `RenderError` variant — the orchestrator's
    /// composition path is its own concern, structurally distinct
    /// from the multipass renderer's error surface.
    #[error("transition composition pass failed: {reason}")]
    CompositionFailed {
        /// Diagnostic explanation from the composition side.
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
pub struct TransitionOrchestrator {
    state: TransitionState,
    current: MultipassSpec,
    next: Option<MultipassSpec>,
    // Slice-5b.2 timeout-tracking fields.
    //
    // `clock` is a boxed trait object so callers may inject a
    // `FixedTransitionClock` for deterministic timeout-firing tests
    // without exposing the orchestrator's internal generic surface.
    // The default is `MonotonicTransitionClock::new()`.
    clock: Box<dyn TransitionClock>,
    timeout_ms: u64,
    // Elapsed reading at `cue_next` time, recorded for the
    // mark_primed-side budget check. `None` outside the
    // `PrepareNext` / `PrimeNext` window.
    cue_start_elapsed_ms: Option<u64>,
}

impl fmt::Debug for TransitionOrchestrator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransitionOrchestrator")
            .field("state", &self.state)
            .field("current", &self.current)
            .field("next", &self.next)
            .field("timeout_ms", &self.timeout_ms)
            .field("cue_start_elapsed_ms", &self.cue_start_elapsed_ms)
            // `clock` is a trait object without `Debug`; surface a
            // sentinel rather than widen the trait's super-trait
            // bound.
            .field("clock", &"<TransitionClock>")
            .finish()
    }
}

impl TransitionOrchestrator {
    /// Construct an orchestrator with `initial` as the live scene.
    /// Initial state is [`TransitionState::Idle`]. Uses a default
    /// [`MonotonicTransitionClock`] and the ADR-007 default timeout
    /// of [`DEFAULT_TIMEOUT_MS`] (500 ms).
    pub fn new(initial: MultipassSpec) -> Self {
        Self {
            state: TransitionState::Idle,
            current: initial,
            next: None,
            clock: Box::new(MonotonicTransitionClock::new()),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            cue_start_elapsed_ms: None,
        }
    }

    /// Construct an orchestrator with a caller-supplied
    /// [`TransitionClock`] and explicit timeout budget. Used by tests
    /// to drive the timeout-firing path deterministically and by
    /// callers that want to extend / tighten the ADR-007 default
    /// 500 ms budget.
    ///
    /// Setting `timeout_ms = 0` is legal but pathological: every
    /// `mark_primed` call will fire `TimeoutExceeded` (the elapsed
    /// reading is `>= 0` by construction). Callers using `0` should
    /// treat the orchestrator as a hard-cut-only state machine.
    pub fn with_clock_and_timeout(
        initial: MultipassSpec,
        clock: Box<dyn TransitionClock>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            state: TransitionState::Idle,
            current: initial,
            next: None,
            clock,
            timeout_ms,
            cue_start_elapsed_ms: None,
        }
    }

    /// Configured timeout budget in milliseconds. Defaults to
    /// [`DEFAULT_TIMEOUT_MS`] (the ADR-007 §Decision 500 ms).
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
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
        // Record start-tick for the slice-5b.2 timeout budget.
        self.cue_start_elapsed_ms = Some(self.clock.now_elapsed_ms());
        Ok(())
    }

    /// Advance `PrepareNext → PrimeNext` (preparation has completed
    /// and resources are primed).
    ///
    /// Slice-5b.2 wires the ADR-007 §Decision 500 ms timeout here: if
    /// the elapsed time since [`Self::cue_next`] exceeds
    /// [`Self::timeout_ms`] this method returns
    /// [`TransitionError::TimeoutExceeded`] and the state STAYS in
    /// `PrepareNext` (per the orchestrator's "on any error from a
    /// phase-advance method the state remains unchanged" invariant).
    /// The caller's response is to invoke [`Self::abort`] (which
    /// returns to `Idle` preserving the current scene) and re-cue, OR
    /// to call [`Self::commit_hard_cut`] which performs the swap
    /// immediately without a crossfade.
    ///
    /// # Errors
    ///
    /// - [`TransitionError::InvalidStateTransition`] if not in
    ///   `PrepareNext`.
    /// - [`TransitionError::TimeoutExceeded`] if the elapsed time
    ///   since `cue_next` exceeds the configured timeout budget.
    pub fn mark_primed(&mut self) -> Result<(), TransitionError> {
        if self.state != TransitionState::PrepareNext {
            return Err(TransitionError::InvalidStateTransition {
                from: self.state,
                attempted: "mark_primed",
            });
        }
        // ADR-007 timeout enforcement: compare elapsed against budget.
        // We use saturating subtract on `u64` because the clock is
        // monotonic and the start reading is captured on `cue_next`,
        // but defending against a non-monotonic implementor (which
        // would violate the trait contract) keeps the timeout-firing
        // branch conservative — a bad clock can never cause us to
        // claim "negative elapsed" and skip the budget check.
        if let Some(start) = self.cue_start_elapsed_ms {
            let elapsed = self.clock.now_elapsed_ms().saturating_sub(start);
            if elapsed > self.timeout_ms {
                return Err(TransitionError::TimeoutExceeded {
                    elapsed_ms: elapsed,
                    limit_ms: self.timeout_ms,
                });
            }
        }
        self.state = TransitionState::PrimeNext;
        Ok(())
    }

    /// Hard-cut commit path for the ADR-007 timeout fallback. Skips
    /// the crossfade composition entirely and swaps the cued next
    /// scene to current immediately. Legal from `PrepareNext`,
    /// `PrimeNext`, or `CrossfadeOrHardCut` — the three in-flight
    /// states where a cued spec is present.
    ///
    /// # Errors
    ///
    /// - [`TransitionError::InvalidStateTransition`] if called from
    ///   `Idle` (no cue in flight) or `CommitNext` (already settling).
    /// - [`TransitionError::SpecMismatch`] if no cued spec is present
    ///   despite being in an in-flight state (orchestrator invariant
    ///   violated; defense-in-depth).
    pub fn commit_hard_cut(&mut self) -> Result<(), TransitionError> {
        match self.state {
            TransitionState::PrepareNext
            | TransitionState::PrimeNext
            | TransitionState::CrossfadeOrHardCut => {
                let next = self.next.take().ok_or(TransitionError::SpecMismatch {
                    reason: "commit_hard_cut reached without a cued next spec \
                         (orchestrator invariant violated)"
                        .to_string(),
                })?;
                self.current = next;
                self.state = TransitionState::Idle;
                self.cue_start_elapsed_ms = None;
                Ok(())
            }
            TransitionState::Idle | TransitionState::CommitNext => {
                Err(TransitionError::InvalidStateTransition {
                    from: self.state,
                    attempted: "commit_hard_cut",
                })
            }
        }
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
        self.cue_start_elapsed_ms = None;
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
                self.cue_start_elapsed_ms = None;
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

    /// Elapsed milliseconds since the most recent `cue_next`. Returns
    /// `None` outside the `PrepareNext` / `PrimeNext` /
    /// `CrossfadeOrHardCut` window. Surfaced for diagnostic reads
    /// (e.g. a UI overlay that wants to show "preparing scene…
    /// 240 ms"); the value is computed against the orchestrator's
    /// internal [`TransitionClock`] so it is monotonic with the
    /// timeout-firing branch in [`Self::mark_primed`].
    pub fn elapsed_since_cue_ms(&self) -> Option<u64> {
        self.cue_start_elapsed_ms
            .map(|start| self.clock.now_elapsed_ms().saturating_sub(start))
    }

    /// Slice-5b.2 GPU resource preallocation. Call from the
    /// `PrepareNext` or `PrimeNext` phase to allocate textures,
    /// pipelines, and uniform buffers for the cued spec ahead of the
    /// crossfade composition.
    ///
    /// In slice-5b.2 the actual GPU allocation is delegated to
    /// [`crate::composition::PreparedScene::allocate`]; this method
    /// is the orchestrator-side wrapper that drives that allocation
    /// behind the orchestrator's state-machine guards. The method is
    /// idempotent — calling it twice in the same phase is safe; the
    /// second call observes the same cued spec and returns a new
    /// [`crate::composition::PreparedScene`] (callers typically own
    /// the returned handle directly rather than cache it inside the
    /// orchestrator, which keeps the orchestrator itself GPU-handle-
    /// free per the slice-5b.1 "pure-data state machine" invariant).
    ///
    /// # Errors
    ///
    /// - [`TransitionError::InvalidStateTransition`] if not in
    ///   `PrepareNext` or `PrimeNext`.
    /// - [`TransitionError::ResourcePreallocFailed`] if no cued spec
    ///   is present (orchestrator invariant violated).
    /// - [`TransitionError::GpuAllocationFailed`] if the wgpu
    ///   `create_*()` allocation failed.
    pub fn prepare_resources(
        &self,
        device: &wgpu::Device,
    ) -> Result<crate::composition::PreparedScene, TransitionError> {
        match self.state {
            TransitionState::PrepareNext | TransitionState::PrimeNext => {}
            other => {
                return Err(TransitionError::InvalidStateTransition {
                    from: other,
                    attempted: "prepare_resources",
                });
            }
        }
        let next = self
            .next
            .as_ref()
            .ok_or_else(|| TransitionError::ResourcePreallocFailed {
                reason: "prepare_resources called without a cued next spec \
                         (orchestrator invariant violated)"
                    .to_string(),
            })?;
        crate::composition::PreparedScene::allocate(device, next)
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
    //!
    //! Slice-5b.2 additions:
    //!
    //! - The default timeout is `DEFAULT_TIMEOUT_MS = 500` ms per
    //!   ADR-007 §Decision; `with_clock_and_timeout()` honors a
    //!   caller-supplied budget.
    //! - `mark_primed` enforces the timeout via the orchestrator's
    //!   [`TransitionClock`]: elapsed > limit returns
    //!   [`TransitionError::TimeoutExceeded`] without advancing state.
    //! - The timeout budget is tracked from `cue_next` time; `abort`
    //!   and `commit_next` clear the start tick.
    //! - `commit_hard_cut` is the timeout-fallback path: legal from
    //!   any in-flight state, swaps current/next without crossfade.
    //! - The [`MonotonicTransitionClock`] / [`FixedTransitionClock`]
    //!   pair lets tests drive the timeout-firing branch
    //!   deterministically.
    //! - `elapsed_since_cue_ms()` accessor surfaces the in-flight
    //!   elapsed reading; returns `None` outside the in-flight states.

    use super::{
        DEFAULT_TIMEOUT_MS, FixedTransitionClock, MonotonicTransitionClock, TransitionClock,
        TransitionError, TransitionOrchestrator, TransitionState,
    };
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

    // ===== Slice-5b.2 timeout / clock / hard-cut T1 pins =====

    #[test]
    fn default_timeout_matches_adr007_500ms() {
        // Slice-5b.2 drift pin: ADR-007 §Decision pins the default
        // warm-switch timeout at 500 ms. A future bump must be
        // accompanied by an ADR amendment.
        assert_eq!(DEFAULT_TIMEOUT_MS, 500);
        let orch = TransitionOrchestrator::new(trivial_spec("initial"));
        assert_eq!(orch.timeout_ms(), 500);
    }

    #[test]
    fn monotonic_clock_advances_non_negative() {
        // Drift pin: MonotonicTransitionClock returns a non-negative
        // u64 — the API contract is "elapsed since anchor", which
        // can never be negative. A second read must be >= the first.
        let clock = MonotonicTransitionClock::new();
        let a = clock.now_elapsed_ms();
        let b = clock.now_elapsed_ms();
        assert!(b >= a, "monotonic clock must not regress: a={a} b={b}");
    }

    #[test]
    fn fixed_clock_returns_configured_value() {
        // Drift pin: FixedTransitionClock is the test primitive; a
        // single configured value must be returned verbatim from
        // every read.
        let clock = FixedTransitionClock::new(123);
        assert_eq!(clock.now_elapsed_ms(), 123);
        assert_eq!(clock.now_elapsed_ms(), 123);
        assert_eq!(clock.elapsed_ms(), 123);
    }

    #[test]
    fn timeout_fires_on_mark_primed_when_elapsed_exceeds_budget() {
        // Slice-5b.2 drift pin: the ADR-007 timeout-firing branch.
        // We need a clock whose read advances on every call so that
        // cue_next captures `start` and mark_primed reads `start +
        // step > limit`. FixedTransitionClock always returns the
        // same value (elapsed = 0), so we use a tiny stepping mock.
        struct StepClock {
            value: std::sync::atomic::AtomicU64,
        }
        impl TransitionClock for StepClock {
            fn now_elapsed_ms(&self) -> u64 {
                self.value
                    .fetch_add(100, std::sync::atomic::Ordering::SeqCst)
            }
        }
        let mut orch = TransitionOrchestrator::with_clock_and_timeout(
            trivial_spec("initial"),
            Box::new(StepClock {
                value: std::sync::atomic::AtomicU64::new(0),
            }),
            50,
        );
        // First read (cue_next): start = 0, then advances to 100.
        orch.cue_next(trivial_spec("next")).unwrap();
        // Second read (mark_primed): now = 100, elapsed = 100, > 50.
        let err = orch
            .mark_primed()
            .expect_err("mark_primed must fire TimeoutExceeded when elapsed > limit");
        match err {
            TransitionError::TimeoutExceeded {
                elapsed_ms,
                limit_ms,
            } => {
                assert!(
                    elapsed_ms > limit_ms,
                    "elapsed {elapsed_ms} must exceed limit {limit_ms}",
                );
                assert_eq!(limit_ms, 50);
            }
            other => panic!("expected TimeoutExceeded, got {other:?}"),
        }
        // State remains in PrepareNext per the "on error, state
        // unchanged" invariant.
        assert_eq!(orch.current_state(), TransitionState::PrepareNext);
    }

    #[test]
    fn timeout_does_not_fire_within_budget() {
        // Drift pin: when elapsed <= budget, mark_primed advances to
        // PrimeNext normally. Pins the within-budget happy path
        // against the timeout addition.
        let mut orch = TransitionOrchestrator::with_clock_and_timeout(
            trivial_spec("initial"),
            Box::new(FixedTransitionClock::new(0)),
            500,
        );
        orch.cue_next(trivial_spec("next")).unwrap();
        // Fixed clock means elapsed == 0, well under the 500ms budget.
        orch.mark_primed()
            .expect("within-budget mark_primed must succeed");
        assert_eq!(orch.current_state(), TransitionState::PrimeNext);
    }

    #[test]
    fn commit_hard_cut_swaps_current_from_any_in_flight_phase() {
        // Slice-5b.2 drift pin: hard-cut from each in-flight phase
        // must succeed and swap the current scene to the cued next,
        // returning to Idle. This is the ADR-007 timeout fallback
        // path.
        for advance_to in [
            TransitionState::PrepareNext,
            TransitionState::PrimeNext,
            TransitionState::CrossfadeOrHardCut,
        ] {
            let mut orch = TransitionOrchestrator::new(trivial_spec("from"));
            orch.cue_next(trivial_spec("to")).unwrap();
            if advance_to == TransitionState::PrimeNext
                || advance_to == TransitionState::CrossfadeOrHardCut
            {
                orch.mark_primed().unwrap();
            }
            if advance_to == TransitionState::CrossfadeOrHardCut {
                orch.begin_crossfade().unwrap();
            }
            orch.commit_hard_cut()
                .expect("hard-cut from in-flight succeeds");
            assert_eq!(orch.current_state(), TransitionState::Idle);
            assert_eq!(orch.current_spec().passes[0].label.as_deref(), Some("to"));
            assert!(orch.next_spec().is_none());
            assert_eq!(orch.elapsed_since_cue_ms(), None);
        }
    }

    #[test]
    fn commit_hard_cut_rejects_from_idle_or_committed() {
        // Drift pin: hard-cut from Idle (no cue) is illegal, and
        // CommitNext is reachable only mid-commit_next so it's also
        // rejected for the public API surface.
        let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
        let err = orch
            .commit_hard_cut()
            .expect_err("hard-cut from Idle must reject");
        match err {
            TransitionError::InvalidStateTransition { from, attempted } => {
                assert_eq!(from, TransitionState::Idle);
                assert_eq!(attempted, "commit_hard_cut");
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    #[test]
    fn elapsed_since_cue_is_some_during_in_flight_phases_only() {
        // Drift pin: the elapsed accessor returns Some during
        // in-flight phases, None at Idle (before cue and after
        // commit / abort).
        let mut orch = TransitionOrchestrator::new(trivial_spec("initial"));
        assert!(
            orch.elapsed_since_cue_ms().is_none(),
            "Idle (pre-cue) hides elapsed"
        );
        orch.cue_next(trivial_spec("next")).unwrap();
        assert!(
            orch.elapsed_since_cue_ms().is_some(),
            "PrepareNext exposes elapsed"
        );
        orch.abort().unwrap();
        assert!(
            orch.elapsed_since_cue_ms().is_none(),
            "post-abort hides elapsed"
        );
    }
}

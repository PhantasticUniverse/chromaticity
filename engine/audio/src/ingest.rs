// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Audio ingest boundary.
//!
//! The [`IngestBoundary`] trait names the audio-thread -> analyser
//! hand-off point that the Phase-2 pipeline consumes. Its shape is
//! aligned with the `CP-02a Audio Snapshot Handoff` contract in
//! `docs/28-implementation-contract-pack.md` and
//! `docs/06-audio-engine.md` §7.11: the analyser pulls the most recent
//! complete block and never blocks on the producer.
//!
//! Slice-2 adds the [`IngestState`] degraded-mode accounting per §7.11:
//! two consecutive failure outcomes (`WouldBlock` / `Closed`) flip
//! the state into [`IngestMode::Degraded`]; the first
//! [`IngestStatus::Ready`] returns it to [`IngestMode::Normal`].
//! [`IngestStatus::Pending`] is neutral and exists for callback-driven
//! sources that can be polled between normal producer bursts. This is
//! the ingest-side half of the §7.11 invariant; the consumer-side
//! snapshot handling lives in the frame scheduler (not this crate).
//!
//! # Standing constraint — cpal FFI-seam panic discipline
//!
//! cpal is dep-authorised in `engine/audio/Cargo.toml` at slice-2 as
//! a **dep-slot only**: no cpal type is imported and no cpal function
//! is invoked in slice-2 Rust code. The dep slot reserves a Phase-2
//! device-capture integration path for slice-3.
//!
//! When slice-3 lands the first real `cpal::Stream` callback, every
//! callback body that crosses the FFI seam MUST be wrapped in
//! `std::panic::catch_unwind`. A Rust panic crossing the platform
//! boundary (CoreAudio render callback on macOS, ALSA on Linux,
//! WASAPI on Windows) is **undefined behaviour** — cpal's internal
//! `unsafe` is permitted because cpal is not first-party code, but
//! the workspace-wide `unsafe_code = "forbid"` rule means the seam
//! cannot rely on an unwinding panic propagating safely.
//!
//! The required discipline, which the slice-3 author inherits and
//! reviewers MUST enforce:
//!
//! 1. Every real cpal callback MUST be wrapped in
//!    `std::panic::catch_unwind(AssertUnwindSafe(|| …))` at the exact
//!    boundary where Rust control passes to C.
//! 2. A panic caught at the seam MUST be coerced to
//!    [`IngestStatus::Closed`]. The source cannot safely continue
//!    after an unwinding panic in a platform callback.
//! 3. The coerced `Closed` MUST be recorded via
//!    [`IngestState::next_state`] so the degraded-mode transition
//!    fires as §7.11 specifies, and the callback MUST NOT re-enter
//!    the platform API once it has returned `Closed`.
//!
//! This is a standing constraint, not a suggestion. A slice-3 PR
//! that lands a cpal callback without catch_unwind at the FFI seam
//! is a merge blocker.

use crate::frame::AudioFrame;

/// Outcome of a non-blocking pull from the ingest source.
///
/// Mirrors the §7.11 degraded-modes vocabulary: the analyser either
/// gets a fresh frame, observes a neutral poll gap, observes a
/// transient underrun it can tolerate, or learns the source has gone
/// away and it must switch to the silent fallback uniforms from the
/// reliability policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestStatus {
    /// A fresh frame was produced.
    Ready,
    /// No frame is available on this poll, but the source is still
    /// healthy and this should not count toward degraded-mode
    /// fallback. Used by callback-driven producers such as cpal
    /// where the analyser can poll between normal callback bursts.
    Pending,
    /// No frame is available yet; retry on the next hop.
    WouldBlock,
    /// The source has closed and will not produce further frames.
    Closed,
}

/// Non-blocking audio ingest boundary.
///
/// Implementations are expected to be driven from the audio analysis
/// thread at the §7.1 hop cadence (`375 Hz` once the real pipeline
/// lands). The method is non-blocking so an under-run never stalls
/// the analyser: the §7.11 degraded-modes policy takes over after two
/// consecutive `WouldBlock` / `Closed` outcomes. Healthy empty polls
/// can return [`IngestStatus::Pending`] without advancing the
/// degraded-mode counter.
///
/// The trait is object-safe on purpose; the first real ingest source
/// will be selected at runtime from a configuration surface, so the
/// analyser stores a `Box<dyn IngestBoundary>`.
pub trait IngestBoundary {
    /// Attempt to pull the next frame into `out`.
    ///
    /// On [`IngestStatus::Ready`] the caller may read `out`. On any
    /// other status the contents of `out` are unspecified and MUST
    /// NOT be consumed.
    fn try_pull(&mut self, out: &mut AudioFrame) -> IngestStatus;
}

/// Phase-1 ingest mode per `docs/06-audio-engine.md` §7.11.
///
/// `Normal` — the most recent outcome was `Ready`, or the consecutive-
/// non-`Ready` counter has not yet crossed the §7.11 threshold.
///
/// `Degraded` — two or more consecutive failure outcomes have been
/// observed; the analyser should fall back to the silent-fallback
/// uniforms from the reliability policy until a `Ready` returns the
/// source to `Normal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IngestMode {
    /// Normal ingest flow; the most recent outcome was `Ready` or the
    /// degraded-mode threshold has not been reached.
    Normal,
    /// Degraded-mode fallback per §7.11; restored to `Normal` on the
    /// first subsequent `Ready`.
    Degraded,
}

impl Default for IngestMode {
    fn default() -> Self {
        Self::Normal
    }
}

/// Ingest-side §7.11 state machine tracking consecutive failure
/// outcomes.
///
/// Slice-2 pins the rule: two consecutive failure outcomes (either
/// `WouldBlock` or `Closed`) flip the mode to
/// [`IngestMode::Degraded`]; the first subsequent `Ready` returns it
/// to [`IngestMode::Normal`]. The threshold of 2 is the §7.11
/// "consecutive underruns" constant the ingest side is required to
/// implement; the matching consumer-side accounting lives in the
/// frame scheduler.
///
/// The `#[allow(dead_code)]` scope is slice-2 transitional: the
/// scheduler-tick path that feeds this state machine lands with
/// slice-3's real device-capture integration.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct IngestState {
    consecutive_underruns: u32,
    mode: IngestMode,
}

#[allow(dead_code)]
impl IngestState {
    /// Feed the latest `try_pull` outcome into the state machine and
    /// return the post-transition [`IngestMode`].
    pub fn next_state(&mut self, status: IngestStatus) -> IngestMode {
        match status {
            IngestStatus::Ready => {
                self.consecutive_underruns = 0;
                self.mode = IngestMode::Normal;
            }
            IngestStatus::Pending => {}
            IngestStatus::WouldBlock | IngestStatus::Closed => {
                self.consecutive_underruns = self.consecutive_underruns.saturating_add(1);
                if self.consecutive_underruns >= 2 {
                    self.mode = IngestMode::Degraded;
                }
            }
        }
        self.mode
    }

    /// Current mode. Useful for callers that want to peek at the
    /// mode without feeding a new outcome.
    #[must_use]
    pub fn mode(&self) -> IngestMode {
        self.mode
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod observability_emission {
    //! Evidence: EV-OBSERVABILITY (T1) — audio emission site against
    //! docs/31 §Phase-1 baseline catalog (ingest_frames,
    //! ingest_samples, ingest_state_transitions; tracker is a
    //! slice-2 synthesis per OSC adapter precedent).
    //!
    //! T1 per `docs/43 §15.5`: each test pins either a catalog-row
    //! invariant (`ingest_frames_total` `status` label increments,
    //! `ingest_samples_total` delta on `Ready`) or a documented
    //! state-machine invariant on the slice-2
    //! [`AudioIngestStateTracker`] synthesis (`Starting -> Running`
    //! on first `Ready`, `Running -> Draining` after two
    //! consecutive non-`Ready`, `Closed` terminal). The tracker
    //! itself is a best-effort slice-2 synthesis per the OSC
    //! adapter precedent (`engine/observability/src/catalog.rs`
    //! lines 167-170 `OscAdapterState` docstring): the canonical
    //! audio-pipeline lifecycle reifies with the slice-3
    //! scheduler, at which point this module's `TODO(slice-3)`
    //! markers reconcile.

    use chromaticity_observability::catalog::{AudioIngestFrameStatus, AudioIngestState};

    use crate::frame::AudioFrame;
    use crate::ingest::{IngestBoundary, IngestMode, IngestStatus};
    use crate::observability::{
        self, AudioIngestStateTracker, current_ingest_mode, frames_count,
        ingest_mode_frames_in_current_count, ingest_mode_transition_count, samples_count,
    };
    use crate::silent::SilentSource;

    #[test]
    fn silent_source_ready_increments_ready_frames_and_samples_by_128() {
        let _serialize = observability::test_serialize_lock();

        let before_ready = frames_count(AudioIngestFrameStatus::Ready);
        let before_samples = samples_count();

        let mut source = SilentSource::new(1);
        let mut frame = AudioFrame::default();
        assert_eq!(source.try_pull(&mut frame), IngestStatus::Ready);

        let after_ready = frames_count(AudioIngestFrameStatus::Ready);
        let after_samples = samples_count();

        // SilentSource publishes a single 128-sample silent block
        // per `Ready` at DEFAULT_SAMPLE_RATE_HZ (silent.rs:52),
        // matching docs/06 §7.1 framing baseline.
        assert_eq!(after_ready - before_ready, 1);
        assert_eq!(after_samples - before_samples, 128);
    }

    #[test]
    fn silent_source_exhaustion_increments_closed_frames() {
        let _serialize = observability::test_serialize_lock();

        let before_closed = frames_count(AudioIngestFrameStatus::Closed);
        let before_samples = samples_count();

        // Hop budget 0 -> first try_pull is Closed.
        let mut source = SilentSource::new(0);
        let mut frame = AudioFrame::default();
        assert_eq!(source.try_pull(&mut frame), IngestStatus::Closed);

        let after_closed = frames_count(AudioIngestFrameStatus::Closed);
        let after_samples = samples_count();

        assert_eq!(after_closed - before_closed, 1);
        // Closed MUST NOT advance the samples counter per docs/31
        // §Phase-1 baseline catalog `ingest_samples_total` row
        // ("Monotonic total PCM samples delivered").
        assert_eq!(after_samples, before_samples);
    }

    #[test]
    fn tracker_transitions_starting_to_running_on_first_ready() {
        // Fresh tracker; no serialize lock needed (local state
        // only, no interaction with the process-wide counters).
        let mut tracker = AudioIngestStateTracker::new();
        assert_eq!(tracker.current_state(), AudioIngestState::Starting);
        assert_eq!(
            tracker.observe(AudioIngestFrameStatus::Ready),
            AudioIngestState::Running,
        );
        assert_eq!(
            tracker.transition_count(AudioIngestState::Starting, AudioIngestState::Running),
            1,
        );
    }

    #[test]
    fn tracker_transitions_running_to_draining_after_two_consecutive_nonready() {
        let mut tracker = AudioIngestStateTracker::new();
        // Leave Starting via Ready so the two-consecutive rule
        // applies from Running (§7.11 parity with IngestState).
        tracker.observe(AudioIngestFrameStatus::Ready);
        assert_eq!(tracker.current_state(), AudioIngestState::Running);
        // First non-Ready does not yet flip the state.
        assert_eq!(
            tracker.observe(AudioIngestFrameStatus::WouldBlock),
            AudioIngestState::Running,
        );
        // Second consecutive non-Ready (mixed kinds count the
        // same, matching IngestState) crosses the threshold.
        assert_eq!(
            tracker.observe(AudioIngestFrameStatus::Closed),
            AudioIngestState::Draining,
        );
        assert_eq!(
            tracker.transition_count(AudioIngestState::Running, AudioIngestState::Draining),
            1,
        );
    }

    #[test]
    fn tracker_closed_is_terminal() {
        let mut tracker = AudioIngestStateTracker::new();
        // Drive Starting -> Running -> Draining -> Closed via the
        // slice-2 mapping: first Ready promotes to Running, two
        // consecutive Closed promotes to Draining, a third Closed
        // while Draining drops us into Closed.
        tracker.observe(AudioIngestFrameStatus::Ready);
        tracker.observe(AudioIngestFrameStatus::Closed);
        tracker.observe(AudioIngestFrameStatus::Closed);
        assert_eq!(tracker.current_state(), AudioIngestState::Draining);
        tracker.observe(AudioIngestFrameStatus::Closed);
        assert_eq!(tracker.current_state(), AudioIngestState::Closed);

        let terminal_transitions =
            tracker.transition_count(AudioIngestState::Draining, AudioIngestState::Closed);
        assert_eq!(terminal_transitions, 1);

        // Subsequent observations are no-ops: the tracker stays
        // Closed and no further transition counts accrue.
        tracker.observe(AudioIngestFrameStatus::Ready);
        assert_eq!(tracker.current_state(), AudioIngestState::Closed);
        tracker.observe(AudioIngestFrameStatus::WouldBlock);
        assert_eq!(tracker.current_state(), AudioIngestState::Closed);

        // `Draining -> Running` would have incremented had the
        // Ready above been honoured; the invariant is that it
        // stayed at the one `Draining -> Closed` recorded above.
        assert_eq!(
            tracker.transition_count(AudioIngestState::Draining, AudioIngestState::Running),
            0,
        );
        assert_eq!(
            tracker.transition_count(AudioIngestState::Draining, AudioIngestState::Closed),
            terminal_transitions,
        );

        // Sanity: transition_count for an unreachable pair reads
        // 0 via the wildcard arm.
        assert_eq!(
            tracker.transition_count(AudioIngestState::Starting, AudioIngestState::Closed),
            0,
        );
    }

    // ---- Slice-2 IngestMode observability widening pins ----

    #[test]
    fn ingest_mode_dwell_increments_on_each_pull_outcome() {
        let _serialize = observability::test_serialize_lock();
        let before_dwell = ingest_mode_frames_in_current_count();

        let mut source = SilentSource::new(1);
        let mut frame = AudioFrame::default();
        // One Ready outcome advances the dwell counter by 1 (or
        // resets it to 1 if it caused a transition out of the
        // initial Normal state — which it can't, since Ready
        // always leaves us in Normal).
        assert_eq!(source.try_pull(&mut frame), IngestStatus::Ready);

        let after_dwell = ingest_mode_frames_in_current_count();
        // Either the dwell increased by exactly 1 (no transition),
        // or it reset to 1 (transition Degraded -> Normal). Both
        // are valid post-transition states; the load-bearing pin
        // is "the slice-2 widening makes the dwell counter
        // mutate on every outcome".
        assert!(
            after_dwell == before_dwell + 1 || after_dwell == 1,
            "dwell counter did not advance after Ready outcome (before={before_dwell}, after={after_dwell})",
        );
        assert_eq!(current_ingest_mode(), IngestMode::Normal);
    }

    #[test]
    fn ingest_mode_transition_normal_to_degraded_increments_on_two_consecutive_nonready() {
        let _serialize = observability::test_serialize_lock();
        let before_normal_to_degraded =
            ingest_mode_transition_count(IngestMode::Normal, IngestMode::Degraded);

        // SilentSource::new(0) yields Closed on the very first
        // try_pull. To trigger Normal -> Degraded we need two
        // consecutive non-Ready outcomes; SilentSource gives us
        // exactly that: hop=0 yields Closed (1st non-Ready) and
        // every subsequent try_pull also yields Closed (2nd, 3rd,
        // ...). Only need 2 calls total to cross the §7.11
        // threshold; the global tracker may carry residual
        // consecutive-underrun state from prior tests, so emit
        // enough Closeds to guarantee a transition into Degraded.
        let mut source = SilentSource::new(0);
        let mut frame = AudioFrame::default();
        for _ in 0..3 {
            // 3 consecutive Closeds is enough to cross the §7.11
            // threshold no matter the prior tracker state.
            let _ = source.try_pull(&mut frame);
        }

        // After 3 consecutive non-Ready outcomes, the tracker MUST
        // be in Degraded and at least one Normal -> Degraded
        // transition MUST have been recorded since this test
        // started (or earlier — counter is monotonic across the
        // process; the pin is "delta is at least 1 if the test
        // crossed the threshold").
        assert_eq!(current_ingest_mode(), IngestMode::Degraded);
        let after_normal_to_degraded =
            ingest_mode_transition_count(IngestMode::Normal, IngestMode::Degraded);
        assert!(
            after_normal_to_degraded > before_normal_to_degraded,
            "Normal->Degraded delta expected > 0, before={before_normal_to_degraded} after={after_normal_to_degraded}",
        );
    }

    #[test]
    fn ingest_mode_transition_degraded_to_normal_increments_on_first_ready() {
        let _serialize = observability::test_serialize_lock();

        // Drive the tracker to Degraded via two Closeds.
        let mut closed_source = SilentSource::new(0);
        let mut frame = AudioFrame::default();
        for _ in 0..3 {
            let _ = closed_source.try_pull(&mut frame);
        }
        assert_eq!(current_ingest_mode(), IngestMode::Degraded);

        let before_degraded_to_normal =
            ingest_mode_transition_count(IngestMode::Degraded, IngestMode::Normal);

        // First Ready returns the tracker to Normal.
        let mut ready_source = SilentSource::new(1);
        assert_eq!(ready_source.try_pull(&mut frame), IngestStatus::Ready);

        assert_eq!(current_ingest_mode(), IngestMode::Normal);
        let after_degraded_to_normal =
            ingest_mode_transition_count(IngestMode::Degraded, IngestMode::Normal);
        assert_eq!(
            after_degraded_to_normal - before_degraded_to_normal,
            1,
            "Degraded->Normal delta must be exactly 1 on first Ready",
        );
    }

    #[test]
    fn ingest_mode_self_loops_do_not_count_as_transitions() {
        let _serialize = observability::test_serialize_lock();
        // First, ensure we're in Normal.
        let mut source = SilentSource::new(1);
        let mut frame = AudioFrame::default();
        assert_eq!(source.try_pull(&mut frame), IngestStatus::Ready);
        assert_eq!(current_ingest_mode(), IngestMode::Normal);

        let before = ingest_mode_transition_count(IngestMode::Normal, IngestMode::Degraded)
            + ingest_mode_transition_count(IngestMode::Degraded, IngestMode::Normal);

        // Second Ready (self-loop Normal -> Normal); SilentSource
        // hop budget exhausted, but next call is Closed, not Ready.
        // To stay in Normal across multiple outcomes we need
        // multiple Ready outcomes. Use SilentSource::new(3) for
        // three consecutive Readys.
        let mut multi = SilentSource::new(3);
        let _ = multi.try_pull(&mut frame); // Ready
        let _ = multi.try_pull(&mut frame); // Ready
        let _ = multi.try_pull(&mut frame); // Ready
        assert_eq!(current_ingest_mode(), IngestMode::Normal);

        let after = ingest_mode_transition_count(IngestMode::Normal, IngestMode::Degraded)
            + ingest_mode_transition_count(IngestMode::Degraded, IngestMode::Normal);
        // Three Ready outcomes from the multi source while already
        // in Normal MUST NOT increment any transition counter.
        assert_eq!(
            after, before,
            "Self-loops Normal->Normal must not bump transition counters",
        );
    }
}

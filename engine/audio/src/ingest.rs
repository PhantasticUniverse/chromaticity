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
//! two consecutive non-[`IngestStatus::Ready`] outcomes flip the state
//! into [`IngestMode::Degraded`]; the first [`IngestStatus::Ready`]
//! returns it to [`IngestMode::Normal`]. This is the ingest-side half
//! of the §7.11 invariant; the consumer-side snapshot handling lives
//! in the frame scheduler (not this crate).
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
/// gets a fresh frame, observes a transient gap it can tolerate, or
/// learns the source has gone away and it must switch to the silent
/// fallback uniforms from the reliability policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestStatus {
    /// A fresh frame was produced.
    Ready,
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
/// consecutive `WouldBlock` / `Closed` outcomes.
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
/// `Degraded` — two or more consecutive non-`Ready` outcomes have been
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

/// Ingest-side §7.11 state machine tracking consecutive non-`Ready`
/// outcomes.
///
/// Slice-2 pins the rule: two consecutive non-`Ready` (either
/// `WouldBlock` or `Closed`) outcomes flip the mode to
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
    use crate::ingest::{IngestBoundary, IngestStatus};
    use crate::observability::{self, AudioIngestStateTracker, frames_count, samples_count};
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
}

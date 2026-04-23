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

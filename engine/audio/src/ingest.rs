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
//! First-slice intent is to name the boundary, not to implement the
//! triple-buffered ring or the actual device capture. Concrete
//! `IngestBoundary` implementations (cpal-backed device ingest, WAV
//! fixture playback for `EV-AUDIO-TRACE`) arrive in later slices.

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

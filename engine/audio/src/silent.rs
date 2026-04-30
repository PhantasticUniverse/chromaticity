// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Silent fixture-style ingest source (production path).
//!
//! [`SilentSource`] is the Phase-1 slice-2 production implementation of
//! [`crate::ingest::IngestBoundary`] that yields a fixed number of zero
//! frames before reporting [`crate::ingest::IngestStatus::Closed`]. It
//! graduated from its slice-1 test-double incarnation when slice-2
//! promoted the `IngestBoundary` trait to first non-stub status; the
//! behaviour and shape are unchanged so the slice-1 drift pins keep
//! their meaning across the move.
//!
//! The slice-2 production call path is deliberately narrow: the
//! `SilentSource` serves as the fallback source the analyser selects
//! when the real device ingest (cpal-backed, slice-3) is unavailable
//! or has entered the Reliability/Fallback Policy degraded mode. Because the buffer the
//! source yields is a single 128-sample silent block at
//! [`crate::frame::DEFAULT_SAMPLE_RATE_HZ`], no allocation happens
//! beyond the initial frame construction.

use crate::frame::{AudioFrame, DEFAULT_SAMPLE_RATE_HZ};
use crate::ingest::{IngestBoundary, IngestStatus};

/// Silent ingest source that produces `remaining_hops` silent frames,
/// then reports [`IngestStatus::Closed`].
///
/// Slice-2 uses this as the production fallback while the real
/// device-capture ingest lands in slice-3. Construction takes the hop
/// budget; the source consumes one hop per [`IngestBoundary::try_pull`]
/// call.
#[derive(Debug)]
pub struct SilentSource {
    remaining_hops: u32,
}

impl SilentSource {
    /// Construct a `SilentSource` that yields `remaining_hops` silent
    /// frames before closing.
    #[must_use]
    pub fn new(remaining_hops: u32) -> Self {
        Self { remaining_hops }
    }
}

impl IngestBoundary for SilentSource {
    fn try_pull(&mut self, out: &mut AudioFrame) -> IngestStatus {
        let status = if self.remaining_hops == 0 {
            IngestStatus::Closed
        } else {
            self.remaining_hops -= 1;
            *out = AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, vec![0.0; 128]);
            IngestStatus::Ready
        };
        // Slice-2 `EV-OBSERVABILITY` emission site: every
        // `IngestBoundary::try_pull` outcome drives the three
        // `docs/31` §Phase-1 baseline catalog audio counters
        // (ingest_frames, ingest_samples, ingest_state_transitions)
        // through the process-wide observability tracker. The
        // samples-count is sourced from the populated frame only
        // on the `Ready` branch; `Closed` leaves `out` unchanged
        // and `record_pull_outcome` ignores the argument.
        crate::observability::record_pull_outcome(&status, out.samples().len());
        status
    }
}

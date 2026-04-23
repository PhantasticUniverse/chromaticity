// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine audio.
//!
//! First-slice surface: the [`frame::AudioFrame`] dataflow type and
//! the [`ingest::IngestBoundary`] trait that names the audio-thread
//! hand-off. The Phase-2 analysis pipeline (FFT, band aggregation,
//! uniform publication) and the real device capture (cpal, WAV
//! fixture playback) arrive in later slices.
//!
//! Evidence: EV-AUDIO-TRACE — baseline placeholder for audio fixture
//! trace comparisons.
//!
//! Governing documents:
//!
//! - `docs/06-audio-engine.md` — audio engine contract, especially
//!   §7.1 (ingest and framing) and §7.11 (snapshot handoff).
//! - `docs/28-implementation-contract-pack.md` CP-02a — audio
//!   snapshot handoff contract.
//! - `docs/adr/ADR-011-rendering-backend.md` — audio/ingest
//!   integration points.

pub mod frame;
pub mod ingest;

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-AUDIO-TRACE — baseline placeholder for audio
    //! fixture trace comparisons. These tests exercise the first-slice
    //! types so later slices (real ingest + analyser) inherit a
    //! regression net rather than a clean sheet.

    use super::frame::{AudioFrame, DEFAULT_SAMPLE_RATE_HZ};
    use super::ingest::{IngestBoundary, IngestStatus};

    #[test]
    fn default_frame_matches_docs_06_baseline() {
        let frame = AudioFrame::default();
        assert_eq!(frame.sample_rate_hz(), DEFAULT_SAMPLE_RATE_HZ);
        assert_eq!(frame.sample_rate_hz(), 48_000);
        assert_eq!(frame.channels(), 1);
        assert!(frame.samples().is_empty());
    }

    #[test]
    fn frame_preserves_constructor_fields() {
        let frame = AudioFrame::new(44_100, 2, vec![0.0, 0.0, 0.5, -0.5]);
        assert_eq!(frame.sample_rate_hz(), 44_100);
        assert_eq!(frame.channels(), 2);
        assert_eq!(frame.samples().len(), 4);
    }

    /// Silent fixture-style ingest source. Serves as the minimal
    /// `IngestBoundary` implementation EV-AUDIO-TRACE will later use
    /// to drive deterministic fixture traces.
    struct SilentSource {
        remaining_hops: u32,
    }

    impl IngestBoundary for SilentSource {
        fn try_pull(&mut self, out: &mut AudioFrame) -> IngestStatus {
            if self.remaining_hops == 0 {
                return IngestStatus::Closed;
            }
            self.remaining_hops -= 1;
            *out = AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, vec![0.0; 128]);
            IngestStatus::Ready
        }
    }

    #[test]
    fn ingest_boundary_reports_ready_then_closed() {
        let mut source = SilentSource { remaining_hops: 1 };
        let mut frame = AudioFrame::default();

        assert_eq!(source.try_pull(&mut frame), IngestStatus::Ready);
        assert_eq!(frame.sample_rate_hz(), DEFAULT_SAMPLE_RATE_HZ);
        assert_eq!(frame.samples().len(), 128);

        assert_eq!(source.try_pull(&mut frame), IngestStatus::Closed);
    }
}

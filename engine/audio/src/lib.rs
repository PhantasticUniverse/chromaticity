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
mod drift_pins {
    //! Evidence: EV-AUDIO-TRACE (T1) — drift pins on the
    //! default-frame baseline and the ingest Ready → Closed
    //! transition.
    //!
    //! T1 per docs/43 §15.5: tests pin the
    //! `docs/06-audio-engine.md` §7.1 default-frame baseline
    //! (sample rate, channel count, empty sample buffer on a silent
    //! sentinel) and the §7.11 non-blocking pull state machine
    //! (Ready on a frame, Closed on source exhaustion). Either pin
    //! fires the moment the canonical baseline or the ingest-boundary
    //! state machine drifts from the governing doc. T1 counts toward
    //! `EV-EVIDENCE-TAG-COVERAGE`; Phase-1 exit requires T2 over
    //! the audio fixture corpus.

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

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod ctor_placeholder {
    //! Evidence: EV-AUDIO-TRACE (T0) — round-trip ctor sanity;
    //! does not advance EV row.
    //!
    //! T0 per docs/43 §15.5: asserts that `AudioFrame::new` stores
    //! its three authoritative fields unchanged and returns them via
    //! the public accessors. The subject module's public surface is
    //! a data-shape declaration with no non-derive-generated
    //! behaviour beyond field storage, which is exactly what T0 is
    //! defined to cover. T0 does NOT count toward
    //! `EV-EVIDENCE-TAG-COVERAGE`; upgrade path is T1 or T2 once the
    //! ingest pipeline lands real constructor-time invariants to
    //! pin.

    use super::frame::AudioFrame;

    #[test]
    fn frame_preserves_constructor_fields() {
        let frame = AudioFrame::new(44_100, 2, vec![0.0, 0.0, 0.5, -0.5]);
        assert_eq!(frame.sample_rate_hz(), 44_100);
        assert_eq!(frame.channels(), 2);
        assert_eq!(frame.samples().len(), 4);
    }
}

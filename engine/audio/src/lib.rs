// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine audio.
//!
//! Phase-1 slice-2 surface:
//!
//! - [`frame::AudioFrame`] and the `DEFAULT_SAMPLE_RATE_HZ` baseline.
//! - [`ingest::IngestBoundary`] trait plus the slice-2 degraded-mode
//!   state machine ([`ingest::IngestMode`] / [`ingest::IngestState`])
//!   that implements the `docs/06-audio-engine.md` §7.11 "two
//!   consecutive non-Ready outcomes" rule.
//! - [`silent::SilentSource`] — the Phase-1 production-path silent
//!   ingest that graduated from its slice-1 test-double incarnation.
//!   Slice-2 wires it as the fallback source the analyser uses until
//!   the real device-capture ingest lands in slice-3.
//! - [`observability`] — the slice-2 `EV-OBSERVABILITY` emission
//!   site for `chromaticity_audio_ingest_frames_total`,
//!   `chromaticity_audio_ingest_samples_total`, and
//!   `chromaticity_audio_ingest_state_transitions_total`. Every
//!   [`ingest::IngestBoundary::try_pull`] implementor routes its
//!   outcome through the module's `record_pull_outcome` at the
//!   outcome boundary so the three `docs/31` §Phase-1 baseline
//!   catalog audio counters accumulate for the Phase-1 exit gate.
//!   The [`observability::AudioIngestStateTracker`] is the slice-2
//!   synthesised lifecycle (`Starting -> Running -> Draining ->
//!   Closed`) per the module-top TODO(slice-3) reconciliation.
//!
//! Slice-2 dep-slot: `cpal 0.15` is authorised in `Cargo.toml` as
//! a dep-slot only — no cpal type is imported and no cpal function
//! is invoked from any file in this crate. The slice-3 author who
//! lands the first real `cpal::Stream` callback MUST honour the
//! FFI-seam `catch_unwind` discipline pinned as a standing
//! constraint at the top of [`ingest`]; reviewers treat omission as
//! a merge blocker. See that module's `# Standing constraint —
//! cpal FFI-seam panic discipline` section.
//!
//! Produces evidence for `EV-AUDIO-TRACE` (Phase-2-exit conformance
//! row). Slice-2 advances the T1 drift-pin coverage alongside the
//! ingest state machine; see `drift_pins` below for the tiered
//! claim.
//!
//! Governing documents:
//!
//! - `docs/06-audio-engine.md` — audio engine contract, especially
//!   §7.1 (ingest and framing) and §7.11 (snapshot handoff and
//!   degraded modes).
//! - `docs/28-implementation-contract-pack.md` CP-02a — audio
//!   snapshot handoff contract.
//! - `docs/adr/ADR-011-rendering-backend.md` — audio / ingest
//!   integration points.

pub mod frame;
pub mod ingest;
pub mod observability;
pub mod silent;

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-AUDIO-TRACE (T1) — drift pins on the default-frame
    //! baseline, the `SilentSource` production-path hop budget, the
    //! `IngestBoundary` Ready → Closed transition, and the
    //! `IngestState` §7.11 Normal ↔ Degraded rule.
    //!
    //! T1 per docs/43 §15.5: every test pins either an external-spec
    //! constant (`docs/06-audio-engine.md` §7.1 default-frame baseline)
    //! or a documented state-machine invariant (§7.11 two-consecutive
    //! non-Ready → Degraded, first Ready → Normal) under the
    //! slice-2 minimal implementation. Any pin fires the moment the
    //! canonical baseline or the documented state transition drifts.
    //! T1 counts toward `EV-EVIDENCE-TAG-COVERAGE`; Phase-1 exit
    //! requires T2 over the full audio fixture corpus.

    use super::frame::{AudioFrame, DEFAULT_SAMPLE_RATE_HZ};
    use super::ingest::{IngestBoundary, IngestMode, IngestState, IngestStatus};
    use super::observability;
    use super::silent::SilentSource;

    #[test]
    fn default_frame_matches_docs_06_baseline() {
        let frame = AudioFrame::default();
        assert_eq!(frame.sample_rate_hz(), DEFAULT_SAMPLE_RATE_HZ);
        assert_eq!(frame.sample_rate_hz(), 48_000);
        assert_eq!(frame.channels(), 1);
        assert!(frame.samples().is_empty());
    }

    #[test]
    fn ingest_boundary_reports_ready_then_closed() {
        // Serialize against the observability emission-site tests;
        // every try_pull call mutates the process-wide tracker.
        let _serialize = observability::test_serialize_lock();
        let mut source = SilentSource::new(1);
        let mut frame = AudioFrame::default();

        assert_eq!(source.try_pull(&mut frame), IngestStatus::Ready);
        assert_eq!(frame.sample_rate_hz(), DEFAULT_SAMPLE_RATE_HZ);
        assert_eq!(frame.samples().len(), 128);

        assert_eq!(source.try_pull(&mut frame), IngestStatus::Closed);
    }

    #[test]
    fn silent_source_honors_hop_count() {
        let _serialize = observability::test_serialize_lock();
        let mut source = SilentSource::new(3);
        let mut frame = AudioFrame::default();

        for _ in 0..3 {
            assert_eq!(source.try_pull(&mut frame), IngestStatus::Ready);
        }
        assert_eq!(source.try_pull(&mut frame), IngestStatus::Closed);
        assert_eq!(source.try_pull(&mut frame), IngestStatus::Closed);
    }

    #[test]
    fn ingest_state_enters_degraded_after_two_consecutive_nonready() {
        let mut state = IngestState::default();
        assert_eq!(state.mode(), IngestMode::Normal);

        // First non-Ready does not yet flip the mode.
        assert_eq!(
            state.next_state(IngestStatus::WouldBlock),
            IngestMode::Normal
        );
        // Second consecutive non-Ready crosses the §7.11 threshold.
        assert_eq!(
            state.next_state(IngestStatus::WouldBlock),
            IngestMode::Degraded
        );
    }

    #[test]
    fn ingest_state_returns_to_normal_on_first_ready() {
        let mut state = IngestState::default();
        state.next_state(IngestStatus::WouldBlock);
        state.next_state(IngestStatus::WouldBlock);
        assert_eq!(state.mode(), IngestMode::Degraded);

        // A single Ready clears the consecutive-underrun counter and
        // restores the mode.
        assert_eq!(state.next_state(IngestStatus::Ready), IngestMode::Normal);
    }

    #[test]
    fn ingest_state_treats_closed_as_degrading_input() {
        let mut state = IngestState::default();

        // `Closed` is a non-Ready outcome under §7.11; it counts
        // toward the consecutive-underrun threshold the same way
        // `WouldBlock` does.
        assert_eq!(state.next_state(IngestStatus::Closed), IngestMode::Normal);
        assert_eq!(state.next_state(IngestStatus::Closed), IngestMode::Degraded);
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

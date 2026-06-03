// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine audio.
//!
//! # Slice progression
//!
//! - **Phase-1 slice-1** (historic): seeded `frame` and the test-double
//!   silent ingest source.
//! - **Phase-1 slice-2** (historic): promoted `IngestBoundary` to first
//!   non-stub status, added the `IngestMode` / `IngestState` Reliability/Fallback Policy
//!   degraded-mode accounting, graduated `SilentSource` to the
//!   production fallback path, and stood up the
//!   `EV-OBSERVABILITY` emission site (`chromaticity_audio_ingest_*`
//!   counters in `observability`).
//! - **Phase-2 EV-AUDIO-TRACE slice-1**: adds the `uniforms`
//!   module, the producer side of the audio → render uniform
//!   marshalling boundary. Pure-data
//!   [`uniforms::AudioUniformSnapshot`] + the
//!   [`uniforms::extract_uniform_snapshot`] entry point produce four
//!   audio analysis primitives (BPM clamped `[50, 220]`, raw RMS
//!   energy, peak magnitude, 8-band log-spaced FFT magnitudes) over
//!   the caller-supplied [`frame::AudioFrame`]. The production
//!   runtime supplies its rolling analysis window for these fields.
//!   The cross-crate adapter (engine/render slice-5a) maps these
//!   onto the `syn_BPM` compat-syn-v1 retention plus
//!   chromaticity-native `chroma_*` audio bindings. Cross-crate
//!   coupling stays consumer-side: `engine/render` (or any other
//!   consumer) lifts the snapshot into its own
//!   [`UniformValue`-shaped] uniform-buffer marshalling, so
//!   `engine/audio` does NOT take a `chromaticity-render` dep. See
//!   the `uniforms` module-level prose for the algorithm /
//!   normalisation choices.
//! - **Phase-2 EV-AUDIO-TRACE slice-2** (this slice): deepens the
//!   BPM surface and widens audio observability:
//!   - [`bpm::BpmEstimator`] history-window median-smoothed BPM
//!     estimator over the last `N` per-frame autocorrelation
//!     results (default 8 frames, configurable `1..=128` via
//!     [`bpm::BpmEstimator::with_window`]). The slice-1 pure
//!     [`uniforms::extract_uniform_snapshot`] entry point STAYS
//!     UNCHANGED — callers that don't want history continue to use
//!     it directly. The estimator reuses the same per-frame
//!     autocorrelation algorithm via the `pub(crate)`
//!     `uniforms::compute_bpm_autocorr_for_frame` helper; the
//!     slice-1 single-frame path remains the deterministic anchor
//!     that the slice-2 estimator builds on.
//!   - [`error::AudioError`] structured error type
//!     (`#[non_exhaustive]`) introduces the first failure surface
//!     in the audio crate. Slice-2 contributes the
//!     [`error::AudioError::WindowConfig`] variant for
//!     window-validation rejection; slice-3+ extends as needed
//!     (cpal callback bind failures, etc.).
//!   - [`observability`] widening: the Reliability/Fallback Policy [`ingest::IngestMode`]
//!     transition counter ([`observability::ingest_mode_transition_count`])
//!     and the in-current-mode dwell counter
//!     ([`observability::ingest_mode_frames_in_current_count`]) ride
//!     the existing `record_pull_outcome` lock-hold so every
//!     [`ingest::IngestBoundary::try_pull`] outcome accumulates
//!     mode-tracker state alongside the slice-1 frame / sample
//!     counters. Distinct from the slice-1
//!     [`observability::AudioIngestStateTracker`] which tracks the
//!     `Starting/Running/Draining/Closed` lifecycle synthesis; the
//!     two share the Reliability/Fallback Policy "two consecutive non-`Ready`" rule and
//!     route through the same emission site.
//!
//!   Deferred to follow-up slices: ingest-mode capability
//!   negotiation (slice-3a, this slice), cpal device-capture
//!   integration (slice-3b, future scope), and the cross-crate
//!   audio→render adapter (`engine/render` slice 5a, landed
//!   separately in PR number 269).
//! - **Phase-2 EV-AUDIO-TRACE slice-3a**: adds the
//!   [`ingest_negotiation`] capability-negotiation surface. The
//!   [`ingest_negotiation::IngestNegotiator`] wraps a primary
//!   [`ingest::IngestBoundary`] plus an internal
//!   [`silent::SilentSource`] fallback and applies a configurable
//!   [`ingest_negotiation::IngestModeCriteria`] policy: when the
//!   primary's consecutive-failure count crosses
//!   [`ingest_negotiation::IngestModeCriteria::degraded_after`]
//!   (default 2, matching the Reliability/Fallback Policy baseline) the negotiator flips
//!   to the fallback and serves silent frames; when the primary
//!   recovers (consecutive-success threshold
//!   [`ingest_negotiation::IngestModeCriteria::normal_after`],
//!   default 1) the negotiator returns to the primary. Slice-3a
//!   keeps slice-2 process-wide counter semantics unchanged: each
//!   underlying source's `try_pull` still calls
//!   `record_pull_outcome`, so the `chromaticity_audio_ingest_*`
//!   totals continue to accumulate verbatim. The
//!   [`ingest_negotiation::IngestSource`] enum reserves a
//!   `#[non_exhaustive]` `Cpal` slot for slice-3b without committing
//!   to the cpal integration in this slice. `AudioError` widens with
//!   the `IngestNegotiation` variant for builder validation.
//!
//! - **Phase-2 EV-AUDIO-TRACE slice-3b** (this slice): activates the
//!   slice-2 cpal `dep-slot` and lands the first real `cpal::Stream`
//!   callback under the FFI-seam `catch_unwind` discipline pinned at
//!   the top of [`ingest`]. The new [`cpal_source::CpalSource`]
//!   implements [`ingest::IngestBoundary`]: the cpal-owned audio
//!   thread pushes [`frame::AudioFrame`] values into a bounded
//!   `crossbeam-channel`, the analyser-side `try_pull` consumes from
//!   the receiver end, and a shared `AtomicBool` closed flag
//!   coerces caught panics / cpal `StreamError`s to
//!   [`ingest::IngestStatus::Closed`]. [`AudioError`] widens with
//!   [`error::AudioError::CpalDeviceCapture`] for the
//!   construction-time failure modes (device-not-found, unsupported
//!   sample format, stream-build / stream-play failure).
//!
//!   Slice-3b shipped `f32`-only at the device side. The follow-up
//!   multi-format widening (PR `#288` / `5ed1a6b`) lifts the
//!   `f32`-only restriction by dispatching over `cpal::SampleFormat`
//!   at stream-build time
//!   and supporting `f32` (passthrough), `i16`, and `u16` device
//!   formats with per-format conversion to `f32` inside the cpal
//!   callback before the frame is pushed into the bounded channel.
//!   Other cpal-reported sample formats (`i8` / `u8` / `i32` /
//!   `u32` / `i64` / `u64` / `f64`, plus any future additions)
//!   continue to surface as
//!   [`error::AudioError::CpalDeviceCapture`]. Multi-format support
//!   is strictly additive cross-platform robustness (Linux ALSA
//!   often defaults to `i16`, Windows WASAPI delivers `i16` / `u16`
//!   on some hardware) and does NOT flip the state of any
//!   `EV-AUDIO-TRACE` row in `docs/26`; it remains "future widening,
//!   NOT a named deferred slice" per the R-37 Tier-8 framing.
//!
//!   No remaining named deferred slices for `EV-AUDIO-TRACE` in this
//!   Phase-2 surface; further widening (device-enumeration UI,
//!   host-thread-bound stream owner for the macOS `!Send`
//!   cpal::Stream platform variant, multi-channel handling
//!   improvements, sample-rate resampling) is feature work tracked
//!   outside the slice-progression list.
//!
//! - **Phase-4a Wave 1c-α-2 production-bootstrap**: adds the
//!   [`runtime`] composition-layer entrypoint. Wave 1c-α shipped the
//!   cross-seam payload-shape contract for `EV-AUDIO-UNIFORM-IPC`
//!   (row 83) producer-side ([`runtime::AudioRuntime`] not yet wired)
//!   and consumer-side (`audioSnapshot.svelte.ts` Svelte store) but
//!   explicitly recorded the live end-to-end producer→Tauri-emit→
//!   consumer wiring as the first deferred axis. Bootstrap PR 1
//!   (`#343` / `804d6cd`) ships the producer-side runtime that wraps
//!   the existing [`cpal_source::try_default_input`] /
//!   [`ingest_negotiation::IngestNegotiator`] /
//!   [`silent::SilentSource`] / [`bpm::BpmEstimator`] /
//!   [`uniforms::extract_uniform_snapshot`] surfaces into a
//!   thread-spawnable unit owning an
//!   `Arc<Mutex<AudioUniformSnapshot>>` shared seam. Bootstrap PR 2
//!   (`#344` / `9a1131e`) wires the Tauri-side `IpcEmit` adapter +
//!   scheduler bootstrap; Bootstrap PR 3 lands the e2e IPC wire test
//!   plus the closing docs. The wave closed 2026-05-06 with row 83
//!   Pass at reduced narrowing (3 of 5 Wave 1c-α deferred axes drop;
//!   stale-but-last-good IPC surfacing through the wire, Spectrum
//!   widget retrofit, and `AudioReactivity` 48-bar FFT visualiser
//!   remain post-MVP).
//!
//! # Surface summary
//!
//! - [`frame::AudioFrame`] and the `DEFAULT_SAMPLE_RATE_HZ` baseline.
//! - [`ingest::IngestBoundary`] trait plus the slice-2 degraded-mode
//!   state machine ([`ingest::IngestMode`] / [`ingest::IngestState`])
//!   that implements the `docs/06-audio-engine.md` Reliability/Fallback Policy "two
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
//! - [`uniforms`] — Phase-2 EV-AUDIO-TRACE slice-1 producer side
//!   (`AudioUniformSnapshot`, `extract_uniform_snapshot`).
//! - [`bpm`] — Phase-2 EV-AUDIO-TRACE slice-2 history-window
//!   median-smoothed BPM estimator
//!   ([`bpm::BpmEstimator`]).
//! - [`error`] — slice-2 structured error type
//!   ([`error::AudioError`]).
//!
//! Slice-3b activates the slice-2 `cpal 0.15` dep-slot. The
//! [`cpal_source`] module imports cpal types and constructs
//! `cpal::Stream` callbacks under the FFI-seam `catch_unwind`
//! discipline pinned as a standing constraint at the top of
//! [`ingest`]; reviewers treat omission as a merge blocker. See that
//! module's `# Standing constraint — cpal FFI-seam panic discipline`
//! section.
//!
//! Produces evidence for `EV-AUDIO-TRACE` (Phase-2-exit conformance
//! row). Phase-2 EV-AUDIO-TRACE slice-1 adds the producer-side T1
//! drift-pin coverage (`uniforms::uniforms_drift_pins`) plus the T2
//! corpus in `engine/audio/tests/uniforms.rs`; see the registry row
//! 30 follow-up commit for the artifact-pointer transition from
//! `pending:artifact`.
//!
//! Governing documents:
//!
//! - `docs/06-audio-engine.md` — audio engine contract, especially
//!   §7.1 (ingest and framing), §7.11 (cadence, snapshot handoff, and
//!   staleness bound), and the Reliability/Fallback Policy section
//!   (degraded-mode fall-back).
//! - `docs/28-implementation-contract-pack.md` CP-02a — audio
//!   snapshot handoff contract.
//! - `docs/36-reactive-symbol-catalog.md` v1.3.0 — `syn_BPM` /
//!   `syn_InputLevel` / `syn_Spectrum` family vocabulary the
//!   slice-1 uniforms map onto.
//! - `docs/adr/ADR-011-rendering-backend.md` — audio / ingest
//!   integration points.
//!
//! [`UniformValue`-shaped]: chromaticity-render's `UniformValue` enum,
//! consumed by callers that elect the audio→render coupling.

pub mod bpm;
pub mod cpal_source;
pub mod error;
pub mod frame;
pub mod ingest;
pub mod ingest_negotiation;
pub mod observability;
pub mod reactive_bands;
pub mod runtime;
pub mod silent;
pub mod uniforms;

pub use bpm::BpmEstimator;
pub use cpal_source::{
    AUDIO_DIAG_ENV, AudioDeviceInfo, AudioDeviceKind, DiagCaptureStats, diag_capture_stats,
    diagnostics_enabled, enumerate_input_devices, enumerate_output_devices,
};
pub use error::AudioError;
pub use ingest_negotiation::{IngestModeCriteria, IngestNegotiator, IngestSource};
pub use reactive_bands::{
    REACTIVE_BAND_COUNT, REACTIVE_FFT_SIZE, ReactiveBandValues, ReactiveBands,
};
pub use uniforms::{AudioAxisAvailability, AudioUniformSnapshot, extract_uniform_snapshot};

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-AUDIO-TRACE (T1) — drift pins on the default-frame
    //! baseline, the `SilentSource` production-path hop budget, the
    //! `IngestBoundary` Ready → Closed transition, and the
    //! `IngestState` Reliability/Fallback Policy Normal ↔ Degraded rule.
    //!
    //! T1 per docs/43 §15.5: every test pins either an external-spec
    //! constant (`docs/06-audio-engine.md` §7.1 default-frame baseline)
    //! or a documented state-machine invariant (Reliability/Fallback Policy two-consecutive
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
        // Second consecutive non-Ready crosses the Reliability/Fallback Policy threshold.
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

        // `Closed` is a non-Ready outcome under Reliability/Fallback Policy; it counts
        // toward the consecutive-underrun threshold the same way
        // `WouldBlock` does.
        assert_eq!(state.next_state(IngestStatus::Closed), IngestMode::Normal);
        assert_eq!(state.next_state(IngestStatus::Closed), IngestMode::Degraded);
    }

    #[test]
    fn ingest_state_treats_pending_as_neutral_poll_gap() {
        let mut state = IngestState::default();
        assert_eq!(state.next_state(IngestStatus::Pending), IngestMode::Normal);
        assert_eq!(state.next_state(IngestStatus::Pending), IngestMode::Normal);
        assert_eq!(state.mode(), IngestMode::Normal);

        assert_eq!(
            state.next_state(IngestStatus::WouldBlock),
            IngestMode::Normal
        );
        assert_eq!(state.next_state(IngestStatus::Pending), IngestMode::Normal);
        assert_eq!(
            state.next_state(IngestStatus::WouldBlock),
            IngestMode::Degraded
        );
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

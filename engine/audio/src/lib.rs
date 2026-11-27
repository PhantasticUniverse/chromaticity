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
pub mod metering;
pub mod observability;
pub mod runtime;
pub mod silent;
pub mod spectrum;
pub mod uniforms;

pub use bpm::BpmEstimator;
pub use cpal_source::{
    AudioDeviceInfo, AudioDeviceKind, enumerate_input_devices, enumerate_output_devices,
};
pub use error::AudioError;
pub use ingest_negotiation::{IngestModeCriteria, IngestNegotiator, IngestSource};
pub use metering::{
    AgcNormalizer, AgcOutput, SpectralFluxOnsetDetector, StereoLevels, apply_input_gain,
    compute_stereo_levels, compute_true_peak_dbtp, db_to_linear,
};
pub use spectrum::{
    BpmEstimate, FusedBpmEstimator, SPECTRUM_BANDS_48, SPECTRUM_BANDS_64, compute_log_spectrum_64,
    downsample_64_to_48,
};
pub use uniforms::{AudioAxisAvailability, AudioUniformSnapshot, extract_uniform_snapshot};

/// Unified single-pass audio analysis snapshot combining legacy 4-band /
/// 8-band uniform meters, stereo true-peak / AGC metering, the 64-band
/// log-spaced FFT spectrum (plus 48-bar visualiser downsampling), and
/// two-stage fused BPM tracking.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtendedAudioFrame {
    /// Legacy producer-side uniform snapshot (`bpm`, `rms`, `peak`, `fft_bands[8]`).
    pub uniform_snapshot: AudioUniformSnapshot,
    /// Per-channel stereo RMS and peak magnitudes for the latest PCM block.
    pub stereo_levels: StereoLevels,
    /// 4× oversampled true peak in `dBTP` (`-100.0` floor on silence).
    pub true_peak_dbtp: f32,
    /// `true` when true peak reaches or exceeds `-1.0 dBTP` or discrete peak `>= 1.0`.
    pub clip: bool,
    /// Rolling p95 AGC normalisation and threshold-gate state.
    pub agc: AgcOutput,
    /// 4-band energy meters `[sub_bass, low_mid, high_mid, treble]` in `[0.0, 1.0]`
    /// aggregated from the 64-band log-spaced spectrum.
    pub four_band_energy: [f32; 4],
    /// 64-band log-spaced FFT spectrum over `20 Hz ..= 20 kHz`, dB-compressed into `[0.0, 1.0]`.
    pub spectrum_64: [f32; SPECTRUM_BANDS_64],
    /// Area-weighted 48-bar downsampled spectrum for visualiser widgets.
    pub spectrum_48: [f32; SPECTRUM_BANDS_48],
    /// Half-wave rectified spectral-flux onset strength in `[0.0, 1.0]`.
    pub onset_strength: f32,
    /// Two-stage fused IOI + autocorrelation BPM estimate with confidence and beat flag.
    pub bpm_estimate: BpmEstimate,
}

impl Default for ExtendedAudioFrame {
    fn default() -> Self {
        Self {
            uniform_snapshot: AudioUniformSnapshot::default(),
            stereo_levels: StereoLevels::default(),
            true_peak_dbtp: metering::TRUE_PEAK_FLOOR_DBTP,
            clip: false,
            agc: AgcOutput::default(),
            four_band_energy: [0.0; 4],
            spectrum_64: [0.0; SPECTRUM_BANDS_64],
            spectrum_48: [0.0; SPECTRUM_BANDS_48],
            onset_strength: 0.0,
            bpm_estimate: BpmEstimate::default(),
        }
    }
}

/// Stateful single-pass audio metering, 64-band log-spectrum, and fused
/// BPM engine.
///
/// Feed interleaved PCM frames via [`Self::process_frame`] (or
/// [`Self::process_pcm`]) to compute legacy 4-band/8-band meters,
/// stereo true-peak / AGC metering, 64-band & 48-bar log spectra, and
/// fused BPM confidence in one coherent pass without altering any
/// existing public audio types.
#[derive(Debug, Clone)]
pub struct AudioMeterEngine {
    mono_window_sr: Option<u32>,
    mono_window: std::collections::VecDeque<f32>,
    agc: AgcNormalizer,
    onset_detector: SpectralFluxOnsetDetector,
    fused_bpm: FusedBpmEstimator,
    prev_spectrum_64: [f32; SPECTRUM_BANDS_64],
    has_prev_spectrum: bool,
    input_gain_linear: f32,
    latest: ExtendedAudioFrame,
}

impl Default for AudioMeterEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioMeterEngine {
    /// Construct a fresh [`AudioMeterEngine`] with unity input gain and
    /// normative `docs/06` / ADR-023 metering defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mono_window_sr: None,
            mono_window: std::collections::VecDeque::with_capacity(spectrum::SPECTRUM_FFT_SIZE),
            agc: AgcNormalizer::new(),
            onset_detector: SpectralFluxOnsetDetector::new(),
            fused_bpm: FusedBpmEstimator::new(),
            prev_spectrum_64: [0.0; SPECTRUM_BANDS_64],
            has_prev_spectrum: false,
            input_gain_linear: 1.0,
            latest: ExtendedAudioFrame::default(),
        }
    }

    /// Set the linear pre-analysis input gain multiplier.
    /// Non-finite or negative values fall back to `1.0` (unity).
    pub fn set_input_gain_linear(&mut self, gain_linear: f32) {
        self.input_gain_linear = if gain_linear.is_finite() && gain_linear >= 0.0 {
            gain_linear
        } else {
            1.0
        };
    }

    /// Configure the AGC noise-gate threshold in `[0.0, 1.0]`.
    pub fn set_agc_threshold(&mut self, threshold: f32) {
        self.agc.set_threshold(threshold);
    }

    /// Reset all rolling windows, AGC history, onset state, and BPM lock.
    pub fn reset(&mut self) {
        self.mono_window_sr = None;
        self.mono_window.clear();
        self.agc.reset();
        self.onset_detector.reset();
        self.fused_bpm.reset();
        self.prev_spectrum_64 = [0.0; SPECTRUM_BANDS_64];
        self.has_prev_spectrum = false;
        self.latest = ExtendedAudioFrame::default();
    }

    /// Return the most recently computed [`ExtendedAudioFrame`].
    #[must_use]
    pub fn latest(&self) -> &ExtendedAudioFrame {
        &self.latest
    }

    /// Convenience wrapper that builds an [`frame::AudioFrame`] from raw
    /// interleaved PCM samples and runs [`Self::process_frame`].
    pub fn process_pcm(
        &mut self,
        sample_rate_hz: u32,
        channels: u16,
        interleaved_samples: &[f32],
    ) -> ExtendedAudioFrame {
        let frame = frame::AudioFrame::new(sample_rate_hz, channels, interleaved_samples.to_vec());
        self.process_frame(&frame)
    }

    /// Analyse one incoming [`frame::AudioFrame`] and return an
    /// [`ExtendedAudioFrame`] containing legacy meters, 64-band/48-bar
    /// log spectra, onset flux, and fused BPM estimate.
    pub fn process_frame(&mut self, input_frame: &frame::AudioFrame) -> ExtendedAudioFrame {
        let channels = input_frame.channels();
        let sample_rate_hz = input_frame.sample_rate_hz();

        let conditioned: frame::AudioFrame;
        let active_frame = if self.input_gain_linear != 1.0 {
            let mut scaled = input_frame.samples().to_vec();
            apply_input_gain(&mut scaled, self.input_gain_linear);
            conditioned = frame::AudioFrame::new(sample_rate_hz, channels, scaled);
            &conditioned
        } else {
            input_frame
        };

        let samples = active_frame.samples();
        let ch_usize = usize::from(channels);
        let stereo_levels = compute_stereo_levels(samples, ch_usize);

        let (true_peak_dbtp, clip) = compute_frame_true_peak(samples, ch_usize);

        // Maintain a bounded 1024-sample mono FFT window so every hop runs in
        // O(N log N) without triggering the legacy 57,601-sample O(N^2) time-domain
        // autocorrelation path; tempo tracking is handled by `FusedBpmEstimator`.
        if self.mono_window_sr != Some(sample_rate_hz) {
            self.mono_window_sr = Some(sample_rate_hz);
            self.mono_window.clear();
        }
        push_mono_samples(&mut self.mono_window, samples, ch_usize);
        let mono_slice: Vec<f32> = self.mono_window.iter().copied().collect();
        let analysis_frame = frame::AudioFrame::new(sample_rate_hz, 1, mono_slice);

        let mut uniform_snapshot = extract_uniform_snapshot(&analysis_frame);
        let agc = self.agc.observe(uniform_snapshot.rms);

        let spectrum_64 = compute_log_spectrum_64(analysis_frame.samples(), sample_rate_hz as f32);
        let spectrum_48 = downsample_64_to_48(&spectrum_64);
        let four_band_energy = aggregate_four_bands(&spectrum_64);

        let raw_flux = if self.has_prev_spectrum {
            let mut acc = 0.0_f32;
            for (&curr, &prev) in spectrum_64.iter().zip(self.prev_spectrum_64.iter()) {
                let diff = curr - prev;
                if diff > 0.0 {
                    acc += diff;
                }
            }
            acc
        } else {
            self.has_prev_spectrum = true;
            0.0
        };
        self.prev_spectrum_64 = spectrum_64;

        let (onset_strength, detector_on_beat) = self.onset_detector.observe(&spectrum_64);

        let frames_per_channel = if ch_usize > 0 {
            samples.len() / ch_usize
        } else {
            0
        };
        let dt_secs = if sample_rate_hz > 0 && frames_per_channel > 0 {
            frames_per_channel as f32 / sample_rate_hz as f32
        } else {
            0.0
        };

        let mut bpm_estimate = self.fused_bpm.push_onset_energy(raw_flux, dt_secs);
        bpm_estimate.on_beat = bpm_estimate.on_beat || detector_on_beat;

        uniform_snapshot.bpm = bpm_estimate.bpm;
        uniform_snapshot.bpm_availability = if bpm_estimate.confidence >= 0.25 {
            AudioAxisAvailability::Ready
        } else {
            AudioAxisAvailability::Warming
        };

        let extended = ExtendedAudioFrame {
            uniform_snapshot,
            stereo_levels,
            true_peak_dbtp,
            clip,
            agc,
            four_band_energy,
            spectrum_64,
            spectrum_48,
            onset_strength,
            bpm_estimate,
        };
        self.latest = extended.clone();
        extended
    }
}

/// Mix interleaved samples to mono and append into a rolling window capped at
/// [`spectrum::SPECTRUM_FFT_SIZE`].
fn push_mono_samples(
    window: &mut std::collections::VecDeque<f32>,
    samples: &[f32],
    channels: usize,
) {
    if samples.is_empty() {
        return;
    }
    if channels <= 1 {
        for &s in samples {
            if window.len() == spectrum::SPECTRUM_FFT_SIZE {
                let _ = window.pop_front();
            }
            window.push_back(s);
        }
        return;
    }
    let inv = 1.0 / channels as f32;
    for frame in samples.chunks_exact(channels) {
        let sum: f32 = frame.iter().copied().sum();
        if window.len() == spectrum::SPECTRUM_FFT_SIZE {
            let _ = window.pop_front();
        }
        window.push_back(sum * inv);
    }
}

/// Compute 4-band mean energy (`[0..16]`, `[16..32]`, `[32..48]`, `[48..64]`)
/// from a 64-band log spectrum.
fn aggregate_four_bands(spectrum_64: &[f32; SPECTRUM_BANDS_64]) -> [f32; 4] {
    let mut out = [0.0_f32; 4];
    for (band_idx, slot) in out.iter_mut().enumerate() {
        let start = band_idx * 16;
        let end = start + 16;
        let sum: f32 = spectrum_64[start..end].iter().copied().sum();
        *slot = (sum / 16.0).clamp(0.0, 1.0);
    }
    out
}

/// Compute true peak across mono or interleaved stereo/multi-channel PCM
/// frames by evaluating each channel independently and taking the maximum.
fn compute_frame_true_peak(samples: &[f32], channels: usize) -> (f32, bool) {
    if samples.is_empty() || channels == 0 {
        return (metering::TRUE_PEAK_FLOOR_DBTP, false);
    }
    if channels == 1 {
        return compute_true_peak_dbtp(samples);
    }
    let frames = samples.len() / channels;
    if frames == 0 {
        return (metering::TRUE_PEAK_FLOOR_DBTP, false);
    }
    let mut ch_buf = Vec::with_capacity(frames);
    let mut max_dbtp = metering::TRUE_PEAK_FLOOR_DBTP;
    let mut any_clip = false;
    let active_channels = channels.min(2);
    for ch in 0..active_channels {
        ch_buf.clear();
        for f in 0..frames {
            ch_buf.push(samples[f * channels + ch]);
        }
        let (dbtp, clip) = compute_true_peak_dbtp(&ch_buf);
        if dbtp > max_dbtp {
            max_dbtp = dbtp;
        }
        any_clip = any_clip || clip;
    }
    (max_dbtp, any_clip)
}

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

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod meter_engine_pins {
    //! Evidence: EV-AUDIO-TRACE (T1) — end-to-end wired 64-band log spectrum,
    //! 4-band / 8-band meters, stereo true-peak, AGC, and fused BPM analysis
    //! pins (AUDIO-SPECTRUM-002).

    use super::*;
    use std::f32::consts::TAU;

    fn expected_log_band(freq_hz: f32) -> usize {
        let ratio = spectrum::SPECTRUM_MAX_HZ / spectrum::SPECTRUM_MIN_HZ;
        let frac = (freq_hz / spectrum::SPECTRUM_MIN_HZ).ln() / ratio.ln();
        ((frac * SPECTRUM_BANDS_64 as f32).floor() as isize)
            .clamp(0, (SPECTRUM_BANDS_64 - 1) as isize) as usize
    }

    fn argmax(values: &[f32]) -> (usize, f32) {
        let mut best_idx = 0_usize;
        let mut best_val = f32::NEG_INFINITY;
        for (i, &v) in values.iter().enumerate() {
            if v > best_val {
                best_val = v;
                best_idx = i;
            }
        }
        (best_idx, best_val)
    }

    #[test]
    fn wired_engine_analyzes_sine_wave_across_64_band_48_bar_and_4_band_meters() {
        let mut engine = AudioMeterEngine::new();
        let sr = frame::DEFAULT_SAMPLE_RATE_HZ;
        let sr_f = sr as f32;

        // Feed 1024-sample stereo frames of a 1000 Hz sine (amplitude 0.5 on L, 0.25 on R).
        let mut stereo_pcm = Vec::with_capacity(1024 * 2);
        for i in 0..1024 {
            let s = (TAU * 1000.0 * i as f32 / sr_f).sin();
            stereo_pcm.push(0.5 * s);
            stereo_pcm.push(0.25 * s);
        }

        let out = engine.process_pcm(sr, 2, &stereo_pcm);

        // Legacy uniform snapshot is populated and FFT is Ready.
        assert_eq!(
            out.uniform_snapshot.fft_availability,
            AudioAxisAvailability::Ready
        );
        assert!(out.uniform_snapshot.rms > 0.2);
        assert!(out.uniform_snapshot.peak > 0.35);

        // Stereo L/R levels preserve genuine channel separation.
        assert!((out.stereo_levels.peak_l - 0.5).abs() < 0.02);
        assert!((out.stereo_levels.peak_r - 0.25).abs() < 0.02);
        assert!(out.true_peak_dbtp > -7.0 && out.true_peak_dbtp < -5.0);
        assert!(!out.clip);

        // 64-band log spectrum peaks at the expected 1000 Hz log band.
        let (peak_bin, peak_val) = argmax(&out.spectrum_64);
        let exp_bin = expected_log_band(1000.0);
        assert!(
            peak_bin.abs_diff(exp_bin) <= 1,
            "1000 Hz sine expected near band {exp_bin}, got {peak_bin} (val={peak_val})"
        );
        assert!(
            peak_val > 0.6,
            "expected strong 1000 Hz peak, got {peak_val}"
        );

        // 48-bar downsampled spectrum and 4-band meters remain in [0, 1] and show energy.
        assert!(
            out.spectrum_48
                .iter()
                .all(|&v| v.is_finite() && (0.0..=1.0).contains(&v))
        );
        assert!(out.spectrum_48.iter().copied().fold(0.0_f32, f32::max) > 0.4);
        assert!(
            out.four_band_energy
                .iter()
                .all(|&v| v.is_finite() && (0.0..=1.0).contains(&v))
        );
        // 1000 Hz lies in the second quarter (bands 16..32 -> four_band_energy[1]) or third quarter.
        assert!(out.four_band_energy[1] > 0.0 || out.four_band_energy[2] > 0.0);

        // `latest()` mirrors the last returned frame.
        assert_eq!(engine.latest(), &out);
    }

    #[test]
    fn wired_engine_locks_onto_128_bpm_kick_pulse_end_to_end() {
        let mut engine = AudioMeterEngine::new();
        let sr = frame::DEFAULT_SAMPLE_RATE_HZ;
        let hop_samples = 512_usize; // ~10.67 ms per hop
        let beat_period_secs = 60.0_f64 / 128.0_f64; // 0.46875 s
        let total_secs = 6.0_f64;
        let total_samples = (total_secs * f64::from(sr)) as usize;

        let mut last = ExtendedAudioFrame::default();
        let mut beat_count = 0_usize;
        let mut sample_cursor = 0_usize;

        while sample_cursor < total_samples {
            let chunk_len = hop_samples.min(total_samples - sample_cursor);
            let mut chunk = Vec::with_capacity(chunk_len);
            for i in 0..chunk_len {
                let t = (sample_cursor + i) as f64 / f64::from(sr);
                let phase_in_beat = t % beat_period_secs;
                // Sharp 45 ms decaying 70 Hz kick pulse on every 128 BPM beat.
                let sample = if phase_in_beat < 0.045 {
                    let env = (-phase_in_beat / 0.015).exp() as f32;
                    env * (TAU * 70.0 * phase_in_beat as f32).sin()
                } else {
                    0.0
                };
                chunk.push(sample);
            }
            sample_cursor += chunk_len;

            last = engine.process_pcm(sr, 1, &chunk);
            if last.bpm_estimate.on_beat {
                beat_count += 1;
            }
        }

        assert!(
            beat_count >= 6,
            "expected at least 6 detected beats across 6 s of 128 BPM kicks, got {beat_count}"
        );
        assert!(
            (last.bpm_estimate.bpm - 128.0).abs() <= 3.0,
            "expected wired FusedBpmEstimator to lock near 128 BPM (±3), got {:.2} BPM (conf={:.3})",
            last.bpm_estimate.bpm,
            last.bpm_estimate.confidence
        );
        assert!(
            last.bpm_estimate.confidence >= 0.45,
            "expected strong BPM lock confidence >= 0.45, got {:.3}",
            last.bpm_estimate.confidence
        );

        // Reset returns engine to clean unlocked baseline.
        engine.reset();
        assert_eq!(engine.latest().bpm_estimate.confidence, 0.0);
        assert_eq!(engine.latest().spectrum_64, [0.0; SPECTRUM_BANDS_64]);
    }
}

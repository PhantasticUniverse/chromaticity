// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-2 EV-AUDIO-TRACE slice-1 audio-uniform extraction.
//!
//! This module is the producer side of the audio → render uniform
//! marshalling boundary: it turns one [`crate::frame::AudioFrame`] into
//! an [`AudioUniformSnapshot`] of plain `f32` features that any
//! consumer (the slice-N render scheduler is the obvious one, but the
//! shape is intentionally render-agnostic) can lift into whatever
//! uniform-buffer layout it owns.
//!
//! # Crate-coupling discipline
//!
//! [`AudioUniformSnapshot`] is a plain-data struct. `engine/audio`
//! does NOT depend on `engine/render`; the render crate (or any
//! other consumer) reads the snapshot fields and marshals them into
//! its own [`chromaticity_render::UniformInput`] / `UniformValue`
//! shape. End-to-end audio→render integration tests live under
//! `engine/audio/tests/` (this slice only ships the producer side
//! plus the producer-side fixture corpus; the render-side
//! integration test lands in a follow-up render slice that knows the
//! consumer-side binding contract).
//!
//! # Symbol vocabulary
//!
//! The four primary slice-1 fields map onto the
//! `docs/36-reactive-symbol-catalog.md` v1.3.0 vocabulary as
//! follows:
//!
//! - [`AudioUniformSnapshot::bpm`] -> `syn_BPM` (catalog §2.7,
//!   clamped `[50, 220]`).
//! - [`AudioUniformSnapshot::rms`] -> the root-mean-square energy
//!   over the caller-supplied mono analysis buffer that
//!   `syn_InputLevel` (catalog §2.10, `audio_indicators.synScene`)
//!   is computed from. The production runtime supplies its rolling
//!   analysis window here. Slice-1 exposes the raw RMS in `[0, 1]`;
//!   the AGC + threshold gate the catalog describes lands in a later
//!   slice once the `syn_Threshold` plumbing arrives.
//! - [`AudioUniformSnapshot::peak`] -> the peak magnitude over the
//!   caller-supplied mono analysis buffer.
//! - [`AudioUniformSnapshot::fft_bands`] -> 8 log-spaced FFT
//!   magnitudes that downstream consumers can resample into the
//!   `syn_Spectrum` (`r=Raw FFT`, catalog §2.6) sampler1D / 2D
//!   texture.
//!
//! The `syn_BPM` clamp range is asserted in the T1 drift pins so a
//! catalog edit that moves the bounds surfaces here at the next
//! `cargo test`. Slice-1 covers only the four primary symbols; the
//! full `syn_*` vocabulary is the EV-AUDIO-TRACE Phase-3 corpus
//! target (see `docs/26` row 30 "Phase 2-3 + release cumulative
//! gate").
//!
//! # Algorithm choices
//!
//! - **RMS:** standard `sqrt(sum(s^2) / N)` over the interleaved
//!   sample buffer. Multi-channel frames are mixed down to mono by
//!   averaging across channels per sample-position before squaring.
//! - **Peak:** `max(abs(samples))` over the same mono mix-down.
//! - **FFT bands:** real-input FFT via [`rustfft`] over a power-of-
//!   two window taken from the head of the mono mix-down. The
//!   window is Hann-ed before transform to suppress spectral
//!   leakage. The eight band edges are log-spaced across `[20 Hz,
//!   20 kHz]`; each band sums the bin magnitudes that fall in its
//!   range, divided by the band's bin count to keep the integration
//!   normalised, then divided by the FFT-bin-magnitude reference
//!   level (which is `(window_size / 2) * 0.5` = the magnitude a
//!   Hann-windowed unit sine produces) so a unit sine in any single
//!   band reads as 1.0. The result is clamped to `[0, 1]` before
//!   storage.
//! - **BPM:** time-domain autocorrelation over the mono mix-down,
//!   peak-picked across the lag range corresponding to `[50, 220]`
//!   BPM at the frame's sample rate. If the frame is shorter than
//!   the longest lag the search range needs (e.g. the 128-sample
//!   silent fallback frame from [`crate::silent::SilentSource`]),
//!   the function returns the slice-1 default of 120.0 BPM. Slice-2
//!   ships the [`crate::bpm::BpmEstimator`] history-window
//!   median-smoother that rides the same per-frame autocorrelation;
//!   this slice's [`extract_uniform_snapshot`] STAYS PURE
//!   (single-frame, no history) so callers that don't want history
//!   continue to use it directly.
//!
//! The single-frame autocorrelation BPM is intentionally low-quality
//! for short frames: the slice-1 corpus exercises it on a synthetic
//! 1-second 120 BPM click-track fixture only. The slice-2
//! history-window estimator covers multi-second smoothing; full
//! Phase-3 production-grade BPM detection (constant-Q chroma, beat-
//! tracking onset detection) lands in a follow-up slice once the
//! analyser scheduler is upstream.

use std::collections::VecDeque;
use std::f32::consts::TAU;
use std::sync::{Arc, Mutex, OnceLock};

use rustfft::{Fft, FftPlanner, num_complex::Complex32};

use crate::frame::AudioFrame;

fn cached_fft_1024() -> Arc<dyn Fft<f32>> {
    static PLAN: OnceLock<Arc<dyn Fft<f32>>> = OnceLock::new();
    Arc::clone(PLAN.get_or_init(|| FftPlanner::<f32>::new().plan_fft_forward(FFT_WINDOW_SIZE)))
}

type CachedBpmFftPair = (usize, Arc<dyn Fft<f32>>, Arc<dyn Fft<f32>>);

fn cached_bpm_fft_pair(fft_len: usize) -> (Arc<dyn Fft<f32>>, Arc<dyn Fft<f32>>) {
    static CACHE: OnceLock<Mutex<Option<CachedBpmFftPair>>> = OnceLock::new();
    let mutex = CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = match mutex.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some((cached_len, fwd, inv)) = guard.as_ref() {
        if *cached_len == fft_len {
            return (Arc::clone(fwd), Arc::clone(inv));
        }
    }
    let mut planner = FftPlanner::<f32>::new();
    let fwd = planner.plan_fft_forward(fft_len);
    let inv = planner.plan_fft_inverse(fft_len);
    *guard = Some((fft_len, Arc::clone(&fwd), Arc::clone(&inv)));
    (fwd, inv)
}

#[inline]
fn sanitize_finite(sample: f32) -> f32 {
    if sample.is_finite() { sample } else { 0.0 }
}

/// Number of FFT bands the slice-1 snapshot exposes.
///
/// Pinned at 8 to match the `chroma_AudioFFT[8]` brief vocabulary; the array
/// arity is encoded in the [`AudioUniformSnapshot::fft_bands`] type
/// signature so consumers can rely on it without a runtime length
/// check.
pub const FFT_BAND_COUNT: usize = 8;

/// Lower edge of the FFT band-decomposition range, in Hz.
///
/// Matches the lower bound of the musical spectrum the catalog's
/// `syn_Spectrum` symbol covers; bins below this frequency
/// (sub-audible rumble, DC bias) are discarded before integration.
pub const FFT_BAND_MIN_HZ: f32 = 20.0;

/// Upper edge of the FFT band-decomposition range, in Hz.
///
/// Matches the upper bound of the human-audible musical spectrum;
/// bins above this frequency are discarded before integration.
pub const FFT_BAND_MAX_HZ: f32 = 20_000.0;

/// FFT window size used by [`extract_uniform_snapshot`].
///
/// Slice-1 pins the window at 1024 samples (~21 ms at 48 kHz) — a
/// power-of-two large enough for ~47 Hz frequency resolution at the
/// canonical sample rate, small enough that a single
/// 1-second-of-audio frame contains many complete windows. The frame
/// length never reaches the FFT path if `samples().len()` is below
/// this constant; the band array reads all-zero on under-window
/// frames (matching the silence baseline the test corpus pins).
pub const FFT_WINDOW_SIZE: usize = 1024;

/// Slice-1 BPM default returned when the frame is too short to
/// support autocorrelation across the `[50, 220]` BPM lag range.
///
/// Pinned at 120 because that is the catalog's nominal "no
/// information" anchor — `syn_BPM` clamps to `[50, 220]` per docs/36
/// §2.7, and 120 sits at the geometric mid-point of the musical
/// tempo range the catalog covers. The constant is pinned here (not
/// inlined) so the value drift surfaces in the T1 drift pins.
pub const BPM_DEFAULT: f32 = 120.0;

/// Lower clamp of `syn_BPM` per `docs/36-reactive-symbol-catalog.md`
/// §2.7 ("Estimated BPM, clamped `[50, 220]`").
pub const BPM_CLAMP_MIN: f32 = 50.0;

/// Upper clamp of `syn_BPM` per `docs/36-reactive-symbol-catalog.md`
/// §2.7.
pub const BPM_CLAMP_MAX: f32 = 220.0;

/// Readiness for a snapshot axis that needs an accumulated analysis
/// window before its numeric value is trustworthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioAxisAvailability {
    /// The producer has observed audio, but not enough history for
    /// this axis' windowed analysis path.
    Warming,
    /// The axis has enough input history for consumers to display its
    /// numeric value.
    Ready,
}

impl Default for AudioAxisAvailability {
    fn default() -> Self {
        Self::Warming
    }
}

/// Rolling mono analysis window capacity for a sample rate.
///
/// The FFT path needs [`FFT_WINDOW_SIZE`] samples. The BPM
/// autocorrelation path needs one sample more than the longest lag in
/// the `[BPM_CLAMP_MIN, BPM_CLAMP_MAX]` search range so at least one
/// overlap sample exists at that lag. Runtime callers use this to
/// accumulate callback-sized frames before asking the pure extractor
/// for FFT/BPM features.
#[must_use]
pub(crate) fn analysis_window_samples(sample_rate_hz: u32) -> usize {
    if sample_rate_hz == 0 {
        return FFT_WINDOW_SIZE;
    }
    let bpm_window = bpm_analysis_window_samples(sample_rate_hz);
    FFT_WINDOW_SIZE.max(bpm_window)
}

fn bpm_analysis_window_samples(sample_rate_hz: u32) -> usize {
    if sample_rate_hz == 0 {
        return usize::MAX;
    }
    (60.0 * sample_rate_hz as f32 / BPM_CLAMP_MIN).ceil() as usize + 1
}

/// Stateful rolling mono window used by the runtime to feed the pure
/// uniform extractor enough samples for FFT and BPM analysis.
///
/// Callback frames can be shorter than [`FFT_WINDOW_SIZE`] and far
/// shorter than the longest BPM autocorrelation lag. This helper keeps
/// the pure [`extract_uniform_snapshot`] contract intact while giving
/// the production runtime a bounded history window to analyse.
#[derive(Debug, Default, Clone)]
pub(crate) struct AudioAnalysisWindow {
    sample_rate_hz: Option<u32>,
    samples: VecDeque<f32>,
}

impl AudioAnalysisWindow {
    /// Push one frame into the rolling mono window without copying the full
    /// BPM history buffer.
    pub(crate) fn push(&mut self, frame: &AudioFrame) {
        let mono = mono_mix(frame);
        let sample_rate_hz = frame.sample_rate_hz();
        if self.sample_rate_hz != Some(sample_rate_hz) {
            self.sample_rate_hz = Some(sample_rate_hz);
            self.samples.clear();
        }

        self.samples.extend(mono);

        let capacity = analysis_window_samples(sample_rate_hz);
        while self.samples.len() > capacity {
            let _ = self.samples.pop_front();
        }
    }

    /// Push one frame into the rolling mono window and return the
    /// current accumulated analysis frame.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn observe(&mut self, frame: &AudioFrame) -> AudioFrame {
        self.push(frame);
        self.full_frame()
    }

    /// Snapshot the full rolling mono window (for 4 Hz BPM autocorrelation).
    #[must_use]
    pub(crate) fn full_frame(&self) -> AudioFrame {
        let sample_rate_hz = self.sample_rate_hz.unwrap_or(0);
        AudioFrame::new(sample_rate_hz, 1, self.samples.iter().copied().collect())
    }

    /// Snapshot the most recent `max_samples` mono samples (e.g.
    /// [`FFT_WINDOW_SIZE`] = 1024 samples ≈ 21.3 ms at 48 kHz) for
    /// low-latency RMS/peak metering and live FFT spectrum extraction.
    #[must_use]
    pub(crate) fn recent_frame(&self, max_samples: usize) -> AudioFrame {
        let sample_rate_hz = self.sample_rate_hz.unwrap_or(0);
        let len = self.samples.len();
        let start = len.saturating_sub(max_samples);
        let samples: Vec<f32> = self.samples.iter().skip(start).copied().collect();
        AudioFrame::new(sample_rate_hz, 1, samples)
    }

    /// Whether the rolling window holds enough samples for BPM analysis.
    #[must_use]
    pub(crate) fn is_bpm_ready(&self) -> bool {
        let Some(sr) = self.sample_rate_hz else {
            return false;
        };
        sr != 0 && self.samples.len() >= bpm_analysis_window_samples(sr)
    }

    /// Number of mono samples currently retained.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.samples.len()
    }
}

/// Plain-data audio uniform snapshot.
///
/// This is the producer-side hand-off shape: every consumer
/// (`engine/render`'s scene→multipass adapter is the obvious one)
/// reads these fields and marshals them into whatever uniform-buffer
/// layout they want. Keeping the snapshot a pure-data struct (no
/// `Default` trickery, no behaviour beyond field storage) is what
/// lets `engine/audio` stay render-agnostic.
///
/// Field semantics (catalog mapping in the module-level prose):
///
/// - `bpm`: estimated tempo, clamped to `[50, 220]`. Slice-1 returns
///   [`BPM_DEFAULT`] when the frame is too short to autocorrelate
///   across the full lag range.
/// - `bpm_availability`: readiness flag for whether `bpm` was
///   computed from a filled BPM analysis window (`Ready`) or is still
///   carrying the short-frame fallback during warmup (`Warming`).
/// - `rms`: root-mean-square energy of the caller-supplied
///   mono-mixed analysis buffer in `[0, 1]`. The production runtime
///   passes its rolling analysis window here; one-shot callers may
///   still pass a single callback frame. Out-of-range input samples
///   are tolerated (the analyser clamps per docs/06 §7.4) but the
///   published RMS is clamped to `[0, 1]` before storage so consumers
///   can rely on the bound.
/// - `peak`: peak magnitude in `[0, 1]`, computed as
///   `min(1.0, max(abs(samples)))` over the caller-supplied
///   mono-mixed analysis buffer.
/// - `fft_bands`: 8 log-spaced FFT magnitudes covering
///   `[FFT_BAND_MIN_HZ, FFT_BAND_MAX_HZ]`, each in `[0, 1]`.
/// - `fft_availability`: readiness flag for whether `fft_bands`
///   comes from a filled FFT window (`Ready`) or should be displayed
///   as warming (`Warming`).
/// - `degraded`: cross-seam health flag set by downstream adapters
///   when the last-known snapshot had to be recovered from a degraded
///   path (for example a poisoned snapshot mutex).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioUniformSnapshot {
    /// Estimated BPM, clamped `[50, 220]`. Maps to the
    /// `syn_BPM` compat-syn-v1 retention via the engine/render
    /// slice-5a adapter (docs/36 line 117).
    pub bpm: f32,
    /// Whether `bpm` is safe for UI display as a real tempo estimate.
    pub bpm_availability: AudioAxisAvailability,
    /// Root-mean-square window energy, `[0, 1]`. Maps to
    /// the `chroma_AudioRMS` chromaticity-native binding via the
    /// engine/render slice-5a adapter.
    pub rms: f32,
    /// Window peak magnitude, `[0, 1]`. Maps to the
    /// `chroma_AudioPeak` chromaticity-native binding via the
    /// engine/render slice-5a adapter.
    pub peak: f32,
    /// 8 log-spaced FFT band magnitudes, each in `[0, 1]`, ordered
    /// ascending in frequency. Maps to the `chroma_AudioFFTLow`
    /// (bands [0..3]) and `chroma_AudioFFTHigh` (bands [4..7])
    /// chromaticity-native bindings via the engine/render slice-5a
    /// adapter (split-vec4 spelling pending the future `audio[]`
    /// array uniform variant).
    pub fft_bands: [f32; FFT_BAND_COUNT],
    /// Whether `fft_bands` is safe for UI display as real spectrum
    /// data.
    pub fft_availability: AudioAxisAvailability,
    /// True once a cross-thread snapshot adapter recovers from a
    /// degraded path and returns last-known state.
    pub degraded: bool,
}

impl Default for AudioUniformSnapshot {
    /// All-zero snapshot, matching the silent-fallback baseline.
    ///
    /// `bpm = 0.0`, `rms = 0.0`, `peak = 0.0`, every band 0.0.
    /// Consumers that read a default-constructed snapshot should
    /// treat it as "no audio observed yet"; the production
    /// extractor never returns this exact shape because BPM clamps
    /// to [`BPM_DEFAULT`] (or an autocorrelation result) on every
    /// real frame.
    fn default() -> Self {
        Self {
            bpm: 0.0,
            bpm_availability: AudioAxisAvailability::Warming,
            rms: 0.0,
            peak: 0.0,
            fft_bands: [0.0; FFT_BAND_COUNT],
            fft_availability: AudioAxisAvailability::Warming,
            degraded: false,
        }
    }
}

/// Extract one [`AudioUniformSnapshot`] from `frame`.
///
/// Pure function: same input frame -> same output snapshot. No
/// state is carried across calls. Slice-2 ships the state-carrying
/// [`crate::bpm::BpmEstimator`] for history-window-smoothed BPM as
/// a separate surface; the slice-1 entry point STAYS PURE so the
/// producer side has no in-crate ordering hazards and consumers
/// that don't want history can keep using it.
///
/// Empty / silent frames return the all-zero baseline (matching
/// [`AudioUniformSnapshot::default`] except `bpm` which holds
/// [`BPM_DEFAULT`] when the frame is non-empty but shorter than the
/// autocorrelation window). Multi-channel frames are mixed down to
/// mono before analysis.
#[must_use]
pub fn extract_uniform_snapshot(frame: &AudioFrame) -> AudioUniformSnapshot {
    let mono = mono_mix(frame);

    if mono.is_empty() {
        // No samples — return the all-zero baseline. The
        // ingest-side `Closed` / `WouldBlock` paths leave the
        // out-frame buffer unchanged (silent.rs line 52 only
        // populates on `Ready`); this branch covers callers that
        // pass a zero-length default-constructed frame.
        return AudioUniformSnapshot::default();
    }

    let rms = compute_rms_over_window(&mono);
    let peak = compute_peak_over_window(&mono);
    let fft_bands = compute_fft_bands(&mono, frame.sample_rate_hz());
    let bpm = compute_bpm_autocorr(&mono, frame.sample_rate_hz());
    let sample_rate_hz = frame.sample_rate_hz();
    let fft_availability = if sample_rate_hz != 0 && mono.len() >= FFT_WINDOW_SIZE {
        AudioAxisAvailability::Ready
    } else {
        AudioAxisAvailability::Warming
    };
    let bpm_availability =
        if sample_rate_hz != 0 && mono.len() >= bpm_analysis_window_samples(sample_rate_hz) {
            AudioAxisAvailability::Ready
        } else {
            AudioAxisAvailability::Warming
        };

    AudioUniformSnapshot {
        bpm,
        bpm_availability,
        rms,
        peak,
        fft_bands,
        fft_availability,
        degraded: false,
    }
}

/// Fast live-path snapshot extractor for the production audio runtime:
/// computes RMS, peak, and 8-band FFT over `recent_frame` (typically the
/// most recent 1024 samples ≈ 21 ms at 48 kHz) while carrying forward the
/// 4 Hz smoothed BPM estimate without re-running autocorrelation.
#[must_use]
pub(crate) fn extract_live_metering_snapshot(
    recent_frame: &AudioFrame,
    bpm: f32,
    bpm_availability: AudioAxisAvailability,
) -> AudioUniformSnapshot {
    let safe_bpm = if bpm.is_finite() {
        bpm.clamp(BPM_CLAMP_MIN, BPM_CLAMP_MAX)
    } else {
        BPM_DEFAULT
    };
    let mono = mono_mix(recent_frame);
    if mono.is_empty() {
        return AudioUniformSnapshot {
            bpm: safe_bpm,
            bpm_availability,
            ..AudioUniformSnapshot::default()
        };
    }

    let rms = compute_rms_over_window(&mono);
    let peak = compute_peak_over_window(&mono);
    let sample_rate_hz = recent_frame.sample_rate_hz();
    let fft_bands = compute_fft_bands(&mono, sample_rate_hz);
    let fft_availability = if sample_rate_hz != 0 && mono.len() >= FFT_WINDOW_SIZE {
        AudioAxisAvailability::Ready
    } else {
        AudioAxisAvailability::Warming
    };

    AudioUniformSnapshot {
        bpm: safe_bpm,
        bpm_availability,
        rms,
        peak,
        fft_bands,
        fft_availability,
        degraded: false,
    }
}

/// Slice-2 reuse helper: compute the per-frame autocorrelation BPM
/// for `frame` using the same algorithm
/// [`extract_uniform_snapshot`] uses internally.
///
/// Crate-private so the slice-2 [`crate::bpm::BpmEstimator`] can
/// share the algorithm without going through the full snapshot
/// path (and without paying the FFT-band cost for a BPM-only
/// observation). External callers continue to use
/// [`extract_uniform_snapshot`].
#[must_use]
pub(crate) fn compute_bpm_autocorr_for_frame(frame: &AudioFrame) -> f32 {
    let mono = mono_mix(frame);
    if mono.is_empty() {
        return BPM_DEFAULT;
    }
    compute_bpm_autocorr(&mono, frame.sample_rate_hz())
}

/// Mix an interleaved multi-channel frame down to mono by averaging
/// across channels.
///
/// Mono and zero-channel frames pass through (zero-channel is
/// degenerate and never produced by [`crate::ingest::IngestBoundary`]
/// implementors, but the early-return keeps the stride-divisor
/// arithmetic safe).
fn mono_mix(frame: &AudioFrame) -> Vec<f32> {
    let channels = frame.channels();
    let samples = frame.samples();

    if channels <= 1 {
        return samples.iter().map(|&s| sanitize_finite(s)).collect();
    }

    let stride = usize::from(channels);
    if stride == 0 || samples.is_empty() {
        return Vec::new();
    }

    let frames = samples.len() / stride;
    let mut out = Vec::with_capacity(frames);
    let inv_stride = 1.0 / stride as f32;
    for f in 0..frames {
        let base = f * stride;
        let mut sum = 0.0_f32;
        for ch in 0..stride {
            // Bounds-checked indexing; `frames * stride <=
            // samples.len()` by construction.
            sum += sanitize_finite(samples[base + ch]);
        }
        out.push(sanitize_finite(sum * inv_stride));
    }
    out
}

/// Root-mean-square over a mono buffer, clamped to `[0, 1]`.
///
/// Empty buffers read 0.0 (no energy). The clamp matches the
/// `syn_Level` family bound the catalog documents (raw RMS is
/// nominally `[0, 1]` for in-range PCM input; out-of-range samples
/// are clamped at the published surface).
fn compute_rms(mono: &[f32]) -> f32 {
    if mono.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = mono
        .iter()
        .map(|&s| {
            let v = f64::from(sanitize_finite(s));
            v * v
        })
        .sum();
    let rms = (sum_sq / mono.len() as f64).sqrt() as f32;
    if rms.is_finite() {
        rms.clamp(0.0, 1.0)
    } else if rms.is_sign_positive() {
        1.0
    } else {
        0.0
    }
}

/// Root-mean-square over a mono analysis window, clamped to `[0, 1]`.
///
/// This is a named wrapper around the scalar RMS primitive so the
/// runtime can pin that RMS is sourced from the same rolling analysis
/// window as FFT/BPM rather than from the most recent callback hop.
pub(crate) fn compute_rms_over_window(samples: &[f32]) -> f32 {
    compute_rms(samples)
}

/// Peak absolute magnitude over a mono buffer, clamped to `[0, 1]`.
fn compute_peak(mono: &[f32]) -> f32 {
    let mut peak = 0.0_f32;
    for &s in mono {
        let m = sanitize_finite(s).abs();
        if m > peak {
            peak = m;
        }
    }
    if peak.is_finite() {
        peak.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Peak absolute magnitude over a mono analysis window, clamped to `[0, 1]`.
///
/// Kept paired with [`compute_rms_over_window`] so the runtime's
/// windowed energy/envelope contract is explicit at the call site.
pub(crate) fn compute_peak_over_window(samples: &[f32]) -> f32 {
    compute_peak(samples)
}

/// Build the [`FFT_BAND_COUNT`]-element band array for `mono`.
///
/// Returns the all-zero array when the buffer is shorter than
/// [`FFT_WINDOW_SIZE`]. Any sub-window head is discarded; slice-2
/// can grow this to a windowed mean over multiple FFT frames.
fn compute_fft_bands(mono: &[f32], sample_rate_hz: u32) -> [f32; FFT_BAND_COUNT] {
    let mut bands = [0.0_f32; FFT_BAND_COUNT];

    if mono.len() < FFT_WINDOW_SIZE || sample_rate_hz == 0 {
        return bands;
    }

    // Hann-window the most recent FFT_WINDOW_SIZE samples of the buffer
    // into a complex FFT input so live spectrum updates track the latest
    // audio instead of lagging by the full BPM history window.
    let start = mono.len() - FFT_WINDOW_SIZE;
    let tail = &mono[start..];
    let mut buffer: Vec<Complex32> = (0..FFT_WINDOW_SIZE)
        .map(|i| {
            // Bounds-checked: tail.len() == FFT_WINDOW_SIZE by construction.
            let s = tail[i];
            let w = hann(i, FFT_WINDOW_SIZE);
            Complex32 { re: s * w, im: 0.0 }
        })
        .collect();

    let fft = cached_fft_1024();
    fft.process(&mut buffer);

    // Hann-windowed unit-sine reference magnitude. A Hann window of
    // length N applied to a unit-amplitude sine that lands on a bin
    // produces a peak FFT magnitude of `(N / 2) * 0.5` (the 0.5 is
    // the Hann window's coherent gain). Dividing by this reference
    // makes a single-bin unit sine read as 1.0 in its band.
    let reference = (FFT_WINDOW_SIZE as f32) * 0.25;

    let bin_hz = sample_rate_hz as f32 / FFT_WINDOW_SIZE as f32;
    let nyquist_bin = FFT_WINDOW_SIZE / 2;

    let edges = log_band_edges();

    for (b, band_slot) in bands.iter_mut().enumerate() {
        let lo_hz = edges[b];
        let hi_hz = edges[b + 1];
        // Bin range covering [lo_hz, hi_hz). Saturating-cast
        // is safe because both values are non-negative.
        let lo_bin = ((lo_hz / bin_hz).floor() as usize).max(1);
        let hi_bin = ((hi_hz / bin_hz).ceil() as usize).min(nyquist_bin);
        if hi_bin <= lo_bin {
            continue;
        }
        // Bounds-checked: hi_bin <= nyquist_bin <= buffer.len().
        let slice = &buffer[lo_bin..hi_bin];
        let mut sum = 0.0_f32;
        for c in slice {
            sum += c.norm();
        }
        let count = slice.len();
        if count == 0 {
            continue;
        }
        let mean = sum / count as f32;
        let normalized = mean / reference;
        *band_slot = if normalized.is_finite() {
            normalized.clamp(0.0, 1.0)
        } else if normalized.is_sign_positive() {
            1.0
        } else {
            0.0
        };
    }

    bands
}

/// Hann window coefficient at sample index `i` over a window of
/// length `n` (`n >= 2`).
fn hann(i: usize, n: usize) -> f32 {
    if n <= 1 {
        return 1.0;
    }
    0.5 - 0.5 * ((TAU * i as f32) / (n - 1) as f32).cos()
}

/// 9 log-spaced band edges (8 bands) from [`FFT_BAND_MIN_HZ`] to
/// [`FFT_BAND_MAX_HZ`].
///
/// Returned as a fixed-size array so the call site can index without
/// a length check.
fn log_band_edges() -> [f32; FFT_BAND_COUNT + 1] {
    let mut edges = [0.0_f32; FFT_BAND_COUNT + 1];
    let log_lo = FFT_BAND_MIN_HZ.ln();
    let log_hi = FFT_BAND_MAX_HZ.ln();
    let step = (log_hi - log_lo) / FFT_BAND_COUNT as f32;
    for (i, edge) in edges.iter_mut().enumerate() {
        *edge = (log_lo + step * i as f32).exp();
    }
    edges
}

/// Single-frame autocorrelation BPM estimate.
///
/// Searches lags corresponding to `[50, 220]` BPM at `sample_rate_hz`
/// and returns the lag with the strongest normalised
/// autocorrelation, mapped back to BPM and clamped to
/// `[BPM_CLAMP_MIN, BPM_CLAMP_MAX]`. Returns [`BPM_DEFAULT`] when
/// the frame is too short to support the longest lag in the search
/// range or when no lag has positive autocorrelation (silent
/// frames).
///
/// Slice-1 deliberately does NOT carry history; the slice-2
/// [`crate::bpm::BpmEstimator`] median-smoothes this same per-frame
/// estimate over a configurable history window for callers that
/// want stabilised tempo-following.
fn compute_bpm_autocorr(mono: &[f32], sample_rate_hz: u32) -> f32 {
    if sample_rate_hz == 0 || mono.is_empty() {
        return BPM_DEFAULT;
    }

    // Lag range corresponding to [BPM_CLAMP_MIN, BPM_CLAMP_MAX].
    // lag_samples = sample_rate / (BPM / 60) = 60 * sr / BPM.
    let max_lag = (60.0 * sample_rate_hz as f32 / BPM_CLAMP_MIN) as usize;
    let min_lag = (60.0 * sample_rate_hz as f32 / BPM_CLAMP_MAX) as usize;

    if mono.len() <= max_lag || min_lag == 0 || min_lag >= max_lag {
        // Frame too short for the longest lag we'd like to
        // search; return the slice-1 fallback. Silent fallback
        // frames (128 samples) hit this branch.
        return BPM_DEFAULT;
    }

    let energy: f32 = mono
        .iter()
        .map(|&s| {
            let v = sanitize_finite(s);
            v * v
        })
        .sum();
    if !energy.is_finite() || energy <= f32::EPSILON {
        return BPM_DEFAULT;
    }

    // Exact linear autocorrelation via the Wiener–Khinchin theorem in
    // O(N log N) instead of O(N * max_lag) (~1B multiply-adds at 48 kHz).
    // Zero-padding to >= 2 * N eliminates circular wrap-around aliasing so
    // `ifft(|fft(padded)|^2)[lag] / fft_len == sum_{i=0}^{N-lag-1} x[i]*x[i+lag]`.
    let Some(fft_len) = mono
        .len()
        .checked_mul(2)
        .and_then(usize::checked_next_power_of_two)
    else {
        return BPM_DEFAULT;
    };
    let (forward, inverse) = cached_bpm_fft_pair(fft_len);

    let mut spectrum = vec![Complex32 { re: 0.0, im: 0.0 }; fft_len];
    for (slot, &sample) in spectrum.iter_mut().zip(mono.iter()) {
        slot.re = sanitize_finite(sample);
    }
    forward.process(&mut spectrum);
    for bin in &mut spectrum {
        bin.re = bin.norm_sqr();
        bin.im = 0.0;
    }
    inverse.process(&mut spectrum);

    let inv_fft_len = 1.0 / fft_len as f32;
    // Floor the overlap divisor at `mono.len() / 8` so very small overlaps
    // near `max_lag` (e.g. `n == 1`) cannot blow up variance and pin BPM to 50.
    let min_divisor = (mono.len() / 8).max(1);
    let mut best_lag = 0_usize;
    let mut best_score = 0.0_f32;
    for (offset, bin) in spectrum[min_lag..=max_lag].iter().enumerate() {
        let lag = min_lag + offset;
        let n = mono.len() - lag;
        if n == 0 {
            break;
        }
        let acc = bin.re * inv_fft_len;
        let score = acc / n.max(min_divisor) as f32;
        if score.is_finite() && score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }

    if best_lag == 0 {
        return BPM_DEFAULT;
    }

    let bpm = 60.0 * sample_rate_hz as f32 / best_lag as f32;
    bpm.clamp(BPM_CLAMP_MIN, BPM_CLAMP_MAX)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod uniforms_drift_pins {
    //! Evidence: EV-AUDIO-TRACE (T1) + EV-AUDIO-ROUTING-MATRIX (T1) —
    //! drift pins on the slice-1 uniform-extraction algorithm
    //! constants, the `AudioUniformSnapshot` default-shape baseline,
    //! AND the channel-routing invariants the routing-matrix corpus
    //! relies on (mono-mix `1/N` aggregation per `docs/06-audio-engine.md`
    //! §7.1, the RMS/peak/band-edge formulas the matrix corpus pins
    //! end-to-end through `extract_uniform_snapshot`).
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: each test pins
    //! either a documented algorithm invariant (RMS / peak / band-edge
    //! formula) or a normative-doc constant
    //! (`docs/36-reactive-symbol-catalog.md` §2.7 `syn_BPM` clamp
    //! range). Any drift in the pinned constants or the math fires
    //! the moment `cargo test` runs, before the wider T2 corpus would
    //! catch the same regression as a noisier multi-test failure.
    //! T1 counts toward `EV-EVIDENCE-TAG-COVERAGE`; T2 corpora live
    //! in `engine/audio/tests/uniforms.rs` (EV-AUDIO-TRACE) and
    //! `engine/audio/tests/routing_matrix_corpus/main.rs`
    //! (EV-AUDIO-ROUTING-MATRIX).
    //!
    //! Dual-tag rationale: the routing-matrix evidence row 31's
    //! validation method ("validate system-audio and channel routing
    //! coverage") rests on the same in-source algorithm invariants
    //! that EV-AUDIO-TRACE row 30 already T1-pinned for the slice-1
    //! uniform extractor. The mono-aggregation `1/N` policy, the RMS
    //! `sqrt(mean(s^2))` formula, the peak `max(abs)` invariant, and
    //! the `[20 Hz, 20 kHz]` log-spaced FFT band edges are all
    //! routing-matrix invariants too; dual-tagging captures that
    //! without duplicating tests.
    //!
    //! Anchors:
    //! - `docs/26-evidence-artifact-registry.md` row 30 — `EV-AUDIO-TRACE`.
    //! - `docs/26-evidence-artifact-registry.md` row 31 — `EV-AUDIO-ROUTING-MATRIX`.
    //! - `docs/36-reactive-symbol-catalog.md` §2.7, §2.10, §2.6.
    //! - `docs/43-development-standards.md` §15.5 — T-tier definitions.
    //! - `docs/06-audio-engine.md` §7.1 — sample-rate / framing / channel policy baseline.

    use super::{
        AudioAnalysisWindow, AudioAxisAvailability, AudioUniformSnapshot, BPM_CLAMP_MAX,
        BPM_CLAMP_MIN, BPM_DEFAULT, FFT_BAND_COUNT, FFT_BAND_MAX_HZ, FFT_BAND_MIN_HZ,
        FFT_WINDOW_SIZE, analysis_window_samples, compute_peak, compute_peak_over_window,
        compute_rms, compute_rms_over_window, extract_uniform_snapshot, log_band_edges,
    };
    use crate::frame::AudioFrame;
    use std::f32::consts::TAU;

    fn variance(values: &[f32]) -> f32 {
        if values.is_empty() {
            return 0.0;
        }
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        values
            .iter()
            .map(|value| {
                let delta = value - mean;
                delta * delta
            })
            .sum::<f32>()
            / values.len() as f32
    }

    #[test]
    fn rms_matches_hand_computed_small_input() {
        // Hand-computed: sqrt((0.5^2 + 0.5^2 + 0.0^2 + 0.0^2) / 4)
        //              = sqrt(0.125) ≈ 0.353553.
        let mono = [0.5_f32, 0.5, 0.0, 0.0];
        let rms = compute_rms(&mono);
        assert!(
            (rms - 0.353_553_4).abs() < 1e-5,
            "expected ~0.353553, got {rms}"
        );
    }

    #[test]
    fn rms_clamps_oob_samples() {
        // Out-of-range PCM input is tolerated by the audio engine
        // (docs/06 §7.4) but the published RMS is clamped to
        // [0, 1]. Two samples at amplitude 2.0 give an unclamped
        // RMS of 2.0; the clamp must pin it at 1.0.
        let mono = [2.0_f32, 2.0];
        assert_eq!(compute_rms(&mono), 1.0);
    }

    #[test]
    fn rms_zero_for_silent_or_empty_buffer() {
        assert_eq!(compute_rms(&[]), 0.0);
        assert_eq!(compute_rms(&[0.0_f32; 16]), 0.0);
    }

    #[test]
    fn peak_picks_max_abs_magnitude() {
        // Mixed sign, peak magnitude is 0.75.
        let mono = [-0.25_f32, 0.5, -0.75, 0.6];
        assert_eq!(compute_peak(&mono), 0.75);
    }

    #[test]
    fn peak_clamps_oob_samples() {
        let mono = [-1.5_f32, 0.0, 1.4];
        assert_eq!(compute_peak(&mono), 1.0);
    }

    #[test]
    fn peak_zero_for_silent_or_empty_buffer() {
        assert_eq!(compute_peak(&[]), 0.0);
        assert_eq!(compute_peak(&[0.0_f32; 32]), 0.0);
    }

    #[test]
    fn window_rms_and_peak_match_known_content() {
        let window = [0.0_f32, 0.25, -0.75, 0.5];
        let expected_rms = ((0.25_f32 * 0.25) + (0.75_f32 * 0.75) + (0.5_f32 * 0.5)).sqrt() / 2.0;

        let rms = compute_rms_over_window(&window);
        assert!(
            (rms - expected_rms).abs() < 1e-6,
            "expected {expected_rms}, got {rms}",
        );
        assert_eq!(compute_peak_over_window(&window), 0.75);
    }

    #[test]
    fn band_edges_span_min_to_max_hz() {
        let edges = log_band_edges();
        // Edge 0 must equal FFT_BAND_MIN_HZ and edge N must equal
        // FFT_BAND_MAX_HZ within FP rounding.
        assert!(
            (edges[0] - FFT_BAND_MIN_HZ).abs() < 1e-3,
            "edge 0 = {} (expected {FFT_BAND_MIN_HZ})",
            edges[0]
        );
        assert!(
            (edges[FFT_BAND_COUNT] - FFT_BAND_MAX_HZ).abs() < 1.0,
            "edge {FFT_BAND_COUNT} = {} (expected {FFT_BAND_MAX_HZ})",
            edges[FFT_BAND_COUNT]
        );
    }

    #[test]
    fn band_edges_are_strictly_log_spaced() {
        // Successive ratios must equal exp((ln(max) - ln(min)) /
        // band_count) within FP rounding. This pins the log-
        // spacing math against an accidental switch to linear or
        // mel scaling.
        let edges = log_band_edges();
        let log_lo = FFT_BAND_MIN_HZ.ln();
        let log_hi = FFT_BAND_MAX_HZ.ln();
        let expected_ratio = ((log_hi - log_lo) / FFT_BAND_COUNT as f32).exp();
        for i in 0..FFT_BAND_COUNT {
            let observed = edges[i + 1] / edges[i];
            assert!(
                (observed - expected_ratio).abs() < 1e-3,
                "band {i} ratio {observed} != expected {expected_ratio}",
            );
        }
    }

    #[test]
    fn fft_window_size_is_power_of_two() {
        // Power-of-two pin: rustfft's planner is happy on any
        // size, but the slice-1 corpus hash assumptions (the
        // reference-magnitude formula in particular) bake in
        // power-of-two windowing. Trip if the constant ever
        // drifts to a non-power-of-two.
        assert!(
            FFT_WINDOW_SIZE.is_power_of_two(),
            "FFT_WINDOW_SIZE must stay a power of two; got {FFT_WINDOW_SIZE}",
        );
    }

    #[test]
    fn bpm_clamp_matches_docs_36_section_2_7() {
        // docs/36-reactive-symbol-catalog.md §2.7: "Estimated BPM,
        // clamped `[50, 220]`". A catalog edit that moves the
        // bounds drifts here at the next test run.
        assert_eq!(BPM_CLAMP_MIN, 50.0);
        assert_eq!(BPM_CLAMP_MAX, 220.0);
        // The slice-1 default sits inside the `[50, 220]` musical-
        // tempo range. Compared at runtime through float locals so
        // the constant-comparison clippy lint stays green; the pin
        // still fires if any of the three constants drifts because
        // the inequality switches sign.
        let default = BPM_DEFAULT;
        let lo = BPM_CLAMP_MIN;
        let hi = BPM_CLAMP_MAX;
        assert!(
            default >= lo,
            "BPM_DEFAULT ({default}) < BPM_CLAMP_MIN ({lo})"
        );
        assert!(
            default <= hi,
            "BPM_DEFAULT ({default}) > BPM_CLAMP_MAX ({hi})"
        );
    }

    #[test]
    fn snapshot_default_is_all_zero() {
        let s = AudioUniformSnapshot::default();
        assert_eq!(s.bpm, 0.0);
        assert_eq!(s.bpm_availability, AudioAxisAvailability::Warming);
        assert_eq!(s.rms, 0.0);
        assert_eq!(s.peak, 0.0);
        assert_eq!(s.fft_bands, [0.0; FFT_BAND_COUNT]);
        assert_eq!(s.fft_availability, AudioAxisAvailability::Warming);
        assert!(!s.degraded);
    }

    #[test]
    fn analysis_window_accumulates_short_frames_for_fft() {
        let mut window = AudioAnalysisWindow::default();
        let sample_rate_hz = 48_000;
        let freq_hz = 440.0_f32;
        let chunk = 128;

        let mut analysis = AudioFrame::default();
        for frame_idx in 0..(FFT_WINDOW_SIZE / chunk) {
            let offset = frame_idx * chunk;
            let samples: Vec<f32> = (0..chunk)
                .map(|i| {
                    let t = (offset + i) as f32 / sample_rate_hz as f32;
                    (TAU * freq_hz * t).sin()
                })
                .collect();
            analysis = window.observe(&AudioFrame::new(sample_rate_hz, 1, samples));
        }

        assert_eq!(window.len(), FFT_WINDOW_SIZE);
        let snapshot = extract_uniform_snapshot(&analysis);
        assert_eq!(snapshot.fft_availability, AudioAxisAvailability::Ready);
        assert!(
            snapshot.fft_bands.iter().any(|band| *band > 0.0),
            "accumulated short frames should feed the FFT path",
        );
    }

    #[test]
    fn analysis_window_marks_axes_warming_until_each_window_fills() {
        let mut window = AudioAnalysisWindow::default();
        let sample_rate_hz = 1_000;

        let warming_frame = window.observe(&AudioFrame::new(sample_rate_hz, 1, vec![0.1; 128]));
        let warming = extract_uniform_snapshot(&warming_frame);
        assert_eq!(warming.bpm_availability, AudioAxisAvailability::Warming);
        assert_eq!(warming.fft_availability, AudioAxisAvailability::Warming);

        let fft_fill = FFT_WINDOW_SIZE - window.len();
        let fft_ready_frame =
            window.observe(&AudioFrame::new(sample_rate_hz, 1, vec![0.1; fft_fill]));
        let fft_ready = extract_uniform_snapshot(&fft_ready_frame);
        assert_eq!(fft_ready.bpm_availability, AudioAxisAvailability::Warming);
        assert_eq!(fft_ready.fft_availability, AudioAxisAvailability::Ready);

        let full_fill = analysis_window_samples(sample_rate_hz) - window.len();
        let ready_frame = window.observe(&AudioFrame::new(sample_rate_hz, 1, vec![0.1; full_fill]));
        let ready = extract_uniform_snapshot(&ready_frame);
        assert_eq!(ready.bpm_availability, AudioAxisAvailability::Ready);
        assert_eq!(ready.fft_availability, AudioAxisAvailability::Ready);
    }

    #[test]
    fn analysis_window_capacity_covers_bpm_lag_and_resets_on_rate_change() {
        let cap_48k = analysis_window_samples(48_000);
        assert!(cap_48k > FFT_WINDOW_SIZE);

        let mut window = AudioAnalysisWindow::default();
        let first = AudioFrame::new(48_000, 1, vec![0.1; 256]);
        let second = AudioFrame::new(44_100, 1, vec![0.2; 128]);
        let _ = window.observe(&first);
        assert_eq!(window.len(), 256);
        let analysis = window.observe(&second);
        assert_eq!(window.len(), 128);
        assert_eq!(analysis.sample_rate_hz(), 44_100);
        assert!(analysis.samples().iter().all(|sample| *sample == 0.2));
    }

    #[test]
    fn analysis_window_rms_variance_is_lower_than_callback_variance() {
        let mut window = AudioAnalysisWindow::default();
        let sample_rate_hz = 1_000;
        let chunk = 128;
        let mut callback_rms = Vec::new();
        let mut windowed_rms = Vec::new();

        for idx in 0..48 {
            let amplitude = if idx % 2 == 0 { 0.9 } else { 0.1 };
            let frame = AudioFrame::new(sample_rate_hz, 1, vec![amplitude; chunk]);
            let callback_snapshot = extract_uniform_snapshot(&frame);
            let analysis = window.observe(&frame);

            if window.len() >= FFT_WINDOW_SIZE {
                let window_snapshot = extract_uniform_snapshot(&analysis);
                callback_rms.push(callback_snapshot.rms);
                windowed_rms.push(window_snapshot.rms);
            }
        }

        let callback_variance = variance(&callback_rms);
        let windowed_variance = variance(&windowed_rms);
        assert!(
            windowed_variance * 10.0 < callback_variance,
            "windowed RMS variance {windowed_variance} should be much lower than callback variance {callback_variance}",
        );
    }

    #[test]
    fn non_finite_samples_never_poison_rms_peak_fft_or_bpm() {
        let mut samples = vec![0.25_f32; FFT_WINDOW_SIZE];
        samples[10] = f32::NAN;
        samples[20] = f32::INFINITY;
        samples[30] = f32::NEG_INFINITY;
        let frame = AudioFrame::new(48_000, 1, samples);
        let snap = extract_uniform_snapshot(&frame);
        assert!(snap.rms.is_finite() && snap.rms > 0.0 && snap.rms <= 1.0);
        assert!(snap.peak.is_finite() && (snap.peak - 0.25).abs() < 1e-5);
        assert!(snap.bpm.is_finite());
        assert!(
            snap.fft_bands
                .iter()
                .all(|b| b.is_finite() && *b >= 0.0 && *b <= 1.0)
        );
    }

    #[test]
    fn recent_frame_tracks_latest_1024_samples_for_low_latency_metering() {
        let mut window = AudioAnalysisWindow::default();
        // Push 2048 loud samples followed by 1024 silent samples.
        window.push(&AudioFrame::new(48_000, 1, vec![0.9_f32; 2048]));
        window.push(&AudioFrame::new(48_000, 1, vec![0.0_f32; FFT_WINDOW_SIZE]));

        let recent = window.recent_frame(FFT_WINDOW_SIZE);
        assert_eq!(recent.samples().len(), FFT_WINDOW_SIZE);
        let live =
            super::extract_live_metering_snapshot(&recent, 120.0, AudioAxisAvailability::Warming);
        // Live metering reflects the latest 21 ms (silence), not stale 0.9 peaks.
        assert_eq!(live.rms, 0.0);
        assert_eq!(live.peak, 0.0);
    }
}

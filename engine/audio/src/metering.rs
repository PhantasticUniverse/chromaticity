// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! ADR-023 world-class audio metering & DSP primitives.
//!
//! Pure-Rust, `unsafe_code = "forbid"`, allocation-disciplined DSP
//! building blocks that close the pre-professional metering gap
//! documented in `docs/adr/ADR-023-audio-metering-data-model.md`:
//!
//! - [`compute_true_peak_dbtp`] — 4× oversampled Catmull-Rom 4-tap
//!   polyphase FIR true-peak estimator (`dBTP`) with honest clip
//!   detection (`dbtp >= -1.0 || max_abs >= 1.0`).
//! - [`StereoLevels`] / [`compute_stereo_levels`] — genuine per-channel
//!   L/R RMS and peak over interleaved PCM (collapsing cleanly to mono
//!   when `channels == 1`).
//! - [`AgcNormalizer`] — `docs/06-audio-engine.md` §7.4 / §7.12 rolling
//!   p95 peak tracker (`L_peak30`) + asymmetric EMA normaliser
//!   (`L_norm` / `L_smooth`) with operator `threshold` gate.
//! - [`SpectralFluxOnsetDetector`] — independently derived half-wave
//!   rectified spectral-flux onset detector over FFT magnitudes with
//!   adaptive median + MAD threshold and refractory cooldown.
//! - [`db_to_linear`] / [`apply_input_gain`] — post-ingest input-gain
//!   scaling with a bit-identical unity fast path so existing
//!   `EV-AUDIO-TRACE` goldens remain unchanged at default gain (`1.0`).
//!
//! Evidence: EV-AUDIO-TRACE (T1) — in-source drift pins cover every
//! normative constant and DSP invariant below.

use std::collections::VecDeque;

/// Clip threshold in dBTP per ADR-023 §1 (`true_peak >= -1.0 dBTP`).
pub const TRUE_PEAK_CLIP_THRESHOLD_DBTP: f32 = -1.0;

/// Linear amplitude floor for dBTP conversion (`1e-5` = `-100.0 dBTP`),
/// matching `src/lib/audio/metering.ts::AMP_FLOOR` so silence stays
/// finite rather than returning `-inf`.
pub const TRUE_PEAK_AMP_FLOOR: f32 = 1.0e-5;

/// Floor true-peak value in dBTP returned for empty or silent buffers.
pub const TRUE_PEAK_FLOOR_DBTP: f32 = -100.0;

/// Default rolling window length for [`AgcNormalizer`] (30 s at the
/// canonical 375 Hz hop cadence per `docs/06-audio-engine.md` §7.4).
pub const AGC_DEFAULT_WINDOW_HOPS: usize = 11_250;

/// Floor denominator for AGC normalisation per `docs/06-audio-engine.md` §7.4.
pub const AGC_EPSILON: f32 = 1.0e-4;

/// Rising asymmetric-EMA coefficient for `L_smooth` per `docs/06` §7.4.
pub const AGC_ALPHA_RISE: f32 = 0.30;

/// Falling asymmetric-EMA coefficient for `L_smooth` per `docs/06` §7.4.
pub const AGC_ALPHA_FALL: f32 = 0.08;

/// Default adaptive-threshold history length for
/// [`SpectralFluxOnsetDetector`] (43 hops ≈ 115 ms at 375 Hz per `docs/06` §7.5).
pub const ONSET_DEFAULT_HISTORY_HOPS: usize = 43;

/// Default MAD multiplier `k` for [`SpectralFluxOnsetDetector`] per `docs/06` §7.5.
pub const ONSET_DEFAULT_MAD_MULTIPLIER: f32 = 1.5;

/// Default refractory cooldown in hops for [`SpectralFluxOnsetDetector`]
/// (19 hops ≈ 50 ms at 375 Hz per `docs/06` §7.5).
pub const ONSET_DEFAULT_REFRACTORY_HOPS: usize = 19;

/// Minimum spectral-flux threshold floor so numerical noise during
/// silence does not trigger false onsets.
pub const ONSET_MIN_THRESHOLD: f32 = 1.0e-4;

/// Exact 4-tap Catmull-Rom polyphase FIR weights for 4× oversampling at
/// fractional phases `t = 0.25`, `0.50`, and `0.75` (phase `0.0` is the
/// discrete sample itself). Every tap weight is an exact multiple of
/// `1/128` (zero IEEE-754 rounding error) and each phase sums to `1.0`.
const CATMULL_ROM_PHASES: [[f32; 4]; 3] = [
    [-0.070_312_5, 0.867_187_5, 0.226_562_5, -0.023_437_5],
    [-0.062_5, 0.562_5, 0.562_5, -0.062_5],
    [-0.023_437_5, 0.226_562_5, 0.867_187_5, -0.070_312_5],
];

/// Compute the 4× oversampled true peak in `dBTP` and clip flag for a
/// mono sample slice.
///
/// Uses 4-tap Catmull-Rom polyphase FIR interpolation at fractional
/// phases `{0.25, 0.50, 0.75}` with edge-clamped boundary extensions
/// (avoiding artificial step-edge Gibbs overshoot on stateless single-
/// buffer calls) to detect inter-sample peaks that exceed discrete
/// sample magnitudes.
///
/// Returns `(dbtp, clip)` where:
/// - `dbtp` is `20 * log10(max_true_peak_linear)`, floored at
///   [`TRUE_PEAK_FLOOR_DBTP`] (`-100.0`) for silent or empty buffers.
/// - `clip` is `dbtp >= -1.0 || max_abs >= 1.0`.
///
/// Non-finite samples (`NaN`, `±inf`) are ignored when computing the
/// finite peak so malformed input never produces `NaN`.
#[must_use]
pub fn compute_true_peak_dbtp(samples: &[f32]) -> (f32, bool) {
    if samples.is_empty() {
        return (TRUE_PEAK_FLOOR_DBTP, false);
    }

    let last_idx = (samples.len() - 1) as isize;
    let sample_at = |idx: isize| -> f32 {
        let clamped = idx.clamp(0, last_idx) as usize;
        let v = samples[clamped];
        if v.is_finite() { v } else { 0.0 }
    };

    let mut max_abs = 0.0_f32;
    let mut max_true_peak = 0.0_f32;

    for i in 0..samples.len() {
        let idx = i as isize;
        let y0 = sample_at(idx - 1);
        let y1 = sample_at(idx);
        let y2 = sample_at(idx + 1);
        let y3 = sample_at(idx + 2);

        let abs_y1 = y1.abs();
        if abs_y1 > max_abs {
            max_abs = abs_y1;
        }
        if abs_y1 > max_true_peak {
            max_true_peak = abs_y1;
        }

        for weights in &CATMULL_ROM_PHASES {
            let interp =
                (weights[0] * y0 + weights[1] * y1 + weights[2] * y2 + weights[3] * y3).abs();
            if interp > max_true_peak {
                max_true_peak = interp;
            }
        }
    }

    let dbtp = if max_true_peak <= TRUE_PEAK_AMP_FLOOR {
        TRUE_PEAK_FLOOR_DBTP
    } else {
        20.0 * max_true_peak.log10()
    };
    let clip = dbtp >= TRUE_PEAK_CLIP_THRESHOLD_DBTP || max_abs >= 1.0;

    (dbtp, clip)
}

/// Per-channel stereo RMS and peak magnitudes (unclamped linear
/// amplitude so over-0-dBFS peaks are preserved per ADR-023 §2).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StereoLevels {
    /// Left-channel RMS linear amplitude (or mono RMS when `channels == 1`).
    pub rms_l: f32,
    /// Right-channel RMS linear amplitude (or mono RMS when `channels == 1`).
    pub rms_r: f32,
    /// Left-channel peak linear magnitude (or mono peak when `channels == 1`).
    pub peak_l: f32,
    /// Right-channel peak linear magnitude (or mono peak when `channels == 1`).
    pub peak_r: f32,
}

/// Compute real per-channel L/R RMS and peak from an interleaved PCM
/// slice.
///
/// - When `interleaved` is empty or `channels == 0`, returns all zeros.
/// - When `channels == 1`, collapses cleanly to mono (`rms_l == rms_r`,
///   `peak_l == peak_r`).
/// - When `channels >= 2`, reads channel 0 as Left and channel 1 as
///   Right across every complete interleaved frame.
#[must_use]
pub fn compute_stereo_levels(interleaved: &[f32], channels: usize) -> StereoLevels {
    if interleaved.is_empty() || channels == 0 {
        return StereoLevels::default();
    }

    if channels == 1 {
        let mut sum_sq = 0.0_f64;
        let mut peak = 0.0_f32;
        for &s in interleaved {
            let v = if s.is_finite() { s } else { 0.0 };
            let abs = v.abs();
            if abs > peak {
                peak = abs;
            }
            sum_sq += f64::from(v) * f64::from(v);
        }
        let rms = (sum_sq / interleaved.len() as f64).sqrt() as f32;
        return StereoLevels {
            rms_l: rms,
            rms_r: rms,
            peak_l: peak,
            peak_r: peak,
        };
    }

    let mut sum_sq_l = 0.0_f64;
    let mut sum_sq_r = 0.0_f64;
    let mut peak_l = 0.0_f32;
    let mut peak_r = 0.0_f32;
    let mut frames = 0_usize;

    for frame in interleaved.chunks_exact(channels) {
        let l = if frame[0].is_finite() { frame[0] } else { 0.0 };
        let r = if frame[1].is_finite() { frame[1] } else { 0.0 };

        let abs_l = l.abs();
        let abs_r = r.abs();
        if abs_l > peak_l {
            peak_l = abs_l;
        }
        if abs_r > peak_r {
            peak_r = abs_r;
        }
        sum_sq_l += f64::from(l) * f64::from(l);
        sum_sq_r += f64::from(r) * f64::from(r);
        frames += 1;
    }

    if frames == 0 {
        return StereoLevels::default();
    }

    StereoLevels {
        rms_l: (sum_sq_l / frames as f64).sqrt() as f32,
        rms_r: (sum_sq_r / frames as f64).sqrt() as f32,
        peak_l,
        peak_r,
    }
}

/// Output of one [`AgcNormalizer::observe`] step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AgcOutput {
    /// Raw pre-AGC level observed on this hop (`L_raw`).
    pub raw_level: f32,
    /// Current rolling p95 peak tracker (`L_peak30`).
    pub peak_p95: f32,
    /// Unclamped `L_raw / max(L_peak30, epsilon)` (`syn_InputLevel` per
    /// `docs/06` §7.12).
    pub input_level: f32,
    /// Clamped `[0, 1]` normalised level (`L_norm`), zeroed when
    /// `gated` is true.
    pub normalized_level: f32,
    /// Asymmetric-EMA smoothed level (`L_smooth`), zeroed when `gated`
    /// is true.
    pub smoothed_level: f32,
    /// True when `raw_level < threshold` (`syn_Threshold` gate).
    pub gated: bool,
}

/// Rolling p95 peak tracker + asymmetric-EMA AGC normaliser with
/// operator threshold gate (`docs/06-audio-engine.md` §7.4 & §7.12).
#[derive(Debug, Clone)]
pub struct AgcNormalizer {
    window_hops: usize,
    history: VecDeque<f32>,
    scratch: Vec<f32>,
    smoothed: f32,
    threshold: f32,
}

impl Default for AgcNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl AgcNormalizer {
    /// Create a normaliser with the canonical 30 s (`11_250`-hop) p95
    /// window and default `0.0` threshold gate.
    #[must_use]
    pub fn new() -> Self {
        Self::with_window(AGC_DEFAULT_WINDOW_HOPS)
    }

    /// Create a normaliser with a custom rolling window length (clamped
    /// to at least `1` hop).
    #[must_use]
    pub fn with_window(window_hops: usize) -> Self {
        let cap = window_hops.max(1);
        Self {
            window_hops: cap,
            history: VecDeque::with_capacity(cap.min(1024)),
            scratch: Vec::new(),
            smoothed: 0.0,
            threshold: 0.0,
        }
    }

    /// Configure the operator noise-gate threshold (`syn_Threshold`),
    /// clamped to `[0.0, 1.0]`. Non-finite inputs collapse to `0.0`.
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = if threshold.is_finite() {
            threshold.clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    /// Return the active operator threshold gate value in `[0.0, 1.0]`.
    #[must_use]
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Reset all rolling history and smoothed EMA state while
    /// preserving the configured window size and threshold.
    pub fn reset(&mut self) {
        self.history.clear();
        self.smoothed = 0.0;
    }

    /// Observe one hop's raw level (`L_raw >= 0.0`) and return the
    /// updated AGC metrics and gate status.
    pub fn observe(&mut self, raw_level: f32) -> AgcOutput {
        let raw = if raw_level.is_finite() && raw_level > 0.0 {
            raw_level
        } else {
            0.0
        };

        if self.history.len() == self.window_hops {
            let _ = self.history.pop_front();
        }
        self.history.push_back(raw);

        let peak_p95 = percentile_95(&self.history, &mut self.scratch);
        let denom = peak_p95.max(AGC_EPSILON);
        let input_level = raw / denom;
        let gated = raw < self.threshold;

        let target_norm = if gated {
            0.0
        } else {
            input_level.clamp(0.0, 1.0)
        };

        let alpha = if target_norm >= self.smoothed {
            AGC_ALPHA_RISE
        } else {
            AGC_ALPHA_FALL
        };
        self.smoothed = alpha * target_norm + (1.0 - alpha) * self.smoothed;

        let smoothed_level = if gated { 0.0 } else { self.smoothed };

        AgcOutput {
            raw_level: raw,
            peak_p95,
            input_level,
            normalized_level: target_norm,
            smoothed_level,
            gated,
        }
    }
}

/// Independently derived half-wave rectified spectral-flux onset
/// detector with adaptive median + MAD threshold and refractory
/// cooldown (`docs/06-audio-engine.md` §7.5 / ADR-023 §6).
#[derive(Debug, Clone)]
pub struct SpectralFluxOnsetDetector {
    history_hops: usize,
    mad_multiplier: f32,
    refractory_hops: usize,
    prev_magnitudes: Vec<f32>,
    flux_history: VecDeque<f32>,
    scratch: Vec<f32>,
    cooldown_remaining: usize,
}

impl Default for SpectralFluxOnsetDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SpectralFluxOnsetDetector {
    /// Create a detector with the normative `docs/06` §7.5 defaults
    /// (`W = 43` hops, `k = 1.5`, refractory `R = 19` hops).
    #[must_use]
    pub fn new() -> Self {
        Self::with_params(
            ONSET_DEFAULT_HISTORY_HOPS,
            ONSET_DEFAULT_MAD_MULTIPLIER,
            ONSET_DEFAULT_REFRACTORY_HOPS,
        )
    }

    /// Create a detector with explicit history window, MAD multiplier,
    /// and refractory cooldown (in hops).
    #[must_use]
    pub fn with_params(history_hops: usize, mad_multiplier: f32, refractory_hops: usize) -> Self {
        let cap = history_hops.max(1);
        let k = if mad_multiplier.is_finite() && mad_multiplier >= 0.0 {
            mad_multiplier
        } else {
            ONSET_DEFAULT_MAD_MULTIPLIER
        };
        Self {
            history_hops: cap,
            mad_multiplier: k,
            refractory_hops,
            prev_magnitudes: Vec::new(),
            flux_history: VecDeque::with_capacity(cap),
            scratch: Vec::with_capacity(cap),
            cooldown_remaining: 0,
        }
    }

    /// Reset previous-frame magnitudes, flux history, and cooldown.
    pub fn reset(&mut self) {
        self.prev_magnitudes.clear();
        self.flux_history.clear();
        self.cooldown_remaining = 0;
    }

    /// Observe one FFT magnitude spectrum and return
    /// `(onset_strength, on_beat)`.
    ///
    /// - `onset_strength` is in `[0.0, 1.0]`, scaled relative to the
    ///   current adaptive threshold (`flux / max(theta, min_threshold)`).
    /// - `on_beat` is `true` when positive half-wave rectified spectral
    ///   flux strictly exceeds the adaptive threshold and the
    ///   refractory cooldown has expired.
    ///
    /// On the first frame (or after a bin-count change), the spectrum
    /// seeds the previous-magnitude buffer and returns `(0.0, false)`
    /// so startup never fires a spurious onset.
    pub fn observe(&mut self, magnitudes: &[f32]) -> (f32, bool) {
        if magnitudes.is_empty() {
            return (0.0, false);
        }

        if self.prev_magnitudes.len() != magnitudes.len() {
            self.prev_magnitudes.clear();
            self.prev_magnitudes.extend(
                magnitudes
                    .iter()
                    .map(|&m| if m.is_finite() && m > 0.0 { m } else { 0.0 }),
            );
            return (0.0, false);
        }

        let mut flux = 0.0_f32;
        for (prev, &curr_raw) in self.prev_magnitudes.iter_mut().zip(magnitudes.iter()) {
            let curr = if curr_raw.is_finite() && curr_raw > 0.0 {
                curr_raw
            } else {
                0.0
            };
            let diff = curr - *prev;
            if diff > 0.0 {
                flux += diff;
            }
            *prev = curr;
        }

        let threshold =
            adaptive_threshold(&self.flux_history, self.mad_multiplier, &mut self.scratch);

        let onset_strength = if flux <= 0.0 {
            0.0
        } else {
            (flux / threshold.max(ONSET_MIN_THRESHOLD)).clamp(0.0, 1.0)
        };

        let on_beat = if self.cooldown_remaining > 0 {
            self.cooldown_remaining -= 1;
            false
        } else if flux > threshold {
            self.cooldown_remaining = self.refractory_hops;
            true
        } else {
            false
        };

        if self.flux_history.len() == self.history_hops {
            let _ = self.flux_history.pop_front();
        }
        self.flux_history.push_back(flux);

        (onset_strength, on_beat)
    }
}

/// Convert a decibel gain value (`dB`) to a linear amplitude multiplier
/// (`10^(db / 20)`). Non-finite inputs return `0.0`.
#[must_use]
pub fn db_to_linear(db: f32) -> f32 {
    if !db.is_finite() {
        return 0.0;
    }
    10.0_f32.powf(db / 20.0)
}

/// Apply a linear input-gain multiplier in-place to a sample buffer.
///
/// Unity gain (`gain_linear == 1.0`) is an explicit no-op so callers
/// preserve 100% bit-identity on existing `EV-AUDIO-TRACE` goldens.
/// Negative or non-finite gain values are sanitised to `0.0` (mute).
pub fn apply_input_gain(samples: &mut [f32], gain_linear: f32) {
    if gain_linear == 1.0 {
        return;
    }
    let gain = if gain_linear.is_finite() && gain_linear >= 0.0 {
        gain_linear
    } else {
        0.0
    };
    for sample in samples.iter_mut() {
        *sample *= gain;
    }
}

/// Compute the 95th percentile of `values` using an in-place O(N)
/// selection on `scratch`.
fn percentile_95(values: &VecDeque<f32>, scratch: &mut Vec<f32>) -> f32 {
    let n = values.len();
    if n == 0 {
        return 0.0;
    }
    scratch.clear();
    scratch.extend(values.iter().copied());
    let idx = (((n - 1) as f32) * 0.95).round() as usize;
    let idx = idx.min(n - 1);
    let (_, nth, _) = scratch.select_nth_unstable_by(idx, f32::total_cmp);
    *nth
}

/// Compute `max(ONSET_MIN_THRESHOLD, median(history) + k * MAD(history))`
/// over the rolling flux history.
fn adaptive_threshold(history: &VecDeque<f32>, mad_multiplier: f32, scratch: &mut Vec<f32>) -> f32 {
    if history.is_empty() {
        return ONSET_MIN_THRESHOLD;
    }
    scratch.clear();
    scratch.extend(history.iter().copied());
    let med = median_in_place(scratch);

    for v in scratch.iter_mut() {
        *v = (*v - med).abs();
    }
    let mad = median_in_place(scratch);
    (med + mad_multiplier * mad).max(ONSET_MIN_THRESHOLD)
}

/// Compute the median of a non-empty slice in-place.
fn median_in_place(values: &mut [f32]) -> f32 {
    let n = values.len();
    if n == 0 {
        return 0.0;
    }
    let mid = n / 2;
    let (_, upper, _) = values.select_nth_unstable_by(mid, f32::total_cmp);
    let hi = *upper;
    if n % 2 == 1 {
        hi
    } else {
        let lo = values[..mid]
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        0.5 * (lo + hi)
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod metering_pins {
    //! Evidence: EV-AUDIO-TRACE (T1) — drift pins for ADR-023 audio
    //! metering and DSP primitives (4× Catmull-Rom true peak, stereo
    //! L/R levels, §7.4 AGC p95 + asymmetric EMA + gate, §7.5
    //! spectral-flux onset detector, and input-gain scaling).

    use super::*;
    use std::f32::consts::TAU;

    #[test]
    fn true_peak_empty_and_silence_return_floor_without_clip() {
        assert_eq!(compute_true_peak_dbtp(&[]), (TRUE_PEAK_FLOOR_DBTP, false));
        assert_eq!(
            compute_true_peak_dbtp(&[0.0; 64]),
            (TRUE_PEAK_FLOOR_DBTP, false)
        );
    }

    #[test]
    fn true_peak_dc_full_scale_reads_zero_dbtp_and_clips() {
        let (dbtp, clip) = compute_true_peak_dbtp(&[1.0; 32]);
        assert!(
            dbtp.abs() < 1e-4,
            "expected ~0.0 dBTP for unit DC, got {dbtp}"
        );
        assert!(clip, "unit DC (0 dBTP) must latch clip");
    }

    #[test]
    fn true_peak_minus_six_db_sine_does_not_clip() {
        let n = 512;
        let samples: Vec<f32> = (0..n)
            .map(|i| 0.5 * (TAU * 1000.0 * i as f32 / 48_000.0).sin())
            .collect();
        let (dbtp, clip) = compute_true_peak_dbtp(&samples);
        let expected = 20.0 * 0.5_f32.log10(); // -6.0206 dB
        assert!(
            (dbtp - expected).abs() < 0.05,
            "expected ~{expected:.2} dBTP, got {dbtp:.3}"
        );
        assert!(!clip, "-6 dBFS sine must not latch clip");
    }

    #[test]
    fn true_peak_detects_inter_sample_peak_above_zero_dbtp() {
        // Every discrete sample has |x| = 0.9 (< 1.0, -0.915 dBFS), yet
        // the 4× Catmull-Rom inter-sample peak between consecutive
        // +0.9, +0.9 samples flanked by -0.9 reaches 1.25 * 0.9 = 1.125
        // (+1.023 dBTP > 0 dBTP).
        let samples = [-0.9_f32, 0.9, 0.9, -0.9, -0.9, 0.9, 0.9, -0.9];
        let (dbtp, clip) = compute_true_peak_dbtp(&samples);
        assert!(
            dbtp > 0.0,
            "expected inter-sample peak > 0.0 dBTP, got {dbtp}"
        );
        assert!(
            (dbtp - 20.0 * 1.125_f32.log10()).abs() < 1e-3,
            "expected +1.023 dBTP, got {dbtp}"
        );
        assert!(clip, "inter-sample peak > 0 dBTP must latch clip");
    }

    #[test]
    fn true_peak_handles_non_finite_samples_without_nan() {
        let samples = [0.0, f32::NAN, 0.5, f32::INFINITY, f32::NEG_INFINITY, 0.0];
        let (dbtp, clip) = compute_true_peak_dbtp(&samples);
        assert!(dbtp.is_finite(), "dbtp must stay finite, got {dbtp}");
        assert!(!clip);
    }

    #[test]
    fn stereo_levels_separate_left_and_right_channels() {
        // Left-only signal: [0.5, 0.0, -0.5, 0.0, ...]
        let left_only = [0.5_f32, 0.0, -0.5, 0.0, 0.5, 0.0, -0.5, 0.0];
        let lv = compute_stereo_levels(&left_only, 2);
        assert!((lv.rms_l - 0.5).abs() < 1e-6);
        assert_eq!(lv.rms_r, 0.0);
        assert!((lv.peak_l - 0.5).abs() < 1e-6);
        assert_eq!(lv.peak_r, 0.0);

        // Right-only signal: [0.0, 0.75, 0.0, -0.75, ...]
        let right_only = [0.0_f32, 0.75, 0.0, -0.75, 0.0, 0.75, 0.0, -0.75];
        let rv = compute_stereo_levels(&right_only, 2);
        assert_eq!(rv.rms_l, 0.0);
        assert!((rv.rms_r - 0.75).abs() < 1e-6);
        assert_eq!(rv.peak_l, 0.0);
        assert!((rv.peak_r - 0.75).abs() < 1e-6);
    }

    #[test]
    fn stereo_levels_collapse_cleanly_for_mono_and_empty() {
        let mono = [0.5_f32, -0.5, 0.5, -0.5];
        let m = compute_stereo_levels(&mono, 1);
        assert!((m.rms_l - 0.5).abs() < 1e-6);
        assert_eq!(m.rms_l, m.rms_r);
        assert!((m.peak_l - 0.5).abs() < 1e-6);
        assert_eq!(m.peak_l, m.peak_r);

        assert_eq!(compute_stereo_levels(&[], 2), StereoLevels::default());
        assert_eq!(compute_stereo_levels(&mono, 0), StereoLevels::default());
    }

    #[test]
    fn agc_normalizer_converges_on_steady_state_and_honours_gate() {
        let mut agc = AgcNormalizer::with_window(64);
        assert_eq!(agc.threshold(), 0.0);

        // Feed steady-state raw level 0.25 -> L_peak30 = 0.25, L_norm = 1.0,
        // and L_smooth converges toward 1.0.
        let mut last = AgcOutput::default();
        for _ in 0..40 {
            last = agc.observe(0.25);
        }
        assert!(!last.gated);
        assert!((last.peak_p95 - 0.25).abs() < 1e-6);
        assert!((last.normalized_level - 1.0).abs() < 1e-6);
        assert!(
            last.smoothed_level > 0.99,
            "expected smoothed level to converge near 1.0, got {}",
            last.smoothed_level
        );

        // Asymmetric EMA check: a drop to 0.0 falls slower (alpha=0.08)
        // than the initial rise (alpha=0.30).
        let after_one_drop = agc.observe(0.0);
        assert!(
            after_one_drop.smoothed_level > 0.90,
            "falling EMA should decay slowly (alpha=0.08), got {}",
            after_one_drop.smoothed_level
        );

        // Threshold gate: raw < 0.20 must report gated=true and zeroed
        // normalized/smoothed output.
        agc.set_threshold(0.20);
        let gated_out = agc.observe(0.10);
        assert!(gated_out.gated);
        assert_eq!(gated_out.normalized_level, 0.0);
        assert_eq!(gated_out.smoothed_level, 0.0);

        // Above threshold un-gates immediately.
        let open_out = agc.observe(0.25);
        assert!(!open_out.gated);
        assert!(open_out.normalized_level > 0.0);
        assert!(open_out.smoothed_level > 0.0);
    }

    #[test]
    fn spectral_flux_onset_detector_fires_on_impulse_and_respects_refractory() {
        let mut detector = SpectralFluxOnsetDetector::with_params(16, 1.5, 4);
        let quiet = [0.01_f32; 8];
        let impulse = [0.9_f32; 8];

        // Warm up with steady low spectrum -> no beats.
        for _ in 0..16 {
            let (strength, on_beat) = detector.observe(&quiet);
            assert!(!on_beat, "steady quiet spectrum must not trigger beat");
            assert_eq!(strength, 0.0);
        }

        // First sudden impulse triggers an onset.
        let (strength, on_beat) = detector.observe(&impulse);
        assert!(on_beat, "sudden spectral impulse must trigger on_beat");
        assert!(
            strength > 0.5,
            "expected strong onset_strength, got {strength}"
        );

        // Drop back to quiet then fire immediately within the 4-hop
        // refractory window -> must be suppressed.
        let _ = detector.observe(&quiet);
        let (_, suppressed) = detector.observe(&impulse);
        assert!(
            !suppressed,
            "impulse inside refractory window must be suppressed"
        );

        // Drain the rest of the refractory window and quiet down.
        for _ in 0..6 {
            let _ = detector.observe(&quiet);
        }

        // After refractory expires, a new impulse fires again.
        let (strength2, on_beat2) = detector.observe(&impulse);
        assert!(on_beat2, "impulse after refractory must trigger on_beat");
        assert!(strength2 > 0.5);
    }

    #[test]
    fn input_gain_db_conversion_and_unity_bit_identity() {
        assert!((db_to_linear(0.0) - 1.0).abs() < 1e-6);
        assert!((db_to_linear(6.0206) - 2.0).abs() < 1e-3);
        assert!((db_to_linear(-6.0206) - 0.5).abs() < 1e-3);
        assert_eq!(db_to_linear(f32::NAN), 0.0);

        // Unity fast path preserves exact bit-identity (including -0.0).
        let original = [-0.75_f32, -0.0, 0.0, 0.123_456_7, 0.999_999_9];
        let mut buf = original;
        apply_input_gain(&mut buf, 1.0);
        for (a, b) in original.iter().zip(buf.iter()) {
            assert_eq!(a.to_bits(), b.to_bits(), "unity gain must be bit-identical");
        }

        // 2.0x scaling (+6 dB).
        let mut scaled = [0.25_f32, -0.5, 0.0];
        apply_input_gain(&mut scaled, 2.0);
        assert_eq!(scaled, [0.5, -1.0, 0.0]);

        // Negative / NaN gain sanitises to mute (0.0).
        let mut muted = [0.5_f32, -0.5];
        apply_input_gain(&mut muted, -2.0);
        assert_eq!(muted, [0.0, 0.0]);

        let mut nan_muted = [0.5_f32, -0.5];
        apply_input_gain(&mut nan_muted, f32::NAN);
        assert_eq!(nan_muted, [0.0, 0.0]);
    }
}

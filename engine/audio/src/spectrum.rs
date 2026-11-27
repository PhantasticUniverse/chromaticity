// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! ADR-023 Waves 5 & 6: 64-band log-spaced spectrum analyser and
//! two-stage fused IOI + autocorrelation BPM estimator.
//!
//! Pure-Rust, `unsafe_code = "forbid"`, zero `unwrap`/`expect` in
//! production paths:
//!
//! - [`compute_log_spectrum_64`] — Hann-windowed 1024-point iterative
//!   radix-2 Cooley-Tukey FFT mapped onto 64 log-spaced frequency bands
//!   covering `20 Hz ..= 20 kHz`, dB-compressed over `-60 dB ..= 0 dB`
//!   into `[0.0, 1.0]`.
//! - [`downsample_64_to_48`] — exact area-weighted 4:3 downsampler for
//!   the 48-bar `AudioReactivity` visualiser widget.
//! - [`FusedBpmEstimator`] / [`BpmEstimate`] — rolling 6-second
//!   onset-flux ring buffer resampled to a fixed 100 Hz tick grid,
//!   combining normalised autocorrelation (with a log-tempo Gaussian
//!   prior centered at 120 BPM to reject half/double-time octave jumps)
//!   with recent peak inter-onset intervals (IOI).
//!
//! Evidence: EV-AUDIO-TRACE (T1) — in-source drift pins cover spectrum
//! silence, pure-tone peak localisation, downsampler bounds, synthetic
//! tempo convergence, and noise/silence low-confidence rejection.

use std::collections::VecDeque;
use std::f32::consts::TAU;

/// Number of log-spaced bands produced by [`compute_log_spectrum_64`].
pub const SPECTRUM_BANDS_64: usize = 64;

/// Number of bars produced by [`downsample_64_to_48`] for the
/// `AudioReactivity` widget.
pub const SPECTRUM_BANDS_48: usize = 48;

/// Fixed radix-2 FFT window length used by [`compute_log_spectrum_64`].
pub const SPECTRUM_FFT_SIZE: usize = 1024;

/// `log2(SPECTRUM_FFT_SIZE)` used for bit-reversal indexing.
const SPECTRUM_FFT_LOG2: u32 = 10;

/// Lower frequency edge of the 64-band log-spaced spectrum, in Hz.
pub const SPECTRUM_MIN_HZ: f32 = 20.0;

/// Upper frequency edge of the 64-band log-spaced spectrum, in Hz.
pub const SPECTRUM_MAX_HZ: f32 = 20_000.0;

/// Lower dB floor for spectrum compression (`-60 dB` maps to `0.0`).
pub const SPECTRUM_DB_FLOOR: f32 = -60.0;

/// Upper dB ceiling for spectrum compression (`0 dB` maps to `1.0`).
pub const SPECTRUM_DB_CEIL: f32 = 0.0;

/// Linear amplitude corresponding to [`SPECTRUM_DB_FLOOR`] (`10^(-60/20)`).
const SPECTRUM_LINEAR_FLOOR: f32 = 1.0e-3;

/// Internal fixed tick rate for [`FusedBpmEstimator`] (100 Hz = 10 ms/tick).
const BPM_TICK_HZ: f32 = 100.0;

/// Duration of one internal tick in seconds.
const BPM_TICK_DT: f64 = 1.0 / (BPM_TICK_HZ as f64);

/// Rolling 6-second onset-flux history length in ticks (`600` ticks at 100 Hz).
const BPM_BUFFER_TICKS: usize = 600;

/// Minimum ticks required before autocorrelation produces an estimate (2.0 s).
const BPM_MIN_ANALYSIS_TICKS: usize = 200;

/// Minimum tempo tracked by [`FusedBpmEstimator`], in BPM.
pub const FUSED_BPM_MIN: f32 = 60.0;

/// Maximum tempo tracked by [`FusedBpmEstimator`], in BPM.
pub const FUSED_BPM_MAX: f32 = 180.0;

/// Nominal default tempo returned prior to lock, in BPM.
pub const FUSED_BPM_DEFAULT: f32 = 120.0;

/// Shortest autocorrelation lag in ticks (corresponds to ~181.8 BPM).
const BPM_LAG_MIN: usize = 33;

/// Longest autocorrelation lag in ticks (corresponds to 60.0 BPM).
const BPM_LAG_MAX: usize = 100;

/// Gaussian log-tempo prior width in octaves around 120 BPM.
const BPM_PRIOR_SIGMA_OCTAVES: f32 = 0.55;

/// Minimum interval between detected onset peaks in seconds (~240 BPM cap).
const BPM_ONSET_REFRACTORY_SECS: f64 = 0.25;

/// Rolling window (in ticks) used for adaptive onset-peak thresholding (1.5 s).
const BPM_ONSET_WINDOW_TICKS: usize = 150;

/// Maximum number of recent onset timestamps retained for IOI analysis.
const BPM_MAX_ONSET_HISTORY: usize = 32;

/// Number of internal ticks between full autocorrelation re-evaluations (50 ms).
const BPM_EVAL_STRIDE_TICKS: usize = 5;

/// Compute a 64-band log-spaced magnitude spectrum over `20 Hz ..= 20 kHz`
/// normalized to `[0.0, 1.0]` with `-60 dB ..= 0 dB` compression.
///
/// - Uses up to [`SPECTRUM_FFT_SIZE`] (`1024`) samples (taking the most
///   recent `1024` samples if `samples` is longer, or zero-padding to
///   `1024` if shorter).
/// - Applies a Hann window to the active sample span before running an
///   in-place pure-Rust iterative radix-2 Cooley-Tukey FFT.
/// - Returns all zeros if `samples` is empty or `sample_rate` is
///   non-finite / non-positive.
#[must_use]
pub fn compute_log_spectrum_64(samples: &[f32], sample_rate: f32) -> [f32; SPECTRUM_BANDS_64] {
    let mut bands = [0.0_f32; SPECTRUM_BANDS_64];
    if samples.is_empty() || !sample_rate.is_finite() || sample_rate <= 0.0 {
        return bands;
    }

    let active = if samples.len() > SPECTRUM_FFT_SIZE {
        &samples[samples.len() - SPECTRUM_FFT_SIZE..]
    } else {
        samples
    };
    let n = active.len();

    let mut re = [0.0_f32; SPECTRUM_FFT_SIZE];
    let mut im = [0.0_f32; SPECTRUM_FFT_SIZE];

    if n == 1 {
        let s = active[0];
        re[0] = if s.is_finite() { s } else { 0.0 };
    } else {
        let denom = (n - 1) as f32;
        for (i, &raw) in active.iter().enumerate() {
            let s = if raw.is_finite() { raw } else { 0.0 };
            let hann = 0.5 * (1.0 - (TAU * i as f32 / denom).cos());
            re[i] = s * hann;
        }
    }

    fft_radix2_1024(&mut re, &mut im);

    // A full-scale unit sine windowed by Hann of length `n` has a
    // positive-frequency bin magnitude of `0.25 * n` (0.5 coherent
    // window gain * 0.5 positive-side split).
    let ref_mag = ((n as f32) * 0.25).max(1.0);
    let half_bins = SPECTRUM_FFT_SIZE / 2;
    let mut mags = [0.0_f32; SPECTRUM_FFT_SIZE / 2 + 1];
    for k in 0..=half_bins {
        mags[k] = re[k].hypot(im[k]) / ref_mag;
    }

    let bin_hz = sample_rate / (SPECTRUM_FFT_SIZE as f32);
    let freq_ratio = SPECTRUM_MAX_HZ / SPECTRUM_MIN_HZ;
    let db_span = SPECTRUM_DB_CEIL - SPECTRUM_DB_FLOOR;

    for (b, slot) in bands.iter_mut().enumerate() {
        let f_lo = SPECTRUM_MIN_HZ * freq_ratio.powf(b as f32 / SPECTRUM_BANDS_64 as f32);
        let f_hi = SPECTRUM_MIN_HZ * freq_ratio.powf((b + 1) as f32 / SPECTRUM_BANDS_64 as f32);

        let bin_lo_f = f_lo / bin_hz;
        let bin_hi_f = f_hi / bin_hz;

        if bin_lo_f > half_bins as f32 {
            *slot = 0.0;
            continue;
        }

        let start_bin = (bin_lo_f.ceil() as usize).clamp(1, half_bins);
        let end_bin = (bin_hi_f.floor() as usize).min(half_bins);

        let band_mag = if start_bin <= end_bin {
            let mut peak = 0.0_f32;
            for &m in &mags[start_bin..=end_bin] {
                if m > peak {
                    peak = m;
                }
            }
            peak
        } else {
            // Band narrower than one FFT bin at low frequencies: sample
            // linearly at the geometric band centre.
            let f_mid = (f_lo * f_hi).sqrt();
            let pos = (f_mid / bin_hz).clamp(1.0, half_bins as f32);
            let idx0 = pos.floor() as usize;
            let idx1 = (idx0 + 1).min(half_bins);
            let frac = pos - idx0 as f32;
            mags[idx0] * (1.0 - frac) + mags[idx1] * frac
        };

        if band_mag <= SPECTRUM_LINEAR_FLOOR {
            *slot = 0.0;
        } else {
            let db = 20.0 * band_mag.log10();
            *slot = ((db - SPECTRUM_DB_FLOOR) / db_span).clamp(0.0, 1.0);
        }
    }

    bands
}

/// Downsample a 64-band spectrum into 48 bars for the `AudioReactivity`
/// widget using exact area-weighted 4:3 resampling.
///
/// Every group of 4 input bands maps onto 3 output bars with weights
/// summing to `1.0`, preserving mean energy and `[0.0, 1.0]` bounds.
/// Non-finite inputs are sanitised to `0.0`.
#[must_use]
pub fn downsample_64_to_48(bands_64: &[f32; SPECTRUM_BANDS_64]) -> [f32; SPECTRUM_BANDS_48] {
    let mut out = [0.0_f32; SPECTRUM_BANDS_48];
    let clean = |v: f32| -> f32 {
        if v.is_finite() {
            v.clamp(0.0, 1.0)
        } else {
            0.0
        }
    };

    for group in 0..16 {
        let i = group * 4;
        let o = group * 3;
        let b0 = clean(bands_64[i]);
        let b1 = clean(bands_64[i + 1]);
        let b2 = clean(bands_64[i + 2]);
        let b3 = clean(bands_64[i + 3]);

        out[o] = (0.75 * b0 + 0.25 * b1).clamp(0.0, 1.0);
        out[o + 1] = (0.50 * b1 + 0.50 * b2).clamp(0.0, 1.0);
        out[o + 2] = (0.25 * b2 + 0.75 * b3).clamp(0.0, 1.0);
    }

    out
}

/// In-place iterative radix-2 Cooley-Tukey forward FFT of length 1024.
fn fft_radix2_1024(re: &mut [f32; SPECTRUM_FFT_SIZE], im: &mut [f32; SPECTRUM_FFT_SIZE]) {
    let shift = usize::BITS - SPECTRUM_FFT_LOG2;
    for i in 0..SPECTRUM_FFT_SIZE {
        let j = i.reverse_bits() >> shift;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    let mut len = 2_usize;
    while len <= SPECTRUM_FFT_SIZE {
        let half = len / 2;
        let angle_step = -TAU / (len as f32);
        for k in 0..half {
            let angle = angle_step * (k as f32);
            let w_re = angle.cos();
            let w_im = angle.sin();

            let mut start = 0_usize;
            while start < SPECTRUM_FFT_SIZE {
                let u_idx = start + k;
                let v_idx = u_idx + half;

                let u_re = re[u_idx];
                let u_im = im[u_idx];
                let v_re = re[v_idx];
                let v_im = im[v_idx];

                let t_re = w_re * v_re - w_im * v_im;
                let t_im = w_re * v_im + w_im * v_re;

                re[u_idx] = u_re + t_re;
                im[u_idx] = u_im + t_im;
                re[v_idx] = u_re - t_re;
                im[v_idx] = u_im - t_im;

                start += len;
            }
        }
        len *= 2;
    }
}

/// Output snapshot produced by [`FusedBpmEstimator::push_onset_energy`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BpmEstimate {
    /// Smoothed tempo estimate clamped to `[60.0, 180.0]` BPM.
    pub bpm: f32,
    /// Tracking confidence in `[0.0, 1.0]` (`< 0.3` on silence or
    /// unperiodic noise; `>= 0.5` on steady rhythmic material).
    pub confidence: f32,
    /// `true` on frames where a new onset peak crosses the adaptive
    /// threshold outside the refractory cooldown.
    pub on_beat: bool,
}

impl Default for BpmEstimate {
    fn default() -> Self {
        Self {
            bpm: FUSED_BPM_DEFAULT,
            confidence: 0.0,
            on_beat: false,
        }
    }
}

/// Two-stage fused inter-onset interval (IOI) + autocorrelation BPM
/// estimator with log-tempo Gaussian octave-jump rejection.
#[derive(Debug, Clone)]
pub struct FusedBpmEstimator {
    /// Rolling 6-second onset-flux ring buffer at 100 Hz.
    flux_ring: VecDeque<f32>,
    /// Recent detected onset timestamps (in seconds).
    onset_times: VecDeque<f64>,
    /// Monotonic elapsed time in seconds.
    elapsed_secs: f64,
    /// Next fixed-grid tick timestamp in seconds.
    next_tick_secs: f64,
    /// Timestamp of the last detected onset peak.
    last_onset_secs: Option<f64>,
    /// Previous caller flux value (for linear resampling when `dt > tick`).
    prev_flux: f32,
    /// Peak flux accumulated within the current sub-tick interval.
    subtick_peak: f32,
    /// Whether any sample has been observed yet.
    has_prev: bool,
    /// Counter for strided autocorrelation updates.
    ticks_since_eval: usize,
    /// Smoothed tempo estimate in `[60.0, 180.0]`.
    smoothed_bpm: f32,
    /// Smoothed confidence in `[0.0, 1.0]`.
    smoothed_confidence: f32,
}

impl Default for FusedBpmEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl FusedBpmEstimator {
    /// Create a fresh estimator anchored at `120.0 BPM` with `0.0` confidence.
    #[must_use]
    pub fn new() -> Self {
        Self {
            flux_ring: VecDeque::with_capacity(BPM_BUFFER_TICKS),
            onset_times: VecDeque::with_capacity(BPM_MAX_ONSET_HISTORY),
            elapsed_secs: 0.0,
            next_tick_secs: BPM_TICK_DT,
            last_onset_secs: None,
            prev_flux: 0.0,
            subtick_peak: 0.0,
            has_prev: false,
            ticks_since_eval: 0,
            smoothed_bpm: FUSED_BPM_DEFAULT,
            smoothed_confidence: 0.0,
        }
    }

    /// Clear all rolling history and return to the unlocked default state.
    pub fn reset(&mut self) {
        self.flux_ring.clear();
        self.onset_times.clear();
        self.elapsed_secs = 0.0;
        self.next_tick_secs = BPM_TICK_DT;
        self.last_onset_secs = None;
        self.prev_flux = 0.0;
        self.subtick_peak = 0.0;
        self.has_prev = false;
        self.ticks_since_eval = 0;
        self.smoothed_bpm = FUSED_BPM_DEFAULT;
        self.smoothed_confidence = 0.0;
    }

    /// Return the current smoothed BPM estimate in `[60.0, 180.0]`.
    #[must_use]
    pub fn bpm(&self) -> f32 {
        self.smoothed_bpm
    }

    /// Return the current tracking confidence in `[0.0, 1.0]`.
    #[must_use]
    pub fn confidence(&self) -> f32 {
        self.smoothed_confidence
    }

    /// Push one onset-flux observation separated by `dt_secs` seconds
    /// from the previous call and return the updated [`BpmEstimate`].
    pub fn push_onset_energy(&mut self, onset_flux: f32, dt_secs: f32) -> BpmEstimate {
        let flux = if onset_flux.is_finite() && onset_flux > 0.0 {
            onset_flux
        } else {
            0.0
        };
        let dt = if dt_secs.is_finite() && dt_secs > 0.0 {
            f64::from(dt_secs).min(0.5)
        } else {
            0.0
        };

        if dt <= 0.0 {
            return BpmEstimate {
                bpm: self.smoothed_bpm,
                confidence: self.smoothed_confidence,
                on_beat: false,
            };
        }

        let t_prev = self.elapsed_secs;
        let t_curr = t_prev + dt;
        self.elapsed_secs = t_curr;

        let prev_val = if self.has_prev { self.prev_flux } else { flux };
        self.has_prev = true;

        // Resample onto the fixed 100 Hz tick grid without creating
        // artificial ripple on flat signals or dropping narrow pulses.
        let mut emitted_ticks = 0_usize;
        if dt <= BPM_TICK_DT {
            if flux > self.subtick_peak {
                self.subtick_peak = flux;
            }
            while self.next_tick_secs <= t_curr + 1.0e-9 {
                let sample = self.subtick_peak;
                self.push_tick(sample);
                self.subtick_peak = 0.0;
                self.next_tick_secs += BPM_TICK_DT;
                emitted_ticks += 1;
            }
        } else {
            while self.next_tick_secs <= t_curr + 1.0e-9 {
                let alpha = ((self.next_tick_secs - t_prev) / dt).clamp(0.0, 1.0) as f32;
                let interp = prev_val + alpha * (flux - prev_val);
                let sample = interp.max(self.subtick_peak);
                self.push_tick(sample);
                self.subtick_peak = 0.0;
                self.next_tick_secs += BPM_TICK_DT;
                emitted_ticks += 1;
            }
        }

        self.prev_flux = flux;

        // Stage 1: adaptive onset-peak detection for `on_beat` and IOI.
        let on_beat = self.detect_onset_peak(flux, t_curr);

        // Stage 2: strided autocorrelation + IOI fusion.
        self.ticks_since_eval += emitted_ticks;
        if on_beat || self.ticks_since_eval >= BPM_EVAL_STRIDE_TICKS {
            self.ticks_since_eval = 0;
            self.update_estimate();
        }

        BpmEstimate {
            bpm: self.smoothed_bpm,
            confidence: self.smoothed_confidence,
            on_beat,
        }
    }

    fn push_tick(&mut self, value: f32) {
        if self.flux_ring.len() == BPM_BUFFER_TICKS {
            let _ = self.flux_ring.pop_front();
        }
        self.flux_ring.push_back(value);
    }

    fn detect_onset_peak(&mut self, flux: f32, t_curr: f64) -> bool {
        if flux <= 1.0e-3 {
            return false;
        }

        if let Some(last_t) = self.last_onset_secs {
            if t_curr - last_t < BPM_ONSET_REFRACTORY_SECS {
                return false;
            }
        }

        let n = self.flux_ring.len().min(BPM_ONSET_WINDOW_TICKS);
        if n < 8 {
            return false;
        }

        let start = self.flux_ring.len() - n;
        let mut sum = 0.0_f64;
        for i in start..self.flux_ring.len() {
            sum += f64::from(self.flux_ring[i]);
        }
        let mean = sum / (n as f64);

        let mut var_sum = 0.0_f64;
        for i in start..self.flux_ring.len() {
            let d = f64::from(self.flux_ring[i]) - mean;
            var_sum += d * d;
        }
        let std_dev = (var_sum / (n as f64)).sqrt();

        // Flat / constant energy has near-zero variance and must not fire beats.
        if std_dev <= 1.0e-4 {
            return false;
        }

        let threshold = (mean + 1.5 * std_dev) as f32;
        if flux > threshold {
            self.last_onset_secs = Some(t_curr);
            if self.onset_times.len() == BPM_MAX_ONSET_HISTORY {
                let _ = self.onset_times.pop_front();
            }
            self.onset_times.push_back(t_curr);
            // Evict stale onsets older than the 6-second analysis window.
            let cutoff = t_curr - (BPM_BUFFER_TICKS as f64) * BPM_TICK_DT;
            while let Some(&front) = self.onset_times.front() {
                if front < cutoff {
                    let _ = self.onset_times.pop_front();
                } else {
                    break;
                }
            }
            return true;
        }

        false
    }

    fn update_estimate(&mut self) {
        let n = self.flux_ring.len();
        if n < BPM_MIN_ANALYSIS_TICKS {
            self.smoothed_confidence *= 0.85;
            return;
        }

        // Copy active ring into a fixed stack buffer and remove DC mean.
        let mut centered = [0.0_f32; BPM_BUFFER_TICKS];
        let mut sum = 0.0_f64;
        for &v in &self.flux_ring {
            sum += f64::from(v);
        }
        let mean = (sum / (n as f64)) as f32;

        let mut energy = 0.0_f64;
        for (i, &v) in self.flux_ring.iter().enumerate() {
            let c = v - mean;
            centered[i] = c;
            energy += f64::from(c) * f64::from(c);
        }

        if energy <= 1.0e-8 {
            self.smoothed_confidence *= 0.6;
            if self.smoothed_confidence < 1.0e-3 {
                self.smoothed_confidence = 0.0;
            }
            return;
        }

        let max_lag = BPM_LAG_MAX.min(n / 2);
        if max_lag <= BPM_LAG_MIN {
            return;
        }

        let mut raw_acf = [0.0_f32; BPM_LAG_MAX + 1];
        let mut acf_sum = 0.0_f32;
        let mut acf_count = 0_usize;

        for lag in BPM_LAG_MIN..=max_lag {
            let overlap = n - lag;
            let mut dot = 0.0_f64;
            let mut e_a = 0.0_f64;
            let mut e_b = 0.0_f64;
            for i in 0..overlap {
                let a = f64::from(centered[i]);
                let b = f64::from(centered[i + lag]);
                dot += a * b;
                e_a += a * a;
                e_b += b * b;
            }
            let denom = (e_a * e_b).sqrt();
            let corr = if denom > 1.0e-12 {
                (dot / denom).max(0.0) as f32
            } else {
                0.0
            };
            raw_acf[lag] = corr;
            acf_sum += corr;
            acf_count += 1;
        }

        // Score each lag with harmonic reinforcement and a Gaussian
        // log-tempo prior centered at 120 BPM to reject octave jumps.
        let mut scores = [0.0_f32; BPM_LAG_MAX + 1];
        let mut best_lag = BPM_LAG_MIN;
        let mut best_score = -1.0_f32;

        for lag in BPM_LAG_MIN..=max_lag {
            let bpm = (60.0 * BPM_TICK_HZ) / (lag as f32);
            let prior = tempo_gaussian_prior(bpm);
            let harmonic = if lag * 2 <= max_lag {
                raw_acf[lag * 2]
            } else {
                raw_acf[lag]
            };
            let comb = 0.85 * raw_acf[lag] + 0.15 * harmonic;
            let score = comb * prior;
            scores[lag] = score;
            if score > best_score {
                best_score = score;
                best_lag = lag;
            }
        }

        // Parabolic peak interpolation around `best_lag` for sub-tick BPM precision.
        let refined_lag = if best_lag > BPM_LAG_MIN && best_lag < max_lag {
            let y0 = scores[best_lag - 1];
            let y1 = scores[best_lag];
            let y2 = scores[best_lag + 1];
            let denom = 2.0 * (y0 - 2.0 * y1 + y2);
            if denom.abs() > 1.0e-6 {
                let delta = ((y0 - y2) / denom).clamp(-0.5, 0.5);
                best_lag as f32 + delta
            } else {
                best_lag as f32
            }
        } else {
            best_lag as f32
        };

        let acf_bpm = ((60.0 * BPM_TICK_HZ) / refined_lag).clamp(FUSED_BPM_MIN, FUSED_BPM_MAX);
        let acf_peak = raw_acf[best_lag].clamp(0.0, 1.0);
        let mean_acf = if acf_count > 0 {
            acf_sum / (acf_count as f32)
        } else {
            0.0
        };
        let prominence = ((acf_peak - mean_acf) / (1.0 - mean_acf).max(0.1)).clamp(0.0, 1.0);
        let acf_conf = (acf_peak * prominence).sqrt().clamp(0.0, 1.0);

        // Stage 1 IOI summary + Stage 2 fusion.
        let (fused_bpm, fused_conf) = match self.estimate_ioi_bpm() {
            Some((ioi_bpm, ioi_conf)) => {
                let rel_diff = (ioi_bpm - acf_bpm).abs() / acf_bpm.max(1.0);
                if rel_diff <= 0.10 {
                    // Strong agreement between IOI and autocorrelation.
                    let w_acf = 0.65;
                    let w_ioi = 0.35 * ioi_conf;
                    let bpm = (w_acf * acf_bpm + w_ioi * ioi_bpm) / (w_acf + w_ioi);
                    let conf = (acf_conf * (0.85 + 0.25 * ioi_conf)).clamp(0.0, 1.0);
                    (bpm, conf)
                } else {
                    // Disagreement (e.g. syncopation or noisy peaks): trust
                    // autocorrelation and keep confidence bounded by it.
                    let conf = (acf_conf * 0.85).clamp(0.0, 1.0);
                    (acf_bpm, conf)
                }
            }
            None => (acf_bpm, (acf_conf * 0.90).clamp(0.0, 1.0)),
        };

        let target_bpm = fused_bpm.clamp(FUSED_BPM_MIN, FUSED_BPM_MAX);
        if fused_conf > 0.25 {
            if self.smoothed_confidence < 0.15 {
                self.smoothed_bpm = target_bpm;
            } else {
                self.smoothed_bpm += 0.25 * (target_bpm - self.smoothed_bpm);
            }
        }
        self.smoothed_bpm = self.smoothed_bpm.clamp(FUSED_BPM_MIN, FUSED_BPM_MAX);

        let conf_alpha = if fused_conf >= self.smoothed_confidence {
            0.25
        } else {
            0.20
        };
        self.smoothed_confidence = (self.smoothed_confidence
            + conf_alpha * (fused_conf - self.smoothed_confidence))
            .clamp(0.0, 1.0);
    }

    fn estimate_ioi_bpm(&self) -> Option<(f32, f32)> {
        if self.onset_times.len() < 4 {
            return None;
        }

        let mut bpms = Vec::with_capacity(self.onset_times.len() - 1);
        for pair in self.onset_times.iter().zip(self.onset_times.iter().skip(1)) {
            let ioi = *pair.1 - *pair.0;
            if ioi <= 0.05 || ioi > 2.5 {
                continue;
            }
            let mut bpm = (60.0 / ioi) as f32;
            while bpm < FUSED_BPM_MIN {
                bpm *= 2.0;
            }
            while bpm > FUSED_BPM_MAX {
                bpm *= 0.5;
            }
            bpms.push(bpm);
        }

        if bpms.len() < 3 {
            return None;
        }

        bpms.sort_by(f32::total_cmp);
        let mid = bpms.len() / 2;
        let median_bpm = if bpms.len() % 2 == 1 {
            bpms[mid]
        } else {
            0.5 * (bpms[mid - 1] + bpms[mid])
        };

        // Measure IOI regularity via mean absolute relative deviation.
        let mut dev_sum = 0.0_f32;
        for &b in &bpms {
            dev_sum += (b - median_bpm).abs() / median_bpm.max(1.0);
        }
        let mean_rel_dev = dev_sum / (bpms.len() as f32);
        let consistency = (1.0 - mean_rel_dev * 4.0).clamp(0.0, 1.0);

        Some((median_bpm, consistency))
    }
}

/// Log-tempo Gaussian prior centered at 120 BPM (`sigma = 0.55` octaves).
fn tempo_gaussian_prior(bpm: f32) -> f32 {
    if !bpm.is_finite() || bpm <= 0.0 {
        return 0.0;
    }
    let octaves = (bpm / FUSED_BPM_DEFAULT).log2();
    let z = octaves / BPM_PRIOR_SIGMA_OCTAVES;
    (-0.5 * z * z).exp()
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod spectrum_pins {
    //! Evidence: EV-AUDIO-TRACE (T1) — drift pins for ADR-023 Waves 5 & 6
    //! (64-band log-spaced FFT spectrum, 64->48 downsampler, and two-stage
    //! fused IOI + autocorrelation BPM estimator).

    use super::*;

    fn expected_log_band(freq_hz: f32) -> usize {
        let ratio = SPECTRUM_MAX_HZ / SPECTRUM_MIN_HZ;
        let frac = (freq_hz / SPECTRUM_MIN_HZ).ln() / ratio.ln();
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
    fn silence_and_invalid_inputs_produce_all_zero_spectrum() {
        assert_eq!(compute_log_spectrum_64(&[], 48_000.0), [0.0; 64]);
        assert_eq!(compute_log_spectrum_64(&[0.0; 1024], 48_000.0), [0.0; 64]);
        assert_eq!(compute_log_spectrum_64(&[0.0; 128], 48_000.0), [0.0; 64]);
        assert_eq!(compute_log_spectrum_64(&[0.5; 1024], 0.0), [0.0; 64]);
        assert_eq!(compute_log_spectrum_64(&[0.5; 1024], f32::NAN), [0.0; 64]);
    }

    #[test]
    fn pure_440hz_and_4000hz_sines_peak_in_expected_log_bins() {
        let sr = 48_000.0_f32;
        let sine_440: Vec<f32> = (0..1024)
            .map(|i| (TAU * 440.0 * i as f32 / sr).sin())
            .collect();
        let sine_4000: Vec<f32> = (0..1024)
            .map(|i| (TAU * 4000.0 * i as f32 / sr).sin())
            .collect();

        let spec_440 = compute_log_spectrum_64(&sine_440, sr);
        let spec_4000 = compute_log_spectrum_64(&sine_4000, sr);

        let (bin_440, peak_440) = argmax(&spec_440);
        let (bin_4000, peak_4000) = argmax(&spec_4000);

        let exp_440 = expected_log_band(440.0);
        let exp_4000 = expected_log_band(4000.0);

        assert!(
            bin_440.abs_diff(exp_440) <= 1,
            "440 Hz expected near band {exp_440}, got band {bin_440} (peak {peak_440})"
        );
        assert!(
            bin_4000.abs_diff(exp_4000) <= 1,
            "4000 Hz expected near band {exp_4000}, got band {bin_4000} (peak {peak_4000})"
        );
        assert!(
            bin_4000 > bin_440,
            "4000 Hz bin ({bin_4000}) must be above 440 Hz bin ({bin_440})"
        );
        assert!(
            peak_440 > 0.5,
            "440 Hz peak must exceed 0.5, got {peak_440}"
        );
        assert!(
            peak_4000 > 0.5,
            "4000 Hz peak must exceed 0.5, got {peak_4000}"
        );
        for &v in spec_440.iter().chain(spec_4000.iter()) {
            assert!(
                (0.0..=1.0).contains(&v),
                "spectrum value out of [0, 1]: {v}"
            );
        }
    }

    #[test]
    fn short_buffer_zero_pads_and_still_locates_tone() {
        let sr = 48_000.0_f32;
        let short_tone: Vec<f32> = (0..512)
            .map(|i| (TAU * 1000.0 * i as f32 / sr).sin())
            .collect();
        let spec = compute_log_spectrum_64(&short_tone, sr);
        let (bin, peak) = argmax(&spec);
        let exp = expected_log_band(1000.0);
        assert!(
            bin.abs_diff(exp) <= 1,
            "1000 Hz short buffer expected near band {exp}, got {bin}"
        );
        assert!(peak > 0.5, "expected peak > 0.5, got {peak}");
    }

    #[test]
    fn downsample_64_to_48_preserves_bounds_and_mean_energy() {
        let mut ramp = [0.0_f32; 64];
        for (i, slot) in ramp.iter_mut().enumerate() {
            *slot = i as f32 / 63.0;
        }
        let down = downsample_64_to_48(&ramp);
        for &v in &down {
            assert!(
                (0.0..=1.0).contains(&v),
                "downsampled value out of [0, 1]: {v}"
            );
        }

        let mean_64: f32 = ramp.iter().sum::<f32>() / 64.0;
        let mean_48: f32 = down.iter().sum::<f32>() / 48.0;
        assert!(
            (mean_64 - mean_48).abs() < 1e-5,
            "area-weighted downsampler must preserve mean energy ({mean_64} vs {mean_48})"
        );

        // Extremes and non-finite inputs clamp cleanly to [0, 1].
        assert_eq!(downsample_64_to_48(&[0.0; 64]), [0.0; 48]);
        assert_eq!(downsample_64_to_48(&[1.0; 64]), [1.0; 48]);
        let mut dirty = [0.5_f32; 64];
        dirty[0] = f32::NAN;
        dirty[1] = f32::INFINITY;
        dirty[2] = -5.0;
        dirty[3] = 2.5;
        let cleaned = downsample_64_to_48(&dirty);
        for &v in &cleaned {
            assert!(v.is_finite() && (0.0..=1.0).contains(&v));
        }
    }

    fn run_pulse_train(target_bpm: f32, fps: f32, duration_secs: f32) -> (BpmEstimate, usize) {
        let mut est = FusedBpmEstimator::new();
        let dt = 1.0 / fps;
        let period = 60.0 / target_bpm;
        let total_frames = (duration_secs * fps).round() as usize;
        let mut next_pulse_t = 0.25_f32;
        let mut last = BpmEstimate::default();
        let mut beat_count = 0_usize;

        for frame in 0..total_frames {
            let t = frame as f32 * dt;
            let flux = if t >= next_pulse_t {
                next_pulse_t += period;
                1.0
            } else {
                0.0
            };
            last = est.push_onset_energy(flux, dt);
            if last.on_beat {
                beat_count += 1;
            }
        }
        (last, beat_count)
    }

    #[test]
    fn fused_bpm_converges_on_120_and_140_bpm_pulse_trains() {
        let (est_120, beats_120) = run_pulse_train(120.0, 60.0, 8.0);
        assert!(
            (est_120.bpm - 120.0).abs() <= 2.5,
            "120 BPM pulse train expected within ±2.5 BPM, got {:.2} (conf {:.2})",
            est_120.bpm,
            est_120.confidence
        );
        assert!(
            est_120.confidence >= 0.5,
            "120 BPM confidence expected >= 0.5, got {:.2}",
            est_120.confidence
        );
        assert!(
            beats_120 >= 10,
            "expected >= 10 detected beats at 120 BPM, got {beats_120}"
        );

        let (est_140, beats_140) = run_pulse_train(140.0, 60.0, 8.0);
        assert!(
            (est_140.bpm - 140.0).abs() <= 2.5,
            "140 BPM pulse train expected within ±2.5 BPM, got {:.2} (conf {:.2})",
            est_140.bpm,
            est_140.confidence
        );
        assert!(
            est_140.confidence >= 0.5,
            "140 BPM confidence expected >= 0.5, got {:.2}",
            est_140.confidence
        );
        assert!(
            beats_140 >= 12,
            "expected >= 12 detected beats at 140 BPM, got {beats_140}"
        );
    }

    #[test]
    fn fused_bpm_yields_low_confidence_on_silence_and_flat_noise() {
        // Silence.
        let mut silent = FusedBpmEstimator::new();
        let mut last_silent = BpmEstimate::default();
        for _ in 0..480 {
            last_silent = silent.push_onset_energy(0.0, 1.0 / 60.0);
            assert!(!last_silent.on_beat, "silence must not trigger on_beat");
        }
        assert!(
            last_silent.confidence < 0.3,
            "silence confidence expected < 0.3, got {:.3}",
            last_silent.confidence
        );

        // Constant non-zero flat floor.
        let mut flat = FusedBpmEstimator::new();
        let mut last_flat = BpmEstimate::default();
        for _ in 0..480 {
            last_flat = flat.push_onset_energy(0.35, 1.0 / 60.0);
            assert!(!last_flat.on_beat, "flat energy must not trigger on_beat");
        }
        assert!(
            last_flat.confidence < 0.3,
            "flat energy confidence expected < 0.3, got {:.3}",
            last_flat.confidence
        );

        // Deterministic white noise (LCG).
        let mut noisy = FusedBpmEstimator::new();
        let mut state: u32 = 0x1234_5678;
        let mut last_noisy = BpmEstimate::default();
        for _ in 0..480 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let sample = ((state >> 8) as f32) / ((1_u32 << 24) as f32);
            last_noisy = noisy.push_onset_energy(sample, 1.0 / 60.0);
        }
        assert!(
            last_noisy.confidence < 0.3,
            "white noise confidence expected < 0.3, got {:.3}",
            last_noisy.confidence
        );
    }
}

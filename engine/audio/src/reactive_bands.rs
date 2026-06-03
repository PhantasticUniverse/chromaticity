// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! MilkDrop-class per-band auto-gain (AGC) reactive analysis.
//!
//! This is the **quality** layer the slice-1 raw FFT (`uniforms.rs`) lacks: it
//! turns instantaneous per-band energy into **relative, volume-independent**
//! values centred on `~1.0`, exactly as MilkDrop / projectM / butterchurn do.
//! `1.0` ≈ "normal for this material", `< 0.7` quiet, `> 1.3` loud — so a scene
//! reacts the same way to a track at -6 dBFS or -30 dBFS. That auto-gain is the
//! single load-bearing idea behind MilkDrop's reactivity, verified verbatim
//! across the three open reference codebases (see `docs/adr/`).
//!
//! ## Algorithm (per observed block)
//!
//! 1. Hann-window the most-recent [`REACTIVE_FFT_SIZE`] mono samples; real FFT
//!    to magnitudes.
//! 2. Sum magnitudes into three perceptual bands (bass / mid / treb) by Hz
//!    boundaries (butterchurn's `20-320 / 320-2800 / 2800-11025` Hz split —
//!    perceptual, vs MilkDrop's equal-sixths bin split which targets its preset
//!    corpus).
//! 3. Maintain two per-band EMAs, both FPS-compensated to a 30 Hz base:
//!    - a **short** average with a fast attack / slower release
//!      ([`SHORT_ATTACK`] when rising, [`SHORT_RELEASE`] when falling), and
//!    - a **long** average ([`LONG_STARTUP`] for the first
//!      [`LONG_STARTUP_FRAMES`] blocks to converge quickly, then [`LONG_STEADY`])
//!      — this is the slow AGC reference.
//! 4. Emit `band = instantaneous / long_avg` and `band_att = short_avg /
//!    long_avg` (guarding a near-silent `long_avg` to `1.0`). Both centre on
//!    `~1.0`; `*_att` is the smoothed (damped-in-time) variant.
//!
//! All smoothing constants are weights on the OLD value (so `0.2` rising = 80 %
//! new = fast attack). Determinism: this is a pure function of the sample
//! stream + elapsed time; it does not feed `canonical_state_hash`.

use std::f32::consts::PI;
use std::sync::Arc;

use rustfft::{Fft, FftPlanner, num_complex::Complex32};

/// Number of reactive bands (bass, mid, treb), MilkDrop-compatible.
pub const REACTIVE_BAND_COUNT: usize = 3;

/// FFT window for the band analysis (matches `uniforms::FFT_WINDOW_SIZE`, ~21 ms
/// at 48 kHz).
pub const REACTIVE_FFT_SIZE: usize = 1024;

/// Short-average EMA weight-on-old when the band is RISING (fast attack: 80 %
/// new). MilkDrop/projectM/butterchurn constant.
const SHORT_ATTACK: f32 = 0.2;
/// Short-average EMA weight-on-old when the band is FALLING (slower release:
/// 50 % new).
const SHORT_RELEASE: f32 = 0.5;
/// Long-average EMA weight-on-old during startup (fast converge, first
/// [`LONG_STARTUP_FRAMES`] blocks).
const LONG_STARTUP: f32 = 0.9;
/// Long-average EMA weight-on-old at steady state (very slow AGC reference,
/// ~tens of seconds).
const LONG_STEADY: f32 = 0.992;
/// Blocks of startup before the long average switches to [`LONG_STEADY`].
const LONG_STARTUP_FRAMES: u64 = 50;
/// Below this, the long average is treated as silence and bands read `1.0`
/// (centred) rather than dividing by ~0.
const SILENCE_FLOOR: f32 = 0.001;
/// Frame rate the smoothing constants are calibrated at; rates are rescaled to
/// the actual block cadence as `rate^(BASE_FPS * dt_seconds)`.
const BASE_FPS: f32 = 30.0;

/// Perceptual band edges in Hz (butterchurn split): bass `20-320`, mid
/// `320-2800`, treb `2800-11025`.
const BAND_EDGES_HZ: [(f32, f32); REACTIVE_BAND_COUNT] =
    [(20.0, 320.0), (320.0, 2800.0), (2800.0, 11025.0)];

/// The six MilkDrop-compatible relative band values, centred on `~1.0`.
///
/// `bass`/`mid`/`treb` are instantaneous-relative; `*_att` are the
/// smoothed (attack/decay) variants. Practical range ~`0..5`, unbounded above
/// (a hit after silence spikes because the long average lags).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReactiveBandValues {
    pub bass: f32,
    pub mid: f32,
    pub treb: f32,
    pub bass_att: f32,
    pub mid_att: f32,
    pub treb_att: f32,
}

impl Default for ReactiveBandValues {
    fn default() -> Self {
        // Silence / pre-roll centres on 1.0 (MilkDrop semantics).
        Self {
            bass: 1.0,
            mid: 1.0,
            treb: 1.0,
            bass_att: 1.0,
            mid_att: 1.0,
            treb_att: 1.0,
        }
    }
}

/// Stateful MilkDrop-class per-band AGC analyser.
///
/// One instance per capture stream; feed the rolling mono buffer via
/// [`ReactiveBands::observe`] each analysis tick.
pub struct ReactiveBands {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    scratch: Vec<Complex32>,
    band_bins: [(usize, usize); REACTIVE_BAND_COUNT],
    imm: [f32; REACTIVE_BAND_COUNT],
    avg: [f32; REACTIVE_BAND_COUNT],
    long_avg: [f32; REACTIVE_BAND_COUNT],
    frame: u64,
}

impl core::fmt::Debug for ReactiveBands {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ReactiveBands")
            .field("frame", &self.frame)
            .field("band_bins", &self.band_bins)
            .field("long_avg", &self.long_avg)
            .finish_non_exhaustive()
    }
}

impl ReactiveBands {
    /// Build an analyser for a given capture sample rate. The Hz band edges are
    /// resolved to FFT bin ranges once, here.
    #[must_use]
    pub fn new(sample_rate_hz: u32) -> Self {
        let size = REACTIVE_FFT_SIZE;
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(size);
        let window = (0..size)
            .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / size as f32).cos())
            .collect();
        let bucket_hz = (sample_rate_hz.max(1) as f32) / size as f32;
        let nyquist_bin = size / 2;
        let band_bins = core::array::from_fn(|b| {
            let (lo_hz, hi_hz) = BAND_EDGES_HZ[b];
            let lo = ((lo_hz / bucket_hz).floor() as usize).min(nyquist_bin);
            let hi = ((hi_hz / bucket_hz).ceil() as usize).clamp(lo + 1, nyquist_bin);
            (lo, hi)
        });
        Self {
            fft,
            window,
            scratch: vec![Complex32::new(0.0, 0.0); size],
            band_bins,
            imm: [0.0; REACTIVE_BAND_COUNT],
            avg: [0.0; REACTIVE_BAND_COUNT],
            long_avg: [0.0; REACTIVE_BAND_COUNT],
            frame: 0,
        }
    }

    /// Observe the most-recent mono samples and advance the AGC state by
    /// `dt_seconds` of wall-clock time (for FPS-independent smoothing).
    ///
    /// Uses the last [`REACTIVE_FFT_SIZE`] samples of `mono` (front-zero-padded
    /// if shorter). Returns the six relative band values centred on `~1.0`.
    pub fn observe(&mut self, mono: &[f32], dt_seconds: f32) -> ReactiveBandValues {
        let size = self.window.len();
        // Window the most-recent `size` samples into the FFT scratch (front-pad).
        let available = mono.len().min(size);
        let pad = size - available;
        let src = &mono[mono.len() - available..];
        for slot in self.scratch.iter_mut().take(pad) {
            *slot = Complex32::new(0.0, 0.0);
        }
        for (i, &sample) in src.iter().enumerate() {
            self.scratch[pad + i] = Complex32::new(sample * self.window[pad + i], 0.0);
        }
        self.fft.process(&mut self.scratch);

        // Sum magnitudes into the three perceptual bands.
        for b in 0..REACTIVE_BAND_COUNT {
            let (lo, hi) = self.band_bins[b];
            let mut sum = 0.0_f32;
            for bin in &self.scratch[lo..hi] {
                sum += bin.norm();
            }
            self.imm[b] = sum;
        }

        // Update the two EMAs (FPS-compensated), then emit the relative ratios.
        let mut out = ReactiveBandValues::default();
        let bands = [
            (&mut out.bass, &mut out.bass_att),
            (&mut out.mid, &mut out.mid_att),
            (&mut out.treb, &mut out.treb_att),
        ];
        for (b, (val, att)) in bands.into_iter().enumerate() {
            let imm = self.imm[b];
            let short_base = if imm > self.avg[b] {
                SHORT_ATTACK
            } else {
                SHORT_RELEASE
            };
            let short_rate = adjust_rate_to_fps(short_base, dt_seconds);
            self.avg[b] = self.avg[b] * short_rate + imm * (1.0 - short_rate);

            let long_base = if self.frame < LONG_STARTUP_FRAMES {
                LONG_STARTUP
            } else {
                LONG_STEADY
            };
            let long_rate = adjust_rate_to_fps(long_base, dt_seconds);
            self.long_avg[b] = self.long_avg[b] * long_rate + imm * (1.0 - long_rate);

            if self.long_avg[b].abs() < SILENCE_FLOOR {
                *val = 1.0;
                *att = 1.0;
            } else {
                *val = imm / self.long_avg[b];
                *att = self.avg[b] / self.long_avg[b];
            }
        }
        self.frame = self.frame.saturating_add(1);
        out
    }
}

/// Rescale a per-frame smoothing weight (calibrated at [`BASE_FPS`]) to the
/// actual block cadence: `rate^(BASE_FPS * dt_seconds)` (projectM/butterchurn
/// equivalence). A larger `dt` (slower cadence) decays more per block.
fn adjust_rate_to_fps(rate: f32, dt_seconds: f32) -> f32 {
    if dt_seconds <= 0.0 {
        return rate;
    }
    rate.powf(BASE_FPS * dt_seconds)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-AUDIO-TRACE (T1) — drift pins on the MilkDrop-class
    //! per-band AGC algorithm invariants (volume independence, band
    //! selectivity, silence centring on 1.0, transient spike behaviour,
    //! and the attack/decay smoothing relation) per
    //! `docs/adr/ADR-024-audio-reactivity-analysis-model.md`.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: each test pins a
    //! documented invariant of the auto-gain model — the
    //! `band = instantaneous / long_avg` ratio centred on ~1.0, the
    //! dual-EMA (fast-attack / slow-release short average + slow long
    //! average) smoothing, and the `SILENCE_FLOOR` guard. Any drift in
    //! the AGC constants or math fires the moment `cargo test` runs; the
    //! broader behavioural quality rubric (volume sweep, frequency
    //! sweep, transient) is scored by the `mini_meter` example. T1
    //! counts toward `EV-EVIDENCE-TAG-COVERAGE`.
    //!
    //! Anchors:
    //! - `docs/26-evidence-artifact-registry.md` row 30 — `EV-AUDIO-TRACE`.
    //! - `docs/adr/ADR-024-audio-reactivity-analysis-model.md` — the AGC model.
    //! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

    use super::*;
    use std::f32::consts::TAU;

    const SR: u32 = 44_100;
    const DT: f32 = REACTIVE_FFT_SIZE as f32 / SR as f32; // one block

    /// Generate `n` mono samples of a sine at `freq_hz` / `amp`, phase-continuous
    /// from `start_sample`.
    fn sine(freq_hz: f32, amp: f32, start_sample: u64, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| {
                let t = (start_sample + i as u64) as f32 / SR as f32;
                amp * (TAU * freq_hz * t).sin()
            })
            .collect()
    }

    /// Drive the analyser with a steady sine for many blocks; return the final
    /// converged band values.
    fn converge(freq_hz: f32, amp: f32, blocks: usize) -> ReactiveBandValues {
        let mut rb = ReactiveBands::new(SR);
        let mut last = ReactiveBandValues::default();
        for blk in 0..blocks {
            let buf = sine(
                freq_hz,
                amp,
                blk as u64 * REACTIVE_FFT_SIZE as u64,
                REACTIVE_FFT_SIZE,
            );
            last = rb.observe(&buf, DT);
        }
        last
    }

    #[test]
    fn steady_sine_bands_centre_on_one() {
        // A steady tone, once the long average converges, must read ~1.0 in its
        // band (the AGC normalises away absolute level).
        let v = converge(2000.0, 0.5, 400); // 2 kHz -> mid band
        assert!(
            (v.mid - 1.0).abs() < 0.15,
            "mid should centre ~1.0, got {}",
            v.mid
        );
    }

    #[test]
    fn volume_independent() {
        // THE killer property: the same tone at very different amplitudes must
        // converge to the same relative band value.
        let quiet = converge(2000.0, 0.02, 400).mid;
        let loud = converge(2000.0, 0.8, 400).mid;
        assert!(
            (quiet - loud).abs() < 0.1,
            "AGC must be volume-independent: quiet={quiet} loud={loud}"
        );
        assert!((quiet - 1.0).abs() < 0.2 && (loud - 1.0).abs() < 0.2);
    }

    #[test]
    fn band_selectivity() {
        // The AGC self-normalises every active band to ~1.0 at steady state, so
        // selectivity is visible in the TRANSIENT phase: a band receiving energy
        // spikes above its lagging long-average, while a band that never sees
        // energy stays pinned at the silence guard (1.0). A fresh analyser + a
        // few blocks shows it — this verifies the FFT band split routes 80 Hz to
        // bass and 8 kHz to treb.
        let bass_tone = converge(80.0, 0.5, 6);
        assert!(
            bass_tone.bass > bass_tone.treb + 0.3,
            "80 Hz: bass {} should spike above guarded treb {}",
            bass_tone.bass,
            bass_tone.treb
        );
        let treb_tone = converge(8000.0, 0.5, 6);
        assert!(
            treb_tone.treb > treb_tone.bass + 0.3,
            "8 kHz: treb {} should spike above guarded bass {}",
            treb_tone.treb,
            treb_tone.bass
        );
    }

    #[test]
    fn silence_centres_on_one() {
        let mut rb = ReactiveBands::new(SR);
        let mut v = ReactiveBandValues::default();
        for _ in 0..100 {
            v = rb.observe(&vec![0.0; REACTIVE_FFT_SIZE], DT);
        }
        assert_eq!(v.bass, 1.0);
        assert_eq!(v.mid, 1.0);
        assert_eq!(v.treb, 1.0);
    }

    #[test]
    fn transient_spikes_above_baseline() {
        // Converge on a quiet baseline, then a sudden loud burst in-band must
        // spike well above 1.0 because the long average lags.
        let mut rb = ReactiveBands::new(SR);
        for blk in 0..200 {
            let buf = sine(
                2000.0,
                0.05,
                blk as u64 * REACTIVE_FFT_SIZE as u64,
                REACTIVE_FFT_SIZE,
            );
            rb.observe(&buf, DT);
        }
        let burst = sine(
            2000.0,
            0.9,
            200 * REACTIVE_FFT_SIZE as u64,
            REACTIVE_FFT_SIZE,
        );
        let v = rb.observe(&burst, DT);
        assert!(v.mid > 1.5, "burst should spike mid >1.5, got {}", v.mid);
    }

    #[test]
    fn att_is_smoother_than_instantaneous() {
        // After a transient, the attack/decay variant should not exceed the
        // instantaneous (it is a smoothed view), and stays finite.
        let mut rb = ReactiveBands::new(SR);
        for blk in 0..200 {
            let buf = sine(
                2000.0,
                0.05,
                blk as u64 * REACTIVE_FFT_SIZE as u64,
                REACTIVE_FFT_SIZE,
            );
            rb.observe(&buf, DT);
        }
        let burst = sine(
            2000.0,
            0.9,
            200 * REACTIVE_FFT_SIZE as u64,
            REACTIVE_FFT_SIZE,
        );
        let v = rb.observe(&burst, DT);
        assert!(
            v.mid_att <= v.mid + 1e-3,
            "att {} should lag imm {}",
            v.mid_att,
            v.mid
        );
        assert!(v.mid_att.is_finite() && v.mid_att > 0.0);
    }
}

// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-AUDIO-TRACE (T2) — Phase-2 slice-1 audio uniform
//! extraction corpus.
//!
//! Advances `docs/26-evidence-artifact-registry.md` row 30
//! (`EV-AUDIO-TRACE`) past the slice-1 producer-side T1 drift pins
//! by exercising the four audio analysis primitives (BPM clamped
//! `[50, 220]` mapped to `chroma_BPM`; raw
//! RMS energy mapped to `chroma_AudioRMS`; peak
//! magnitude mapped to `chroma_AudioPeak`; 8-band log-spaced FFT
//! magnitudes mapped to `chroma_AudioFFTLow` / `chroma_AudioFFTHigh`
//! by the engine/render slice-5a adapter) over four synthesised
//! fixtures: silent baseline, 440 Hz tone, deterministic white
//! noise, 1 kHz tone. T2 per `docs/43-development-standards.md`
//! §15.5: corpus conformance over a fixture set with bounded
//! tolerances on every published field.
//!
//! The corpus is single-file (matches the slice-1 producer scope);
//! the slice-2 multi-frame BPM history-window corpus and the
//! adapter-side audio→render corpus land in follow-up slices.
//!
//! Anchors:
//! - `docs/26-evidence-artifact-registry.md` row 30 — `EV-AUDIO-TRACE`.
//! - `docs/36-reactive-symbol-catalog.md` §2.6, §2.7, §2.10.
//! - `docs/06-audio-engine.md` §7.1 — sample-rate / framing baseline.
//! - `engine/audio/src/uniforms.rs` — module-level algorithm prose.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::f32::consts::TAU;

use chromaticity_audio::frame::{AudioFrame, DEFAULT_SAMPLE_RATE_HZ};
use chromaticity_audio::uniforms::{
    AudioUniformSnapshot, BPM_DEFAULT, FFT_BAND_COUNT, FFT_BAND_MAX_HZ, FFT_BAND_MIN_HZ,
    extract_uniform_snapshot,
};

/// One-second corpus length at the canonical sample rate. Matches
/// docs/06 §7.1 default-frame baseline (48 kHz mono); long enough
/// to contain many FFT_WINDOW_SIZE windows and to support
/// autocorrelation across the full `[50, 220]` BPM lag range.
const SAMPLES_ONE_SECOND: usize = 48_000;

fn one_second_silence() -> AudioFrame {
    AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, vec![0.0_f32; SAMPLES_ONE_SECOND])
}

fn one_second_sine(frequency_hz: f32, amplitude: f32) -> AudioFrame {
    let mut samples = Vec::with_capacity(SAMPLES_ONE_SECOND);
    let step = TAU * frequency_hz / DEFAULT_SAMPLE_RATE_HZ as f32;
    for i in 0..SAMPLES_ONE_SECOND {
        samples.push(amplitude * (step * i as f32).sin());
    }
    AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, samples)
}

/// Deterministic linear-congruential RNG matching glibc's `rand`
/// constants. Seedable; repeated calls produce a fixed sequence so
/// the white-noise fixture's hash is reproducible across runs.
struct LcgRng {
    state: u64,
}

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next f32 in `[-1.0, 1.0]`.
    fn next_f32(&mut self) -> f32 {
        // Glibc's LCG: x_{n+1} = (1103515245 * x_n + 12345) mod 2^31.
        self.state = self.state.wrapping_mul(1_103_515_245).wrapping_add(12_345) & 0x7fff_ffff;
        // Map [0, 2^31) -> [-1.0, 1.0).
        (self.state as f32 / (1u64 << 30) as f32) - 1.0
    }
}

fn one_second_white_noise(seed: u64) -> AudioFrame {
    let mut rng = LcgRng::new(seed);
    let mut samples = Vec::with_capacity(SAMPLES_ONE_SECOND);
    for _ in 0..SAMPLES_ONE_SECOND {
        // Scale to amplitude 0.5 so the time-domain peak stays
        // comfortably inside [-1, 1] and FFT integration doesn't
        // saturate any single band.
        samples.push(rng.next_f32() * 0.5);
    }
    AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, samples)
}

/// Map a target frequency to the slice-1 8-band log-spaced index
/// covering [`FFT_BAND_MIN_HZ`, `FFT_BAND_MAX_HZ`].
fn band_index_for_hz(frequency_hz: f32) -> usize {
    let log_lo = FFT_BAND_MIN_HZ.ln();
    let log_hi = FFT_BAND_MAX_HZ.ln();
    let step = (log_hi - log_lo) / FFT_BAND_COUNT as f32;
    let normalised = (frequency_hz.ln() - log_lo) / step;
    let idx = normalised.floor() as isize;
    idx.clamp(0, (FFT_BAND_COUNT - 1) as isize) as usize
}

#[test]
fn silent_frame_zero_energy_zero_bands_default_bpm() {
    let frame = one_second_silence();
    let snap = extract_uniform_snapshot(&frame);
    assert_eq!(snap.rms, 0.0, "silent RMS must be exactly 0");
    assert_eq!(snap.peak, 0.0, "silent peak must be exactly 0");
    for (i, band) in snap.fft_bands.iter().enumerate() {
        assert_eq!(*band, 0.0, "band {i} must be 0 on silent input, got {band}");
    }
    // Silent buffer has zero autocorrelation energy; the extractor
    // returns the slice-1 fallback BPM.
    assert_eq!(
        snap.bpm, BPM_DEFAULT,
        "silent BPM must equal slice-1 fallback {BPM_DEFAULT}",
    );
}

#[test]
fn unit_sine_440_hz_drives_expected_band_to_dominance() {
    // Unit sine -> RMS = 1/sqrt(2) ≈ 0.7071, peak = 1.0.
    let frame = one_second_sine(440.0, 1.0);
    let snap = extract_uniform_snapshot(&frame);
    let expected_rms = 1.0_f32 / 2.0_f32.sqrt();
    assert!(
        (snap.rms - expected_rms).abs() < 1e-2,
        "440 Hz unit-sine RMS expected ~{expected_rms}, got {}",
        snap.rms,
    );
    assert!(
        (snap.peak - 1.0).abs() < 1e-3,
        "440 Hz unit-sine peak expected ~1.0, got {}",
        snap.peak,
    );

    let target_band = band_index_for_hz(440.0);
    // The target band must be the strict maximum: any other band
    // dominating means the FFT integration broke (band edges, bin
    // mapping, or windowing).
    let target_value = snap.fft_bands[target_band];
    for (i, band) in snap.fft_bands.iter().enumerate() {
        if i == target_band {
            continue;
        }
        assert!(
            *band < target_value,
            "band {i} ({band}) must be below 440 Hz band {target_band} ({target_value})",
        );
    }
    // The target band itself should hold meaningful energy. A unit
    // Hann-windowed sine that lands near a bin boundary reads
    // close to 1.0 after the reference-magnitude divide; allow a
    // wide tolerance for the sub-bin offset (440 Hz at sr=48000,
    // window=1024 sits between bins 9 and 10).
    assert!(
        target_value > 0.2,
        "440 Hz target band {target_band} value {target_value} must be > 0.2",
    );
}

#[test]
fn pure_one_kilohertz_tone_is_dominated_by_one_band() {
    let frame = one_second_sine(1_000.0, 0.8);
    let snap = extract_uniform_snapshot(&frame);

    let target_band = band_index_for_hz(1_000.0);
    let target_value = snap.fft_bands[target_band];
    assert!(
        target_value > 0.05,
        "1 kHz target band {target_band} value {target_value} must be > 0.05",
    );

    // Strict dominance: the target band must be at least 4x any
    // other band. For a clean sine at 0.8 amplitude this is well
    // satisfied (Hann sidelobes on a pure tone fall off fast).
    for (i, band) in snap.fft_bands.iter().enumerate() {
        if i == target_band {
            continue;
        }
        assert!(
            target_value >= 4.0 * *band,
            "1 kHz dominance broken: band {i} ({band}) >= target {target_band} ({target_value}) / 4",
        );
    }
}

#[test]
fn white_noise_distributes_energy_across_all_bands() {
    let frame = one_second_white_noise(0xc0ff_ee01);
    let snap = extract_uniform_snapshot(&frame);

    // White-noise RMS at amplitude 0.5 across [-0.5, 0.5] uniform:
    // E[s^2] = (0.5 - (-0.5))^2 / 12 = 1/12 ≈ 0.0833.
    // RMS = sqrt(1/12) ≈ 0.2887. Wider tolerance because LCG-
    // generated samples are quasi-uniform and the realised RMS
    // varies a few percent across seeds.
    assert!(
        (0.2..=0.4).contains(&snap.rms),
        "white-noise RMS expected in [0.2, 0.4], got {}",
        snap.rms,
    );
    // Peak should saturate close to 0.5 (the amplitude scale)
    // over 48000 samples.
    assert!(
        snap.peak > 0.45 && snap.peak <= 1.0,
        "white-noise peak expected near 0.5, got {}",
        snap.peak,
    );

    // No band should be empty; all eight should hold non-trivial
    // energy. The mean of the eight bands sets the reference; the
    // slice-1 corpus tolerates a 10x ratio between max and min
    // because log-spaced bins integrate increasingly more linear-
    // bin counts at higher frequencies (the highest band covers a
    // ~6.2 kHz wide range, the lowest band ~21 Hz). The strict
    // pin is just "every band lit at non-trivial energy".
    let nonzero_bands = snap.fft_bands.iter().filter(|b| **b > 1e-3).count();
    assert_eq!(
        nonzero_bands, FFT_BAND_COUNT,
        "white noise must light every band; lit count = {nonzero_bands}",
    );
}

#[test]
fn snapshot_default_matches_silent_for_empty_buffer() {
    let frame = AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, Vec::new());
    let snap = extract_uniform_snapshot(&frame);
    assert_eq!(
        snap,
        AudioUniformSnapshot::default(),
        "empty-buffer extract must equal default snapshot",
    );
}

/// Slice-2 surface-stability pin.
///
/// The slice-2 [`chromaticity_audio::bpm::BpmEstimator`] addition
/// rides the same `pub(crate)` per-frame autocorrelation helper
/// that [`extract_uniform_snapshot`] uses; this test is a load-
/// bearing regression pin that the slice-1 single-frame entry
/// point produces the same shape it always has on a sine fixture
/// regardless of the slice-2 history-window code paths.
#[test]
fn slice_2_compatibility_extract_uniform_snapshot_unchanged_on_440_hz() {
    let frame = one_second_sine(440.0, 1.0);
    let snap = extract_uniform_snapshot(&frame);

    // RMS, peak, and band shape continuity (same anchors as the
    // slice-1 `unit_sine_440_hz_drives_expected_band_to_dominance`
    // test, repeated here so the slice-2 BpmEstimator addition
    // can't silently drift the slice-1 surface).
    let expected_rms = 1.0_f32 / 2.0_f32.sqrt();
    assert!(
        (snap.rms - expected_rms).abs() < 1e-2,
        "slice-1 RMS regression: expected ~{expected_rms}, got {}",
        snap.rms,
    );
    assert!(
        (snap.peak - 1.0).abs() < 1e-3,
        "slice-1 peak regression: expected ~1.0, got {}",
        snap.peak,
    );
    let target_band = band_index_for_hz(440.0);
    let target_value = snap.fft_bands[target_band];
    for (i, band) in snap.fft_bands.iter().enumerate() {
        if i == target_band {
            continue;
        }
        assert!(
            *band < target_value,
            "slice-1 band-dominance regression: band {i} ({band}) >= target {target_band} ({target_value})",
        );
    }
}

#[test]
fn multichannel_frame_mixes_to_mono_before_analysis() {
    // Stereo frame: L = +0.5 sine at 1 kHz, R = -0.5 same-phase
    // sine. The mono mix-down averages to 0; RMS / peak / FFT all
    // read silence. This pins the mono mix-down step against an
    // accidental switch to per-channel max or first-channel-only.
    let mut samples = Vec::with_capacity(SAMPLES_ONE_SECOND * 2);
    let step = TAU * 1_000.0 / DEFAULT_SAMPLE_RATE_HZ as f32;
    for i in 0..SAMPLES_ONE_SECOND {
        let s = 0.5 * (step * i as f32).sin();
        samples.push(s); // L
        samples.push(-s); // R
    }
    let frame = AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 2, samples);
    let snap = extract_uniform_snapshot(&frame);
    assert!(
        snap.rms < 1e-3,
        "stereo cancellation must mix to silence, RMS = {}",
        snap.rms,
    );
    assert!(
        snap.peak < 1e-3,
        "stereo cancellation must mix to silence, peak = {}",
        snap.peak,
    );
    for (i, band) in snap.fft_bands.iter().enumerate() {
        assert!(
            *band < 1e-3,
            "stereo cancellation must zero band {i}, got {band}",
        );
    }
}

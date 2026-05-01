// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-3 wave PR 2 EV-AUDIO-ROUTING-MATRIX corpus. Tier
//! declaration and evidence tag live on the `fixtures` inner module
//! below, matching the pattern established by
//! `engine/audio/tests/uniforms.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary counts
//! this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-AUDIO-ROUTING-MATRIX (T2) — Phase-3 wave PR 2
    //! audio routing + format conversion conformance corpus, pinned
    //! end-to-end through `extract_uniform_snapshot` on a synthetic
    //! `AudioFrame` matrix spanning (sample-format, channel-count)
    //! tuples.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! the evidence-producing behaviour against a fixture corpus.
    //! Each fixture (a) constructs a synthetic `AudioFrame` of a
    //! specific (sample-rate, channel-count, sample-buffer) shape,
    //! (b) calls `extract_uniform_snapshot` on it, and (c) asserts
    //! the resulting `AudioUniformSnapshot` fields (`rms`, `peak`,
    //! `bpm`, `fft_bands`) against expected values within a tight
    //! tolerance band derived from the documented algorithm
    //! invariants in `docs/06-audio-engine.md` §7.1 (mono-aggregation
    //! `1/sqrt(N)` channel policy, `sqrt(mean(s^2))` RMS,
    //! `max(abs(s))` peak, log-spaced [20 Hz, 20 kHz] FFT bands).
    //!
    //! Format-conversion math (`i16` → `f32`, `u16` → `f32`) is
    //! covered separately by the `cpal_source_pins` T1 module under
    //! EV-AUDIO-TRACE (`engine/audio/src/cpal_source.rs` lines
    //! 558+); the routing-matrix corpus exercises post-conversion
    //! `f32` frames so this corpus pins the routing / aggregation /
    //! extraction layer specifically. Test-side helper functions
    //! `i16_to_f32` and `u16_to_f32` mirror the canonical conversion
    //! formulas (`f32::from(s) / f32::from(i16::MAX)` and
    //! `(f32::from(s) - 32_768.0) / 32_768.0`) so fixtures that
    //! synthesise i16/u16-shaped inputs land at the right
    //! post-conversion `f32` values without depending on
    //! `pub(crate)` cpal_source helpers.
    //!
    //! Backend-invariance. Outputs are pure-Rust algorithmic results
    //! (no GPU involvement) so Linux Vulkan / Mesa llvmpipe (PR-time
    //! CI), macOS Metal (locally and via the `macos-nightly.yml`
    //! workflow merged in PR #300), and any future backend will all
    //! produce identical snapshots within FP rounding tolerance. No
    //! per-backend ACCEPTED-set widening required, matching the
    //! Phase-3 wave PR 1 EV-STATE-REPLAY corpus precedent.
    //!
    //! Cumulative-release-gate scope.
    //! `docs/26-evidence-artifact-registry.md` row 31 declares
    //! `Gate Usage = "Phase 3 + release cumulative gate"`. This
    //! corpus satisfies the Phase-3 baseline; future widening to
    //! cover platform-host-specific routing (CoreAudio device
    //! enumeration matrices, WASAPI loopback validation, ALSA
    //! latency-mode selection) happens under the Phase-4a
    //! `EV-INTEGRATION-MATRIX` row alongside the Tauri-IPC bridge
    //! framework that exposes device-enum UI.

    use chromaticity_audio::frame::AudioFrame;
    use chromaticity_audio::uniforms::{BPM_DEFAULT, FFT_BAND_COUNT, extract_uniform_snapshot};

    /// Default sample rate for fixture frames; matches
    /// `chromaticity_audio::frame::DEFAULT_SAMPLE_RATE_HZ` and
    /// `docs/06-audio-engine.md` §7.1's canonical 48 kHz.
    const SR: u32 = 48_000;

    /// Test-side mirror of `cpal_source::convert_i16_sample` for
    /// fixture construction (the cpal helper is `pub(crate)` and
    /// its conversion math is T1-pinned in `cpal_source_pins`).
    fn i16_to_f32(s: i16) -> f32 {
        f32::from(s) / f32::from(i16::MAX)
    }

    /// Test-side mirror of `cpal_source::convert_u16_sample`.
    fn u16_to_f32(s: u16) -> f32 {
        (f32::from(s) - 32_768.0) / 32_768.0
    }

    // --- Channel-routing fixtures (mono aggregation policy) ----------

    #[test]
    fn f32_mono_unit_amplitude_yields_unit_rms_and_peak() {
        // Fixture 1: f32/1ch baseline. All samples 1.0 -> RMS=1.0,
        // PEAK=1.0. Single-channel passthrough is the simplest
        // routing path.
        let frame = AudioFrame::new(SR, 1, vec![1.0_f32; 512]);
        let snap = extract_uniform_snapshot(&frame);
        assert!(
            (snap.rms - 1.0).abs() < 1e-5,
            "f32/1ch unit: rms expected 1.0, got {}",
            snap.rms,
        );
        assert!(
            (snap.peak - 1.0).abs() < 1e-5,
            "f32/1ch unit: peak expected 1.0, got {}",
            snap.peak,
        );
    }

    #[test]
    fn f32_mono_silence_yields_zero_rms_and_default_bpm() {
        // Fixture 2: silence baseline. All zeros -> RMS=0, PEAK=0;
        // BPM falls back to BPM_DEFAULT because a silent frame's
        // autocorrelation has no meaningful peak.
        let frame = AudioFrame::new(SR, 1, vec![0.0_f32; 1024]);
        let snap = extract_uniform_snapshot(&frame);
        assert_eq!(snap.rms, 0.0, "silent frame must have rms=0.0");
        assert_eq!(snap.peak, 0.0, "silent frame must have peak=0.0");
        assert_eq!(
            snap.bpm, BPM_DEFAULT,
            "silent frame must fall back to BPM_DEFAULT",
        );
        for (i, band) in snap.fft_bands.iter().enumerate() {
            assert_eq!(*band, 0.0, "silent frame fft_bands[{i}] must be 0.0");
        }
    }

    #[test]
    fn f32_stereo_identical_channels_aggregate_to_unit_rms() {
        // Fixture 3: 2ch/identical-channel routing. Per docs/06
        // §7.1, multi-channel inputs sum to mono with `1/N`
        // averaging (the slice-1 mono_mix implementation in
        // uniforms.rs). Two identical channels at amplitude 1.0
        // average to 1.0 (not 1/sqrt(2) — the documented policy is
        // arithmetic-mean averaging, not power-preserving
        // `1/sqrt(N)` scaling).
        let interleaved: Vec<f32> = vec![1.0, 1.0].into_iter().cycle().take(1024).collect();
        let frame = AudioFrame::new(SR, 2, interleaved);
        let snap = extract_uniform_snapshot(&frame);
        assert!(
            (snap.rms - 1.0).abs() < 1e-5,
            "2ch/identical: rms expected 1.0 (mean of two 1.0s), got {}",
            snap.rms,
        );
        assert!(
            (snap.peak - 1.0).abs() < 1e-5,
            "2ch/identical: peak expected 1.0, got {}",
            snap.peak,
        );
    }

    #[test]
    fn f32_stereo_anticorrelated_channels_cancel_to_zero() {
        // Fixture 4: 2ch/anticorrelated (L=+1, R=-1) sums to 0.0
        // under arithmetic-mean mono mixing. Pins the
        // direction-of-mixing convention: `(L+R)/2`, not `(L-R)/2`
        // or `|L|+|R|`.
        let interleaved: Vec<f32> = vec![1.0, -1.0].into_iter().cycle().take(1024).collect();
        let frame = AudioFrame::new(SR, 2, interleaved);
        let snap = extract_uniform_snapshot(&frame);
        assert_eq!(
            snap.rms, 0.0,
            "2ch/anticorrelated: arithmetic-mean mixing zeroes the signal",
        );
        assert_eq!(snap.peak, 0.0, "2ch/anticorrelated: peak post-mix is 0.0",);
    }

    #[test]
    fn f32_quad_identical_channels_aggregate_to_unit_rms() {
        // Fixture 5: 4ch/identical-channel routing. Same `1/N`
        // arithmetic-mean policy — four identical channels at 1.0
        // average to 1.0 (NOT 0.5 — that would be a power-preserving
        // `1/sqrt(N)` policy not used here).
        let interleaved: Vec<f32> = vec![1.0_f32; 4].into_iter().cycle().take(2048).collect();
        let frame = AudioFrame::new(SR, 4, interleaved);
        let snap = extract_uniform_snapshot(&frame);
        assert!(
            (snap.rms - 1.0).abs() < 1e-5,
            "4ch/identical: rms expected 1.0, got {}",
            snap.rms,
        );
    }

    // --- Format-conversion routing fixtures (post-conversion synthesis) -------

    #[test]
    fn i16_at_max_post_conversion_yields_unit_rms() {
        // Fixture 6: i16::MAX -> f32 1.0 (per cpal_source's
        // convert_i16_sample T1 pin). The matrix corpus ingests the
        // post-conversion f32 frame and verifies the snapshot lands
        // at unit RMS / peak. The test-side `i16_to_f32` helper
        // mirrors the canonical formula exactly.
        let i16_samples: Vec<i16> = vec![i16::MAX; 512];
        let f32_post: Vec<f32> = i16_samples.into_iter().map(i16_to_f32).collect();
        let frame = AudioFrame::new(SR, 1, f32_post);
        let snap = extract_uniform_snapshot(&frame);
        assert!(
            (snap.rms - 1.0).abs() < 1e-5,
            "i16::MAX post-conv: rms expected 1.0, got {}",
            snap.rms,
        );
        assert!(
            (snap.peak - 1.0).abs() < 1e-5,
            "i16::MAX post-conv: peak expected 1.0, got {}",
            snap.peak,
        );
    }

    #[test]
    fn i16_at_min_post_conversion_yields_unit_rms_via_clamp() {
        // Fixture 7: i16::MIN converts to f32 just BELOW -1.0 (per
        // the two's-complement asymmetry pinned in
        // `cpal_source_pins::i16_to_f32_conversion_at_min`). The
        // routing-matrix corpus pins that `compute_peak`'s clamp to
        // [0, 1] takes effect here: the raw |peak| value is just
        // ABOVE 1.0, but the snapshot field is clamped to 1.0
        // exactly.
        let i16_samples: Vec<i16> = vec![i16::MIN; 512];
        let f32_post: Vec<f32> = i16_samples.into_iter().map(i16_to_f32).collect();
        let frame = AudioFrame::new(SR, 1, f32_post);
        let snap = extract_uniform_snapshot(&frame);
        // RMS is clamped to 1.0 even though the |samples| are just
        // above 1.0 in raw-conversion magnitude. This pin guards the
        // clamp behaviour of `compute_rms`.
        assert!(
            (snap.rms - 1.0).abs() < 1e-5,
            "i16::MIN post-conv: rms clamped to 1.0, got {}",
            snap.rms,
        );
        assert!(
            (snap.peak - 1.0).abs() < 1e-5,
            "i16::MIN post-conv: peak clamped to 1.0, got {}",
            snap.peak,
        );
    }

    #[test]
    fn u16_at_zero_post_conversion_yields_unit_peak_after_clamp() {
        // Fixture 8: u16=0 converts to f32 -1.0 (per
        // `cpal_source_pins::u16_to_f32_conversion_at_zero`). The
        // sample magnitude |s| = 1.0 lands at the clamp boundary;
        // routing-matrix pins peak=1.0 exactly.
        let u16_samples: Vec<u16> = vec![0_u16; 512];
        let f32_post: Vec<f32> = u16_samples.into_iter().map(u16_to_f32).collect();
        let frame = AudioFrame::new(SR, 1, f32_post);
        let snap = extract_uniform_snapshot(&frame);
        assert!(
            (snap.rms - 1.0).abs() < 1e-5,
            "u16=0 post-conv: rms expected 1.0, got {}",
            snap.rms,
        );
        assert!(
            (snap.peak - 1.0).abs() < 1e-5,
            "u16=0 post-conv: peak expected 1.0, got {}",
            snap.peak,
        );
    }

    #[test]
    fn u16_at_center_post_conversion_yields_zero_rms() {
        // Fixture 9: u16=32_768 (the unsigned-zero centroid)
        // converts to f32 0.0 (per
        // `cpal_source_pins::u16_to_f32_conversion_at_center`).
        // Routing-matrix pins the snapshot zeroes accordingly.
        let u16_samples: Vec<u16> = vec![32_768_u16; 512];
        let f32_post: Vec<f32> = u16_samples.into_iter().map(u16_to_f32).collect();
        let frame = AudioFrame::new(SR, 1, f32_post);
        let snap = extract_uniform_snapshot(&frame);
        assert_eq!(
            snap.rms, 0.0,
            "u16=32_768 post-conv: rms expected exactly 0.0",
        );
        assert_eq!(
            snap.peak, 0.0,
            "u16=32_768 post-conv: peak expected exactly 0.0",
        );
    }

    // --- Edge-case fixtures (clamp + mixed-amplitude routing) ----------------

    #[test]
    fn f32_overshoot_amplitude_clamps_to_unit_peak_and_rms() {
        // Fixture 10: synthetic f32 amplitude 2.0 (above the [-1,1]
        // canonical range). RMS and peak clamps in
        // `compute_rms` / `compute_peak` saturate at 1.0.
        let frame = AudioFrame::new(SR, 1, vec![2.0_f32; 512]);
        let snap = extract_uniform_snapshot(&frame);
        assert!(
            (snap.rms - 1.0).abs() < 1e-5,
            "overshoot 2.0: rms clamped to 1.0, got {}",
            snap.rms,
        );
        assert!(
            (snap.peak - 1.0).abs() < 1e-5,
            "overshoot 2.0: peak clamped to 1.0, got {}",
            snap.peak,
        );
    }

    #[test]
    fn fft_band_count_matches_documented_constant() {
        // Fixture 11: the routing-matrix corpus relies on the
        // 8-band FFT shape per `docs/36` §2.11. A snapshot from any
        // frame must carry exactly `FFT_BAND_COUNT` bands; future
        // widening would require a routing-matrix recalibration.
        let frame = AudioFrame::new(SR, 1, vec![0.5_f32; 1024]);
        let snap = extract_uniform_snapshot(&frame);
        assert_eq!(
            snap.fft_bands.len(),
            FFT_BAND_COUNT,
            "fft_bands array length must match FFT_BAND_COUNT constant",
        );
        assert_eq!(FFT_BAND_COUNT, 8, "docs/36 §2.11 pins 8 FFT bands");
    }

    #[test]
    fn empty_frame_yields_zeroed_snapshot() {
        // Fixture 12: an empty frame is the zero-input edge case.
        // Per `extract_uniform_snapshot`'s contract documented in
        // `engine/audio/src/uniforms.rs` line 223, an empty
        // `samples` slice produces an all-zero snapshot — including
        // `bpm = 0.0`. The `BPM_DEFAULT = 120.0` fallback applies
        // only when the frame is NON-EMPTY but shorter than the
        // analysis window. Routing-matrix pins this distinction so
        // future scheduler-tick widenings that change the empty-vs-
        // short-frame branch surface as a corpus failure.
        let frame = AudioFrame::new(SR, 1, Vec::new());
        let snap = extract_uniform_snapshot(&frame);
        assert_eq!(snap.rms, 0.0, "empty frame: rms 0.0");
        assert_eq!(snap.peak, 0.0, "empty frame: peak 0.0");
        assert_eq!(snap.bpm, 0.0, "empty frame: bpm 0.0 (NOT BPM_DEFAULT)");
        for (i, b) in snap.fft_bands.iter().enumerate() {
            assert_eq!(*b, 0.0, "empty frame: fft_bands[{i}] 0.0");
        }
    }

    #[test]
    fn short_nonempty_frame_yields_default_bpm() {
        // Fixture 13: a non-empty frame shorter than the
        // FFT_WINDOW_SIZE (1024) analysis window falls back to
        // `BPM_DEFAULT = 120.0`. Pins the BPM contract documented at
        // `engine/audio/src/uniforms.rs` line 223 ("non-empty but
        // shorter than the analysis window"). 64-sample frame is
        // well below the 1024-sample window.
        let frame = AudioFrame::new(SR, 1, vec![0.5_f32; 64]);
        let snap = extract_uniform_snapshot(&frame);
        assert_eq!(
            snap.bpm, BPM_DEFAULT,
            "short non-empty frame: BPM_DEFAULT fallback",
        );
    }
}

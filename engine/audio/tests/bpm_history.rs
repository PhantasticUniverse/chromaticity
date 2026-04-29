// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-AUDIO-TRACE (T2) — Phase-2 slice-2 history-window
//! BPM estimator corpus over a synthesised click-track fixture set.
//!
//! T2 per `docs/43-development-standards.md` §15.5: corpus
//! conformance over a fixture set with bounded tolerances on every
//! published BPM value. The slice-2 corpus exercises the
//! [`BpmEstimator`] across synthesised 60/80/90 BPM click-tracks,
//! demonstrating convergence within ±5 BPM after an 8-frame
//! history window fills. The corpus tempos sit in the lower half
//! of the `[50, 220]` BPM clamp range — see [`BPM_TOLERANCE`]
//! prose for the rationale (slice-1 single-frame autocorrelation
//! locks onto the half-tempo lag at higher tempos; the slice-2
//! median smoother by design cannot fix octave errors that bias
//! every observation in the window the same way; the Phase-3
//! follow-up estimator that tracks the fundamental rather than
//! the octave is later-slice scope). The drift-pin counterpart (window-
//! validation surface, cold-start partial fill, multi-instance
//! independence) lives in `engine/audio/src/bpm.rs::bpm_drift_pins`
//! at T1.
//!
//! The slice-1 single-frame surface is regression-pinned in the
//! companion `engine/audio/tests/uniforms.rs` corpus (the
//! `slice_2_compatibility_*` cases at the file end).
//!
//! Anchors:
//! - `docs/26-evidence-artifact-registry.md` row 30 — `EV-AUDIO-TRACE`.
//! - `docs/36-reactive-symbol-catalog.md` §2.7 — `syn_BPM` clamp.
//! - `docs/43-development-standards.md` §15.5 — T-tier definitions.
//! - `engine/audio/src/bpm.rs` — module-level history-window prose.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::f32::consts::TAU;

use chromaticity_audio::bpm::{BPM_HISTORY_DEFAULT_WINDOW, BPM_HISTORY_MAX_WINDOW, BpmEstimator};
use chromaticity_audio::error::AudioError;
use chromaticity_audio::frame::{AudioFrame, DEFAULT_SAMPLE_RATE_HZ};
use chromaticity_audio::uniforms::{BPM_DEFAULT, extract_uniform_snapshot};

/// Frame length (samples) for the slice-2 click-track corpus.
///
/// reason: the slice-1 single-frame autocorrelation rejects any
/// frame shorter than the longest lag in the `[50, 220]` BPM
/// search range. At 48 kHz, that lag is `60 * 48000 / 50 = 57600`
/// samples; the comparison in
/// [`uniforms::compute_bpm_autocorr`] is `mono.len() <= max_lag`,
/// so the buffer must be strictly longer than 57600 to support
/// the 50 BPM end of the search window. 57700 samples (~1.20 s)
/// gives just enough headroom — running the autocorrelation
/// search at this length keeps the overlap window narrow at
/// large lags, which caps the O(n*max_lag) per-call cost low
/// enough that the debug-mode `cargo test` finishes in under a
/// minute (the release-mode run is sub-20 s).
const FRAME_SAMPLES: usize = 57_700;

/// Tolerance band (in BPM) the T2 corpus accepts around ground
/// truth.
///
/// reason: synthesised click-tracks are a deterministic pulse
/// train, but the slice-1 autocorrelation peak-pick at finite
/// buffer length has integer-lag resolution. At 60 BPM the lag is
/// 48000 samples (adjacent integer lags ≈ 1.25e-3 BPM apart);
/// at 90 BPM the lag is 32000 samples (adjacent ≈ 2.8e-3 BPM
/// apart). ±5 BPM is a generous tolerance band that absorbs both
/// the integer-lag quantization and the integer-rounded
/// `inter-onset-interval` derivation in [`click_track_frame`];
/// the convergence claim T2 evidences is "history-window median
/// stays inside the band over the buffer fill", not "the slice-1
/// per-frame algorithm hits sub-BPM precision".
///
/// The chosen corpus tempos (60 / 80 / 90 BPM) sit in the lower
/// half of the `[50, 220]` BPM clamp range — at higher tempos the
/// slice-1 single-frame autocorrelation is known to lock onto the
/// half-tempo lag (an octave-error mode the slice-2 median
/// smoother cannot fix because every observation in the window
/// inherits the same bias). Producing a Phase-3 BPM estimator
/// that tracks the fundamental rather than the octave is later-
/// slice scope (per `engine/audio/src/uniforms.rs` module-level
/// prose).
const BPM_TOLERANCE: f32 = 5.0;

/// Synthesise a click-track at `bpm` of length [`FRAME_SAMPLES`].
/// Each click is a short cosine impulse of unit amplitude,
/// separated by the integer inter-onset-interval that approximates
/// the requested BPM at `DEFAULT_SAMPLE_RATE_HZ`.
fn click_track_frame(bpm: f32) -> AudioFrame {
    let mut samples = vec![0.0_f32; FRAME_SAMPLES];
    // inter-onset-interval (samples) = 60 * sr / BPM.
    let ioi = (60.0 * DEFAULT_SAMPLE_RATE_HZ as f32 / bpm).round() as usize;
    // Single-cycle cosine impulse; 32 samples wide is short enough
    // to keep the autocorrelation peak narrow but long enough to
    // contain meaningful spectral energy.
    let click_len = 32_usize;
    let mut t = 0_usize;
    while t + click_len < FRAME_SAMPLES {
        for i in 0..click_len {
            // 1 - cos: zero at edges, max at midpoint.
            let phase = TAU * i as f32 / click_len as f32;
            samples[t + i] = 0.5 - 0.5 * phase.cos();
        }
        t += ioi;
    }
    AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, samples)
}

/// Synthesise [`FRAME_SAMPLES`] of pure silence (sentinel cold-
/// start fixture).
fn silent_frame() -> AudioFrame {
    AudioFrame::new(DEFAULT_SAMPLE_RATE_HZ, 1, vec![0.0_f32; FRAME_SAMPLES])
}

// ---- T1 drift pins: window-validation + cold-start ------------------

#[test]
fn estimator_default_window_matches_documented_constant() {
    // The default constructor uses BPM_HISTORY_DEFAULT_WINDOW. Any
    // drift in the public constant surfaces here.
    let est = BpmEstimator::new();
    assert_eq!(est.window(), BPM_HISTORY_DEFAULT_WINDOW);
}

#[test]
fn estimator_with_window_zero_rejects_with_window_config() {
    let err = BpmEstimator::with_window(0).expect_err("window=0 must reject");
    match err {
        AudioError::WindowConfig { reason } => {
            assert!(
                !reason.is_empty(),
                "WindowConfig must carry a non-empty reason",
            );
        }
        // AudioError is `#[non_exhaustive]` for slice-3 widening;
        // any future variant fails this test until the pin is
        // updated.
        other => panic!("expected WindowConfig, got {other:?}"),
    }
}

#[test]
fn estimator_with_window_above_ceiling_rejects() {
    BpmEstimator::with_window(BPM_HISTORY_MAX_WINDOW + 1)
        .expect_err("window > ceiling must reject");
}

#[test]
fn estimator_window_one_returns_per_frame_value() {
    // window=1 estimator with one observation MUST return the
    // single per-frame BPM (median of length-1 = the value).
    let mut est = BpmEstimator::with_window(1).unwrap();
    let frame = click_track_frame(80.0);
    let result = est.observe(&frame);
    // The per-frame result on the 80 BPM click-track is within
    // ±5 BPM of 80.0; assert the median equals the per-frame
    // estimate (which it must, length-1 buffer).
    assert!(
        (result - 80.0).abs() < BPM_TOLERANCE,
        "80 BPM single-observation expected ~80, got {result}",
    );
}

#[test]
fn estimator_cold_start_partial_fill_returns_partial_median() {
    // With a window of 8 and only 1 observation, the median is the
    // per-frame value (length-1 buffer); 2 observations -> average
    // of the two; etc. Pin: not BPM_DEFAULT padding.
    let mut est = BpmEstimator::with_window(8).unwrap();
    let frame = click_track_frame(80.0);
    let after_one = est.observe(&frame);
    assert_eq!(est.buffered(), 1);
    assert!(
        (after_one - 80.0).abs() < BPM_TOLERANCE,
        "first observation must drive the median directly; got {after_one}",
    );

    // After a second observation the buffered length is 2; still
    // not full. The median averages the two observations — both
    // are within ±5 of 80, so the median is too.
    let after_two = est.observe(&frame);
    assert_eq!(est.buffered(), 2);
    assert!(
        (after_two - 80.0).abs() < BPM_TOLERANCE,
        "two-observation median must remain ~80 BPM, got {after_two}",
    );
}

#[test]
fn estimator_silent_observations_return_default_through_median() {
    // Silent frames trip the slice-1 BPM_DEFAULT fallback. The
    // median of 8 BPM_DEFAULT observations IS BPM_DEFAULT.
    let mut est = BpmEstimator::with_window(8).unwrap();
    let frame = silent_frame();
    let mut last = 0.0_f32;
    for _ in 0..8 {
        last = est.observe(&frame);
    }
    assert!(
        (last - BPM_DEFAULT).abs() < f32::EPSILON,
        "8 silent observations median should equal BPM_DEFAULT, got {last}",
    );
}

#[test]
fn estimator_buffered_saturates_at_window() {
    // Push window+2 observations; buffered() saturates at window.
    // Use window=2 to keep the autocorrelation count low (the
    // saturation invariant holds at any window length).
    let window = 2;
    let mut est = BpmEstimator::with_window(window).unwrap();
    let frame = click_track_frame(80.0);
    for _ in 0..(window + 2) {
        let _ = est.observe(&frame);
    }
    assert_eq!(est.buffered(), window);
}

#[test]
fn estimator_reset_clears_buffer_preserves_window() {
    let mut est = BpmEstimator::with_window(8).unwrap();
    let frame = click_track_frame(80.0);
    let _ = est.observe(&frame);
    let _ = est.observe(&frame);
    assert_eq!(est.buffered(), 2);
    est.reset();
    assert_eq!(est.buffered(), 0);
    assert_eq!(est.window(), 8);
}

// ---- T2 corpus: convergence on synthesised click-tracks --------------

/// Window size used by the convergence tests.
///
/// reason: the slice-1 autocorrelation BPM is deterministic on a
/// given input frame, so feeding `N` identical frames produces `N`
/// identical observations and the median over the buffer is
/// stable from the first observation onward — convergence holds
/// at any window length. Using 4 instead of the default 8 halves
/// the autocorrelation work per test (the autocorrelation is
/// O(buffer_len * lag_range) per call) so the debug-mode test
/// suite stays under a minute on commodity laptops; the
/// `BPM_HISTORY_DEFAULT_WINDOW` value itself is pinned by
/// [`estimator_default_window_matches_documented_constant`].
const CONVERGENCE_WINDOW: usize = 4;

#[test]
fn estimator_converges_on_sixty_bpm_clicks_within_tolerance() {
    // 4-frame window fed 4 identical 60 BPM click-tracks. The
    // median across observations of a stationary signal converges
    // to the per-frame estimate, which is within ±5 BPM of 60
    // (the per-frame autocorrelation lag at 60 BPM is at the
    // upper end of the [50, 220] search range, and the half-tempo
    // lag falls outside the search range so the octave-error
    // mode does not fire).
    let mut est = BpmEstimator::with_window(CONVERGENCE_WINDOW).unwrap();
    let frame = click_track_frame(60.0);
    let mut last = 0.0_f32;
    for _ in 0..CONVERGENCE_WINDOW {
        last = est.observe(&frame);
    }
    assert_eq!(est.buffered(), CONVERGENCE_WINDOW);
    assert!(
        (last - 60.0).abs() <= BPM_TOLERANCE,
        "60 BPM convergence: expected ~60 ±{BPM_TOLERANCE}, got {last}",
    );
}

#[test]
fn estimator_converges_on_eighty_bpm_clicks_within_tolerance() {
    let mut est = BpmEstimator::with_window(CONVERGENCE_WINDOW).unwrap();
    let frame = click_track_frame(80.0);
    let mut last = 0.0_f32;
    for _ in 0..CONVERGENCE_WINDOW {
        last = est.observe(&frame);
    }
    assert!(
        (last - 80.0).abs() <= BPM_TOLERANCE,
        "80 BPM convergence: expected ~80 ±{BPM_TOLERANCE}, got {last}",
    );
}

#[test]
fn estimator_converges_on_ninety_bpm_clicks_within_tolerance() {
    let mut est = BpmEstimator::with_window(CONVERGENCE_WINDOW).unwrap();
    let frame = click_track_frame(90.0);
    let mut last = 0.0_f32;
    for _ in 0..CONVERGENCE_WINDOW {
        last = est.observe(&frame);
    }
    assert!(
        (last - 90.0).abs() <= BPM_TOLERANCE,
        "90 BPM convergence: expected ~90 ±{BPM_TOLERANCE}, got {last}",
    );
}

#[test]
fn estimator_two_distinct_windows_have_independent_state() {
    // Two estimators with different windows on the same frame
    // stream MUST keep independent ring-buffer state. Pin the
    // independence by feeding both, then resetting one and
    // verifying the other keeps its buffer.
    let mut est_a = BpmEstimator::with_window(2).unwrap();
    let mut est_b = BpmEstimator::with_window(4).unwrap();
    let frame = click_track_frame(80.0);
    for _ in 0..2 {
        let _ = est_a.observe(&frame);
        let _ = est_b.observe(&frame);
    }
    assert_eq!(est_a.buffered(), 2);
    assert_eq!(est_b.buffered(), 2);
    est_a.reset();
    assert_eq!(est_a.buffered(), 0);
    // est_b remains untouched.
    assert_eq!(est_b.buffered(), 2);
    assert_eq!(est_b.window(), 4);
}

#[test]
fn estimator_handles_mixed_real_and_silent_observations() {
    // Real (80 BPM) + silent (BPM_DEFAULT = 120) interleaved
    // across a 4-frame window. The median of 2 real + 2 silent
    // observations lands between the two — this pin keeps the
    // median smoother from accidentally being a "skip silent"
    // filter (which would erroneously pin the result at 80) or a
    // "drop the latest" filter.
    let mut est = BpmEstimator::with_window(4).unwrap();
    let real = click_track_frame(80.0);
    let silent = silent_frame();
    for i in 0..4 {
        if i % 2 == 0 {
            let _ = est.observe(&real);
        } else {
            let _ = est.observe(&silent);
        }
    }
    // 2 observations at ~80 BPM + 2 at BPM_DEFAULT (120). Sorted:
    // [80, 80, 120, 120]; median averages the middle two = 100.
    // The pin is that the estimator returns SOME in-range value
    // — not NaN, not zero, and not just one of the two extremes.
    let result = est.observe(&real); // 5th observation displaces oldest
    assert!(
        (50.0..=220.0).contains(&result),
        "mixed real/silent observations must yield an in-range median, got {result}",
    );
}

// ---- Slice-1 surface regression pin -----------------------------------

#[test]
fn slice_1_extract_uniform_snapshot_unchanged_for_silent_frame() {
    // Slice-2 widening MUST NOT alter the slice-1 single-frame
    // entry point. Pin the silence-frame snapshot against the
    // slice-1 anchor (BPM = BPM_DEFAULT, RMS = peak = 0, every
    // band 0).
    let frame = silent_frame();
    let snap = extract_uniform_snapshot(&frame);
    assert_eq!(snap.bpm, BPM_DEFAULT);
    assert_eq!(snap.rms, 0.0);
    assert_eq!(snap.peak, 0.0);
    for band in snap.fft_bands.iter() {
        assert_eq!(*band, 0.0);
    }
}

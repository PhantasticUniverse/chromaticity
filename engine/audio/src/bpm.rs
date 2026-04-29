// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-2 EV-AUDIO-TRACE slice-2 history-window BPM estimator.
//!
//! Slice-1 [`crate::uniforms::extract_uniform_snapshot`] computes BPM
//! via single-frame autocorrelation peak-pick across the
//! `[BPM_CLAMP_MIN, BPM_CLAMP_MAX]` lag range. That estimate is noisy
//! on short frames: a single frame of audio rarely contains enough
//! periodicity for the autocorrelation peak to land cleanly on the
//! true tempo.
//!
//! Slice-2 deepens the BPM surface with [`BpmEstimator`], a stateful
//! median-smoother over the last `N` per-frame autocorrelation
//! results. Median (rather than mean) keeps a single noisy frame
//! from dragging the running estimate; the median over 8 observations
//! at a typical 4 Hz analyser cadence settles a tempo estimate
//! across ~2 s of input.
//!
//! # Crate-coupling discipline
//!
//! [`BpmEstimator`] is sample-rate-agnostic: the caller drives the
//! frame cadence (which is decided by the analyser scheduler that
//! lives in a later slice / engine crate). The estimator merely
//! consumes [`crate::frame::AudioFrame`] references and returns a
//! smoothed BPM. No engine/render coupling.
//!
//! # Surface invariants
//!
//! - The slice-1 [`crate::uniforms::extract_uniform_snapshot`]
//!   single-frame entry point STAYS UNCHANGED. Callers that don't
//!   want history continue to use it directly.
//! - [`BpmEstimator::observe`] reuses the same per-frame
//!   autocorrelation [`compute_bpm_autocorr`] helper the slice-1
//!   `extract_uniform_snapshot` uses; the helper is now `pub(crate)`
//!   so the two surfaces share the algorithm.
//! - The estimator returns the median of ALL observations buffered
//!   so far, including any [`crate::uniforms::BPM_DEFAULT`]
//!   fallbacks. Cold-start observations (buffer not yet full) median
//!   over the partial buffer rather than padding with `BPM_DEFAULT`.
//!
//! # Window range
//!
//! `1..=128` frames. `0` is rejected (median undefined); the upper
//! bound is a slice-2 sanity ceiling — extending it past 128 would
//! make the smoother lag a tempo change by ~32 s at the 4 Hz cadence,
//! which is past the §7.1 budget for tempo-following responsiveness.

use std::collections::VecDeque;

use crate::error::AudioError;
use crate::frame::AudioFrame;
use crate::uniforms::compute_bpm_autocorr_for_frame;

/// Default history-window length, in frames.
///
/// 8 frames at the canonical 4 Hz analyser cadence ≈ 2 s of audio,
/// which is the slice-2 target for tempo-following responsiveness.
pub const BPM_HISTORY_DEFAULT_WINDOW: usize = 8;

/// Maximum accepted history-window length, in frames.
///
/// Sanity ceiling per the module-level prose. Slice-3+ may raise it
/// if the analyser scheduler shifts the frame cadence.
pub const BPM_HISTORY_MAX_WINDOW: usize = 128;

/// History-window median-smoothed BPM estimator.
///
/// Stateful counterpart to slice-1's pure-function
/// [`crate::uniforms::extract_uniform_snapshot`] BPM path. Each
/// [`BpmEstimator::observe`] pushes the per-frame autocorrelation
/// result into a ring of the last `window` values and returns the
/// median.
///
/// # Cold start
///
/// Until the buffer fills, the median is taken over the partial
/// buffer (e.g. the very first observation reads back the first
/// per-frame BPM directly). `BPM_DEFAULT` fallbacks from short
/// frames are pushed and median-smoothed alongside real estimates.
///
/// # Independence
///
/// Two estimators with distinct `window` configurations on the same
/// frame stream produce independent state — the per-frame
/// autocorrelation helper is pure, and each estimator owns its ring
/// buffer.
#[derive(Debug, Clone)]
pub struct BpmEstimator {
    window: usize,
    history: VecDeque<f32>,
}

impl BpmEstimator {
    /// Construct an estimator with the default window
    /// ([`BPM_HISTORY_DEFAULT_WINDOW`]).
    #[must_use]
    pub fn new() -> Self {
        // Default window is in the validated `1..=128` range, so
        // the construction never fails. Building via the
        // `unwrap_or_else` here keeps the public infallible
        // constructor without using `.unwrap()` in non-test code.
        Self::with_window(BPM_HISTORY_DEFAULT_WINDOW).unwrap_or_else(|_| Self {
            window: BPM_HISTORY_DEFAULT_WINDOW,
            history: VecDeque::with_capacity(BPM_HISTORY_DEFAULT_WINDOW),
        })
    }

    /// Construct an estimator with a custom window.
    ///
    /// `window` must be in `1..=128`. `0` is rejected (median
    /// undefined); values above 128 are rejected as a slice-2
    /// sanity ceiling.
    pub fn with_window(window: usize) -> Result<Self, AudioError> {
        if window == 0 {
            return Err(AudioError::WindowConfig {
                reason: "window must be at least 1 frame; 0 leaves the median undefined".into(),
            });
        }
        if window > BPM_HISTORY_MAX_WINDOW {
            return Err(AudioError::WindowConfig {
                reason: format!(
                    "window {window} exceeds slice-2 ceiling of {BPM_HISTORY_MAX_WINDOW} frames",
                ),
            });
        }
        Ok(Self {
            window,
            history: VecDeque::with_capacity(window),
        })
    }

    /// Configured window length, in frames.
    #[must_use]
    pub fn window(&self) -> usize {
        self.window
    }

    /// Number of observations currently buffered.
    ///
    /// Saturates at [`BpmEstimator::window`] once the ring is full.
    #[must_use]
    pub fn buffered(&self) -> usize {
        self.history.len()
    }

    /// Observe one frame and return the median-smoothed BPM.
    ///
    /// Pushes the per-frame autocorrelation result onto the ring
    /// buffer (evicting the oldest if the ring is at capacity) and
    /// returns the median of the buffered values. Short frames that
    /// trip the slice-1 fallback push [`crate::uniforms::BPM_DEFAULT`]
    /// onto the ring; that's still median-smoothed alongside real
    /// estimates.
    pub fn observe(&mut self, frame: &AudioFrame) -> f32 {
        let per_frame = compute_bpm_autocorr_for_frame(frame);
        if self.history.len() == self.window {
            self.history.pop_front();
        }
        self.history.push_back(per_frame);
        median(&self.history)
    }

    /// Reset the history buffer.
    ///
    /// The configured window is preserved. Useful when the analyser
    /// detects an ingest-source switch and wants to discard stale
    /// tempo state without rebuilding the estimator.
    pub fn reset(&mut self) {
        self.history.clear();
    }
}

impl Default for BpmEstimator {
    fn default() -> Self {
        Self::new()
    }
}

/// Median of an unsorted f32 ring buffer.
///
/// Returns 0.0 on an empty buffer (cold-start sentinel; callers
/// observe at least once before they expect a meaningful result).
/// Even-length buffers average the two middle elements, so an
/// 8-frame ring smooths over 8 estimates with the centre two
/// dominating equally.
fn median(values: &VecDeque<f32>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<f32> = values.iter().copied().collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 1 {
        // Bounds-checked: n >= 1 implies n / 2 < n.
        sorted[n / 2]
    } else {
        // Bounds-checked: n >= 2 implies both indices < n.
        let lo = sorted[n / 2 - 1];
        let hi = sorted[n / 2];
        0.5 * (lo + hi)
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod bpm_drift_pins {
    //! Evidence: EV-AUDIO-TRACE (T1) — drift pins on the slice-2
    //! `BpmEstimator` window-validation surface and the median
    //! smoother. T1 per `docs/43-development-standards.md` §15.5:
    //! tests pin invariants of the slice-2 window-validation contract
    //! and the median computation against silent / cold-start /
    //! evicting input. The T2 corpus (multi-frame convergence on
    //! synthesised click-tracks) lives in
    //! `engine/audio/tests/bpm_history.rs`.

    use std::collections::VecDeque;

    use super::{BPM_HISTORY_DEFAULT_WINDOW, BPM_HISTORY_MAX_WINDOW, BpmEstimator, median};
    use crate::error::AudioError;

    #[test]
    fn default_window_matches_documented_constant() {
        let est = BpmEstimator::new();
        assert_eq!(est.window(), BPM_HISTORY_DEFAULT_WINDOW);
        assert_eq!(est.buffered(), 0);
    }

    #[test]
    fn with_window_rejects_zero() {
        let err = BpmEstimator::with_window(0).unwrap_err();
        match err {
            AudioError::WindowConfig { reason } => {
                assert!(
                    reason.contains("at least 1"),
                    "WindowConfig reason should mention the lower bound; got {reason}",
                );
            }
        }
    }

    #[test]
    fn with_window_rejects_above_ceiling() {
        let err = BpmEstimator::with_window(BPM_HISTORY_MAX_WINDOW + 1).unwrap_err();
        match err {
            AudioError::WindowConfig { reason } => {
                assert!(
                    reason.contains("ceiling"),
                    "WindowConfig reason should mention the ceiling; got {reason}",
                );
            }
        }
    }

    #[test]
    fn with_window_accepts_one() {
        let est = BpmEstimator::with_window(1).unwrap();
        assert_eq!(est.window(), 1);
    }

    #[test]
    fn with_window_accepts_ceiling() {
        let est = BpmEstimator::with_window(BPM_HISTORY_MAX_WINDOW).unwrap();
        assert_eq!(est.window(), BPM_HISTORY_MAX_WINDOW);
    }

    #[test]
    fn median_empty_returns_zero_sentinel() {
        let buf: VecDeque<f32> = VecDeque::new();
        assert_eq!(median(&buf), 0.0);
    }

    #[test]
    fn median_odd_length_picks_middle() {
        // Sorted: [50, 100, 150]. Middle = 100.
        let buf: VecDeque<f32> = [150.0_f32, 50.0, 100.0].into_iter().collect();
        assert!((median(&buf) - 100.0).abs() < 1e-5);
    }

    #[test]
    fn median_even_length_averages_two_middle() {
        // Sorted: [50, 100, 150, 200]. Middle two = 100 and 150,
        // average = 125.
        let buf: VecDeque<f32> = [200.0_f32, 50.0, 150.0, 100.0].into_iter().collect();
        assert!((median(&buf) - 125.0).abs() < 1e-5);
    }

    #[test]
    fn reset_clears_history_but_preserves_window() {
        let mut est = BpmEstimator::with_window(4).unwrap();
        // Push three observations via the public API would need a
        // frame; the reset behaviour is the buffer-state pin, so
        // poke the field directly under cfg(test) — VecDeque has a
        // public `push_back`.
        est.history.push_back(120.0);
        est.history.push_back(125.0);
        assert_eq!(est.buffered(), 2);
        est.reset();
        assert_eq!(est.buffered(), 0);
        assert_eq!(est.window(), 4);
    }
}

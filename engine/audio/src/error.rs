// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Audio crate error type.
//!
//! Phase-2 EV-AUDIO-TRACE slice-2 introduces the first structured
//! error type for `engine/audio`. Slice-1 had no failure modes that
//! warranted a structured error (`extract_uniform_snapshot` is total
//! over any [`crate::frame::AudioFrame`] shape — empty buffers
//! degrade to the zero baseline rather than failing). Slice-2 adds
//! [`BpmEstimator::with_window`] which validates its window argument,
//! and that's the first call site where a caller-supplied invalid
//! input has to surface as a `Result`.
//!
//! The enum is `#[non_exhaustive]` so future slices can add variants
//! (e.g. ingest-source bind failures when the cpal slice-3 callback
//! lands) without breaking downstream callers.

use std::fmt;

/// Audio crate error.
///
/// Slice-2 surface: only one variant ([`AudioError::WindowConfig`])
/// for [`crate::bpm::BpmEstimator::with_window`] argument validation.
/// `#[non_exhaustive]` reserves room for slice-3+ failure modes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AudioError {
    /// [`crate::bpm::BpmEstimator::with_window`] rejected an invalid
    /// window length. The accepted range is `1..=128` frames; `0`
    /// makes the median undefined and any value above 128 is
    /// rejected as a slice-2 sanity ceiling (raise it in a follow-up
    /// slice if the analyser ever grows a need for a longer window).
    WindowConfig {
        /// Human-readable rationale for inclusion in error logs.
        reason: String,
    },
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WindowConfig { reason } => {
                write!(f, "invalid BPM-estimator window: {reason}")
            }
        }
    }
}

impl std::error::Error for AudioError {}

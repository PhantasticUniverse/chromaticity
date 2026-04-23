// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Audio-frame dataflow types.
//!
//! A single [`AudioFrame`] holds one block of interleaved PCM samples
//! tagged with the sample rate and channel count that produced it. The
//! frame is the unit the ingest boundary (see `crate::ingest`) hands
//! off into the Phase-2 analysis pipeline described in
//! `docs/06-audio-engine.md` §7.1.
//!
//! The sample buffer is owned (`Vec<f32>`) rather than borrowed. The
//! Phase-2 analyser owns its working buffers and copies-in from the
//! ingest source on each hop, so the zero-copy story is tied to the
//! analyser's ring-buffer design, not to this struct. Keeping the
//! buffer owned here makes the first-slice ingest contract trivially
//! `Send`-safe and avoids committing to a lifetime shape that a later
//! ADR may revise.

/// Canonical internal sample rate (Hz) for the audio pipeline.
///
/// Matches the baseline in `docs/06-audio-engine.md` §7.1: all
/// analysis runs at 48 kHz; 44.1 kHz inputs are resampled before
/// reaching the analysis stage.
pub const DEFAULT_SAMPLE_RATE_HZ: u32 = 48_000;

/// One block of interleaved PCM samples handed across the ingest
/// boundary.
///
/// Samples are `f32` in the nominal range `[-1.0, 1.0]`; out-of-range
/// values are tolerated (analyser clamps per §7.4) but not expected
/// from well-behaved sources. Interleaving order is frame-major: for
/// `channels = 2` the buffer is `[L0, R0, L1, R1, ...]`.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioFrame {
    sample_rate_hz: u32,
    channels: u16,
    samples: Vec<f32>,
}

impl AudioFrame {
    /// Build a frame from its three authoritative fields.
    ///
    /// No validation here; the ingest boundary validates shape before
    /// publishing a frame. Callers that synthesise frames in tests are
    /// expected to pass a `samples.len()` that is a multiple of
    /// `channels`.
    #[must_use]
    pub fn new(sample_rate_hz: u32, channels: u16, samples: Vec<f32>) -> Self {
        Self {
            sample_rate_hz,
            channels,
            samples,
        }
    }

    /// Sample rate that produced this frame, in Hz.
    #[must_use]
    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    /// Channel count (1 for mono, 2 for stereo, and so on).
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Read-only view over the interleaved sample buffer.
    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
}

impl Default for AudioFrame {
    /// A silent, zero-length mono frame at [`DEFAULT_SAMPLE_RATE_HZ`].
    ///
    /// Useful as a staleness / silence sentinel during the Phase-1
    /// scaffold before the real device ingest lands.
    fn default() -> Self {
        Self {
            sample_rate_hz: DEFAULT_SAMPLE_RATE_HZ,
            channels: 1,
            samples: Vec::new(),
        }
    }
}

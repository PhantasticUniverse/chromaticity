// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Smart Auto-Listen BPM clock priority cascade coordinator.
//!
//! Resolves the active tempo and beat phase across all live synchronization
//! sources using an explicit, deterministic priority cascade:
//!
//! `ProDjLink > AbletonLink > MidiClock (24 PPQN) > Audio (Fused BPM)`
//!
//! If a higher-priority source goes silent for longer than `timeout_ms`
//! (default `3000 ms`), [`ClockSyncCoordinator`] automatically falls back to
//! the next healthy source without freezing visuals, and seamlessly reclaims
//! priority as soon as the higher-priority source resumes. Operators can also
//! override the cascade via [`ClockSyncMode::Manual`] or [`ClockSyncMode::Off`],
//! and apply a beatmatch nudge via `phase_offset_beats` (`-0.5..=+0.5`).

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

/// Default freshness timeout (3 seconds) before a silent clock source yields priority.
pub const DEFAULT_CLOCK_TIMEOUT_MS: u64 = 3_000;

/// Minimum supported BPM across clock estimation and manual lock modes.
pub const MIN_CLOCK_BPM: f32 = 40.0;

/// Maximum supported BPM across clock estimation and manual lock modes.
pub const MAX_CLOCK_BPM: f32 = 240.0;

/// Fallback BPM used before any clock source has reported a valid tempo.
pub const DEFAULT_CLOCK_BPM: f32 = 120.0;

/// Minimum allowed beatmatch phase offset in beats (`-0.5` beats).
pub const MIN_PHASE_OFFSET_BEATS: f32 = -0.5;

/// Maximum allowed beatmatch phase offset in beats (`+0.5` beats).
pub const MAX_PHASE_OFFSET_BEATS: f32 = 0.5;

/// Standard MIDI Clock pulses per quarter note (24 PPQN).
pub const MIDI_CLOCK_PPQN: u32 = 24;

/// Default sliding-window size for 24 PPQN MIDI Clock inter-pulse averaging (1 quarter-note beat).
pub const DEFAULT_MIDI_CLOCK_WINDOW: usize = 24;

/// MIDI System Real-Time Timing Clock byte (`0xF8`).
pub const MIDI_RT_TIMING_CLOCK: u8 = 0xF8;

/// MIDI System Real-Time Start byte (`0xFA`).
pub const MIDI_RT_START: u8 = 0xFA;

/// MIDI System Real-Time Continue byte (`0xFB`).
pub const MIDI_RT_CONTINUE: u8 = 0xFB;

/// MIDI System Real-Time Stop byte (`0xFC`).
pub const MIDI_RT_STOP: u8 = 0xFC;

/// Maximum valid inter-pulse gap (1 second = 1,000,000 µs) before resetting the MIDI window.
const MAX_MIDI_PULSE_DELTA_US: u64 = 1_000_000;

/// Active synchronization source selected by [`ClockSyncCoordinator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClockSourceKind {
    /// Pioneer / Rekordbox Pro DJ Link over Ethernet/UDP (highest auto priority).
    ProDjLink,
    /// Ableton Link peer-to-peer LAN tempo/phase session.
    AbletonLink,
    /// 24 PPQN MIDI System Real-Time Clock (`0xF8` / `0xFA` / `0xFB` / `0xFC`).
    MidiClock,
    /// Live audio-derived fused BPM estimator (lowest auto priority / fallback).
    Audio,
    /// Operator-locked manual BPM override.
    Manual,
    /// Clock synchronization explicitly disabled.
    Off,
}

impl ClockSourceKind {
    /// Returns the compact uppercase UI badge label for this clock source.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ProDjLink => "PRO DJ",
            Self::AbletonLink => "LINK",
            Self::MidiClock => "MIDI",
            Self::Audio => "AUDIO",
            Self::Manual => "LOCK",
            Self::Off => "OFF",
        }
    }

    /// Returns the numeric auto-cascade priority rank (higher wins; `0` for non-auto modes).
    #[must_use]
    pub const fn priority_rank(self) -> u8 {
        match self {
            Self::ProDjLink => 4,
            Self::AbletonLink => 3,
            Self::MidiClock => 2,
            Self::Audio => 1,
            Self::Manual | Self::Off => 0,
        }
    }
}

impl fmt::Display for ClockSourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Real-time 24 PPQN MIDI System Real-Time Clock estimator.
///
/// Processes `0xF8` (Timing Clock), `0xFA` (Start), `0xFB` (Continue), and
/// `0xFC` (Stop) bytes with a sliding window of inter-pulse intervals to
/// produce a jitter-resistant BPM in `40.0..=240.0`, continuous beat phase
/// in `[0.0, 1.0)`, and transport running state.
#[derive(Debug, Clone, PartialEq)]
pub struct MidiClockEstimator {
    window_size: usize,
    deltas_us: VecDeque<u64>,
    last_pulse_us: Option<u64>,
    last_updated_ms: Option<u64>,
    bpm: f32,
    pulse_in_beat: u32,
    total_pulses: u64,
    running: bool,
    explicitly_stopped: bool,
}

impl Default for MidiClockEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiClockEstimator {
    /// Creates a new estimator with the default 24-pulse (1-beat) sliding window.
    #[must_use]
    pub fn new() -> Self {
        Self::with_window_size(DEFAULT_MIDI_CLOCK_WINDOW)
    }

    /// Creates a new estimator with a custom sliding-window size (clamped to at least `1`).
    #[must_use]
    pub fn with_window_size(window_size: usize) -> Self {
        let clamped_window = window_size.max(1);
        Self {
            window_size: clamped_window,
            deltas_us: VecDeque::with_capacity(clamped_window),
            last_pulse_us: None,
            last_updated_ms: None,
            bpm: DEFAULT_CLOCK_BPM,
            pulse_in_beat: 0,
            total_pulses: 0,
            running: false,
            explicitly_stopped: false,
        }
    }

    /// Resets all accumulated timing state and stops transport.
    pub fn reset(&mut self) {
        self.deltas_us.clear();
        self.last_pulse_us = None;
        self.last_updated_ms = None;
        self.bpm = DEFAULT_CLOCK_BPM;
        self.pulse_in_beat = 0;
        self.total_pulses = 0;
        self.running = false;
        self.explicitly_stopped = false;
    }

    /// Processes a single MIDI byte stamped in milliseconds.
    ///
    /// Returns `true` if the byte was a recognized MIDI Clock / transport
    /// System Real-Time message (`0xF8`, `0xFA`, `0xFB`, or `0xFC`).
    pub fn process_byte(&mut self, byte: u8, now_ms: u64) -> bool {
        self.process_byte_us(byte, now_ms.saturating_mul(1_000))
    }

    /// Processes a single MIDI byte stamped in microseconds for sub-millisecond precision.
    ///
    /// Returns `true` if the byte was a recognized MIDI Clock / transport
    /// System Real-Time message (`0xF8`, `0xFA`, `0xFB`, or `0xFC`).
    pub fn process_byte_us(&mut self, byte: u8, now_us: u64) -> bool {
        let now_ms = now_us / 1_000;
        match byte {
            MIDI_RT_START => {
                self.running = true;
                self.explicitly_stopped = false;
                self.pulse_in_beat = 0;
                self.last_updated_ms = Some(now_ms);
                true
            }
            MIDI_RT_CONTINUE => {
                self.running = true;
                self.explicitly_stopped = false;
                self.last_updated_ms = Some(now_ms);
                true
            }
            MIDI_RT_STOP => {
                self.running = false;
                self.explicitly_stopped = true;
                self.last_updated_ms = Some(now_ms);
                true
            }
            MIDI_RT_TIMING_CLOCK => {
                self.ingest_timing_clock(now_us, now_ms);
                true
            }
            _ => false,
        }
    }

    fn ingest_timing_clock(&mut self, now_us: u64, now_ms: u64) {
        if let Some(prev_us) = self.last_pulse_us {
            if now_us > prev_us {
                let delta_us = now_us - prev_us;
                if delta_us > MAX_MIDI_PULSE_DELTA_US {
                    // Long stall or cable reconnect: flush stale intervals and re-lock cleanly.
                    self.deltas_us.clear();
                } else {
                    if self.deltas_us.len() >= self.window_size {
                        let _ = self.deltas_us.pop_front();
                    }
                    self.deltas_us.push_back(delta_us);
                    self.recompute_bpm();
                }
            }
        }

        self.last_pulse_us = Some(now_us);
        self.last_updated_ms = Some(now_ms);
        self.total_pulses = self.total_pulses.saturating_add(1);

        if self.running || !self.explicitly_stopped {
            self.pulse_in_beat = (self.pulse_in_beat + 1) % MIDI_CLOCK_PPQN;
        }
    }

    fn recompute_bpm(&mut self) {
        if self.deltas_us.is_empty() {
            return;
        }
        let sum_us: u128 = self.deltas_us.iter().map(|&d| u128::from(d)).sum();
        let count = self.deltas_us.len() as f64;
        let mean_delta_us = (sum_us as f64) / count;
        if mean_delta_us > 0.0 {
            let raw_bpm = 60_000_000.0 / (mean_delta_us * f64::from(MIDI_CLOCK_PPQN));
            self.bpm = sanitize_bpm(raw_bpm as f32);
        }
    }

    /// Returns the current smoothed BPM clamped to `40.0..=240.0`.
    #[must_use]
    pub const fn bpm(&self) -> f32 {
        self.bpm
    }

    /// Returns the continuous beat phase in `[0.0, 1.0)`.
    #[must_use]
    pub fn beat_phase(&self) -> f32 {
        wrap_phase((self.pulse_in_beat as f32) / (MIDI_CLOCK_PPQN as f32))
    }

    /// Returns whether MIDI transport is currently running (`Start` / `Continue` active).
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.running
    }

    /// Returns `true` once at least one valid inter-pulse interval has been measured.
    #[must_use]
    pub fn has_lock(&self) -> bool {
        !self.deltas_us.is_empty()
    }

    /// Returns the timestamp in milliseconds of the most recent MIDI real-time byte.
    #[must_use]
    pub const fn last_updated_ms(&self) -> Option<u64> {
        self.last_updated_ms
    }

    /// Returns total `0xF8` timing clock pulses ingested since creation or reset.
    #[must_use]
    pub const fn total_pulses(&self) -> u64 {
        self.total_pulses
    }

    /// Returns an estimation confidence score in `0.0..=1.0` based on sliding-window fill.
    #[must_use]
    pub fn confidence(&self) -> f32 {
        if self.deltas_us.is_empty() {
            return 0.0;
        }
        let fill = (self.deltas_us.len() as f32) / (self.window_size as f32);
        fill.clamp(0.25, 1.0)
    }
}

/// Snapshot of an Ableton Link session state ingested by [`ClockSyncCoordinator`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AbletonLinkSnapshot {
    /// Number of connected Ableton Link peers on the local network.
    pub peer_count: u32,
    /// Current session tempo in beats per minute.
    pub bpm: f32,
    /// Normalized beat phase in `[0.0, 1.0)`.
    pub beat_phase: f32,
    /// Link session quantum (beats per bar/cycle, typically `4.0`).
    pub quantum: f32,
    /// Timestamp in milliseconds when this snapshot was captured.
    pub updated_ms: u64,
}

/// Operator-selected clock synchronization mode.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum ClockSyncMode {
    /// Automatically listen to all sources and pick the highest-priority healthy clock.
    #[default]
    Auto,
    /// Lock tempo to a fixed operator-specified BPM, ignoring external clocks.
    Manual {
        /// Locked tempo in beats per minute (clamped to `40.0..=240.0`).
        bpm: f32,
    },
    /// Disable clock synchronization completely.
    Off,
}

/// Resolved tempo and beat phase produced by [`ClockSyncCoordinator::resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResolvedClock {
    /// Clock source that produced this reading.
    pub source: ClockSourceKind,
    /// Effective tempo in beats per minute (`0.0` in [`ClockSourceKind::Off`]).
    pub bpm: f32,
    /// Effective beat phase in `[0.0, 1.0)` with beatmatch `phase_offset_beats` applied.
    pub beat_phase: f32,
    /// Confidence score in `0.0..=1.0`.
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TimedClockSample {
    bpm: f32,
    beat_phase: f32,
    confidence: f32,
    updated_ms: u64,
}

/// Multi-source Auto-Listen BPM coordinator implementing the strict priority cascade:
///
/// `ProDjLink > AbletonLink > MidiClock > Audio`
#[derive(Debug, Clone, PartialEq)]
pub struct ClockSyncCoordinator {
    mode: ClockSyncMode,
    timeout_ms: u64,
    phase_offset_beats: f32,
    pro_dj_link: Option<TimedClockSample>,
    ableton_link: Option<AbletonLinkSnapshot>,
    midi_clock: MidiClockEstimator,
    audio_bpm: Option<TimedClockSample>,
}

impl Default for ClockSyncCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockSyncCoordinator {
    /// Creates a new coordinator in [`ClockSyncMode::Auto`] with a `3000 ms` timeout.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mode: ClockSyncMode::Auto,
            timeout_ms: DEFAULT_CLOCK_TIMEOUT_MS,
            phase_offset_beats: 0.0,
            pro_dj_link: None,
            ableton_link: None,
            midi_clock: MidiClockEstimator::new(),
            audio_bpm: None,
        }
    }

    /// Overrides the freshness timeout in milliseconds (clamped to at least `1 ms`).
    #[must_use]
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms.max(1);
        self
    }

    /// Returns the configured freshness timeout in milliseconds.
    #[must_use]
    pub const fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    /// Updates the freshness timeout in milliseconds (clamped to at least `1 ms`).
    pub fn set_timeout_ms(&mut self, timeout_ms: u64) {
        self.timeout_ms = timeout_ms.max(1);
    }

    /// Returns the current [`ClockSyncMode`].
    #[must_use]
    pub const fn mode(&self) -> ClockSyncMode {
        self.mode
    }

    /// Updates the [`ClockSyncMode`].
    pub fn set_mode(&mut self, mode: ClockSyncMode) {
        self.mode = match mode {
            ClockSyncMode::Manual { bpm } => ClockSyncMode::Manual {
                bpm: sanitize_bpm(bpm),
            },
            other => other,
        };
    }

    /// Returns the configured beatmatch phase offset in beats (`-0.5..=+0.5`).
    #[must_use]
    pub const fn phase_offset_beats(&self) -> f32 {
        self.phase_offset_beats
    }

    /// Sets the beatmatch phase offset in beats, clamped to `-0.5..=+0.5`.
    pub fn set_phase_offset_beats(&mut self, offset_beats: f32) {
        self.phase_offset_beats = if offset_beats.is_finite() {
            offset_beats.clamp(MIN_PHASE_OFFSET_BEATS, MAX_PHASE_OFFSET_BEATS)
        } else {
            0.0
        };
    }

    /// Returns a read-only reference to the internal 24 PPQN [`MidiClockEstimator`].
    #[must_use]
    pub const fn midi_estimator(&self) -> &MidiClockEstimator {
        &self.midi_clock
    }

    /// Ingests a Pro DJ Link master-deck beat update.
    pub fn update_pro_dj_link(&mut self, bpm: f32, beat_phase: f32, now_ms: u64) {
        if !bpm.is_finite() || bpm <= 0.0 {
            return;
        }
        self.pro_dj_link = Some(TimedClockSample {
            bpm: sanitize_bpm(bpm),
            beat_phase: wrap_phase(beat_phase),
            confidence: 1.0,
            updated_ms: now_ms,
        });
    }

    /// Ingests an Ableton Link session snapshot.
    pub fn update_ableton_link(&mut self, snapshot: AbletonLinkSnapshot) {
        self.ableton_link = Some(AbletonLinkSnapshot {
            peer_count: snapshot.peer_count,
            bpm: sanitize_bpm(snapshot.bpm),
            beat_phase: wrap_phase(snapshot.beat_phase),
            quantum: if snapshot.quantum.is_finite() && snapshot.quantum > 0.0 {
                snapshot.quantum
            } else {
                4.0
            },
            updated_ms: snapshot.updated_ms,
        });
    }

    /// Ingests a single MIDI System Real-Time byte stamped in milliseconds.
    pub fn ingest_midi_byte(&mut self, byte: u8, now_ms: u64) -> bool {
        self.midi_clock.process_byte(byte, now_ms)
    }

    /// Ingests a single MIDI System Real-Time byte stamped in microseconds.
    pub fn ingest_midi_byte_us(&mut self, byte: u8, now_us: u64) -> bool {
        self.midi_clock.process_byte_us(byte, now_us)
    }

    /// Ingests a live audio-fused BPM estimate.
    pub fn update_audio_bpm(&mut self, bpm: f32, beat_phase: f32, confidence: f32, now_ms: u64) {
        if !bpm.is_finite() || bpm <= 0.0 {
            return;
        }
        self.audio_bpm = Some(TimedClockSample {
            bpm: sanitize_bpm(bpm),
            beat_phase: wrap_phase(beat_phase),
            confidence: if confidence.is_finite() {
                confidence.clamp(0.0, 1.0)
            } else {
                0.0
            },
            updated_ms: now_ms,
        });
    }

    /// Resolves the active clock at `now_ms`, honoring mode, freshness, priority cascade,
    /// and beatmatch `phase_offset_beats`.
    #[must_use]
    pub fn resolve(&self, now_ms: u64) -> ResolvedClock {
        match self.mode {
            ClockSyncMode::Off => ResolvedClock {
                source: ClockSourceKind::Off,
                bpm: 0.0,
                beat_phase: 0.0,
                confidence: 0.0,
            },
            ClockSyncMode::Manual { bpm } => {
                let clamped_bpm = sanitize_bpm(bpm);
                let free_phase = free_running_phase(clamped_bpm, now_ms);
                ResolvedClock {
                    source: ClockSourceKind::Manual,
                    bpm: clamped_bpm,
                    beat_phase: self.apply_phase_offset(free_phase),
                    confidence: 1.0,
                }
            }
            ClockSyncMode::Auto => self.resolve_auto(now_ms),
        }
    }

    fn resolve_auto(&self, now_ms: u64) -> ResolvedClock {
        // 1. Pro DJ Link (highest priority)
        if let Some(sample) = self.pro_dj_link {
            if self.is_fresh(sample.updated_ms, now_ms) {
                return ResolvedClock {
                    source: ClockSourceKind::ProDjLink,
                    bpm: sample.bpm,
                    beat_phase: self.apply_phase_offset(sample.beat_phase),
                    confidence: sample.confidence,
                };
            }
        }

        // 2. Ableton Link (requires at least one connected peer)
        if let Some(link) = self.ableton_link {
            if link.peer_count > 0 && self.is_fresh(link.updated_ms, now_ms) {
                return ResolvedClock {
                    source: ClockSourceKind::AbletonLink,
                    bpm: link.bpm,
                    beat_phase: self.apply_phase_offset(link.beat_phase),
                    confidence: 1.0,
                };
            }
        }

        // 3. 24 PPQN MIDI Clock
        if self.midi_clock.has_lock() {
            if let Some(updated_ms) = self.midi_clock.last_updated_ms() {
                if self.is_fresh(updated_ms, now_ms) {
                    return ResolvedClock {
                        source: ClockSourceKind::MidiClock,
                        bpm: self.midi_clock.bpm(),
                        beat_phase: self.apply_phase_offset(self.midi_clock.beat_phase()),
                        confidence: self.midi_clock.confidence(),
                    };
                }
            }
        }

        // 4. Audio Fused BPM (healthy or graceful hold so visuals never freeze)
        if let Some(audio) = self.audio_bpm {
            if self.is_fresh(audio.updated_ms, now_ms) {
                return ResolvedClock {
                    source: ClockSourceKind::Audio,
                    bpm: audio.bpm,
                    beat_phase: self.apply_phase_offset(audio.beat_phase),
                    confidence: audio.confidence,
                };
            }
            // Audio sample is stale, but keep visuals moving smoothly at last known BPM.
            return ResolvedClock {
                source: ClockSourceKind::Audio,
                bpm: audio.bpm,
                beat_phase: self.apply_phase_offset(free_running_phase(audio.bpm, now_ms)),
                confidence: 0.0,
            };
        }

        ResolvedClock {
            source: ClockSourceKind::Audio,
            bpm: DEFAULT_CLOCK_BPM,
            beat_phase: self.apply_phase_offset(free_running_phase(DEFAULT_CLOCK_BPM, now_ms)),
            confidence: 0.0,
        }
    }

    #[must_use]
    const fn is_fresh(&self, updated_ms: u64, now_ms: u64) -> bool {
        now_ms.saturating_sub(updated_ms) < self.timeout_ms
    }

    #[must_use]
    fn apply_phase_offset(&self, beat_phase: f32) -> f32 {
        wrap_phase(beat_phase + self.phase_offset_beats)
    }
}

#[must_use]
fn sanitize_bpm(bpm: f32) -> f32 {
    if bpm.is_finite() {
        bpm.clamp(MIN_CLOCK_BPM, MAX_CLOCK_BPM)
    } else {
        DEFAULT_CLOCK_BPM
    }
}

#[must_use]
fn wrap_phase(phase: f32) -> f32 {
    if !phase.is_finite() {
        return 0.0;
    }
    let wrapped = phase.rem_euclid(1.0);
    if wrapped >= 1.0 || wrapped == 0.0 {
        0.0
    } else {
        wrapped
    }
}

#[must_use]
fn free_running_phase(bpm: f32, now_ms: u64) -> f32 {
    let beats = (now_ms as f64 / 60_000.0) * f64::from(sanitize_bpm(bpm));
    wrap_phase(beats.rem_euclid(1.0) as f32)
}

#[cfg(test)]
mod tests {
    //! Evidence: EV-SCHEDULER-TICK (T1) — verifies 24 PPQN MIDI clock estimation,
    //! strict ProDjLink > AbletonLink > MidiClock > Audio priority cascade,
    //! 3-second timeout fallback and recovery, manual/off overrides, and beatmatch phase offset.

    use super::*;

    fn feed_midi_clock_pulses(
        coordinator: &mut ClockSyncCoordinator,
        bpm: f64,
        start_ms: u64,
        pulse_count: u32,
    ) -> u64 {
        let pulse_interval_ms = 60_000.0 / (bpm * f64::from(MIDI_CLOCK_PPQN));
        let mut last_ms = start_ms;
        for i in 0..pulse_count {
            let t_ms = start_ms + (f64::from(i) * pulse_interval_ms).round() as u64;
            let recognized = coordinator.ingest_midi_byte(MIDI_RT_TIMING_CLOCK, t_ms);
            assert!(recognized);
            last_ms = t_ms;
        }
        last_ms
    }

    #[test]
    fn clock_source_labels_match_contract() {
        assert_eq!(ClockSourceKind::ProDjLink.label(), "PRO DJ");
        assert_eq!(ClockSourceKind::AbletonLink.label(), "LINK");
        assert_eq!(ClockSourceKind::MidiClock.label(), "MIDI");
        assert_eq!(ClockSourceKind::Audio.label(), "AUDIO");
        assert_eq!(ClockSourceKind::Manual.label(), "LOCK");
        assert_eq!(ClockSourceKind::Off.label(), "OFF");
        assert_eq!(ClockSourceKind::ProDjLink.to_string(), "PRO DJ");
    }

    #[test]
    fn midi_clock_24_ppqn_converges_at_128_bpm_and_tracks_start_stop() {
        let mut estimator = MidiClockEstimator::new();
        assert!(!estimator.is_running());
        assert!(!estimator.has_lock());

        // Start transport at t = 0 ms
        assert!(estimator.process_byte(MIDI_RT_START, 0));
        assert!(estimator.is_running());
        assert!((estimator.beat_phase() - 0.0).abs() < f32::EPSILON);

        // Stream 48 pulses (2 full beats) at 128.0 BPM
        let target_bpm = 128.0_f64;
        let pulse_interval_ms = 60_000.0 / (target_bpm * f64::from(MIDI_CLOCK_PPQN));
        let mut last_ms = 0_u64;
        for i in 0..48_u32 {
            let t_ms = (f64::from(i) * pulse_interval_ms).round() as u64;
            assert!(estimator.process_byte(MIDI_RT_TIMING_CLOCK, t_ms));
            last_ms = t_ms;
            let phase = estimator.beat_phase();
            assert!((0.0..1.0).contains(&phase));
        }

        assert!(estimator.has_lock());
        assert!(
            (estimator.bpm() - 128.0).abs() <= 0.5,
            "expected BPM within ±0.5 of 128.0, got {}",
            estimator.bpm()
        );
        assert!((estimator.confidence() - 1.0).abs() < f32::EPSILON);

        // Stop freezes transport state and beat phase
        let phase_before_stop = estimator.beat_phase();
        assert!(estimator.process_byte(MIDI_RT_STOP, last_ms + 10));
        assert!(!estimator.is_running());
        assert!(estimator.process_byte(MIDI_RT_TIMING_CLOCK, last_ms + 30));
        assert!((estimator.beat_phase() - phase_before_stop).abs() < f32::EPSILON);

        // Continue resumes running and advances phase on next pulse
        assert!(estimator.process_byte(MIDI_RT_CONTINUE, last_ms + 40));
        assert!(estimator.is_running());
        assert!(estimator.process_byte(MIDI_RT_TIMING_CLOCK, last_ms + 60));
        assert!((estimator.beat_phase() - phase_before_stop).abs() > 0.01);

        // Unrecognized MIDI byte is ignored cleanly
        assert!(!estimator.process_byte(0x90, last_ms + 70));
    }

    #[test]
    fn priority_cascade_pro_dj_link_over_ableton_midi_audio() {
        let mut coord = ClockSyncCoordinator::new();

        // Populate all four sources cleanly around t = 1_000 ms
        coord.update_audio_bpm(120.0, 0.10, 0.7, 1_000);
        let resolved_audio = coord.resolve(1_050);
        assert_eq!(resolved_audio.source, ClockSourceKind::Audio);
        assert!((resolved_audio.bpm - 120.0).abs() < f32::EPSILON);

        let midi_end = feed_midi_clock_pulses(&mut coord, 124.0, 1_000, 26);
        let resolved_midi = coord.resolve(midi_end + 10);
        assert_eq!(resolved_midi.source, ClockSourceKind::MidiClock);
        assert!((resolved_midi.bpm - 124.0).abs() <= 0.5);

        coord.update_ableton_link(AbletonLinkSnapshot {
            peer_count: 2,
            bpm: 126.5,
            beat_phase: 0.42,
            quantum: 4.0,
            updated_ms: midi_end + 20,
        });
        let resolved_link = coord.resolve(midi_end + 30);
        assert_eq!(resolved_link.source, ClockSourceKind::AbletonLink);
        assert!((resolved_link.bpm - 126.5).abs() < f32::EPSILON);

        coord.update_pro_dj_link(130.0, 0.75, midi_end + 40);
        let resolved_pro_dj = coord.resolve(midi_end + 50);
        assert_eq!(resolved_pro_dj.source, ClockSourceKind::ProDjLink);
        assert!((resolved_pro_dj.bpm - 130.0).abs() < f32::EPSILON);
        assert!((resolved_pro_dj.beat_phase - 0.75).abs() < 1e-6);
    }

    #[test]
    fn timeout_fallback_cascade_and_seamless_recovery() {
        let mut coord = ClockSyncCoordinator::new();
        assert_eq!(coord.timeout_ms(), DEFAULT_CLOCK_TIMEOUT_MS);

        // 1. All four clocks active at t = 1_000 ms -> Pro DJ Link wins
        coord.update_pro_dj_link(132.0, 0.25, 1_000);
        coord.update_ableton_link(AbletonLinkSnapshot {
            peer_count: 1,
            bpm: 128.0,
            beat_phase: 0.50,
            quantum: 4.0,
            updated_ms: 2_500,
        });
        let midi_t = feed_midi_clock_pulses(&mut coord, 125.0, 5_000, 26);
        coord.update_audio_bpm(118.0, 0.80, 0.85, 9_000);

        // At t = 3_999 ms (elapsed 2_999 ms < 3_000 ms), Pro DJ Link is still fresh
        assert_eq!(coord.resolve(3_999).source, ClockSourceKind::ProDjLink);

        // 2. At t = 4_000 ms (3_000 ms after Pro DJ Link's last beat), falls back to Ableton Link
        let fallback_link = coord.resolve(4_000);
        assert_eq!(fallback_link.source, ClockSourceKind::AbletonLink);
        assert!((fallback_link.bpm - 128.0).abs() < f32::EPSILON);

        // 3. At t = 5_600 ms (Ableton Link last updated at 2_500 ms -> 3_100 ms elapsed),
        //    falls back to healthy MIDI Clock (last pulse around midi_t ~ 5_500 ms)
        let fallback_midi = coord.resolve(midi_t + 100);
        assert_eq!(fallback_midi.source, ClockSourceKind::MidiClock);
        assert!((fallback_midi.bpm - 125.0).abs() <= 0.5);

        // 4. At t = midi_t + 3_000 ms (MIDI timed out), falls back to healthy Audio Fused BPM
        let fallback_audio = coord.resolve(midi_t + 3_000);
        assert_eq!(fallback_audio.source, ClockSourceKind::Audio);
        assert!((fallback_audio.bpm - 118.0).abs() < f32::EPSILON);

        // 5. Pro DJ Link Ethernet plugged back in -> immediately reclaims priority
        let recovery_ms = midi_t + 3_500;
        coord.update_pro_dj_link(134.0, 0.125, recovery_ms);
        let recovered = coord.resolve(recovery_ms + 20);
        assert_eq!(recovered.source, ClockSourceKind::ProDjLink);
        assert!((recovered.bpm - 134.0).abs() < f32::EPSILON);
        assert!((recovered.beat_phase - 0.125).abs() < 1e-6);
    }

    #[test]
    fn manual_lock_and_off_override_all_external_clocks() {
        let mut coord = ClockSyncCoordinator::new();
        coord.update_pro_dj_link(135.0, 0.25, 1_000);
        coord.update_ableton_link(AbletonLinkSnapshot {
            peer_count: 3,
            bpm: 128.0,
            beat_phase: 0.5,
            quantum: 4.0,
            updated_ms: 1_000,
        });
        let _ = feed_midi_clock_pulses(&mut coord, 126.0, 1_000, 26);
        coord.update_audio_bpm(120.0, 0.75, 0.9, 1_000);

        // Manual lock overrides all healthy external clocks
        coord.set_mode(ClockSyncMode::Manual { bpm: 140.0 });
        let locked = coord.resolve(1_500);
        assert_eq!(locked.source, ClockSourceKind::Manual);
        assert!((locked.bpm - 140.0).abs() < f32::EPSILON);
        assert!((locked.confidence - 1.0).abs() < f32::EPSILON);
        assert!((0.0..1.0).contains(&locked.beat_phase));

        // Off mode overrides all external clocks and zeroes out tempo/phase/confidence
        coord.set_mode(ClockSyncMode::Off);
        let off = coord.resolve(1_500);
        assert_eq!(off.source, ClockSourceKind::Off);
        assert!((off.bpm - 0.0).abs() < f32::EPSILON);
        assert!((off.beat_phase - 0.0).abs() < f32::EPSILON);
        assert!((off.confidence - 0.0).abs() < f32::EPSILON);

        // Returning to Auto immediately restores Pro DJ Link while still fresh
        coord.set_mode(ClockSyncMode::Auto);
        let back_to_auto = coord.resolve(1_500);
        assert_eq!(back_to_auto.source, ClockSourceKind::ProDjLink);
        assert!((back_to_auto.bpm - 135.0).abs() < f32::EPSILON);
    }

    #[test]
    fn phase_offset_beats_clamps_and_wraps_phase() {
        let mut coord = ClockSyncCoordinator::new();
        coord.update_pro_dj_link(128.0, 0.85, 1_000);

        // Out-of-range offsets clamp to -0.5..=+0.5
        coord.set_phase_offset_beats(0.9);
        assert!((coord.phase_offset_beats() - 0.5).abs() < f32::EPSILON);
        let wrapped_pos = coord.resolve(1_050);
        assert!((wrapped_pos.beat_phase - 0.35).abs() < 1e-6);

        coord.set_phase_offset_beats(-0.9);
        assert!((coord.phase_offset_beats() + 0.5).abs() < f32::EPSILON);
        coord.update_pro_dj_link(128.0, 0.10, 1_100);
        let wrapped_neg = coord.resolve(1_150);
        assert!((wrapped_neg.beat_phase - 0.60).abs() < 1e-6);
    }

    #[test]
    fn ableton_link_zero_peers_yields_to_audio_fallback() {
        let mut coord = ClockSyncCoordinator::new();
        coord.update_ableton_link(AbletonLinkSnapshot {
            peer_count: 0,
            bpm: 130.0,
            beat_phase: 0.2,
            quantum: 4.0,
            updated_ms: 1_000,
        });
        coord.update_audio_bpm(122.0, 0.4, 0.8, 1_000);

        let resolved = coord.resolve(1_050);
        assert_eq!(resolved.source, ClockSourceKind::Audio);
        assert!((resolved.bpm - 122.0).abs() < f32::EPSILON);
    }
}

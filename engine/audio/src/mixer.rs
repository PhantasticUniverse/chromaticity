// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Real-time-safe multi-bus broadcast audio mixer & dynamics DSP.
//!
//! Pure-Rust, zero-GPL, `#![forbid(unsafe_code)]`, allocation-free mixing
//! engine designed for Chromaticity's native broadcast pipeline (System / DJ
//! Master, Microphone, Media / YouTube, Alerts, and indexed custom buses).
//!
//! # Real-Time Guarantees
//!
//! - Every per-bus delay line (`0..=500 ms`) is pre-allocated at mixer
//!   construction or bus registration (`configure_bus`).
//! - [`BroadcastMixer::mix_stereo_block`] performs **zero heap allocations**,
//!   never panics, sanitizes `NaN` / `±Inf` samples to `0.0`, and enforces a
//!   strict `-0.1 dBFS` (`<= 0.9886` linear) master brickwall ceiling.

#![forbid(unsafe_code)]

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use crate::frame::DEFAULT_SAMPLE_RATE_HZ;
use crate::metering::db_to_linear;

/// Minimum allowed per-bus gain in dB.
pub const MIN_BUS_GAIN_DB: f32 = -60.0;

/// Maximum allowed per-bus gain in dB.
pub const MAX_BUS_GAIN_DB: f32 = 24.0;

/// Maximum allowed per-bus A/V sync offset in milliseconds.
pub const MAX_SYNC_OFFSET_MS: u32 = 500;

/// Master brickwall limiter ceiling in dBFS (`-0.1 dBFS`).
pub const BRICKWALL_CEILING_DB: f32 = -0.1;

/// Master brickwall limiter linear ceiling (`10^(-0.1 / 20) ≈ 0.9885531 <= 0.9886`).
pub const BRICKWALL_CEILING_LINEAR: f32 = 0.988_553_1;

/// Floor in dB used when converting near-zero linear magnitudes to decibels.
const DB_FLOOR: f32 = -120.0;

/// Linear amplitude floor corresponding to [`DB_FLOOR`] (`10^(-120 / 20) = 1e-6`).
const LINEAR_FLOOR: f32 = 1.0e-6;

/// Default master brickwall limiter release time in milliseconds.
const MASTER_LIMITER_RELEASE_MS: f32 = 50.0;

/// Identifier for an audio bus in [`BroadcastMixer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BusId {
    /// Primary system or DJ master stereo bus.
    MasterAudio,
    /// Host / vocalist microphone bus.
    Microphone,
    /// Media / browser / YouTube playback bus.
    MediaSource,
    /// Stream alerts / stingers bus.
    Alerts,
    /// User-defined auxiliary bus indexed by `0..=255`.
    Custom(u8),
}

/// Optional per-bus dynamics processor configuration (Noise Gate + Soft-Knee
/// Compressor).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicsConfig {
    /// Enable the noise gate stage.
    pub gate_enabled: bool,
    /// Noise gate open threshold in dBFS (`-96.0..=0.0`).
    pub gate_threshold_db: f32,
    /// Hysteresis below `gate_threshold_db` before closing (`0.0..=24.0` dB).
    pub gate_hysteresis_db: f32,
    /// Noise gate attack time in milliseconds (`0.1..=200.0`).
    pub gate_attack_ms: f32,
    /// Noise gate release time in milliseconds (`5.0..=2000.0`).
    pub gate_release_ms: f32,
    /// Attenuation applied when the gate is closed (`-120.0..=0.0` dB).
    pub gate_floor_db: f32,

    /// Enable the soft-knee compressor stage.
    pub compressor_enabled: bool,
    /// Compressor threshold in dBFS (`-60.0..=0.0`).
    pub compressor_threshold_db: f32,
    /// Compression ratio `N:1` (`1.0..=20.0`).
    pub compressor_ratio: f32,
    /// Soft-knee width in dB (`0.0..=24.0`; `0.0` = hard knee).
    pub compressor_knee_db: f32,
    /// Compressor attack time in milliseconds (`0.1..=200.0`).
    pub compressor_attack_ms: f32,
    /// Compressor release time in milliseconds (`5.0..=2000.0`).
    pub compressor_release_ms: f32,
    /// Post-compression makeup gain in dB (`0.0..=24.0`).
    pub compressor_makeup_db: f32,
}

impl Default for DynamicsConfig {
    fn default() -> Self {
        Self {
            gate_enabled: true,
            gate_threshold_db: -45.0,
            gate_hysteresis_db: 4.0,
            gate_attack_ms: 2.0,
            gate_release_ms: 50.0,
            gate_floor_db: -80.0,
            compressor_enabled: true,
            compressor_threshold_db: -18.0,
            compressor_ratio: 4.0,
            compressor_knee_db: 6.0,
            compressor_attack_ms: 5.0,
            compressor_release_ms: 60.0,
            compressor_makeup_db: 0.0,
        }
    }
}

impl DynamicsConfig {
    /// Return a copy with all fields clamped to safe, finite DSP ranges.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            gate_enabled: self.gate_enabled,
            gate_threshold_db: sanitize_clamped(self.gate_threshold_db, -96.0, 0.0, -45.0),
            gate_hysteresis_db: sanitize_clamped(self.gate_hysteresis_db, 0.0, 24.0, 4.0),
            gate_attack_ms: sanitize_clamped(self.gate_attack_ms, 0.1, 200.0, 2.0),
            gate_release_ms: sanitize_clamped(self.gate_release_ms, 5.0, 2000.0, 50.0),
            gate_floor_db: sanitize_clamped(self.gate_floor_db, DB_FLOOR, 0.0, -80.0),
            compressor_enabled: self.compressor_enabled,
            compressor_threshold_db: sanitize_clamped(
                self.compressor_threshold_db,
                -60.0,
                0.0,
                -18.0,
            ),
            compressor_ratio: sanitize_clamped(self.compressor_ratio, 1.0, 20.0, 4.0),
            compressor_knee_db: sanitize_clamped(self.compressor_knee_db, 0.0, 24.0, 6.0),
            compressor_attack_ms: sanitize_clamped(self.compressor_attack_ms, 0.1, 200.0, 5.0),
            compressor_release_ms: sanitize_clamped(self.compressor_release_ms, 5.0, 2000.0, 60.0),
            compressor_makeup_db: sanitize_clamped(self.compressor_makeup_db, 0.0, 24.0, 0.0),
        }
    }
}

/// Per-bus routing, level, panning, sync-offset, and dynamics configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BusConfig {
    /// Channel fader gain in dB (`-60.0..=+24.0`).
    pub gain_db: f32,
    /// Stereo pan / balance in `-1.0..=+1.0` (constant-power law).
    pub pan: f32,
    /// Mute flag; muted buses contribute silence to the master sum.
    pub muted: bool,
    /// Solo flag; when any bus is soloed, only soloed non-muted buses are heard.
    pub solo: bool,
    /// A/V sync delay offset in milliseconds (`0..=500`).
    pub sync_offset_ms: u32,
    /// Optional per-bus noise gate + soft-knee compressor.
    pub dynamics: Option<DynamicsConfig>,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            pan: 0.0,
            muted: false,
            solo: false,
            sync_offset_ms: 0,
            dynamics: None,
        }
    }
}

impl BusConfig {
    /// Return a copy with all fields clamped to valid broadcast ranges.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            gain_db: sanitize_clamped(self.gain_db, MIN_BUS_GAIN_DB, MAX_BUS_GAIN_DB, 0.0),
            pan: sanitize_clamped(self.pan, -1.0, 1.0, 0.0),
            muted: self.muted,
            solo: self.solo,
            sync_offset_ms: self.sync_offset_ms.min(MAX_SYNC_OFFSET_MS),
            dynamics: self.dynamics.map(DynamicsConfig::sanitized),
        }
    }
}

/// Real-time per-bus (or master) level and dynamics telemetry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BusMeter {
    /// Left-channel RMS magnitude over the most recent mixed block.
    pub left_rms: f32,
    /// Right-channel RMS magnitude over the most recent mixed block.
    pub right_rms: f32,
    /// Left-channel peak magnitude over the most recent mixed block.
    pub left_peak: f32,
    /// Right-channel peak magnitude over the most recent mixed block.
    pub right_peak: f32,
    /// Peak gain reduction in dB (`>= 0.0`) applied by compressor or limiter.
    pub gain_reduction_db: f32,
    /// Whether the noise gate ended the block in the open state (`true` when
    /// no gate is enabled).
    pub gate_open: bool,
}

impl Default for BusMeter {
    fn default() -> Self {
        Self {
            left_rms: 0.0,
            right_rms: 0.0,
            left_peak: 0.0,
            right_peak: 0.0,
            gain_reduction_db: 0.0,
            gate_open: true,
        }
    }
}

/// Constant-power mono pan gains `(left, right)` for `pan in -1.0..=+1.0`.
///
/// Satisfies `left^2 + right^2 == 1.0` across the entire pan law, giving
/// `-3.01 dB` (`sqrt(2)/2`) at center (`pan = 0.0`), `(1.0, 0.0)` at hard left
/// (`-1.0`), and `(0.0, 1.0)` at hard right (`+1.0`).
#[must_use]
pub fn constant_power_pan_gains(pan: f32) -> (f32, f32) {
    let p = sanitize_clamped(pan, -1.0, 1.0, 0.0);
    let theta = (p + 1.0) * FRAC_PI_4;
    (theta.cos(), theta.sin())
}

/// Constant-power stereo balance gains `(left, right)` for interleaved stereo
/// buses (`pan in -1.0..=+1.0`).
///
/// Preserves unity `(1.0, 1.0)` at center (`pan = 0.0`) and smoothly attenuates
/// the opposite channel along a sine curve down to `(1.0, 0.0)` at `-1.0` and
/// `(0.0, 1.0)` at `+1.0`.
#[must_use]
pub fn stereo_balance_gains(pan: f32) -> (f32, f32) {
    let p = sanitize_clamped(pan, -1.0, 1.0, 0.0);
    if p < 0.0 {
        (1.0, ((1.0 + p) * FRAC_PI_2).sin())
    } else if p > 0.0 {
        (((1.0 - p) * FRAC_PI_2).sin(), 1.0)
    } else {
        (1.0, 1.0)
    }
}

/// Pre-allocated interleaved stereo ring buffer for per-bus sync delay.
#[derive(Debug, Clone)]
struct StereoDelayLine {
    /// Interleaved `[L, R, L, R, ...]` storage of length `capacity_frames * 2`.
    buffer: Vec<f32>,
    capacity_frames: usize,
    max_delay_frames: usize,
    write_pos: usize,
}

impl StereoDelayLine {
    fn new(max_delay_frames: usize) -> Self {
        let capacity_frames = max_delay_frames.saturating_add(1).max(1);
        Self {
            buffer: vec![0.0_f32; capacity_frames * 2],
            capacity_frames,
            max_delay_frames,
            write_pos: 0,
        }
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }

    #[inline]
    fn process(&mut self, left: f32, right: f32, delay_frames: usize) -> (f32, f32) {
        let d = delay_frames.min(self.max_delay_frames);
        let write_idx = self.write_pos * 2;
        self.buffer[write_idx] = left;
        self.buffer[write_idx + 1] = right;

        let read_pos = (self.write_pos + self.capacity_frames - d) % self.capacity_frames;
        let read_idx = read_pos * 2;
        let out_l = self.buffer[read_idx];
        let out_r = self.buffer[read_idx + 1];

        self.write_pos += 1;
        if self.write_pos >= self.capacity_frames {
            self.write_pos = 0;
        }
        (out_l, out_r)
    }
}

/// Per-bus real-time dynamics DSP state (noise gate + soft-knee compressor).
#[derive(Debug, Clone)]
struct DynamicsState {
    gate_open: bool,
    gate_gain: f32,
    comp_env: f32,
}

impl Default for DynamicsState {
    fn default() -> Self {
        Self {
            gate_open: false,
            gate_gain: 0.0,
            comp_env: 0.0,
        }
    }
}

impl DynamicsState {
    fn reset(&mut self) {
        self.gate_open = false;
        self.gate_gain = 0.0;
        self.comp_env = 0.0;
    }

    /// Process one stereo sample pair through the noise gate and soft-knee
    /// compressor. Returns `(out_l, out_r, gain_reduction_db, gate_open)`.
    #[inline]
    fn process_sample(
        &mut self,
        left: f32,
        right: f32,
        cfg: &DynamicsConfig,
        sample_rate_hz: f32,
    ) -> (f32, f32, f32, bool) {
        let mut l = left;
        let mut r = right;

        // 1. Noise gate with hysteresis and smooth attack/release.
        let gate_open = if cfg.gate_enabled {
            let peak = l.abs().max(r.abs());
            let level_db = linear_to_db(peak);
            if self.gate_open {
                if level_db < cfg.gate_threshold_db - cfg.gate_hysteresis_db {
                    self.gate_open = false;
                }
            } else if level_db >= cfg.gate_threshold_db {
                self.gate_open = true;
            }

            let target = if self.gate_open {
                1.0
            } else if cfg.gate_floor_db <= -96.0 {
                0.0
            } else {
                db_to_linear(cfg.gate_floor_db)
            };

            let coeff = if target > self.gate_gain {
                time_constant_coeff(cfg.gate_attack_ms, sample_rate_hz)
            } else {
                time_constant_coeff(cfg.gate_release_ms, sample_rate_hz)
            };
            self.gate_gain = target + coeff * (self.gate_gain - target);
            l *= self.gate_gain;
            r *= self.gate_gain;
            self.gate_open
        } else {
            self.gate_open = true;
            self.gate_gain = 1.0;
            true
        };

        // 2. Soft-knee feed-forward compressor (stereo-linked peak detector).
        let gain_reduction_db = if cfg.compressor_enabled && cfg.compressor_ratio > 1.0 {
            let peak = l.abs().max(r.abs());
            let env_coeff = if peak > self.comp_env {
                time_constant_coeff(cfg.compressor_attack_ms, sample_rate_hz)
            } else {
                time_constant_coeff(cfg.compressor_release_ms, sample_rate_hz)
            };
            self.comp_env = peak + env_coeff * (self.comp_env - peak);

            let env_db = linear_to_db(self.comp_env);
            let gr_db = soft_knee_gain_reduction_db(
                env_db,
                cfg.compressor_threshold_db,
                cfg.compressor_ratio,
                cfg.compressor_knee_db,
            );
            let gain = db_to_linear(-gr_db + cfg.compressor_makeup_db);
            l *= gain;
            r *= gain;
            gr_db
        } else {
            0.0
        };

        (l, r, gain_reduction_db, gate_open)
    }
}

/// Internal per-bus slot owning its config, pre-allocated delay line,
/// dynamics state, and latest meter snapshot.
#[derive(Debug, Clone)]
struct BusSlot {
    id: BusId,
    config: BusConfig,
    delay_line: StereoDelayLine,
    dynamics: DynamicsState,
    meter: BusMeter,
}

impl BusSlot {
    fn new(id: BusId, config: BusConfig, max_delay_frames: usize) -> Self {
        Self {
            id,
            config: config.sanitized(),
            delay_line: StereoDelayLine::new(max_delay_frames),
            dynamics: DynamicsState::default(),
            meter: BusMeter::default(),
        }
    }

    fn reset(&mut self) {
        self.delay_line.reset();
        self.dynamics.reset();
        self.meter = BusMeter::default();
    }
}

/// Real-time-safe multi-bus broadcast stereo mixer with per-bus sync delay,
/// noise gate, soft-knee compressor, and master `-0.1 dBFS` brickwall limiter.
#[derive(Debug, Clone)]
pub struct BroadcastMixer {
    sample_rate_hz: u32,
    max_delay_frames: usize,
    buses: Vec<BusSlot>,
    master_limiter_gain: f32,
    master_meter: BusMeter,
}

impl Default for BroadcastMixer {
    fn default() -> Self {
        Self::new(DEFAULT_SAMPLE_RATE_HZ)
    }
}

impl BroadcastMixer {
    /// Construct a new [`BroadcastMixer`] at `sample_rate_hz` with the four
    /// standard broadcast buses (`MasterAudio`, `Microphone`, `MediaSource`,
    /// `Alerts`) pre-allocated with `0..=500 ms` delay ring buffers.
    #[must_use]
    pub fn new(sample_rate_hz: u32) -> Self {
        let sr = sample_rate_hz.max(8_000);
        let max_delay_frames = ms_to_samples(MAX_SYNC_OFFSET_MS, sr);
        let default_ids = [
            BusId::MasterAudio,
            BusId::Microphone,
            BusId::MediaSource,
            BusId::Alerts,
        ];
        let mut buses = Vec::with_capacity(default_ids.len());
        for id in default_ids {
            buses.push(BusSlot::new(id, BusConfig::default(), max_delay_frames));
        }
        Self {
            sample_rate_hz: sr,
            max_delay_frames,
            buses,
            master_limiter_gain: 1.0,
            master_meter: BusMeter::default(),
        }
    }

    /// Return the configured mixer sample rate in Hz.
    #[must_use]
    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    /// Configure an existing bus or register and pre-allocate a new
    /// [`BusId::Custom`] bus (call outside the real-time audio callback).
    pub fn configure_bus(&mut self, id: BusId, config: BusConfig) {
        let sanitized = config.sanitized();
        if let Some(slot) = self.buses.iter_mut().find(|slot| slot.id == id) {
            slot.config = sanitized;
        } else {
            self.buses
                .push(BusSlot::new(id, sanitized, self.max_delay_frames));
        }
    }

    /// Return the active [`BusConfig`] for `id`, if registered.
    #[must_use]
    pub fn bus_config(&self, id: BusId) -> Option<BusConfig> {
        self.buses
            .iter()
            .find(|slot| slot.id == id)
            .map(|slot| slot.config)
    }

    /// Return the latest [`BusMeter`] telemetry for `id`, if registered.
    #[must_use]
    pub fn bus_meter(&self, id: BusId) -> Option<BusMeter> {
        self.buses
            .iter()
            .find(|slot| slot.id == id)
            .map(|slot| slot.meter)
    }

    /// Return the latest post-limiter master [`BusMeter`] telemetry.
    #[must_use]
    pub fn master_meter(&self) -> BusMeter {
        self.master_meter
    }

    /// Clear all delay lines, dynamics envelopes, limiter state, and meters.
    pub fn reset(&mut self) {
        for slot in &mut self.buses {
            slot.reset();
        }
        self.master_limiter_gain = 1.0;
        self.master_meter = BusMeter::default();
    }

    /// Mix all provided bus buffers into interleaved stereo `out_stereo`
    /// (`[L0, R0, L1, R1, ...]`) with **zero heap allocations**.
    ///
    /// - Each bus input slice may be interleaved stereo (`len == out_stereo.len()`)
    ///   or mono (`len == out_stereo.len() / 2`); mono inputs are panned with
    ///   [`constant_power_pan_gains`], while stereo inputs use
    ///   [`stereo_balance_gains`].
    /// - Solo / mute precedence: if any registered bus has `solo == true`,
    ///   only buses with `solo && !muted` are summed; otherwise all non-muted
    ///   buses are summed.
    /// - Non-finite input samples (`NaN`, `±Inf`) are sanitized to `0.0`.
    /// - The summed master output passes through a strict `-0.1 dBFS`
    ///   (`<= 0.9886` linear) brickwall limiter.
    pub fn mix_stereo_block(&mut self, bus_inputs: &[(BusId, &[f32])], out_stereo: &mut [f32]) {
        let frames = out_stereo.len() / 2;
        out_stereo.fill(0.0);
        if frames == 0 {
            self.master_meter = BusMeter::default();
            return;
        }

        let any_solo = self.buses.iter().any(|slot| slot.config.solo);
        let sr = self.sample_rate_hz;
        let sr_f = sr as f32;

        // Zero meters for buses that receive no input this block, while still
        // draining their delay lines if they have pending delayed samples.
        for slot in &mut self.buses {
            let input_slice = bus_inputs
                .iter()
                .find(|(id, _)| *id == slot.id)
                .map(|(_, data)| *data);

            let audible = if any_solo {
                slot.config.solo && !slot.config.muted
            } else {
                !slot.config.muted
            };

            let delay_frames = ms_to_samples(slot.config.sync_offset_ms, sr);
            let fader_gain = if slot.config.gain_db <= MIN_BUS_GAIN_DB {
                0.0
            } else {
                db_to_linear(slot.config.gain_db)
            };

            let is_mono = matches!(input_slice, Some(data) if data.len() == frames);
            let (pan_l, pan_r) = if is_mono {
                constant_power_pan_gains(slot.config.pan)
            } else {
                stereo_balance_gains(slot.config.pan)
            };

            let mut sum_sq_l = 0.0_f64;
            let mut sum_sq_r = 0.0_f64;
            let mut peak_l = 0.0_f32;
            let mut peak_r = 0.0_f32;
            let mut max_gr_db = 0.0_f32;
            let mut gate_open = true;

            for f in 0..frames {
                let (raw_l, raw_r) = match input_slice {
                    Some(data) if is_mono => {
                        let m = sanitize_sample(data[f]);
                        (m, m)
                    }
                    Some(data) => {
                        let base = f * 2;
                        let l = data.get(base).copied().map_or(0.0, sanitize_sample);
                        let r = data.get(base + 1).copied().map_or(0.0, sanitize_sample);
                        (l, r)
                    }
                    None => (0.0, 0.0),
                };

                // Per-bus sync-offset delay (always ticked so mute/unmute stays aligned).
                let (mut l, mut r) = slot.delay_line.process(raw_l, raw_r, delay_frames);

                // Per-bus dynamics (noise gate + soft-knee compressor).
                if let Some(ref dyn_cfg) = slot.config.dynamics {
                    let (dl, dr, gr_db, is_open) =
                        slot.dynamics.process_sample(l, r, dyn_cfg, sr_f);
                    l = dl;
                    r = dr;
                    if gr_db > max_gr_db {
                        max_gr_db = gr_db;
                    }
                    gate_open = is_open;
                }

                // Channel fader + constant-power pan / balance.
                l *= fader_gain * pan_l;
                r *= fader_gain * pan_r;

                if audible {
                    let out_idx = f * 2;
                    out_stereo[out_idx] += l;
                    out_stereo[out_idx + 1] += r;

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
                }
            }

            let inv_frames = 1.0 / (frames as f64);
            slot.meter = BusMeter {
                left_rms: (sum_sq_l * inv_frames).sqrt() as f32,
                right_rms: (sum_sq_r * inv_frames).sqrt() as f32,
                left_peak: peak_l,
                right_peak: peak_r,
                gain_reduction_db: max_gr_db,
                gate_open,
            };
        }

        // Master brickwall limiter (-0.1 dBFS ceiling) + NaN/Inf sanitization.
        let release_coeff = time_constant_coeff(MASTER_LIMITER_RELEASE_MS, sr_f);
        let mut master_sum_sq_l = 0.0_f64;
        let mut master_sum_sq_r = 0.0_f64;
        let mut master_peak_l = 0.0_f32;
        let mut master_peak_r = 0.0_f32;
        let mut master_max_gr_db = 0.0_f32;

        for f in 0..frames {
            let idx = f * 2;
            let in_l = sanitize_sample(out_stereo[idx]);
            let in_r = sanitize_sample(out_stereo[idx + 1]);
            let peak = in_l.abs().max(in_r.abs());

            let target_gain = if peak > BRICKWALL_CEILING_LINEAR {
                BRICKWALL_CEILING_LINEAR / peak
            } else {
                1.0
            };

            if target_gain < self.master_limiter_gain {
                // Instant attack guarantees strict brickwall ceiling on transients.
                self.master_limiter_gain = target_gain;
            } else {
                self.master_limiter_gain =
                    target_gain + release_coeff * (self.master_limiter_gain - target_gain);
            }

            let out_l = (in_l * self.master_limiter_gain)
                .clamp(-BRICKWALL_CEILING_LINEAR, BRICKWALL_CEILING_LINEAR);
            let out_r = (in_r * self.master_limiter_gain)
                .clamp(-BRICKWALL_CEILING_LINEAR, BRICKWALL_CEILING_LINEAR);

            out_stereo[idx] = out_l;
            out_stereo[idx + 1] = out_r;

            let gr_db = if self.master_limiter_gain < 1.0 {
                -linear_to_db(self.master_limiter_gain)
            } else {
                0.0
            };
            if gr_db > master_max_gr_db {
                master_max_gr_db = gr_db;
            }

            let abs_l = out_l.abs();
            let abs_r = out_r.abs();
            if abs_l > master_peak_l {
                master_peak_l = abs_l;
            }
            if abs_r > master_peak_r {
                master_peak_r = abs_r;
            }
            master_sum_sq_l += f64::from(out_l) * f64::from(out_l);
            master_sum_sq_r += f64::from(out_r) * f64::from(out_r);
        }

        let inv_frames = 1.0 / (frames as f64);
        self.master_meter = BusMeter {
            left_rms: (master_sum_sq_l * inv_frames).sqrt() as f32,
            right_rms: (master_sum_sq_r * inv_frames).sqrt() as f32,
            left_peak: master_peak_l,
            right_peak: master_peak_r,
            gain_reduction_db: master_max_gr_db,
            gate_open: true,
        };
    }
}

/// Convert milliseconds to exact integer sample count at `sample_rate_hz`.
#[inline]
fn ms_to_samples(ms: u32, sample_rate_hz: u32) -> usize {
    ((u64::from(sample_rate_hz) * u64::from(ms)) / 1_000) as usize
}

/// Convert linear magnitude to dBFS, floored at [`DB_FLOOR`].
#[inline]
fn linear_to_db(linear: f32) -> f32 {
    if !linear.is_finite() || linear <= LINEAR_FLOOR {
        DB_FLOOR
    } else {
        (20.0 * linear.log10()).max(DB_FLOOR)
    }
}

/// One-pole exponential smoothing coefficient `exp(-1 / (tau_s * sr))`.
#[inline]
fn time_constant_coeff(time_ms: f32, sample_rate_hz: f32) -> f32 {
    let tau_samples = (time_ms * 0.001) * sample_rate_hz;
    if !tau_samples.is_finite() || tau_samples <= 0.0 {
        0.0
    } else {
        (-1.0 / tau_samples).exp()
    }
}

/// Compute soft-knee compression gain reduction in dB (`>= 0.0`).
#[inline]
fn soft_knee_gain_reduction_db(input_db: f32, threshold_db: f32, ratio: f32, knee_db: f32) -> f32 {
    let overshoot = input_db - threshold_db;
    let inv_ratio = 1.0 / ratio;
    let out_db = if knee_db <= 0.0 {
        if overshoot <= 0.0 {
            input_db
        } else {
            threshold_db + overshoot * inv_ratio
        }
    } else if 2.0 * overshoot < -knee_db {
        input_db
    } else if 2.0 * overshoot.abs() <= knee_db {
        let x = overshoot + 0.5 * knee_db;
        input_db + (inv_ratio - 1.0) * (x * x) / (2.0 * knee_db)
    } else {
        threshold_db + overshoot * inv_ratio
    };
    (input_db - out_db).max(0.0)
}

/// Replace `NaN` / `±Inf` with `0.0`.
#[inline]
fn sanitize_sample(sample: f32) -> f32 {
    if sample.is_finite() { sample } else { 0.0 }
}

/// Clamp a finite `f32` to `[min, max]`, falling back to `default` if non-finite.
#[inline]
fn sanitize_clamped(value: f32, min: f32, max: f32, default: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        default
    }
}

#[cfg(test)]
mod tests {
    //! Evidence: EV-AUDIO-TRACE (T1) — multi-bus broadcast audio mixer,
    //! constant-power pan, exact sync-offset delay, noise gate, soft-knee
    //! compressor, master brickwall limiter, and non-finite sanitization pins.

    use super::*;

    #[test]
    fn multi_bus_summing_mute_and_solo_precedence() {
        let mut mixer = BroadcastMixer::new(48_000);
        let frames = 64;
        let master_in = vec![0.25_f32; frames * 2];
        let mic_in = vec![0.15_f32; frames * 2];
        let media_in = vec![0.10_f32; frames * 2];
        let mut out = vec![0.0_f32; frames * 2];

        // 1. Multi-bus summing: 0.25 + 0.15 + 0.10 = 0.50 on both channels.
        mixer.mix_stereo_block(
            &[
                (BusId::MasterAudio, &master_in),
                (BusId::Microphone, &mic_in),
                (BusId::MediaSource, &media_in),
            ],
            &mut out,
        );
        for &s in &out {
            assert!((s - 0.50).abs() < 1e-5, "expected 0.50 sum, got {s}");
        }

        // 2. Mute isolation: mute Microphone -> 0.25 + 0.10 = 0.35.
        mixer.configure_bus(
            BusId::Microphone,
            BusConfig {
                muted: true,
                ..BusConfig::default()
            },
        );
        mixer.mix_stereo_block(
            &[
                (BusId::MasterAudio, &master_in),
                (BusId::Microphone, &mic_in),
                (BusId::MediaSource, &media_in),
            ],
            &mut out,
        );
        for &s in &out {
            assert!(
                (s - 0.35).abs() < 1e-5,
                "expected 0.35 with mic muted, got {s}"
            );
        }
        let mic_meter = mixer.bus_meter(BusId::Microphone).unwrap_or_default();
        assert_eq!(mic_meter.left_peak, 0.0);
        assert_eq!(mic_meter.right_peak, 0.0);

        // 3. Solo exclusivity: unmute and solo MediaSource -> only 0.10 passes.
        mixer.configure_bus(BusId::Microphone, BusConfig::default());
        mixer.configure_bus(
            BusId::MediaSource,
            BusConfig {
                solo: true,
                ..BusConfig::default()
            },
        );
        mixer.mix_stereo_block(
            &[
                (BusId::MasterAudio, &master_in),
                (BusId::Microphone, &mic_in),
                (BusId::MediaSource, &media_in),
            ],
            &mut out,
        );
        for &s in &out {
            assert!(
                (s - 0.10).abs() < 1e-5,
                "expected 0.10 with MediaSource soloed, got {s}"
            );
        }
    }

    #[test]
    fn sync_offset_delay_is_sample_exact_at_48khz() {
        let mut mixer = BroadcastMixer::new(48_000);
        mixer.configure_bus(
            BusId::Microphone,
            BusConfig {
                sync_offset_ms: 10, // 10 ms @ 48 kHz = exact 480-sample delay
                ..BusConfig::default()
            },
        );

        let total_frames = 600;
        let mut mic_in = vec![0.0_f32; total_frames * 2];
        // Unit impulse (0.5 amplitude to stay below limiter threshold) at frame 0.
        mic_in[0] = 0.5;
        mic_in[1] = -0.5;

        let mut out = vec![0.0_f32; total_frames * 2];
        mixer.mix_stereo_block(&[(BusId::Microphone, &mic_in)], &mut out);

        for f in 0..total_frames {
            let l = out[f * 2];
            let r = out[f * 2 + 1];
            if f == 480 {
                assert!(
                    (l - 0.5).abs() < 1e-6 && (r + 0.5).abs() < 1e-6,
                    "expected impulse at exact frame 480, got L={l}, R={r}"
                );
            } else {
                assert!(
                    l.abs() < 1e-6 && r.abs() < 1e-6,
                    "expected silence at frame {f}, got L={l}, R={r}"
                );
            }
        }
    }

    #[test]
    fn noise_gate_closes_on_noise_floor_and_opens_on_loud_signal() {
        let mut mixer = BroadcastMixer::new(48_000);
        mixer.configure_bus(
            BusId::Microphone,
            BusConfig {
                dynamics: Some(DynamicsConfig {
                    gate_enabled: true,
                    gate_threshold_db: -30.0,
                    gate_hysteresis_db: 3.0,
                    gate_attack_ms: 0.5,
                    gate_release_ms: 10.0,
                    gate_floor_db: -100.0,
                    compressor_enabled: false,
                    ..DynamicsConfig::default()
                }),
                ..BusConfig::default()
            },
        );

        let frames = 1_024;
        // Sub-threshold noise floor (~-60 dBFS = 0.001 linear).
        let quiet_in = vec![0.001_f32; frames * 2];
        let mut out = vec![0.0_f32; frames * 2];
        mixer.mix_stereo_block(&[(BusId::Microphone, &quiet_in)], &mut out);

        let quiet_meter = mixer.bus_meter(BusId::Microphone).unwrap_or_default();
        assert!(
            !quiet_meter.gate_open,
            "gate must stay closed on -60 dBFS noise floor"
        );
        assert!(
            quiet_meter.left_peak < 1e-5,
            "closed gate must attenuate noise floor, got peak {}",
            quiet_meter.left_peak
        );

        // Loud signal (-6 dBFS ≈ 0.5 linear) opens the gate cleanly.
        let loud_in = vec![0.5_f32; frames * 2];
        mixer.mix_stereo_block(&[(BusId::Microphone, &loud_in)], &mut out);

        let loud_meter = mixer.bus_meter(BusId::Microphone).unwrap_or_default();
        assert!(loud_meter.gate_open, "gate must open on -6 dBFS signal");
        assert!(
            out[frames * 2 - 2] > 0.48,
            "open gate must pass loud signal near unity, got {}",
            out[frames * 2 - 2]
        );
    }

    #[test]
    fn compressor_reduces_gain_and_brickwall_limiter_caps_12db_overload() {
        let mut mixer = BroadcastMixer::new(48_000);
        mixer.configure_bus(
            BusId::MasterAudio,
            BusConfig {
                dynamics: Some(DynamicsConfig {
                    gate_enabled: false,
                    compressor_enabled: true,
                    compressor_threshold_db: -12.0,
                    compressor_ratio: 4.0,
                    compressor_knee_db: 4.0,
                    compressor_attack_ms: 1.0,
                    compressor_release_ms: 50.0,
                    compressor_makeup_db: 0.0,
                    ..DynamicsConfig::default()
                }),
                ..BusConfig::default()
            },
        );

        // 1. Compressor test: 0 dBFS (1.0) into -12 dB threshold @ 4:1 -> ~9 dB reduction.
        let frames = 2_048;
        let hot_in = vec![1.0_f32; frames * 2];
        let mut out = vec![0.0_f32; frames * 2];
        mixer.mix_stereo_block(&[(BusId::MasterAudio, &hot_in)], &mut out);

        let comp_meter = mixer.bus_meter(BusId::MasterAudio).unwrap_or_default();
        assert!(
            comp_meter.gain_reduction_db > 6.0,
            "compressor must apply substantial gain reduction on 0 dBFS peak, got {} dB",
            comp_meter.gain_reduction_db
        );
        // Settled output should be noticeably below 1.0 (around -3 dBFS ≈ 0.35..0.55).
        assert!(
            out[frames * 2 - 2] < 0.6,
            "compressed output must be attenuated, got {}",
            out[frames * 2 - 2]
        );

        // 2. Brickwall limiter test: +12 dB overload (linear ~3.981) with no compressor.
        mixer.reset();
        mixer.configure_bus(
            BusId::MasterAudio,
            BusConfig {
                gain_db: 12.0,
                dynamics: None,
                ..BusConfig::default()
            },
        );
        let overload_in = vec![1.0_f32; frames * 2];
        mixer.mix_stereo_block(&[(BusId::MasterAudio, &overload_in)], &mut out);

        for (i, &s) in out.iter().enumerate() {
            assert!(
                s.is_finite() && s.abs() <= 0.99 && s.abs() <= BRICKWALL_CEILING_LINEAR + 1e-6,
                "brickwall limiter violated at sample {i}: {s}"
            );
        }
        let master = mixer.master_meter();
        assert!(
            master.left_peak <= 0.9886 && master.right_peak <= 0.9886,
            "master peak exceeded -0.1 dBFS ceiling: L={}, R={}",
            master.left_peak,
            master.right_peak
        );
        assert!(
            master.gain_reduction_db > 10.0,
            "master limiter must report >10 dB GR on +12 dB overload, got {} dB",
            master.gain_reduction_db
        );
    }

    #[test]
    fn constant_power_pan_and_nan_inf_sanitization() {
        // Verify constant-power pan law invariant: L^2 + R^2 == 1.0 across [-1, 1].
        for step in -10..=10 {
            let pan = step as f32 / 10.0;
            let (l, r) = constant_power_pan_gains(pan);
            let power = l * l + r * r;
            assert!(
                (power - 1.0).abs() < 1e-5,
                "constant-power pan law violated at pan={pan}: L={l}, R={r}, power={power}"
            );
        }

        // Verify NaN / ±Inf sanitization and Custom(u8) bus registration.
        let mut mixer = BroadcastMixer::new(48_000);
        let custom = BusId::Custom(3);
        mixer.configure_bus(
            custom,
            BusConfig {
                pan: -1.0, // hard left
                ..BusConfig::default()
            },
        );

        let frames = 4;
        // Mono input with NaN, +Inf, -Inf, and valid 0.5 sample.
        let mono_in = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.5_f32];
        let mut out = vec![0.0_f32; frames * 2];
        mixer.mix_stereo_block(&[(custom, &mono_in)], &mut out);

        assert!(out.iter().all(|s| s.is_finite()));
        // Frames 0..3 sanitized to 0.0; frame 3 hard-left panned to (0.5, 0.0).
        for &sample in out.iter().take(6) {
            assert_eq!(sample, 0.0);
        }
        assert!((out[6] - 0.5).abs() < 1e-6);
        assert!(out[7].abs() < 1e-6);
    }
}

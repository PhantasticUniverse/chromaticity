// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#![forbid(unsafe_code)]

//! GPU cost classification and hysteresis-guarded adaptive resolution scaling.
//!
//! Live performers running complex raymarched or multi-pass scenes need clear
//! visibility into per-scene GPU cost and automatic protection against mid-set
//! frame-rate drops. This module provides:
//!
//! - [`GpuCostTier`] — classifies a frame time relative to the target frame
//!   budget into [`GpuCostTier::Light`], [`GpuCostTier::Medium`], or
//!   [`GpuCostTier::Heavy`].
//! - [`AdaptiveScaleController`] — tracks a rolling exponential moving average
//!   (EMA) of GPU frame times and smoothly steps render resolution scale down
//!   along [`SCALE_LADDER`] (`1.00 -> 0.85 -> 0.75 -> 0.65 -> 0.55 -> 0.50`)
//!   when GPU load exceeds the step-down threshold, then steps back up once
//!   sustained headroom returns. A wide hysteresis deadband plus a per-change
//!   frame cooldown prevents oscillation/thrashing.
//! - [`AdaptiveScaleController::scaled_dimensions`] — maps logical output
//!   dimensions to scaled render target dimensions that are always even and
//!   `>= 64`, keeping 4:2:0 NV12 / VideoToolbox / Media Foundation encoders
//!   strictly satisfied.

use std::fmt;

/// Default target frame rate in frames per second.
pub const DEFAULT_TARGET_FPS: f32 = 60.0;

/// Minimum render dimension (width or height) returned by
/// [`AdaptiveScaleController::scaled_dimensions`]. Guaranteed even so 4:2:0
/// chroma subsampling (NV12 / VideoToolbox) never receives odd dimensions.
pub const MIN_SCALED_DIMENSION: u32 = 64;

/// Discrete resolution-scale ladder ordered from highest to lowest quality.
pub const SCALE_LADDER: &[f32] = &[1.00, 0.85, 0.75, 0.65, 0.55, 0.50];

/// Upper ratio bound (exclusive) of the frame budget classified as [`GpuCostTier::Light`].
const LIGHT_MAX_RATIO: f32 = 0.35;

/// Upper ratio bound (inclusive) of the frame budget classified as [`GpuCostTier::Medium`].
const MEDIUM_MAX_RATIO: f32 = 0.75;

/// Tolerance used when comparing floating-point scale values against ladder stops.
const SCALE_EPSILON: f32 = 1e-4;

/// Per-scene GPU cost tier derived from real GPU frame time relative to the
/// target frame budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpuCostTier {
    /// Frame time is `< 35%` of the target frame budget.
    Light,
    /// Frame time is `35%..=75%` of the target frame budget.
    Medium,
    /// Frame time is `> 75%` of the target frame budget.
    Heavy,
}

impl GpuCostTier {
    /// Classify a GPU frame time (in milliseconds) against `target_fps`.
    ///
    /// - `< 35%` of frame budget (`1000.0 / target_fps`) → [`GpuCostTier::Light`]
    /// - `35%..=75%` of frame budget → [`GpuCostTier::Medium`]
    /// - `> 75%` of frame budget → [`GpuCostTier::Heavy`]
    ///
    /// Non-positive or non-finite `target_fps` falls back to
    /// [`DEFAULT_TARGET_FPS`]. Non-finite `frame_time_ms` is treated
    /// conservatively as [`GpuCostTier::Heavy`]; negative values clamp to `0.0`.
    #[must_use]
    pub fn classify(frame_time_ms: f32, target_fps: f32) -> Self {
        if !frame_time_ms.is_finite() {
            return Self::Heavy;
        }
        let fps = sanitize_fps(target_fps);
        let budget_ms = 1000.0 / fps;
        let ratio = frame_time_ms.max(0.0) / budget_ms;
        if ratio < LIGHT_MAX_RATIO {
            Self::Light
        } else if ratio <= MEDIUM_MAX_RATIO {
            Self::Medium
        } else {
            Self::Heavy
        }
    }

    /// Human-readable display label for UI badges and telemetry.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Medium => "Medium",
            Self::Heavy => "Heavy",
        }
    }
}

impl fmt::Display for GpuCostTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Configuration for [`AdaptiveScaleController`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveScaleConfig {
    /// Target frame rate in FPS (default: `60.0`).
    pub target_fps: f32,
    /// Lowest permitted resolution scale factor (default: `0.50`).
    pub min_scale: f32,
    /// Highest permitted resolution scale factor (default: `1.00`).
    pub max_scale: f32,
    /// Ratio of EMA frame time to frame budget above which scale steps down
    /// (default: `0.85`).
    pub step_down_threshold_ratio: f32,
    /// Ratio of EMA frame time to frame budget below which scale steps up
    /// (default: `0.55`).
    pub step_up_threshold_ratio: f32,
    /// Minimum number of frames to hold a scale level after a change before
    /// allowing another adjustment (default: `30`).
    pub cooldown_frames: u32,
    /// Exponential moving average smoothing factor in `(0.0, 1.0]`
    /// (default: `0.15`).
    pub ema_alpha: f32,
}

impl Default for AdaptiveScaleConfig {
    fn default() -> Self {
        Self {
            target_fps: DEFAULT_TARGET_FPS,
            min_scale: 0.50,
            max_scale: 1.00,
            step_down_threshold_ratio: 0.85,
            step_up_threshold_ratio: 0.55,
            cooldown_frames: 30,
            ema_alpha: 0.15,
        }
    }
}

impl AdaptiveScaleConfig {
    /// Return a sanitized copy with finite, in-range fields so controller
    /// arithmetic can never diverge or produce `NaN`.
    #[must_use]
    fn sanitized(self) -> Self {
        let defaults = Self::default();
        let target_fps = sanitize_fps(self.target_fps);

        let raw_min = if self.min_scale.is_finite() {
            self.min_scale.clamp(0.10, 2.00)
        } else {
            defaults.min_scale
        };
        let raw_max = if self.max_scale.is_finite() {
            self.max_scale.clamp(0.10, 2.00)
        } else {
            defaults.max_scale
        };
        let (min_scale, max_scale) = if raw_min <= raw_max {
            (raw_min, raw_max)
        } else {
            (raw_max, raw_min)
        };

        let mut step_down = if self.step_down_threshold_ratio.is_finite() {
            self.step_down_threshold_ratio.clamp(0.10, 2.00)
        } else {
            defaults.step_down_threshold_ratio
        };
        let mut step_up = if self.step_up_threshold_ratio.is_finite() {
            self.step_up_threshold_ratio.clamp(0.05, 1.95)
        } else {
            defaults.step_up_threshold_ratio
        };
        if step_up >= step_down {
            step_up = (step_down - 0.05).max(0.05);
            step_down = step_down.max(step_up + 0.05);
        }

        let ema_alpha = if self.ema_alpha.is_finite() && self.ema_alpha > 0.0 {
            self.ema_alpha.clamp(0.01, 1.0)
        } else {
            defaults.ema_alpha
        };

        Self {
            target_fps,
            min_scale,
            max_scale,
            step_down_threshold_ratio: step_down,
            step_up_threshold_ratio: step_up,
            cooldown_frames: self.cooldown_frames,
            ema_alpha,
        }
    }
}

/// Output of [`AdaptiveScaleController::record_frame`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleDecision {
    /// Active render resolution scale after evaluating the latest frame.
    pub new_scale: f32,
    /// `true` if `new_scale` changed on this frame; `false` otherwise.
    pub changed: bool,
    /// Current [`GpuCostTier`] derived from the rolling EMA frame time.
    pub tier: GpuCostTier,
}

/// Hysteresis-guarded adaptive resolution scaler driven by rolling EMA GPU
/// frame times.
#[derive(Debug, Clone)]
pub struct AdaptiveScaleController {
    config: AdaptiveScaleConfig,
    scale: f32,
    ema_ms: f32,
    samples: u64,
    frames_since_change: u32,
}

impl Default for AdaptiveScaleController {
    fn default() -> Self {
        Self::new(AdaptiveScaleConfig::default())
    }
}

impl AdaptiveScaleController {
    /// Construct a new controller initialized at `config.max_scale`.
    #[must_use]
    pub fn new(config: AdaptiveScaleConfig) -> Self {
        let config = config.sanitized();
        Self {
            scale: config.max_scale,
            ema_ms: 0.0,
            samples: 0,
            frames_since_change: config.cooldown_frames,
            config,
        }
    }

    /// Return the active (sanitized) configuration.
    #[must_use]
    pub const fn config(&self) -> AdaptiveScaleConfig {
        self.config
    }

    /// Current render resolution scale factor in `[min_scale, max_scale]`.
    #[must_use]
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    /// Current rolling exponential moving average GPU frame time in milliseconds.
    #[must_use]
    pub const fn ema_ms(&self) -> f32 {
        self.ema_ms
    }

    /// Current [`GpuCostTier`] based on [`Self::ema_ms`] and `config.target_fps`.
    #[must_use]
    pub fn tier(&self) -> GpuCostTier {
        GpuCostTier::classify(self.ema_ms, self.config.target_fps)
    }

    /// Reset the controller to `max_scale` with a cleared EMA history.
    pub fn reset(&mut self) {
        self.scale = self.config.max_scale;
        self.ema_ms = 0.0;
        self.samples = 0;
        self.frames_since_change = self.config.cooldown_frames;
    }

    /// Record a measured GPU frame time in milliseconds and return the updated
    /// [`ScaleDecision`].
    ///
    /// Non-finite inputs are ignored (returning the current scale with
    /// `changed: false`). Negative values are clamped to `0.0`.
    pub fn record_frame(&mut self, frame_time_ms: f32) -> ScaleDecision {
        if !frame_time_ms.is_finite() {
            return ScaleDecision {
                new_scale: self.scale,
                changed: false,
                tier: self.tier(),
            };
        }

        let sample_ms = frame_time_ms.max(0.0);
        if self.samples == 0 {
            self.ema_ms = sample_ms;
        } else {
            let alpha = self.config.ema_alpha;
            self.ema_ms = alpha * sample_ms + (1.0 - alpha) * self.ema_ms;
        }
        self.samples = self.samples.saturating_add(1);

        let budget_ms = 1000.0 / self.config.target_fps;
        let ratio = self.ema_ms / budget_ms;
        let tier = GpuCostTier::classify(self.ema_ms, self.config.target_fps);

        let mut changed = false;
        if self.frames_since_change >= self.config.cooldown_frames {
            if ratio > self.config.step_down_threshold_ratio {
                let next = self.next_scale_down();
                if (next - self.scale).abs() > SCALE_EPSILON {
                    self.scale = next;
                    self.frames_since_change = 0;
                    changed = true;
                }
            } else if ratio < self.config.step_up_threshold_ratio {
                let next = self.next_scale_up();
                if (next - self.scale).abs() > SCALE_EPSILON {
                    self.scale = next;
                    self.frames_since_change = 0;
                    changed = true;
                }
            }
        }

        if !changed {
            self.frames_since_change = self.frames_since_change.saturating_add(1);
        }

        ScaleDecision {
            new_scale: self.scale,
            changed,
            tier,
        }
    }

    /// Compute scaled render-target `(width, height)` for a logical output size.
    ///
    /// Both axes are rounded down to an even integer and clamped to at least
    /// [`MIN_SCALED_DIMENSION`] (`64`) so 4:2:0 NV12 and VideoToolbox encoders
    /// never receive odd or degenerate dimensions.
    #[must_use]
    pub fn scaled_dimensions(&self, base_w: u32, base_h: u32) -> (u32, u32) {
        (
            scale_axis_even(base_w, self.scale),
            scale_axis_even(base_h, self.scale),
        )
    }

    /// Next lower scale stop along [`SCALE_LADDER`], clamped to `min_scale`.
    fn next_scale_down(&self) -> f32 {
        let min = self.config.min_scale;
        if self.scale <= min + SCALE_EPSILON {
            return min;
        }
        for &stop in SCALE_LADDER {
            if stop < self.scale - SCALE_EPSILON && stop >= min - SCALE_EPSILON {
                return stop.max(min);
            }
        }
        min
    }

    /// Next higher scale stop along [`SCALE_LADDER`], clamped to `max_scale`.
    fn next_scale_up(&self) -> f32 {
        let max = self.config.max_scale;
        if self.scale >= max - SCALE_EPSILON {
            return max;
        }
        for &stop in SCALE_LADDER.iter().rev() {
            if stop > self.scale + SCALE_EPSILON && stop <= max + SCALE_EPSILON {
                return stop.min(max);
            }
        }
        max
    }
}

/// Sanitize a caller-supplied FPS value, falling back to [`DEFAULT_TARGET_FPS`].
#[must_use]
fn sanitize_fps(target_fps: f32) -> f32 {
    if target_fps.is_finite() && target_fps > 0.0 {
        target_fps
    } else {
        DEFAULT_TARGET_FPS
    }
}

/// Scale a single pixel axis and guarantee an even integer `>= MIN_SCALED_DIMENSION`.
#[must_use]
fn scale_axis_even(base: u32, scale: f32) -> u32 {
    let scaled = (f64::from(base) * f64::from(scale)).round().max(0.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let raw = if scaled >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        scaled as u32
    };
    let even = raw & !1;
    even.max(MIN_SCALED_DIMENSION)
}

#[cfg(test)]
mod tests {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — GPU cost tier classification,
    //! hysteresis-guarded adaptive resolution scaling, and even-dimension
    //! NV12/VideoToolbox invariants.

    use super::*;

    #[test]
    fn gpu_cost_tier_classifies_light_medium_heavy_at_60fps() {
        // 60 FPS budget = 16.6667 ms.
        // Light: < 35% (< 5.833 ms)
        assert_eq!(GpuCostTier::classify(0.0, 60.0), GpuCostTier::Light);
        assert_eq!(GpuCostTier::classify(3.5, 60.0), GpuCostTier::Light);
        assert_eq!(GpuCostTier::classify(5.8, 60.0), GpuCostTier::Light);

        // Medium: 35%..=75% (5.833 ms..=12.5 ms)
        assert_eq!(GpuCostTier::classify(5.9, 60.0), GpuCostTier::Medium);
        assert_eq!(GpuCostTier::classify(10.0, 60.0), GpuCostTier::Medium);
        assert_eq!(GpuCostTier::classify(12.5, 60.0), GpuCostTier::Medium);

        // Heavy: > 75% (> 12.5 ms)
        assert_eq!(GpuCostTier::classify(12.6, 60.0), GpuCostTier::Heavy);
        assert_eq!(GpuCostTier::classify(20.0, 60.0), GpuCostTier::Heavy);
        assert_eq!(GpuCostTier::classify(f32::NAN, 60.0), GpuCostTier::Heavy);
        assert_eq!(
            GpuCostTier::classify(f32::INFINITY, 60.0),
            GpuCostTier::Heavy
        );
        assert_eq!(GpuCostTier::classify(-2.0, 60.0), GpuCostTier::Light);

        // Labels & Display
        assert_eq!(GpuCostTier::Light.label(), "Light");
        assert_eq!(GpuCostTier::Medium.label(), "Medium");
        assert_eq!(GpuCostTier::Heavy.label(), "Heavy");
        assert_eq!(format!("{}", GpuCostTier::Heavy), "Heavy");
    }

    #[test]
    fn gpu_cost_tier_adapts_to_target_fps() {
        // At 30 FPS (33.33 ms budget), 10 ms is 30% => Light.
        assert_eq!(GpuCostTier::classify(10.0, 30.0), GpuCostTier::Light);
        // At 120 FPS (8.33 ms budget), 7 ms is 84% => Heavy.
        assert_eq!(GpuCostTier::classify(7.0, 120.0), GpuCostTier::Heavy);
        // Invalid FPS falls back to 60 FPS (10 ms => Medium).
        assert_eq!(GpuCostTier::classify(10.0, 0.0), GpuCostTier::Medium);
        assert_eq!(GpuCostTier::classify(10.0, -60.0), GpuCostTier::Medium);
        assert_eq!(GpuCostTier::classify(10.0, f32::NAN), GpuCostTier::Medium);
    }

    #[test]
    fn smooth_step_down_on_sustained_heavy_frames() {
        let mut ctrl = AdaptiveScaleController::new(AdaptiveScaleConfig {
            cooldown_frames: 10,
            ema_alpha: 0.5,
            ..AdaptiveScaleConfig::default()
        });
        assert!((ctrl.scale() - 1.00).abs() < 1e-5);

        let mut observed_scales = vec![ctrl.scale()];
        // Feed sustained heavy frames (18 ms > 85% of 16.67 ms budget).
        for _ in 0..80 {
            let decision = ctrl.record_frame(18.0);
            assert_eq!(decision.tier, GpuCostTier::Heavy);
            if decision.changed {
                observed_scales.push(decision.new_scale);
            }
        }

        // Must step smoothly down the ladder: 1.00 -> 0.85 -> 0.75 -> 0.65 -> 0.55 -> 0.50
        let expected = [1.00, 0.85, 0.75, 0.65, 0.55, 0.50];
        assert_eq!(observed_scales.len(), expected.len());
        for (actual, exp) in observed_scales.iter().zip(expected.iter()) {
            assert!((actual - exp).abs() < 1e-5, "actual={actual}, exp={exp}");
        }
        assert!((ctrl.scale() - 0.50).abs() < 1e-5);
    }

    #[test]
    fn hysteresis_and_cooldown_prevent_thrashing() {
        let mut ctrl = AdaptiveScaleController::new(AdaptiveScaleConfig::default());

        // Trigger one step-down from 1.00 -> 0.85 with heavy frames.
        let d0 = ctrl.record_frame(16.0); // 96% of budget -> steps down immediately
        assert!(d0.changed);
        assert!((d0.new_scale - 0.85).abs() < 1e-5);

        // During the 30-frame cooldown, even extreme frames must NOT change scale.
        for _ in 0..30 {
            let d = ctrl.record_frame(25.0);
            assert!(!d.changed, "must not step during cooldown");
            assert!((d.new_scale - 0.85).abs() < 1e-5);
        }

        // Now feed frames inside the hysteresis deadband (55%..=85% of 16.67 ms = 9.17..=14.17 ms).
        // Let EMA settle to 11.5 ms (69% of budget) and confirm zero scale changes.
        let mut ctrl_deadband = AdaptiveScaleController::new(AdaptiveScaleConfig {
            cooldown_frames: 5,
            ema_alpha: 0.5,
            ..AdaptiveScaleConfig::default()
        });
        // Seed at 0.85 first.
        let first = ctrl_deadband.record_frame(16.0);
        assert!((first.new_scale - 0.85).abs() < 1e-5);

        // Alternate between 10.0 ms (60%) and 13.0 ms (78%) — both inside deadband.
        for i in 0..100 {
            let ms = if i % 2 == 0 { 10.0 } else { 13.0 };
            let d = ctrl_deadband.record_frame(ms);
            assert!(
                !d.changed,
                "frame {i} inside hysteresis deadband should not thrash"
            );
            assert!((d.new_scale - 0.85).abs() < 1e-5);
        }
    }

    #[test]
    fn smooth_step_up_recovery_on_light_frames() {
        let mut ctrl = AdaptiveScaleController::new(AdaptiveScaleConfig {
            cooldown_frames: 10,
            ema_alpha: 0.5,
            ..AdaptiveScaleConfig::default()
        });

        // Drive scale down to 0.50 first.
        for _ in 0..80 {
            ctrl.record_frame(18.0);
        }
        assert!((ctrl.scale() - 0.50).abs() < 1e-5);

        // Now feed sustained light frames (4.0 ms = 24% of 16.67 ms budget).
        let mut recovered = vec![ctrl.scale()];
        for _ in 0..100 {
            let decision = ctrl.record_frame(4.0);
            if decision.changed {
                recovered.push(decision.new_scale);
            }
        }

        let expected = [0.50, 0.55, 0.65, 0.75, 0.85, 1.00];
        assert_eq!(recovered.len(), expected.len());
        for (actual, exp) in recovered.iter().zip(expected.iter()) {
            assert!((actual - exp).abs() < 1e-5, "actual={actual}, exp={exp}");
        }
        assert_eq!(ctrl.tier(), GpuCostTier::Light);
        assert!((ctrl.scale() - 1.00).abs() < 1e-5);
    }

    #[test]
    fn scaled_dimensions_always_even_and_at_least_64() {
        let mut ctrl = AdaptiveScaleController::new(AdaptiveScaleConfig {
            cooldown_frames: 0,
            ema_alpha: 1.0,
            ..AdaptiveScaleConfig::default()
        });

        let test_resolutions = [
            (3840, 2160),
            (1920, 1080),
            (1921, 1081),
            (1280, 720),
            (853, 479),
            (100, 100),
            (63, 65),
            (1, 1),
            (0, 0),
        ];

        // Test across every scale stop on the ladder.
        for step in 0..=SCALE_LADDER.len() {
            for &(w, h) in &test_resolutions {
                let (sw, sh) = ctrl.scaled_dimensions(w, h);
                assert_eq!(sw % 2, 0, "width {sw} must be even (base={w}, step={step})");
                assert_eq!(
                    sh % 2,
                    0,
                    "height {sh} must be even (base={h}, step={step})"
                );
                assert!(
                    sw >= MIN_SCALED_DIMENSION,
                    "width {sw} must be >= {MIN_SCALED_DIMENSION}"
                );
                assert!(
                    sh >= MIN_SCALED_DIMENSION,
                    "height {sh} must be >= {MIN_SCALED_DIMENSION}"
                );
            }
            ctrl.record_frame(20.0);
        }

        // Exact check at 1080p full scale after reset.
        ctrl.reset();
        assert_eq!(ctrl.scaled_dimensions(1920, 1080), (1920, 1080));
        // One step down to 0.85 -> 1632x918.
        ctrl.record_frame(20.0);
        assert_eq!(ctrl.scaled_dimensions(1920, 1080), (1632, 918));
    }

    #[test]
    fn non_finite_frame_times_are_safely_ignored() {
        let mut ctrl = AdaptiveScaleController::default();
        let d = ctrl.record_frame(f32::NAN);
        assert!(!d.changed);
        assert!((d.new_scale - 1.00).abs() < 1e-5);
        assert!((ctrl.ema_ms() - 0.0).abs() < 1e-5);
    }
}

// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

#![forbid(unsafe_code)]

//! Mix compositor capabilities and GPU cost / adaptive-resolution telemetry
//! Tauri IPC commands.
//!
//! Backed by [`chromaticity_render::gpu_cost::AdaptiveScaleController`] and
//! [`chromaticity_render::compositor::{LayerStack, BlendMode}`] managed on
//! [`crate::state::AppState`].

use crate::state::AppState;
use chromaticity_render::compositor::BlendMode;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tauri::State;

/// Maximum number of concurrent compositing layers exposed to `Mix.svelte`.
pub const COMPOSITOR_MAX_LAYERS: u32 = 8;

/// Serializable GPU cost & adaptive-scaling telemetry payload returned to the
/// frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpuCostTelemetryDto {
    /// Cost tier classification: `"Light"`, `"Medium"`, or `"Heavy"`.
    pub tier: String,
    /// Rolling exponential moving average GPU frame time in milliseconds.
    pub ema_ms: f32,
    /// Active render resolution scale factor (`1.0` when adaptive scaling is disabled).
    pub current_scale: f32,
    /// Effective scaled render target width in pixels (even, `>= 64`).
    pub scaled_width: u32,
    /// Effective scaled render target height in pixels (even, `>= 64`).
    pub scaled_height: u32,
    /// Whether adaptive resolution scaling is currently enabled.
    pub adaptive_enabled: bool,
}

/// Serializable GPU multi-layer compositor capability summary for `Mix.svelte`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompositorCapabilitiesDto {
    /// Supported GPU blend and keying mode identifiers matching [`BlendMode`].
    pub blend_modes: Vec<String>,
    /// Maximum supported simultaneous layers in the compositor stack.
    pub max_layers: u32,
    /// Number of layers currently registered in [`AppState::compositor_stack`].
    pub active_layer_count: usize,
    /// Whether the shell compositor backend is ready to composite frames.
    pub ready: bool,
}

/// Return the canonical label for a [`BlendMode`] variant.
#[must_use]
const fn blend_mode_label(mode: BlendMode) -> &'static str {
    match mode {
        BlendMode::Normal => "Normal",
        BlendMode::Additive => "Additive",
        BlendMode::Screen => "Screen",
        BlendMode::Multiply => "Multiply",
        BlendMode::LumaKey { .. } => "LumaKey",
        BlendMode::ChromaKey { .. } => "ChromaKey",
    }
}

/// Canonical list of all GPU blend and keying modes implemented by
/// `chromaticity_render::compositor`.
#[must_use]
fn supported_blend_modes() -> Vec<String> {
    let variants = [
        BlendMode::Normal,
        BlendMode::Additive,
        BlendMode::Screen,
        BlendMode::Multiply,
        BlendMode::LumaKey {
            threshold: 0.1,
            softness: 0.05,
        },
        BlendMode::ChromaKey {
            key_rgb: [0.0, 1.0, 0.0],
            similarity: 0.25,
            smoothness: 0.1,
        },
    ];
    variants
        .into_iter()
        .map(|mode| blend_mode_label(mode).to_string())
        .collect()
}

/// Inner implementation for [`get_gpu_cost_telemetry`], unit-testable without
/// a Tauri runtime.
pub fn get_gpu_cost_telemetry_impl(app: &AppState) -> Result<GpuCostTelemetryDto, String> {
    let adaptive_enabled = app.adaptive_scale_enabled.load(Ordering::Relaxed);
    let (base_w, base_h) = match app.adaptive_base_size.lock() {
        Ok(g) => *g,
        Err(p) => *p.into_inner(),
    };
    let ctrl = match app.adaptive_scale.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    let tier = ctrl.tier().label().to_string();
    let ema_ms = ctrl.ema_ms();
    let (current_scale, scaled_width, scaled_height) = if adaptive_enabled {
        let (sw, sh) = ctrl.scaled_dimensions(base_w, base_h);
        (ctrl.scale(), sw, sh)
    } else {
        let even_w = (base_w & !1).max(chromaticity_render::gpu_cost::MIN_SCALED_DIMENSION);
        let even_h = (base_h & !1).max(chromaticity_render::gpu_cost::MIN_SCALED_DIMENSION);
        (1.0_f32, even_w, even_h)
    };

    Ok(GpuCostTelemetryDto {
        tier,
        ema_ms,
        current_scale,
        scaled_width,
        scaled_height,
        adaptive_enabled,
    })
}

/// Inner implementation for [`set_adaptive_scale_enabled`], unit-testable
/// without a Tauri runtime.
pub fn set_adaptive_scale_enabled_impl(
    app: &AppState,
    enabled: bool,
) -> Result<GpuCostTelemetryDto, String> {
    app.adaptive_scale_enabled.store(enabled, Ordering::Relaxed);
    get_gpu_cost_telemetry_impl(app)
}

/// Inner implementation for [`get_compositor_capabilities`], unit-testable
/// without a Tauri runtime.
#[must_use]
pub fn get_compositor_capabilities_impl(app: &AppState) -> CompositorCapabilitiesDto {
    let active_layer_count = match app.compositor_stack.lock() {
        Ok(g) => g.len(),
        Err(p) => p.into_inner().len(),
    };
    let ready = {
        let gpu = match app.gpu_renderer.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        gpu.state_name() != "error"
    };
    CompositorCapabilitiesDto {
        blend_modes: supported_blend_modes(),
        max_layers: COMPOSITOR_MAX_LAYERS,
        active_layer_count,
        ready,
    }
}

/// Return live GPU cost tier (`"Light" | "Medium" | "Heavy"`), rolling EMA
/// frame time, active resolution scale, scaled dimensions, and adaptive-scale
/// toggle state.
#[tauri::command]
pub fn get_gpu_cost_telemetry(
    app_state: State<'_, AppState>,
) -> Result<GpuCostTelemetryDto, String> {
    get_gpu_cost_telemetry_impl(app_state.inner())
}

/// Enable or disable adaptive resolution scaling and return the updated GPU
/// cost telemetry snapshot.
#[tauri::command]
pub fn set_adaptive_scale_enabled(
    app_state: State<'_, AppState>,
    enabled: bool,
) -> Result<GpuCostTelemetryDto, String> {
    set_adaptive_scale_enabled_impl(app_state.inner(), enabled)
}

/// Return supported GPU compositor blend/keying modes, maximum layer count,
/// active layer count, and live compositor readiness for `Mix.svelte`.
#[tauri::command]
pub fn get_compositor_capabilities(app_state: State<'_, AppState>) -> CompositorCapabilitiesDto {
    get_compositor_capabilities_impl(app_state.inner())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod mix_ipc_tests {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — Mix compositor capability and
    //! GPU cost / adaptive resolution telemetry IPC contract pins.

    use super::*;
    use chromaticity_render::compositor::{CompositeLayer, LayerTransform};

    #[test]
    fn gpu_cost_telemetry_reports_tier_scale_and_toggle_state() {
        let app = AppState::new();

        // Initial state: Light tier, 0.0 ms EMA, full 1.0 scale, adaptive enabled.
        let initial = get_gpu_cost_telemetry_impl(&app).expect("initial telemetry");
        assert_eq!(initial.tier, "Light");
        assert!((initial.ema_ms - 0.0).abs() < 1e-5);
        assert!((initial.current_scale - 1.0).abs() < 1e-5);
        assert_eq!(initial.scaled_width, 1920);
        assert_eq!(initial.scaled_height, 1080);
        assert!(initial.adaptive_enabled);

        // Feed sustained heavy frames (18 ms > 85% of 16.67 ms budget) to trigger step-down.
        for _ in 0..45 {
            app.record_gpu_frame_cost(1920, 1080, 18.0);
        }
        let heavy = get_gpu_cost_telemetry_impl(&app).expect("heavy telemetry");
        assert_eq!(heavy.tier, "Heavy");
        assert!(heavy.ema_ms > 12.5);
        assert!(heavy.current_scale < 1.0);
        assert!(heavy.scaled_width < 1920);
        assert!(heavy.scaled_height < 1080);
        assert_eq!(heavy.scaled_width % 2, 0);
        assert_eq!(heavy.scaled_height % 2, 0);
        assert!(heavy.adaptive_enabled);

        // Disabling adaptive scaling restores 1.0x full resolution while preserving EMA/tier.
        let disabled =
            set_adaptive_scale_enabled_impl(&app, false).expect("disable adaptive scale");
        assert!(!disabled.adaptive_enabled);
        assert_eq!(disabled.tier, "Heavy");
        assert!((disabled.current_scale - 1.0).abs() < 1e-5);
        assert_eq!(disabled.scaled_width, 1920);
        assert_eq!(disabled.scaled_height, 1080);

        // Re-enabling adaptive scaling resumes the controller's active scale factor.
        let reenabled =
            set_adaptive_scale_enabled_impl(&app, true).expect("re-enable adaptive scale");
        assert!(reenabled.adaptive_enabled);
        assert!(reenabled.current_scale < 1.0);
        assert_eq!(reenabled.scaled_width, heavy.scaled_width);
        assert_eq!(reenabled.scaled_height, heavy.scaled_height);
    }

    #[test]
    fn compositor_capabilities_exposes_blend_modes_layers_and_readiness() {
        let app = AppState::new();
        let caps = get_compositor_capabilities_impl(&app);
        assert!(caps.ready);
        assert_eq!(caps.max_layers, COMPOSITOR_MAX_LAYERS);
        assert_eq!(caps.active_layer_count, 0);
        assert_eq!(
            caps.blend_modes,
            vec![
                "Normal",
                "Additive",
                "Screen",
                "Multiply",
                "LumaKey",
                "ChromaKey",
            ]
        );

        // Pushing a layer onto AppState::compositor_stack reflects in active_layer_count.
        {
            let mut stack = app.compositor_stack.lock().unwrap();
            stack.push_layer(CompositeLayer {
                id: "deck-a".to_string(),
                visible: true,
                z_index: 0,
                blend: BlendMode::Additive,
                transform: LayerTransform::default(),
                width: 2,
                height: 2,
                rgba_pixels: vec![255; 16],
            });
        }
        let updated = get_compositor_capabilities_impl(&app);
        assert_eq!(updated.active_layer_count, 1);
    }
}

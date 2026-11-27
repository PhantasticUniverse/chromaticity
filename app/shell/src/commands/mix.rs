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
use chromaticity_render::compositor::{BlendMode, CompositeLayer, LayerStack, LayerTransform};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::Ordering;
use tauri::State;

/// Maximum number of concurrent compositing layers exposed to `Mix.svelte`.
pub const COMPOSITOR_MAX_LAYERS: u32 = 16;

/// Minimum clamped layer scale factor on each axis.
const MIN_LAYER_SCALE: f32 = 0.05;

/// Maximum clamped layer scale factor on each axis.
const MAX_LAYER_SCALE: f32 = 4.0;

/// Default Rec. 709 luma-key threshold when using `luma-key`.
const DEFAULT_LUMA_THRESHOLD: f32 = 0.1;

/// Default Rec. 709 luma-key softness when using `luma-key`.
const DEFAULT_LUMA_SOFTNESS: f32 = 0.05;

/// Default chroma-key color (pure green screen).
const DEFAULT_CHROMA_KEY_RGB: [f32; 3] = [0.0, 1.0, 0.0];

/// Default chroma-key similarity cutoff.
const DEFAULT_CHROMA_SIMILARITY: f32 = 0.25;

/// Default chroma-key edge smoothness.
const DEFAULT_CHROMA_SMOOTHNESS: f32 = 0.1;

const fn default_layer_scale() -> f32 {
    1.0
}

/// Serializable per-layer specification sent from `Mix.svelte` to synchronize
/// [`AppState::compositor_stack`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MixLayerSpecDto {
    /// Stable layer identifier.
    pub id: String,
    /// Blend or keying mode identifier: `"normal"`, `"screen"`, `"add"`,
    /// `"multiply"`, `"luma-key"`, `"chroma-key"`, `"overlay"`, `"hard-light"`,
    /// `"difference"`, or `"exclusion"`.
    pub blend_mode: String,
    /// Master layer opacity (`0.0..=1.0`, clamped).
    pub opacity: f32,
    /// Whether this layer is visible in the compositor stack.
    pub visible: bool,
    /// Normalized horizontal translation offset.
    #[serde(default)]
    pub x: f32,
    /// Normalized vertical translation offset.
    #[serde(default)]
    pub y: f32,
    /// Horizontal scale factor (`0.05..=4.0`, clamped).
    #[serde(default = "default_layer_scale")]
    pub scale_x: f32,
    /// Vertical scale factor (`0.05..=4.0`, clamped).
    #[serde(default = "default_layer_scale")]
    pub scale_y: f32,
    /// Counter-clockwise rotation angle in degrees.
    #[serde(default)]
    pub rotation_deg: f32,
    /// Optional hex color string (`#RGB` or `#RRGGBB`) for `chroma-key`.
    #[serde(default)]
    pub chroma_key_color: Option<String>,
    /// Optional similarity cutoff (`0.0..=1.0`, clamped) for `chroma-key`.
    #[serde(default)]
    pub chroma_similarity: Option<f32>,
    /// Optional edge smoothness (`0.0..=1.0`, clamped) for `chroma-key`.
    #[serde(default)]
    pub chroma_smoothness: Option<f32>,
}

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
        BlendMode::Overlay => "Overlay",
        BlendMode::HardLight => "HardLight",
        BlendMode::Difference => "Difference",
        BlendMode::Exclusion => "Exclusion",
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
        BlendMode::Overlay,
        BlendMode::HardLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
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

/// Clamp `value` into `[min, max]`, substituting `default` for non-finite inputs.
#[must_use]
fn clamp_finite(value: f32, min: f32, max: f32, default: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        default
    }
}

/// Return `value` when finite, otherwise `default`.
#[must_use]
fn finite_or(value: f32, default: f32) -> f32 {
    if value.is_finite() { value } else { default }
}

/// Parse an ASCII `#RGB`, `RGB`, `#RRGGBB`, or `RRGGBB` hex string into
/// normalized `[r, g, b]` channels in `0.0..=1.0`.
fn parse_hex_rgb(raw: &str) -> Result<[f32; 3], String> {
    let trimmed = raw.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if !hex.is_ascii() {
        return Err(format!("invalid chroma_key_color '{raw}'"));
    }
    match hex.len() {
        6 => {
            let parse_pair = |slice: &str| -> Result<f32, String> {
                u8::from_str_radix(slice, 16)
                    .map(|v| f32::from(v) / 255.0)
                    .map_err(|_| format!("invalid chroma_key_color '{raw}'"))
            };
            Ok([
                parse_pair(&hex[0..2])?,
                parse_pair(&hex[2..4])?,
                parse_pair(&hex[4..6])?,
            ])
        }
        3 => {
            let parse_nibble = |slice: &str| -> Result<f32, String> {
                u8::from_str_radix(slice, 16)
                    .map(|v| f32::from(v * 17) / 255.0)
                    .map_err(|_| format!("invalid chroma_key_color '{raw}'"))
            };
            Ok([
                parse_nibble(&hex[0..1])?,
                parse_nibble(&hex[1..2])?,
                parse_nibble(&hex[2..3])?,
            ])
        }
        _ => Err(format!(
            "invalid chroma_key_color '{raw}' (expected #RGB or #RRGGBB)"
        )),
    }
}

/// Translate a frontend [`MixLayerSpecDto`] blend mode string and optional
/// keying parameters into a [`BlendMode`].
fn parse_blend_mode(dto: &MixLayerSpecDto) -> Result<BlendMode, String> {
    let normalized = dto.blend_mode.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "normal" => Ok(BlendMode::Normal),
        "add" | "additive" => Ok(BlendMode::Additive),
        "screen" => Ok(BlendMode::Screen),
        "multiply" => Ok(BlendMode::Multiply),
        "overlay" => Ok(BlendMode::Overlay),
        "hard-light" | "hardlight" | "hard_light" => Ok(BlendMode::HardLight),
        "difference" => Ok(BlendMode::Difference),
        "exclusion" => Ok(BlendMode::Exclusion),
        "luma-key" | "lumakey" | "luma_key" => Ok(BlendMode::LumaKey {
            threshold: DEFAULT_LUMA_THRESHOLD,
            softness: DEFAULT_LUMA_SOFTNESS,
        }),
        "chroma-key" | "chromakey" | "chroma_key" => {
            let key_rgb = match dto.chroma_key_color.as_deref() {
                Some(raw) if !raw.trim().is_empty() => parse_hex_rgb(raw)?,
                _ => DEFAULT_CHROMA_KEY_RGB,
            };
            let similarity = dto
                .chroma_similarity
                .map_or(DEFAULT_CHROMA_SIMILARITY, |v| {
                    clamp_finite(v, 0.0, 1.0, DEFAULT_CHROMA_SIMILARITY)
                });
            let smoothness = dto
                .chroma_smoothness
                .map_or(DEFAULT_CHROMA_SMOOTHNESS, |v| {
                    clamp_finite(v, 0.0, 1.0, DEFAULT_CHROMA_SMOOTHNESS)
                });
            Ok(BlendMode::ChromaKey {
                key_rgb,
                similarity,
                smoothness,
            })
        }
        _ => Err(format!(
            "unsupported blend_mode '{}' for layer '{}'",
            dto.blend_mode, dto.id
        )),
    }
}

/// Inner implementation for [`set_mix_layer_stack`], unit-testable without a
/// Tauri runtime.
pub fn set_mix_layer_stack_impl(
    app: &AppState,
    layers: Vec<MixLayerSpecDto>,
) -> Result<CompositorCapabilitiesDto, String> {
    let max_layers = COMPOSITOR_MAX_LAYERS as usize;
    if layers.len() > max_layers {
        return Err(format!(
            "layer stack exceeds maximum of {max_layers} layers (got {})",
            layers.len()
        ));
    }

    let mut validated = Vec::with_capacity(layers.len());
    for (idx, dto) in layers.into_iter().enumerate() {
        let id = dto.id.trim().to_string();
        if id.is_empty() {
            return Err(format!("layer at index {idx} has an empty id"));
        }
        let blend = parse_blend_mode(&dto)?;
        let transform = LayerTransform {
            offset_xy: [finite_or(dto.x, 0.0), finite_or(dto.y, 0.0)],
            scale_xy: [
                clamp_finite(dto.scale_x, MIN_LAYER_SCALE, MAX_LAYER_SCALE, 1.0),
                clamp_finite(dto.scale_y, MIN_LAYER_SCALE, MAX_LAYER_SCALE, 1.0),
            ],
            rotation_rad: finite_or(dto.rotation_deg, 0.0).to_radians(),
            opacity: clamp_finite(dto.opacity, 0.0, 1.0, 1.0),
        };
        let z_index = i32::try_from(idx).unwrap_or(i32::MAX);
        validated.push((id, dto.visible, z_index, blend, transform));
    }

    {
        let mut stack = match app.compositor_stack.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        // Preserve previously uploaded pixel buffers for layers with matching IDs.
        let mut existing_buffers: HashMap<String, (u32, u32, Vec<u8>)> = stack
            .layers()
            .iter()
            .map(|layer| {
                (
                    layer.id.clone(),
                    (layer.width, layer.height, layer.rgba_pixels.clone()),
                )
            })
            .collect();

        let mut next_stack = LayerStack::new();
        for (id, visible, z_index, blend, transform) in validated {
            let (width, height, rgba_pixels) = existing_buffers
                .remove(&id)
                .unwrap_or_else(|| (1, 1, vec![0_u8; 4]));
            next_stack.push_layer(CompositeLayer {
                id,
                visible,
                z_index,
                blend,
                transform,
                width,
                height,
                rgba_pixels,
            });
        }
        *stack = next_stack;
    }

    Ok(get_compositor_capabilities_impl(app))
}

/// Synchronize the frontend mix layer stack into [`AppState::compositor_stack`]
/// and return updated compositor capabilities.
#[tauri::command]
pub fn set_mix_layer_stack(
    app_state: State<'_, AppState>,
    layers: Vec<MixLayerSpecDto>,
) -> Result<CompositorCapabilitiesDto, String> {
    set_mix_layer_stack_impl(app_state.inner(), layers)
}

/// Serializable resolved direct video stream for a YouTube visual layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct YouTubeStreamDto {
    /// Validated 11-character YouTube video identifier.
    pub video_id: String,
    /// Video title returned by `yt-dlp`.
    pub title: String,
    /// Direct HTTPS media stream URL suitable for native `<video>` playback.
    pub stream_url: String,
}

/// Validate a strict 11-character YouTube video ID (`[A-Za-z0-9_-]{11}`).
#[must_use]
pub fn is_valid_youtube_video_id(candidate: &str) -> bool {
    candidate.len() == 11
        && candidate
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Locate a local `yt-dlp` binary across common macOS/Linux/user install paths
/// and `PATH`.
#[must_use]
pub fn resolve_yt_dlp_binary() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("HOME") {
        let local_bin = PathBuf::from(home).join(".local/bin/yt-dlp");
        if local_bin.is_file() {
            return Some(local_bin);
        }
    }
    for candidate in [
        PathBuf::from("/opt/homebrew/bin/yt-dlp"),
        PathBuf::from("/usr/local/bin/yt-dlp"),
    ] {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let bin_name = if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(bin_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Parse `yt-dlp --get-title --get-url` stdout into `(title, stream_url)`.
pub fn parse_yt_dlp_output(video_id: &str, stdout: &str) -> Result<YouTubeStreamDto, String> {
    let lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let stream_url = lines
        .iter()
        .rev()
        .find(|line| line.starts_with("https://"))
        .copied()
        .ok_or_else(|| "yt-dlp did not return a direct HTTPS stream URL".to_string())?;
    let title = lines
        .iter()
        .find(|line| !line.starts_with("https://") && !line.starts_with("http://"))
        .copied()
        .unwrap_or(video_id);
    Ok(YouTubeStreamDto {
        video_id: video_id.to_string(),
        title: title.to_string(),
        stream_url: stream_url.to_string(),
    })
}

/// Resolve a direct playable HTTPS video stream URL for a validated YouTube
/// video ID using `yt-dlp`. Runs synchronously (call via `spawn_blocking`).
pub fn resolve_youtube_stream_impl(video_id: &str) -> Result<YouTubeStreamDto, String> {
    let trimmed = video_id.trim();
    if !is_valid_youtube_video_id(trimmed) {
        return Err("invalid YouTube video ID (expected 11 characters)".to_string());
    }
    let yt_dlp = resolve_yt_dlp_binary()
        .ok_or_else(|| "yt-dlp not found on system PATH or ~/.local/bin".to_string())?;
    eprintln!(
        "[chromaticity][youtube] resolving {trimmed} via {}",
        yt_dlp.display()
    );
    let watch_url = format!("https://www.youtube.com/watch?v={trimmed}");
    let output = Command::new(&yt_dlp)
        .args([
            "--no-update",
            "--no-playlist",
            "--no-warnings",
            "-f",
            "bestvideo[ext=mp4][vcodec^=avc1][height<=1080]/bestvideo[ext=mp4][height<=1080]/best[ext=mp4][height<=1080]/best",
            "--get-title",
            "--get-url",
            "--",
            &watch_url,
        ])
        .output()
        .map_err(|err| format!("failed to run yt-dlp: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        let msg = if detail.is_empty() {
            "yt-dlp failed to resolve YouTube stream".to_string()
        } else {
            format!("yt-dlp error: {detail}")
        };
        eprintln!("[chromaticity][youtube] failed for {trimmed}: {msg}");
        return Err(msg);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let resolved = parse_yt_dlp_output(trimmed, &stdout)?;
    eprintln!(
        "[chromaticity][youtube] resolved {trimmed}: {}",
        resolved.title
    );
    Ok(resolved)
}

/// Resolve a direct playable HTTPS stream URL for a YouTube video ID so
/// `Mix.svelte` can render it in a native `<video>` element (bypassing
/// WKWebView iframe Error 153 and enabling direct canvas compositing).
#[tauri::command]
pub fn resolve_youtube_stream(video_id: String) -> Result<YouTubeStreamDto, String> {
    resolve_youtube_stream_impl(&video_id)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod mix_ipc_tests {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — Mix compositor capability and
    //! GPU cost / adaptive resolution telemetry IPC contract pins.

    use super::*;

    #[test]
    fn youtube_video_id_validation_and_output_parsing() {
        assert!(is_valid_youtube_video_id("8aBvuRwcbds"));
        assert!(!is_valid_youtube_video_id("short"));
        assert!(!is_valid_youtube_video_id("8aBvuRwcbds;rm"));
        assert!(resolve_youtube_stream_impl("bad;id").is_err());

        let parsed = parse_yt_dlp_output(
            "8aBvuRwcbds",
            "Best of WRC 2025\nhttps://rr3.googlevideo.com/videoplayback?itag=18\n",
        )
        .expect("valid yt-dlp output");
        assert_eq!(parsed.video_id, "8aBvuRwcbds");
        assert_eq!(parsed.title, "Best of WRC 2025");
        assert_eq!(
            parsed.stream_url,
            "https://rr3.googlevideo.com/videoplayback?itag=18"
        );
        assert!(parse_yt_dlp_output("8aBvuRwcbds", "Title only\n").is_err());
    }

    fn sample_layer_dto(id: &str, blend_mode: &str) -> MixLayerSpecDto {
        MixLayerSpecDto {
            id: id.to_string(),
            blend_mode: blend_mode.to_string(),
            opacity: 1.0,
            visible: true,
            x: 0.0,
            y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation_deg: 0.0,
            chroma_key_color: None,
            chroma_similarity: None,
            chroma_smoothness: None,
        }
    }

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
                "Overlay",
                "HardLight",
                "Difference",
                "Exclusion",
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

    #[test]
    fn set_mix_layer_stack_maps_all_six_blend_modes_and_updates_active_count() {
        let app = AppState::new();

        let layers = vec![
            MixLayerSpecDto {
                x: 0.25,
                y: -0.15,
                scale_x: 1.5,
                scale_y: 0.75,
                rotation_deg: 90.0,
                opacity: 0.8,
                ..sample_layer_dto("layer-normal", "normal")
            },
            sample_layer_dto("layer-screen", "screen"),
            sample_layer_dto("layer-add", "add"),
            MixLayerSpecDto {
                visible: false,
                ..sample_layer_dto("layer-multiply", "multiply")
            },
            sample_layer_dto("layer-luma", "luma-key"),
            MixLayerSpecDto {
                chroma_key_color: Some("#0080ff".to_string()),
                chroma_similarity: Some(0.35),
                chroma_smoothness: Some(0.12),
                ..sample_layer_dto("layer-chroma", "chroma-key")
            },
        ];

        let caps = set_mix_layer_stack_impl(&app, layers).expect("set 6-layer stack");
        assert_eq!(caps.active_layer_count, 6);
        assert_eq!(caps.max_layers, COMPOSITOR_MAX_LAYERS);

        let stack = app.compositor_stack.lock().unwrap();
        let stored = stack.layers();
        assert_eq!(stored.len(), 6);

        // 0: normal + transform verification
        assert_eq!(stored[0].id, "layer-normal");
        assert_eq!(stored[0].z_index, 0);
        assert!(stored[0].visible);
        assert_eq!(stored[0].blend, BlendMode::Normal);
        assert!((stored[0].transform.offset_xy[0] - 0.25).abs() < 1e-6);
        assert!((stored[0].transform.offset_xy[1] + 0.15).abs() < 1e-6);
        assert!((stored[0].transform.scale_xy[0] - 1.5).abs() < 1e-6);
        assert!((stored[0].transform.scale_xy[1] - 0.75).abs() < 1e-6);
        assert!((stored[0].transform.rotation_rad - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!((stored[0].transform.opacity - 0.8).abs() < 1e-6);

        // 1..3: screen, add, multiply (hidden)
        assert_eq!(stored[1].blend, BlendMode::Screen);
        assert_eq!(stored[2].blend, BlendMode::Additive);
        assert_eq!(stored[3].blend, BlendMode::Multiply);
        assert!(!stored[3].visible);

        // 4: luma-key
        match stored[4].blend {
            BlendMode::LumaKey {
                threshold,
                softness,
            } => {
                assert!((threshold - DEFAULT_LUMA_THRESHOLD).abs() < 1e-6);
                assert!((softness - DEFAULT_LUMA_SOFTNESS).abs() < 1e-6);
            }
            other => panic!("expected LumaKey, got {other:?}"),
        }

        // 5: chroma-key with #0080ff
        match stored[5].blend {
            BlendMode::ChromaKey {
                key_rgb,
                similarity,
                smoothness,
            } => {
                assert!((key_rgb[0] - 0.0).abs() < 1e-5);
                assert!((key_rgb[1] - (128.0 / 255.0)).abs() < 1e-5);
                assert!((key_rgb[2] - 1.0).abs() < 1e-5);
                assert!((similarity - 0.35).abs() < 1e-6);
                assert!((smoothness - 0.12).abs() < 1e-6);
            }
            other => panic!("expected ChromaKey, got {other:?}"),
        }
    }

    #[test]
    fn set_mix_layer_stack_clamps_values_and_enforces_validation() {
        let app = AppState::new();

        // Seed an existing pixel buffer to verify preservation across updates.
        {
            let mut stack = app.compositor_stack.lock().unwrap();
            stack.push_layer(CompositeLayer {
                id: "clamped".to_string(),
                visible: true,
                z_index: 0,
                blend: BlendMode::Normal,
                transform: LayerTransform::default(),
                width: 2,
                height: 2,
                rgba_pixels: vec![42_u8; 16],
            });
        }

        let clamped_layers = vec![
            MixLayerSpecDto {
                opacity: 2.5,
                scale_x: 0.0,
                scale_y: 12.0,
                x: f32::NAN,
                y: f32::INFINITY,
                rotation_deg: f32::NEG_INFINITY,
                ..sample_layer_dto("clamped", "normal")
            },
            MixLayerSpecDto {
                opacity: -0.5,
                scale_x: f32::NAN,
                scale_y: -3.0,
                chroma_key_color: Some("#0f0".to_string()),
                chroma_similarity: Some(3.0),
                chroma_smoothness: Some(-0.2),
                ..sample_layer_dto("neg", "chroma-key")
            },
        ];

        let caps = set_mix_layer_stack_impl(&app, clamped_layers).expect("clamped stack");
        assert_eq!(caps.active_layer_count, 2);

        {
            let stack = app.compositor_stack.lock().unwrap();
            let layers = stack.layers();
            assert!((layers[0].transform.opacity - 1.0).abs() < 1e-6);
            assert!((layers[0].transform.scale_xy[0] - MIN_LAYER_SCALE).abs() < 1e-6);
            assert!((layers[0].transform.scale_xy[1] - MAX_LAYER_SCALE).abs() < 1e-6);
            assert!((layers[0].transform.offset_xy[0] - 0.0).abs() < 1e-6);
            assert!((layers[0].transform.offset_xy[1] - 0.0).abs() < 1e-6);
            assert!((layers[0].transform.rotation_rad - 0.0).abs() < 1e-6);
            // Existing pixel buffer preserved.
            assert_eq!(layers[0].width, 2);
            assert_eq!(layers[0].height, 2);
            assert_eq!(layers[0].rgba_pixels, vec![42_u8; 16]);

            assert!((layers[1].transform.opacity - 0.0).abs() < 1e-6);
            assert!((layers[1].transform.scale_xy[0] - 1.0).abs() < 1e-6);
            assert!((layers[1].transform.scale_xy[1] - MIN_LAYER_SCALE).abs() < 1e-6);
            assert_eq!(
                layers[1].blend,
                BlendMode::ChromaKey {
                    key_rgb: [0.0, 1.0, 0.0],
                    similarity: 1.0,
                    smoothness: 0.0,
                }
            );
        }

        // Exact 16-layer cap is accepted.
        let max_stack: Vec<MixLayerSpecDto> = (0..COMPOSITOR_MAX_LAYERS)
            .map(|i| sample_layer_dto(&format!("l-{i}"), "normal"))
            .collect();
        let max_caps = set_mix_layer_stack_impl(&app, max_stack).expect("16 layers accepted");
        assert_eq!(max_caps.active_layer_count, COMPOSITOR_MAX_LAYERS as usize);

        // 17 layers exceeds cap and is rejected without mutating existing stack.
        let over_cap: Vec<MixLayerSpecDto> = (0..=COMPOSITOR_MAX_LAYERS)
            .map(|i| sample_layer_dto(&format!("l-{i}"), "normal"))
            .collect();
        let err = set_mix_layer_stack_impl(&app, over_cap).unwrap_err();
        assert!(err.contains("exceeds maximum"));
        assert_eq!(
            get_compositor_capabilities_impl(&app).active_layer_count,
            COMPOSITOR_MAX_LAYERS as usize
        );

        // Unknown blend mode, empty id, and malformed chroma hex are rejected.
        assert!(
            set_mix_layer_stack_impl(&app, vec![sample_layer_dto("bad", "bogus-mode")]).is_err()
        );
        assert!(set_mix_layer_stack_impl(&app, vec![sample_layer_dto("  ", "normal")]).is_err());
        let bad_hex = MixLayerSpecDto {
            chroma_key_color: Some("not-a-hex".to_string()),
            ..sample_layer_dto("bad-hex", "chroma-key")
        };
        assert!(set_mix_layer_stack_impl(&app, vec![bad_hex]).is_err());
    }

    #[test]
    fn set_mix_layer_stack_parses_and_round_trips_all_ten_blend_modes() {
        let app = AppState::new();

        let layers = vec![
            sample_layer_dto("l-normal", "normal"),
            sample_layer_dto("l-add", "additive"),
            sample_layer_dto("l-screen", "screen"),
            sample_layer_dto("l-multiply", "multiply"),
            sample_layer_dto("l-luma", "luma-key"),
            sample_layer_dto("l-chroma", "chroma-key"),
            sample_layer_dto("l-overlay", "overlay"),
            sample_layer_dto("l-hard-light", "hard-light"),
            sample_layer_dto("l-difference", "difference"),
            sample_layer_dto("l-exclusion", "exclusion"),
            sample_layer_dto("l-hardlight-alias", "hardlight"),
            sample_layer_dto("l-hard-light-snake", "hard_light"),
        ];

        let caps = set_mix_layer_stack_impl(&app, layers).expect("all 10 modes + aliases parse");
        assert_eq!(caps.active_layer_count, 12);
        assert_eq!(caps.blend_modes.len(), 10);

        let stack = app.compositor_stack.lock().unwrap();
        let stored = stack.layers();
        let expected_labels = [
            "Normal",
            "Additive",
            "Screen",
            "Multiply",
            "LumaKey",
            "ChromaKey",
            "Overlay",
            "HardLight",
            "Difference",
            "Exclusion",
        ];
        for (idx, expected_label) in expected_labels.iter().enumerate() {
            assert_eq!(blend_mode_label(stored[idx].blend), *expected_label);
            assert_eq!(caps.blend_modes[idx], *expected_label);
        }
        assert_eq!(stored[6].blend, BlendMode::Overlay);
        assert_eq!(stored[7].blend, BlendMode::HardLight);
        assert_eq!(stored[8].blend, BlendMode::Difference);
        assert_eq!(stored[9].blend, BlendMode::Exclusion);
        assert_eq!(stored[10].blend, BlendMode::HardLight);
        assert_eq!(stored[11].blend, BlendMode::HardLight);
    }
}

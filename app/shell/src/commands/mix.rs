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
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
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

/// Maximum bytes fetched per upstream YouTube Range request (4 MiB).
/// YouTube DASH servers reject open-ended `Range: bytes=0-` or large GETs with
/// `403 Forbidden`, while 4 MiB streamed chunks (~10s of 720p video) keep
/// macOS WKWebView/AVFoundation smoothly buffered without mid-playback pauses.
const YOUTUBE_PROXY_MAX_CHUNK_BYTES: u64 = 4 * 1024 * 1024;

/// Maximum concurrent local proxy connections to prevent thread/process exhaustion.
const YOUTUBE_PROXY_MAX_CONCURRENT_REQUESTS: usize = 8;

/// Maximum cached stream entries in the local proxy map.
const YOUTUBE_PROXY_MAX_STREAMS: usize = 32;

/// Maximum total HTTP request header bytes accepted by the local proxy (8 KiB).
const YOUTUBE_PROXY_MAX_HEADER_BYTES: u64 = 8 * 1024;

/// Maximum HTTP request header lines accepted by the local proxy.
const YOUTUBE_PROXY_MAX_HEADER_LINES: usize = 64;

/// Socket read/write timeouts for the local YouTube proxy.
const YOUTUBE_PROXY_READ_TIMEOUT: Duration = Duration::from_secs(10);
const YOUTUBE_PROXY_WRITE_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
struct ProxiedYouTubeStream {
    upstream_url: String,
    total_len: u64,
    inserted_at: Instant,
}

struct YouTubeProxyServer {
    port: u16,
    session_token: String,
    streams: Arc<Mutex<HashMap<String, ProxiedYouTubeStream>>>,
    shutdown: Arc<AtomicBool>,
}

static YOUTUBE_PROXY: OnceLock<Result<YouTubeProxyServer, String>> = OnceLock::new();

/// Shut down the local YouTube media proxy accept loop cleanly on app exit.
pub fn shutdown_youtube_proxy() {
    if let Some(Ok(server)) = YOUTUBE_PROXY.get() {
        server.shutdown.store(true, Ordering::Release);
        // Nudge the blocking accept() with a bounded timeout so exit can never hang.
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], server.port));
        let _ = TcpStream::connect_timeout(&addr, Duration::from_millis(200));
    }
}

/// Generate a per-session unpredictable hex token using OS-seeded `RandomState` + time + PID.
fn generate_proxy_session_token() -> String {
    let mut h1 = RandomState::new().build_hasher();
    let mut h2 = RandomState::new().build_hasher();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    h1.write_u128(nanos);
    h1.write_u32(std::process::id());
    h2.write_u64(h1.finish());
    h2.write_u128(nanos.rotate_left(37));
    format!("{:016x}{:016x}", h1.finish(), h2.finish())
}

/// Validate that an upstream YouTube media URL uses `https://` and targets an
/// allowlisted YouTube/GoogleVideo media host (prevents SSRF / backslash authority
/// confusion / localhost redirects).
#[must_use]
pub fn is_allowed_youtube_media_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    if rest.bytes().any(|b| b < 0x21 || b == 0x7f) {
        return false;
    }
    // Split on `\` as well as `/`, `?`, `#` so WHATWG backslash-delimiter
    // authority smuggling (`https://evil.example\.googlevideo.com`) is rejected.
    let authority = rest.split(['/', '\\', '?', '#']).next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    let (host_raw, port_opt) = match authority.split_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (authority, None),
    };
    if let Some(port_str) = port_opt {
        if port_str.is_empty()
            || !port_str.bytes().all(|b| b.is_ascii_digit())
            || !port_str.parse::<u16>().is_ok_and(|p| p > 0)
        {
            return false;
        }
    }
    if host_raw.is_empty()
        || host_raw.starts_with('.')
        || host_raw.ends_with('.')
        || host_raw.contains("..")
        || !host_raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
    {
        return false;
    }
    let host = host_raw.to_ascii_lowercase();
    host == "googlevideo.com"
        || host.ends_with(".googlevideo.com")
        || host == "youtube.com"
        || host.ends_with(".youtube.com")
}

/// Check whether an HTTP `Origin` header belongs to Chromaticity's local webview/dev server.
#[must_use]
pub fn is_allowed_proxy_origin(origin: &str) -> bool {
    let trimmed = origin.trim();
    if trimmed.is_empty() || trimmed.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return false;
    }
    if matches!(
        trimmed,
        "tauri://localhost"
            | "https://tauri.localhost"
            | "http://tauri.localhost"
            | "http://localhost"
            | "http://127.0.0.1"
    ) {
        return true;
    }
    for prefix in ["http://localhost:", "http://127.0.0.1:"] {
        if let Some(port_str) = trimmed.strip_prefix(prefix) {
            return !port_str.is_empty()
                && port_str.bytes().all(|b| b.is_ascii_digit())
                && port_str.parse::<u16>().is_ok_and(|p| p > 0);
        }
    }
    false
}

fn cors_header_for_origin(origin: Option<&str>) -> String {
    match origin {
        Some(o) if is_allowed_proxy_origin(o) => {
            format!(
                "Access-Control-Allow-Origin: {}\r\nVary: Origin\r\n",
                o.trim()
            )
        }
        _ => String::new(),
    }
}

/// Resolve `curl` across macOS, Linux, and Windows.
#[must_use]
fn resolve_curl_binary() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "curl.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        if std::path::Path::new("/usr/bin/curl").is_file() {
            "/usr/bin/curl"
        } else {
            "curl"
        }
    }
}

#[must_use]
fn null_device_path() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "NUL"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "/dev/null"
    }
}

/// Extract the `clen=<bytes>` query parameter from a `googlevideo.com` URL.
#[must_use]
pub fn parse_clen_from_url(url: &str) -> Option<u64> {
    let query = url.split_once('?')?.1;
    for pair in query.split('&') {
        if let Some(val) = pair.strip_prefix("clen=") {
            if let Ok(parsed) = val.parse::<u64>() {
                if parsed > 0 {
                    return Some(parsed);
                }
            }
        }
    }
    None
}

/// Parse an HTTP `Range: bytes=<start>-<end>` header into `(start, Option<end>)`.
#[must_use]
pub fn parse_range_header(header_value: &str) -> Option<(u64, Option<u64>)> {
    let spec = header_value.trim().strip_prefix("bytes=")?;
    let first_range = spec.split(',').next()?.trim();
    let (start_str, end_str) = first_range.split_once('-')?;
    let start = start_str.trim().parse::<u64>().ok()?;
    let end = if end_str.trim().is_empty() {
        None
    } else {
        Some(end_str.trim().parse::<u64>().ok()?)
    };
    Some((start, end))
}

/// Maximum allowlisted `3xx` CDN redirect hops followed for `googlevideo.com`.
const YOUTUBE_PROXY_MAX_REDIRECTS: usize = 3;

/// Parse an HTTP status code from a status line like `HTTP/1.1 206 Partial Content` or `HTTP/2 302`.
fn parse_http_status_code(status_line: &str) -> Option<u16> {
    let mut parts = status_line.split_whitespace();
    let proto = parts.next()?;
    if !proto.starts_with("HTTP/") {
        return None;
    }
    parts.next()?.parse::<u16>().ok()
}

fn probe_upstream_content_length(url: &str) -> Result<u64, String> {
    if !is_allowed_youtube_media_url(url) {
        return Err("upstream YouTube media URL host is not allowlisted".to_string());
    }
    if let Some(clen) = parse_clen_from_url(url) {
        return Ok(clen);
    }
    let mut current_url = url.to_string();
    for _ in 0..=YOUTUBE_PROXY_MAX_REDIRECTS {
        let output = Command::new(resolve_curl_binary())
            .args([
                "-s",
                "-g",
                "--max-redirs",
                "0",
                "--proto",
                "=https",
                "--max-time",
                "15",
                "-D",
                "-",
                "-o",
                null_device_path(),
                "-r",
                "0-0",
                "--",
                &current_url,
            ])
            .output()
            .map_err(|err| format!("failed to probe stream length via curl: {err}"))?;
        let headers = String::from_utf8_lossy(&output.stdout);
        let status = headers.lines().next().and_then(parse_http_status_code);
        if matches!(status, Some(301 | 302 | 303 | 307 | 308)) {
            let location = headers
                .lines()
                .find_map(|line| {
                    let (k, v) = line.split_once(':')?;
                    if k.trim().eq_ignore_ascii_case("location") {
                        Some(v.trim().to_string())
                    } else {
                        None
                    }
                })
                .ok_or_else(|| "upstream redirect missing Location header".to_string())?;
            if !is_allowed_youtube_media_url(&location) {
                return Err("upstream redirect target host is not allowlisted".to_string());
            }
            current_url = location;
            continue;
        }
        for line in headers.lines().rev() {
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("content-range:") {
                if let Some((_, total_str)) = line.rsplit_once('/') {
                    if let Ok(total) = total_str.trim().parse::<u64>() {
                        if total > 0 {
                            return Ok(total);
                        }
                    }
                }
            }
        }
        break;
    }
    Err("unable to determine YouTube stream content length".to_string())
}

struct ActiveConnGuard(Arc<AtomicUsize>);

impl Drop for ActiveConnGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn handle_youtube_proxy_connection(
    mut stream: TcpStream,
    streams: &Arc<Mutex<HashMap<String, ProxiedYouTubeStream>>>,
    session_token: &str,
) {
    let _ = stream.set_read_timeout(Some(YOUTUBE_PROXY_READ_TIMEOUT));
    let _ = stream.set_write_timeout(Some(YOUTUBE_PROXY_WRITE_TIMEOUT));

    let cloned = match stream.try_clone() {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut reader = BufReader::new(cloned.take(YOUTUBE_PROXY_MAX_HEADER_BYTES));
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
        return;
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    let mut range_header: Option<String> = None;
    let mut origin_header: Option<String> = None;
    let mut header_lines = 0usize;
    let mut headers_terminated = false;
    let header_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if header_lines >= YOUTUBE_PROXY_MAX_HEADER_LINES || Instant::now() >= header_deadline {
            break;
        }
        header_lines += 1;
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    headers_terminated = true;
                    break;
                }
                if let Some((name, val)) = trimmed.split_once(':') {
                    let key = name.trim();
                    if key.eq_ignore_ascii_case("range") {
                        range_header = Some(val.trim().to_string());
                    } else if key.eq_ignore_ascii_case("origin") {
                        origin_header = Some(val.trim().to_string());
                    }
                }
            }
        }
    }

    if !headers_terminated {
        let _ = stream.write_all(
            b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        return;
    }

    let cors_hdr = cors_header_for_origin(origin_header.as_deref());

    if method.eq_ignore_ascii_case("OPTIONS") {
        let resp = format!(
            "HTTP/1.1 204 No Content\r\n{cors_hdr}Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\nAccess-Control-Allow-Headers: Range\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    if !method.eq_ignore_ascii_case("GET") && !method.eq_ignore_ascii_case("HEAD") {
        let resp = format!(
            "HTTP/1.1 405 Method Not Allowed\r\n{cors_hdr}Allow: GET, HEAD, OPTIONS\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    // Require per-session bearer token in `/video/<token>/<video_id>.mp4`.
    let clean_path = path.split('?').next().unwrap_or("");
    let expected_prefix = format!("/video/{session_token}/");
    let Some(video_id) = clean_path
        .strip_prefix(&expected_prefix)
        .and_then(|rest| rest.strip_suffix(".mp4"))
    else {
        let resp = format!(
            "HTTP/1.1 403 Forbidden\r\n{cors_hdr}Content-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    };

    let entry = {
        let map = match streams.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        map.get(video_id).cloned()
    };

    let Some(entry) = entry else {
        let resp = format!(
            "HTTP/1.1 404 Not Found\r\n{cors_hdr}Content-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    };

    if !is_allowed_youtube_media_url(&entry.upstream_url) {
        let resp = format!(
            "HTTP/1.1 502 Bad Gateway\r\n{cors_hdr}Content-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    if method.eq_ignore_ascii_case("HEAD") {
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\n{cors_hdr}Connection: close\r\n\r\n",
            entry.total_len
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    let (req_start, req_end) = range_header
        .as_deref()
        .and_then(parse_range_header)
        .unwrap_or((0, None));

    if req_start >= entry.total_len || matches!(req_end, Some(end) if end < req_start) {
        let resp = format!(
            "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\n{cors_hdr}Content-Length: 0\r\nConnection: close\r\n\r\n",
            entry.total_len
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    let max_end = entry.total_len.saturating_sub(1);
    let clamped_chunk_end = req_start
        .saturating_add(YOUTUBE_PROXY_MAX_CHUNK_BYTES.saturating_sub(1))
        .min(max_end);
    let fetch_end = match req_end {
        Some(end) => end.min(clamped_chunk_end),
        None => clamped_chunk_end,
    };

    let range_arg = format!("{req_start}-{fetch_end}");
    let expected_len = fetch_end.saturating_sub(req_start).saturating_add(1);
    let mut current_url = entry.upstream_url.clone();

    for _ in 0..=YOUTUBE_PROXY_MAX_REDIRECTS {
        let mut child = match Command::new(resolve_curl_binary())
            .args([
                "-s",
                "-g",
                "-i",
                "--max-redirs",
                "0",
                "--proto",
                "=https",
                "--max-time",
                "25",
                "-r",
                &range_arg,
                "--",
                &current_url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => {
                let resp = format!(
                    "HTTP/1.1 502 Bad Gateway\r\n{cors_hdr}Content-Length: 0\r\nConnection: close\r\n\r\n"
                );
                let _ = stream.write_all(resp.as_bytes());
                return;
            }
        };

        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            let resp = format!(
                "HTTP/1.1 502 Bad Gateway\r\n{cors_hdr}Content-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(resp.as_bytes());
            return;
        };

        let mut upstream = BufReader::new(stdout);
        let mut status_line = String::new();
        if upstream.read_line(&mut status_line).unwrap_or(0) == 0 {
            let _ = child.kill();
            let _ = child.wait();
            let resp = format!(
                "HTTP/1.1 502 Bad Gateway\r\n{cors_hdr}Content-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(resp.as_bytes());
            return;
        }
        let status_code = parse_http_status_code(&status_line);
        let mut location: Option<String> = None;
        let mut hdr_count = 0usize;
        loop {
            if hdr_count >= YOUTUBE_PROXY_MAX_HEADER_LINES {
                break;
            }
            hdr_count += 1;
            let mut hline = String::new();
            match upstream.read_line(&mut hline) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let trimmed = hline.trim();
                    if trimmed.is_empty() {
                        break;
                    }
                    if let Some((k, v)) = trimmed.split_once(':') {
                        if k.trim().eq_ignore_ascii_case("location") {
                            location = Some(v.trim().to_string());
                        }
                    }
                }
            }
        }

        // Follow allowlisted CDN edge redirects (up to 3 hops) without opening SSRF.
        if matches!(status_code, Some(301 | 302 | 303 | 307 | 308)) {
            let _ = child.kill();
            let _ = child.wait();
            if let Some(loc) = location.filter(|u| is_allowed_youtube_media_url(u)) {
                current_url = loc;
                continue;
            }
            let resp = format!(
                "HTTP/1.1 502 Bad Gateway\r\n{cors_hdr}Content-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(resp.as_bytes());
            return;
        }

        if !matches!(status_code, Some(200 | 206)) {
            let _ = child.kill();
            let _ = child.wait();
            let resp = format!(
                "HTTP/1.1 502 Bad Gateway\r\n{cors_hdr}Content-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(resp.as_bytes());
            return;
        }

        let mut first_buf = vec![0u8; 16 * 1024];
        let first_n = match upstream.read(&mut first_buf) {
            Ok(n) if n > 0 => n,
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                let resp = format!(
                    "HTTP/1.1 502 Bad Gateway\r\n{cors_hdr}Content-Length: 0\r\nConnection: close\r\n\r\n"
                );
                let _ = stream.write_all(resp.as_bytes());
                return;
            }
        };

        let header = format!(
            "HTTP/1.1 206 Partial Content\r\nContent-Type: video/mp4\r\nAccept-Ranges: bytes\r\nContent-Range: bytes {req_start}-{fetch_end}/{}\r\nContent-Length: {expected_len}\r\n{cors_hdr}Cache-Control: no-store\r\nConnection: close\r\n\r\n",
            entry.total_len,
        );
        let first_write = first_n.min(expected_len as usize);
        let remaining = expected_len.saturating_sub(first_write as u64);
        if stream.write_all(header.as_bytes()).is_ok()
            && stream.write_all(&first_buf[..first_write]).is_ok()
            && remaining > 0
        {
            let _ = std::io::copy(&mut upstream.take(remaining), &mut stream);
        }
        let _ = child.kill();
        let _ = child.wait();
        return;
    }

    let resp = format!(
        "HTTP/1.1 502 Bad Gateway\r\n{cors_hdr}Content-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let _ = stream.write_all(resp.as_bytes());
}

fn ensure_youtube_proxy() -> Result<&'static YouTubeProxyServer, String> {
    let init = YOUTUBE_PROXY.get_or_init(|| {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|err| format!("failed to bind YouTube media proxy: {err}"))?;
        let port = listener
            .local_addr()
            .map_err(|err| format!("failed to read YouTube media proxy port: {err}"))?
            .port();
        let session_token = generate_proxy_session_token();
        let streams: Arc<Mutex<HashMap<String, ProxiedYouTubeStream>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let active_conns = Arc::new(AtomicUsize::new(0));

        let streams_bg = Arc::clone(&streams);
        let shutdown_bg = Arc::clone(&shutdown);
        let token_bg = session_token.clone();
        std::thread::Builder::new()
            .name("chromaticity-yt-proxy".to_string())
            .spawn(move || {
                for mut incoming in listener.incoming().flatten() {
                    if shutdown_bg.load(Ordering::Acquire) {
                        break;
                    }
                    let prev = active_conns.fetch_add(1, Ordering::AcqRel);
                    if prev >= YOUTUBE_PROXY_MAX_CONCURRENT_REQUESTS {
                        active_conns.fetch_sub(1, Ordering::AcqRel);
                        let _ = incoming.set_write_timeout(Some(Duration::from_millis(200)));
                        let _ = incoming.write_all(
                            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        );
                        continue;
                    }
                    let guard = ActiveConnGuard(Arc::clone(&active_conns));
                    let streams_conn = Arc::clone(&streams_bg);
                    let token_conn = token_bg.clone();
                    let _ = std::thread::Builder::new()
                        .name("chromaticity-yt-proxy-req".to_string())
                        .spawn(move || {
                            let _guard = guard;
                            handle_youtube_proxy_connection(incoming, &streams_conn, &token_conn);
                        });
                }
            })
            .map_err(|err| format!("failed to spawn YouTube media proxy thread: {err}"))?;
        eprintln!("[chromaticity][youtube] local media proxy listening on 127.0.0.1:{port}");
        Ok(YouTubeProxyServer {
            port,
            session_token,
            streams,
            shutdown,
        })
    });
    match init {
        Ok(server) => Ok(server),
        Err(msg) => Err(msg.clone()),
    }
}

/// Resolve a direct playable HTTPS video stream URL for a validated YouTube
/// video ID using `yt-dlp` and expose it through Chromaticity's localhost
/// Range-clamping proxy (`127.0.0.1`) so WKWebView/AVFoundation never hits
/// `403 Forbidden` on open-ended Range requests and canvas capture stays CORS-clean.
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
            "--socket-timeout",
            "15",
            "-f",
            "bestvideo[ext=mp4][vcodec^=avc1][height<=720]/bestvideo[ext=mp4][vcodec^=avc1][height<=1080]/best[ext=mp4][vcodec^=avc1][height<=720]/best[ext=mp4][height<=720]/best",
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
    let mut resolved = parse_yt_dlp_output(trimmed, &stdout)?;
    if !is_allowed_youtube_media_url(&resolved.stream_url) {
        return Err("resolved YouTube media URL host is not allowlisted".to_string());
    }
    let is_hls = resolved.stream_url.contains(".m3u8")
        || resolved.stream_url.contains("manifest.googlevideo.com");
    if is_hls {
        eprintln!(
            "[chromaticity][youtube] resolved HLS livestream {trimmed}: {}",
            resolved.title
        );
        return Ok(resolved);
    }
    let total_len = probe_upstream_content_length(&resolved.stream_url)?;
    let proxy = ensure_youtube_proxy()?;
    {
        let mut map = match proxy.streams.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if map.len() >= YOUTUBE_PROXY_MAX_STREAMS && !map.contains_key(trimmed) {
            if let Some(oldest_key) = map
                .iter()
                .min_by_key(|(_, v)| v.inserted_at)
                .map(|(k, _)| k.clone())
            {
                map.remove(&oldest_key);
            }
        }
        map.insert(
            trimmed.to_string(),
            ProxiedYouTubeStream {
                upstream_url: resolved.stream_url.clone(),
                total_len,
                inserted_at: Instant::now(),
            },
        );
    }
    resolved.stream_url = format!(
        "http://127.0.0.1:{}/video/{}/{trimmed}.mp4",
        proxy.port, proxy.session_token
    );
    eprintln!(
        "[chromaticity][youtube] resolved {trimmed}: {} ({total_len} bytes via 127.0.0.1:{}/video/[REDACTED]/{trimmed}.mp4)",
        resolved.title, proxy.port
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

    #[test]
    fn youtube_proxy_security_parsers_and_loopback_http_invariants() {
        assert_eq!(
            parse_clen_from_url(
                "https://rr1.googlevideo.com/videoplayback?itag=136&clen=424242&mime=video/mp4"
            ),
            Some(424_242)
        );
        assert_eq!(
            parse_clen_from_url("https://rr1.googlevideo.com/videoplayback?clen=0"),
            None
        );
        assert_eq!(parse_range_header("bytes=0-1023"), Some((0, Some(1023))));
        assert_eq!(parse_range_header("bytes=4096-"), Some((4096, None)));
        assert_eq!(parse_range_header("items=0-5"), None);

        // SSRF & origin allowlists (including backslash authority & spoofed-port attacks)
        assert!(is_allowed_youtube_media_url(
            "https://rr1---sn-abc.googlevideo.com/videoplayback"
        ));
        assert!(is_allowed_youtube_media_url(
            "https://manifest.googlevideo.com/api/manifest/hls"
        ));
        assert!(!is_allowed_youtube_media_url(
            "http://rr1.googlevideo.com/videoplayback"
        ));
        assert!(!is_allowed_youtube_media_url("https://127.0.0.1/secret"));
        assert!(!is_allowed_youtube_media_url(
            "https://evil.com@googlevideo.com/x"
        ));
        assert!(!is_allowed_youtube_media_url(
            "https://attacker.example\\.googlevideo.com/x"
        ));
        assert!(is_allowed_proxy_origin("tauri://localhost"));
        assert!(is_allowed_proxy_origin("http://localhost:1420"));
        assert!(!is_allowed_proxy_origin("https://evil.example.com"));
        assert!(!is_allowed_proxy_origin("http://localhost:80.evil.com"));
        assert!(!is_allowed_proxy_origin("http://127.0.0.1:evil.com"));

        // Loopback TCP protocol invariants (403 bad token, 405 POST, 404 missing, 200 HEAD, 416 OOB)
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let addr = listener.local_addr().expect("local addr");
        let streams: Arc<Mutex<HashMap<String, ProxiedYouTubeStream>>> =
            Arc::new(Mutex::new(HashMap::from([(
                "dQw4w9WgXcQ".to_string(),
                ProxiedYouTubeStream {
                    upstream_url: "https://rr1.googlevideo.com/videoplayback?clen=1024".to_string(),
                    total_len: 1024,
                    inserted_at: Instant::now(),
                },
            )])));
        let token = "testtoken1234";

        let send_req = |req: &str| -> String {
            let streams_bg = Arc::clone(&streams);
            let listener_clone = listener.try_clone().expect("clone listener");
            let t = token.to_string();
            let h = std::thread::spawn(move || {
                let (conn, _) = listener_clone.accept().expect("accept");
                handle_youtube_proxy_connection(conn, &streams_bg, &t);
            });
            let mut client = TcpStream::connect(addr).expect("connect");
            client.write_all(req.as_bytes()).expect("write req");
            let mut resp = String::new();
            client.read_to_string(&mut resp).expect("read resp");
            let _ = h.join();
            resp
        };

        let r403 =
            send_req("GET /video/wrongtoken/dQw4w9WgXcQ.mp4 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        assert!(r403.starts_with("HTTP/1.1 403 Forbidden"));

        let r405 = send_req(
            "POST /video/testtoken1234/dQw4w9WgXcQ.mp4 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        );
        assert!(r405.starts_with("HTTP/1.1 405 Method Not Allowed"));

        let r404 = send_req(
            "GET /video/testtoken1234/unknown1234.mp4 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        );
        assert!(r404.starts_with("HTTP/1.1 404 Not Found"));

        let r200 = send_req(
            "HEAD /video/testtoken1234/dQw4w9WgXcQ.mp4 HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: tauri://localhost\r\n\r\n",
        );
        assert!(r200.starts_with("HTTP/1.1 200 OK"));
        assert!(r200.contains("Content-Length: 1024"));
        assert!(r200.contains("Access-Control-Allow-Origin: tauri://localhost"));

        let r_evil = send_req(
            "HEAD /video/testtoken1234/dQw4w9WgXcQ.mp4 HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: https://evil.example\r\n\r\n",
        );
        assert!(!r_evil.contains("Access-Control-Allow-Origin:"));

        let r416 = send_req(
            "GET /video/testtoken1234/dQw4w9WgXcQ.mp4 HTTP/1.1\r\nHost: 127.0.0.1\r\nRange: bytes=2048-\r\n\r\n",
        );
        assert!(r416.starts_with("HTTP/1.1 416 Range Not Satisfiable"));

        let r416_inv = send_req(
            "GET /video/testtoken1234/dQw4w9WgXcQ.mp4 HTTP/1.1\r\nHost: 127.0.0.1\r\nRange: bytes=500-100\r\n\r\n",
        );
        assert!(r416_inv.starts_with("HTTP/1.1 416 Range Not Satisfiable"));

        let r204 = send_req(
            "OPTIONS /video/testtoken1234/dQw4w9WgXcQ.mp4 HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: http://localhost:1420\r\n\r\n",
        );
        assert!(r204.starts_with("HTTP/1.1 204 No Content"));
        assert!(r204.contains("Access-Control-Allow-Origin: http://localhost:1420"));
    }
}

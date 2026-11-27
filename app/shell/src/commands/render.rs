// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Live GPU preview frame and telemetry Tauri commands.

use crate::render::{
    BpmOverrideMode, BpmOverrideState, DEFAULT_MANUAL_BPM, GpuPreviewEngine, GpuRenderStatus,
    resolve_bpm_uniforms,
};
use crate::state::AppState;
use chromaticity_audio::AudioUniformSnapshot;
use chromaticity_runtime::ControlId;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use tauri::State;

/// Default thumbnail width in pixels.
const THUMBNAIL_DEFAULT_WIDTH: u32 = 320;

/// Default thumbnail height in pixels.
const THUMBNAIL_DEFAULT_HEIGHT: u32 = 180;

/// Minimum thumbnail dimension in pixels.
const THUMBNAIL_MIN_DIM: u32 = 32;

/// Maximum thumbnail width in pixels.
const THUMBNAIL_MAX_WIDTH: u32 = 640;

/// Maximum thumbnail height in pixels.
const THUMBNAIL_MAX_HEIGHT: u32 = 360;

/// Fixed timestamp (seconds) for deterministic scene thumbnail capture.
const THUMBNAIL_ELAPSED_SECS: f32 = 2.5;

/// Fixed frame index for deterministic scene thumbnail capture.
const THUMBNAIL_FRAME_INDEX: u64 = 150;

/// Shared offscreen GPU preview engine for library thumbnail rendering so the
/// Metal/wgpu device is initialized once across thumbnail requests.
static THUMBNAIL_ENGINE: OnceLock<Mutex<GpuPreviewEngine>> = OnceLock::new();

/// Render the currently active `.chroma` scene's `main.glsl` on the GPU at
/// `(width, height)` and return raw RGBA8 pixels as a binary IPC response.
#[tauri::command]
pub fn render_gpu_frame(
    app_state: State<'_, AppState>,
    width: u32,
    height: u32,
) -> Result<tauri::ipc::Response, String> {
    let elapsed_secs = app_state.started_at.elapsed().as_secs_f32();
    let bpm_override = app_state.current_bpm_override();
    let (frame, audio, mut controls) = {
        let state_guard = match app_state.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let frame = state_guard.frame();
        let audio = state_guard
            .latest_audio_snapshot()
            .copied()
            .unwrap_or_else(AudioUniformSnapshot::default);
        let mut controls = [0.5_f32; 16];
        for (i, slot) in controls.iter_mut().enumerate() {
            if let Some(val) = state_guard.control_value(ControlId(i as u32)) {
                *slot = val.0;
            }
        }
        // Pack 2D XY pad controls (legacy index 5 -> slots 5..6, flagship index 11 -> slots 11..12).
        if let Some([x, y]) = state_guard.xy_control_value(ControlId(5)) {
            controls[5] = x;
            controls[6] = y;
        }
        if let Some([x, y]) = state_guard.xy_control_value(ControlId(11)) {
            controls[11] = x;
            controls[12] = y;
        }
        (frame, audio, controls)
    };

    let resolved_bpm = resolve_bpm_uniforms(elapsed_secs, audio.bpm, &bpm_override);
    let mut script_audio = audio;
    script_audio.bpm = resolved_bpm.bpm;

    // Tick the QuickJS `script.js` host if active and fold emitted `ctrl_0..ctrl_15`
    // reactive overrides into this frame's uniform controls.
    {
        let mut script_guard = match app_state.script_host.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let emitted = script_guard.tick(elapsed_secs, 1.0 / 60.0, frame, &script_audio);
        for (i, slot) in controls.iter_mut().enumerate() {
            let key = format!("ctrl_{i}");
            if let Some(&val) = emitted.get(&key) {
                *slot = val;
            }
        }
    }

    let (bytes, last_frame_ms) = {
        let mut gpu = match app_state.gpu_renderer.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let bytes = gpu.render_frame_with_bpm(
            width,
            height,
            elapsed_secs,
            frame,
            &audio,
            controls,
            &bpm_override,
        )?;
        let last_frame_ms = gpu.status().last_frame_ms;
        (bytes, last_frame_ms)
    };
    app_state.record_gpu_frame_cost(width, height, last_frame_ms);
    Ok(tauri::ipc::Response::new(bytes))
}

/// Inner implementation for [`set_bpm_override`], unit-testable without a Tauri runtime.
pub fn set_bpm_override_impl(
    app: &AppState,
    mode: &str,
    bpm: Option<f32>,
    phase_offset_beats: Option<f32>,
    resync_epoch_ms: Option<u64>,
    resync_now: Option<bool>,
) -> Result<BpmOverrideState, String> {
    let prev = app.current_bpm_override();
    let parsed_mode = match mode.trim().to_ascii_lowercase().as_str() {
        "auto" => BpmOverrideMode::Auto,
        "manual" => {
            let fallback = match prev.mode {
                BpmOverrideMode::Manual { bpm: prev_bpm } => prev_bpm,
                _ => DEFAULT_MANUAL_BPM,
            };
            BpmOverrideMode::Manual {
                bpm: bpm.unwrap_or(fallback),
            }
        }
        "off" => BpmOverrideMode::Off,
        other => {
            return Err(format!(
                "unsupported BPM override mode {other:?}; expected \"auto\", \"manual\", or \"off\""
            ));
        }
    };

    let next_phase = phase_offset_beats.unwrap_or(prev.phase_offset_beats);
    let next_resync_epoch_ms = if resync_now.unwrap_or(false) {
        app.started_at
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    } else if let Some(epoch_ms) = resync_epoch_ms {
        epoch_ms
    } else {
        prev.resync_epoch_ms
    };

    let next = BpmOverrideState {
        mode: parsed_mode,
        phase_offset_beats: next_phase,
        resync_epoch_ms: next_resync_epoch_ms,
    };
    Ok(app.store_bpm_override(next))
}

/// Inner implementation for [`get_bpm_override`], unit-testable without a Tauri runtime.
pub fn get_bpm_override_impl(app: &AppState) -> Result<BpmOverrideState, String> {
    Ok(app.current_bpm_override())
}

/// Update the live BPM synchronization mode (`"auto"` | `"manual"` | `"off"`),
/// optional manual tempo, beatmatch phase scrub (`phase_offset_beats`), and
/// beat-0 resync anchor (`resync_epoch_ms` or `resync_now: true`).
#[tauri::command]
pub fn set_bpm_override(
    app_state: State<'_, AppState>,
    mode: String,
    bpm: Option<f32>,
    phase_offset_beats: Option<f32>,
    resync_epoch_ms: Option<u64>,
    resync_now: Option<bool>,
) -> Result<BpmOverrideState, String> {
    set_bpm_override_impl(
        app_state.inner(),
        &mode,
        bpm,
        phase_offset_beats,
        resync_epoch_ms,
        resync_now,
    )
}

/// Return the currently active live BPM override and beatmatch phase-scrub state.
#[tauri::command]
pub fn get_bpm_override(app_state: State<'_, AppState>) -> Result<BpmOverrideState, String> {
    get_bpm_override_impl(app_state.inner())
}

/// Return the current GPU preview engine status (state, active scene id,
/// frames rendered, last frame time in ms, and error if any).
#[tauri::command]
pub fn gpu_render_status(app_state: State<'_, AppState>) -> Result<GpuRenderStatus, String> {
    let gpu = match app_state.gpu_renderer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    Ok(gpu.status())
}

/// Compile and arm an in-memory GLSL 4.50 fragment shader from the Author
/// editor, running an immediate 16×16 `naga` + `wgpu` validation pass so
/// compile diagnostics surface synchronously while live preview updates.
#[tauri::command]
pub fn compile_and_preview_shader(
    app_state: State<'_, AppState>,
    scene_id: String,
    fragment_glsl: String,
) -> Result<GpuRenderStatus, String> {
    let mut gpu = match app_state.gpu_renderer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    gpu.set_inline_shader(scene_id, fragment_glsl);
    let _ = gpu.render_frame(16, 16, 0.0, 0, &AudioUniformSnapshot::default(), [0.5; 16])?;
    Ok(gpu.status())
}

/// Pure GPU thumbnail renderer for a `.chroma` scene directory.
///
/// Reads `<scene_path>/main.glsl`, clamps `(width, height)` to
/// `32..=640` × `32..=360`, and renders a single RGBA8 frame at
/// `elapsed_secs = 2.5` with a representative musical preview audio snapshot.
pub fn render_scene_thumbnail_impl(
    scene_path: &Path,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let shader_path = scene_path.join("main.glsl");
    if !shader_path.is_file() {
        return Err(format!("scene shader not found: {}", shader_path.display()));
    }

    let w = width.clamp(THUMBNAIL_MIN_DIM, THUMBNAIL_MAX_WIDTH);
    let h = height.clamp(THUMBNAIL_MIN_DIM, THUMBNAIL_MAX_HEIGHT);
    let scene_id = scene_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("thumbnail-scene")
        .to_string();

    let preview_audio = AudioUniformSnapshot {
        bpm: 128.0,
        rms: 0.45,
        peak: 0.72,
        fft_bands: [0.6, 0.5, 0.4, 0.35, 0.3, 0.25, 0.2, 0.15],
        ..AudioUniformSnapshot::default()
    };

    let mutex = THUMBNAIL_ENGINE.get_or_init(|| Mutex::new(GpuPreviewEngine::new()));
    let mut engine = match mutex.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    engine.set_scene(scene_id, scene_path);
    engine.render_frame(
        w,
        h,
        THUMBNAIL_ELAPSED_SECS,
        THUMBNAIL_FRAME_INDEX,
        &preview_audio,
        [0.5; 16],
    )
}

/// Render a live GPU RGBA8 thumbnail for a `.chroma` scene package at
/// `(width, height)` (default `320×180`, clamped to `32..=640` × `32..=360`).
#[tauri::command]
pub fn render_scene_thumbnail(
    scene_path: String,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<tauri::ipc::Response, String> {
    let w = width.unwrap_or(THUMBNAIL_DEFAULT_WIDTH);
    let h = height.unwrap_or(THUMBNAIL_DEFAULT_HEIGHT);
    let bytes = render_scene_thumbnail_impl(Path::new(&scene_path), w, h)?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    use super::*;

    #[test]
    fn render_scene_thumbnail_impl_renders_non_black_rgba_frame_from_local_scene() {
        let scenes_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("scenes-local");
        if !scenes_root.is_dir() {
            return;
        }
        let mut candidates: Vec<_> = std::fs::read_dir(&scenes_root)
            .expect("read scenes-local")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.extension().and_then(|s| s.to_str()) == Some("chroma")
                    && p.join("main.glsl").is_file()
            })
            .collect();
        candidates.sort();
        let Some(scene_dir) = candidates.first() else {
            return;
        };

        let bytes = render_scene_thumbnail_impl(scene_dir, 160, 90)
            .expect("thumbnail render should succeed on Metal/wgpu");
        assert_eq!(bytes.len(), 160 * 90 * 4);
        assert!(
            bytes
                .chunks_exact(4)
                .any(|px| px[0] > 0 || px[1] > 0 || px[2] > 0),
            "rendered thumbnail for {} should not be all-black",
            scene_dir.display()
        );
    }

    #[test]
    fn render_scene_thumbnail_impl_rejects_missing_shader() {
        let missing = std::env::temp_dir().join("chromaticity-nonexistent-scene-thumb.chroma");
        let err = render_scene_thumbnail_impl(&missing, 160, 90)
            .expect_err("missing main.glsl should return Err");
        assert!(err.contains("scene shader not found"));
    }

    #[test]
    fn bpm_override_ipc_transitions_auto_manual_off_and_phase_scrub() {
        let app = AppState::new();
        let initial = get_bpm_override_impl(&app).expect("initial bpm override");
        assert_eq!(initial, BpmOverrideState::default());
        assert_eq!(initial.mode, BpmOverrideMode::Auto);

        // Lock manual 130 BPM with +0.125 beat phase scrub and explicit resync epoch.
        let manual =
            set_bpm_override_impl(&app, "manual", Some(130.0), Some(0.125), Some(1500), None)
                .expect("manual bpm override");
        assert_eq!(manual.mode, BpmOverrideMode::Manual { bpm: 130.0 });
        assert_eq!(manual.phase_offset_beats, 0.125);
        assert_eq!(manual.resync_epoch_ms, 1500);
        assert_eq!(
            get_bpm_override_impl(&app).expect("read back manual"),
            manual
        );

        // Clamp out-of-range manual BPM (e.g. 999.0 -> 300.0) and test resync_now.
        let clamped = set_bpm_override_impl(&app, "manual", Some(999.0), None, None, Some(true))
            .expect("clamped manual");
        assert_eq!(clamped.mode, BpmOverrideMode::Manual { bpm: 300.0 });
        assert_eq!(clamped.phase_offset_beats, 0.125);

        // Disable BPM sync.
        let off = set_bpm_override_impl(&app, "off", None, Some(0.0), None, None).expect("off");
        assert_eq!(off.mode, BpmOverrideMode::Off);
        assert_eq!(off.phase_offset_beats, 0.0);

        // Return to Auto with negative phase scrub.
        let auto =
            set_bpm_override_impl(&app, "AUTO", None, Some(-0.25), Some(0), None).expect("auto");
        assert_eq!(auto.mode, BpmOverrideMode::Auto);
        assert_eq!(auto.phase_offset_beats, -0.25);
        assert_eq!(auto.resync_epoch_ms, 0);

        // Reject unknown mode.
        let err = set_bpm_override_impl(&app, "turbo", None, None, None, None)
            .expect_err("unknown mode should fail");
        assert!(err.contains("unsupported BPM override mode"));
    }
}

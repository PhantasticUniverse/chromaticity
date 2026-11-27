// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Live GPU preview frame and telemetry Tauri commands.

use crate::render::GpuRenderStatus;
use crate::state::AppState;
use chromaticity_audio::AudioUniformSnapshot;
use chromaticity_runtime::ControlId;
use tauri::State;

/// Render the currently active `.chroma` scene's `main.glsl` on the GPU at
/// `(width, height)` and return raw RGBA8 pixels as a binary IPC response.
#[tauri::command]
pub fn render_gpu_frame(
    app_state: State<'_, AppState>,
    width: u32,
    height: u32,
) -> Result<tauri::ipc::Response, String> {
    let elapsed_secs = app_state.started_at.elapsed().as_secs_f32();
    let (frame, audio, controls) = {
        let state_guard = match app_state.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let frame = state_guard.frame();
        let audio = state_guard
            .latest_audio_snapshot()
            .copied()
            .unwrap_or_else(AudioUniformSnapshot::default);
        let mut controls = [0.5_f32; 8];
        for (i, slot) in controls.iter_mut().enumerate() {
            if let Some(val) = state_guard.control_value(ControlId(i as u32)) {
                *slot = val.0;
            }
        }
        (frame, audio, controls)
    };

    let mut gpu = match app_state.gpu_renderer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let bytes = gpu.render_frame(width, height, elapsed_secs, frame, &audio, controls)?;
    Ok(tauri::ipc::Response::new(bytes))
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

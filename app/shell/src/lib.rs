// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Chromaticity desktop application shell entrypoint.
// Tauri 2 bootstrap that registers the shell-side `AppState` and the
// Phase-1 D2 IPC handlers (`set_control`, `get_control`) so the UI can
// drive the runtime command bus through typed IPC. ts-rs binding
// generation lives in `tests/export_bindings.rs` (test-target export
// per ts-rs 10's conventional pattern).

pub mod commands;
pub mod state;

use crate::state::AppState;

/// Launch the Tauri application event loop.
///
/// Returns an error if the Tauri runtime fails to start (bad context,
/// capability mismatch, window creation failure, etc.); the binary
/// entry in `src/main.rs` logs the error and exits non-zero.
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::control::set_control,
            commands::control::get_control,
            commands::snap::recall_snap,
            commands::snap::save_snap,
            commands::snap::clear_snap,
            commands::snap::active_snap_slot,
            commands::snap::snap_slot_saved,
            commands::preset::recall_preset,
            commands::preset::active_preset,
            commands::playlist::advance_playlist,
            commands::playlist::playlist_position,
            commands::library::list_scene_packages,
            commands::library::list_bundled_scene_packages,
            commands::library::default_scene_root,
            commands::library::bundled_scene_root,
            commands::osc::osc_health_snapshot,
            commands::picker::pick_folder,
            commands::license::install_license,
            commands::license::remove_license,
            commands::license::request_trial,
        ])
        .run(tauri::generate_context!())
}

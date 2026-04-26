// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Playlist advance IPC handlers — Phase-1 D4.
//!
//! Mirrors the D2 control and D3 Snap round-trip patterns: the mutator
//! ([`advance_playlist`]) dispatches a
//! [`chromaticity_runtime::Command::PlaylistAdvance`] through the bus
//! held in [`crate::state::AppState`], and applies the committed event
//! to canonical state via
//! [`chromaticity_runtime::CommandBus::commit_one_into`]. The reader
//! ([`playlist_position`]) projects the resulting cursor position back
//! across the IPC boundary as `i64`.
//!
//! Errors collapse to `Result<_, String>` so the Tauri serializer
//! carries a human-readable failure across the IPC boundary;
//! [`chromaticity_runtime::CommandBusError`] feeds [`std::fmt::Display`]
//! into the wrapper.

use chromaticity_runtime::{Command, CommandBus};
use tauri::State;

use crate::state::AppState;

/// Advance the active playlist by `step` entries (negative values move
/// backward) and apply the committed event to canonical state
/// synchronously.
#[tauri::command]
pub fn advance_playlist(app_state: State<'_, AppState>, step: i32) -> Result<(), String> {
    app_state
        .bus
        .dispatch(Command::PlaylistAdvance { step })
        .map_err(|err| err.to_string())?;
    let mut state = app_state
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    let _ = app_state.bus.commit_one_into(&mut state);
    Ok(())
}

/// Return the current playlist cursor position. Defaults to 0 before
/// any advance has been applied.
#[tauri::command]
pub fn playlist_position(app_state: State<'_, AppState>) -> Result<i64, String> {
    let state = app_state
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    Ok(state.playlist_position())
}

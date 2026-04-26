// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Preset recall IPC handlers — Phase-1 D4.
//!
//! Mirrors the D2 control and D3 Snap round-trip patterns: the mutator
//! ([`recall_preset`]) wraps the raw `u32` into a
//! [`chromaticity_runtime::PresetId`], dispatches a
//! [`chromaticity_runtime::Command::PresetRecall`] through the bus held
//! in [`crate::state::AppState`], and applies the committed event to
//! canonical state via
//! [`chromaticity_runtime::CommandBus::commit_one_into`]. The reader
//! ([`active_preset`]) projects the resulting state back across the IPC
//! boundary as `Option<u32>`.
//!
//! Errors collapse to `Result<_, String>` so the Tauri serializer
//! carries a human-readable failure across the IPC boundary;
//! [`chromaticity_runtime::CommandBusError`] feeds [`std::fmt::Display`]
//! into the wrapper.

use chromaticity_runtime::{Command, CommandBus, PresetId};
use tauri::State;

use crate::state::AppState;

/// Recall the preset identified by `id` and apply the committed event
/// to canonical state synchronously.
#[tauri::command]
pub fn recall_preset(app_state: State<'_, AppState>, id: u32) -> Result<(), String> {
    app_state
        .bus
        .dispatch(Command::PresetRecall { id: PresetId(id) })
        .map_err(|err| err.to_string())?;
    let mut state = app_state
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    let _ = app_state.bus.commit_one_into(&mut state);
    Ok(())
}

/// Return the active preset id, or `None` when no preset has been
/// recalled.
#[tauri::command]
pub fn active_preset(app_state: State<'_, AppState>) -> Result<Option<u32>, String> {
    let state = app_state
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    Ok(state.active_preset().map(|p| p.0))
}

// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Snap{Recall, Save, Clear} IPC handlers — Phase-1 D3.
//!
//! Mirrors the D2 control round-trip pattern in
//! [`super::control`]: each mutator handler validates the slot
//! through [`chromaticity_runtime::SnapSlotId::new`], dispatches the
//! corresponding [`chromaticity_runtime::Command`] through the bus
//! held in [`crate::state::AppState`], and applies the committed
//! event to canonical state via
//! [`chromaticity_runtime::CommandBus::commit_one_into`]. The reader
//! handlers ([`active_snap_slot`], [`snap_slot_saved`]) project the
//! resulting state back across the IPC boundary.
//!
//! Errors collapse to `Result<_, String>` so the Tauri serializer
//! carries a human-readable failure across the IPC boundary;
//! [`chromaticity_runtime::CommandBusError`] feeds [`std::fmt::Display`]
//! into the wrapper.

use chromaticity_runtime::{Command, CommandBus, SnapSlotId};
use tauri::State;

use crate::state::AppState;

/// Validate `slot` against the engine-side `0..=15` invariant or
/// return a human-readable IPC error.
fn parse_slot(slot: u8) -> Result<SnapSlotId, String> {
    SnapSlotId::new(slot)
        .ok_or_else(|| format!("invalid snap slot {slot}: max is {}", SnapSlotId::MAX))
}

/// Recall the Snap slot identified by `slot` and apply the committed
/// event to canonical state synchronously.
#[tauri::command]
pub fn recall_snap(app_state: State<'_, AppState>, slot: u8) -> Result<(), String> {
    let slot = parse_slot(slot)?;
    app_state
        .bus
        .dispatch(Command::SnapRecall { slot })
        .map_err(|err| err.to_string())?;
    let mut state = app_state
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    let _ = app_state.bus.commit_one_into(&mut state);
    Ok(())
}

/// Save the current state into the Snap slot identified by `slot`.
#[tauri::command]
pub fn save_snap(app_state: State<'_, AppState>, slot: u8) -> Result<(), String> {
    let slot = parse_slot(slot)?;
    app_state
        .bus
        .dispatch(Command::SnapSave { slot })
        .map_err(|err| err.to_string())?;
    let mut state = app_state
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    let _ = app_state.bus.commit_one_into(&mut state);
    Ok(())
}

/// Clear the Snap slot identified by `slot`.
#[tauri::command]
pub fn clear_snap(app_state: State<'_, AppState>, slot: u8) -> Result<(), String> {
    let slot = parse_slot(slot)?;
    app_state
        .bus
        .dispatch(Command::SnapClear { slot })
        .map_err(|err| err.to_string())?;
    let mut state = app_state
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    let _ = app_state.bus.commit_one_into(&mut state);
    Ok(())
}

/// Return the active Snap slot's zero-based index, or `None` when no
/// slot has been recalled or saved.
#[tauri::command]
pub fn active_snap_slot(app_state: State<'_, AppState>) -> Result<Option<u8>, String> {
    let state = app_state
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    Ok(state.active_snap_slot().map(SnapSlotId::get))
}

/// Return whether the Snap slot identified by `slot` currently holds
/// saved state.
#[tauri::command]
pub fn snap_slot_saved(app_state: State<'_, AppState>, slot: u8) -> Result<bool, String> {
    let snap = parse_slot(slot)?;
    let state = app_state
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    Ok(state.snap_slot_saved(snap))
}

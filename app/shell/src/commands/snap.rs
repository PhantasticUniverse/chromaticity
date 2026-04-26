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
//!
//! Each Tauri-facing handler is a thin wrapper over an inner
//! `*_impl(&AppState, ...)` function so the round-trip glue is unit
//! testable without a Tauri runtime.

use chromaticity_runtime::{Command, CommandBus, SnapSlotId};
use tauri::State;

use crate::state::AppState;

/// Validate `slot` against the engine-side `0..=15` invariant or
/// return a human-readable IPC error.
fn parse_slot(slot: u8) -> Result<SnapSlotId, String> {
    SnapSlotId::new(slot)
        .ok_or_else(|| format!("invalid snap slot {slot}: max is {}", SnapSlotId::MAX))
}

fn recall_snap_impl(app: &AppState, slot: u8) -> Result<(), String> {
    let slot = parse_slot(slot)?;
    app.bus
        .dispatch(Command::SnapRecall { slot })
        .map_err(|err| err.to_string())?;
    let mut state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    let _ = app.bus.commit_one_into(&mut state);
    Ok(())
}

fn save_snap_impl(app: &AppState, slot: u8) -> Result<(), String> {
    let slot = parse_slot(slot)?;
    app.bus
        .dispatch(Command::SnapSave { slot })
        .map_err(|err| err.to_string())?;
    let mut state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    let _ = app.bus.commit_one_into(&mut state);
    Ok(())
}

fn clear_snap_impl(app: &AppState, slot: u8) -> Result<(), String> {
    let slot = parse_slot(slot)?;
    app.bus
        .dispatch(Command::SnapClear { slot })
        .map_err(|err| err.to_string())?;
    let mut state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    let _ = app.bus.commit_one_into(&mut state);
    Ok(())
}

fn active_snap_slot_impl(app: &AppState) -> Result<Option<u8>, String> {
    let state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    Ok(state.active_snap_slot().map(SnapSlotId::get))
}

fn snap_slot_saved_impl(app: &AppState, slot: u8) -> Result<bool, String> {
    let snap = parse_slot(slot)?;
    let state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    Ok(state.snap_slot_saved(snap))
}

/// Recall the Snap slot identified by `slot` and apply the committed
/// event to canonical state synchronously.
#[tauri::command]
pub fn recall_snap(app_state: State<'_, AppState>, slot: u8) -> Result<(), String> {
    recall_snap_impl(app_state.inner(), slot)
}

/// Save the current state into the Snap slot identified by `slot`.
#[tauri::command]
pub fn save_snap(app_state: State<'_, AppState>, slot: u8) -> Result<(), String> {
    save_snap_impl(app_state.inner(), slot)
}

/// Clear the Snap slot identified by `slot`.
#[tauri::command]
pub fn clear_snap(app_state: State<'_, AppState>, slot: u8) -> Result<(), String> {
    clear_snap_impl(app_state.inner(), slot)
}

/// Return the active Snap slot's zero-based index, or `None` when no
/// slot has been recalled or saved.
#[tauri::command]
pub fn active_snap_slot(app_state: State<'_, AppState>) -> Result<Option<u8>, String> {
    active_snap_slot_impl(app_state.inner())
}

/// Return whether the Snap slot identified by `slot` currently holds
/// saved state.
#[tauri::command]
pub fn snap_slot_saved(app_state: State<'_, AppState>, slot: u8) -> Result<bool, String> {
    snap_slot_saved_impl(app_state.inner(), slot)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Unit tests for the D3 Snap round-trip handler glue. Pins the
    //! SHELL-side `parse_slot → dispatch → commit_one_into → readback`
    //! chain plus the `0..=15` slot-validation surface.

    use super::{
        active_snap_slot_impl, clear_snap_impl, recall_snap_impl, save_snap_impl,
        snap_slot_saved_impl,
    };
    use crate::state::AppState;

    #[test]
    fn save_snap_then_active_snap_slot_round_trip() {
        let app = AppState::new();
        save_snap_impl(&app, 5).unwrap();
        assert_eq!(active_snap_slot_impl(&app).unwrap(), Some(5));
    }

    #[test]
    fn save_snap_then_snap_slot_saved_returns_true() {
        let app = AppState::new();
        save_snap_impl(&app, 5).unwrap();
        assert!(snap_slot_saved_impl(&app, 5).unwrap());
        assert!(!snap_slot_saved_impl(&app, 7).unwrap());
    }

    #[test]
    fn clear_snap_resets_active_and_saved() {
        let app = AppState::new();
        save_snap_impl(&app, 5).unwrap();
        clear_snap_impl(&app, 5).unwrap();
        assert_eq!(active_snap_slot_impl(&app).unwrap(), None);
        assert!(!snap_slot_saved_impl(&app, 5).unwrap());
    }

    #[test]
    fn recall_snap_unsaved_slot_sets_active_without_saving() {
        // Per `engine/runtime/src/state.rs::apply_event`,
        // `SnapRecalled` flips `active` to `Some(slot)` regardless of
        // whether the slot was previously saved; the slot's `saved`
        // bit is independent. Pin that contract at the shell seam.
        let app = AppState::new();
        recall_snap_impl(&app, 7).unwrap();
        assert_eq!(active_snap_slot_impl(&app).unwrap(), Some(7));
        assert!(!snap_slot_saved_impl(&app, 7).unwrap());
    }

    #[test]
    fn save_snap_invalid_slot_returns_error() {
        let app = AppState::new();
        let err = save_snap_impl(&app, 16).unwrap_err();
        assert!(err.contains("invalid snap slot 16"), "got: {err}");
        // State must remain unchanged on a validation rejection.
        assert_eq!(active_snap_slot_impl(&app).unwrap(), None);
    }

    #[test]
    fn recall_snap_invalid_slot_returns_error() {
        let app = AppState::new();
        let err = recall_snap_impl(&app, 99).unwrap_err();
        assert!(err.contains("invalid snap slot 99"), "got: {err}");
    }

    #[test]
    fn clear_snap_invalid_slot_returns_error() {
        let app = AppState::new();
        let err = clear_snap_impl(&app, 200).unwrap_err();
        assert!(err.contains("invalid snap slot 200"), "got: {err}");
    }
}

// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Control set/get IPC handlers — the project's first end-to-end
//! UI ↔ runtime round-trip.
//!
//! Frontend → [`set_control`] dispatches a
//! [`chromaticity_runtime::Command::ControlSet`] through the bus held
//! in [`crate::state::AppState`], commits the resulting event into
//! canonical state, and returns. A subsequent
//! [`get_control`] reads the post-commit value back through the same
//! `AppState`.
//!
//! Errors collapse to `Result<_, String>` so the Tauri serializer
//! carries a human-readable failure across the IPC boundary; the
//! engine-side error variants ([`chromaticity_runtime::CommandBusError::Backpressure`],
//! [`chromaticity_runtime::CommandBusError::Closed`]) feed
//! [`std::fmt::Display`] into the wrapper.

use chromaticity_runtime::{Command, CommandBus};
use chromaticity_ui_bridge::{ControlId, ControlValue};
use tauri::State;

use crate::state::AppState;

/// Set the control identified by `id` to `value`, applying the
/// committed event to canonical state synchronously.
#[tauri::command]
pub fn set_control(
    app_state: State<'_, AppState>,
    id: ControlId,
    value: ControlValue,
) -> Result<(), String> {
    app_state
        .bus
        .dispatch(Command::ControlSet {
            id: id.into(),
            value: value.into(),
        })
        .map_err(|err| err.to_string())?;
    let mut state = app_state
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    let _ = app_state.bus.commit_one_into(&mut state);
    Ok(())
}

/// Read the current value of the control identified by `id`. Returns
/// `None` when the control has never been set.
#[tauri::command]
pub fn get_control(
    app_state: State<'_, AppState>,
    id: ControlId,
) -> Result<Option<ControlValue>, String> {
    let state = app_state
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    Ok(state.control_value(id.into()).map(Into::into))
}

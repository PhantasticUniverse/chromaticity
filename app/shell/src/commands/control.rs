// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Control set/get IPC handlers — the project's first end-to-end
//! UI ↔ runtime round-trip.
//!
//! Frontend → [`set_control`] dispatches a
//! [`chromaticity_runtime::Command::ControlSet`] through the bus held
//! in [`crate::state::AppState`], drains older queued commands, commits
//! its own resulting event into canonical state, and returns. A subsequent
//! [`get_control`] reads the post-commit value back through the same
//! `AppState`.
//!
//! Errors collapse to `Result<_, String>` so the Tauri serializer
//! carries a human-readable failure across the IPC boundary; the
//! engine-side error variants ([`chromaticity_runtime::CommandBusError::Backpressure`],
//! [`chromaticity_runtime::CommandBusError::Closed`]) feed
//! [`std::fmt::Display`] into the wrapper.
//!
//! Each Tauri-facing handler is a thin wrapper over an inner
//! `*_impl(&AppState, ...)` function so the round-trip glue is unit
//! testable without a Tauri runtime.

use chromaticity_runtime::{Command, CommandBus};
use chromaticity_ui_bridge::{ControlId, ControlValue};
use tauri::State;

use crate::state::AppState;

/// Inner implementation invoked by both the Tauri wrapper and unit
/// tests; takes a borrowed [`AppState`] directly.
fn set_control_impl(app: &AppState, id: ControlId, value: ControlValue) -> Result<(), String> {
    let mut state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    app.bus
        .dispatch_and_commit_into(
            Command::ControlSet {
                id: id.into(),
                value: value.into(),
            },
            &mut state,
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}

/// Inner implementation invoked by both the Tauri wrapper and unit
/// tests; takes a borrowed [`AppState`] directly.
fn get_control_impl(app: &AppState, id: ControlId) -> Result<Option<ControlValue>, String> {
    let state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    Ok(state.control_value(id.into()).map(Into::into))
}

/// Set the control identified by `id` to `value`, applying the
/// committed event to canonical state synchronously.
#[tauri::command]
pub fn set_control(
    app_state: State<'_, AppState>,
    id: ControlId,
    value: ControlValue,
) -> Result<(), String> {
    set_control_impl(app_state.inner(), id, value)
}

/// Read the current value of the control identified by `id`. Returns
/// `None` when the control has never been set.
#[tauri::command]
pub fn get_control(
    app_state: State<'_, AppState>,
    id: ControlId,
) -> Result<Option<ControlValue>, String> {
    get_control_impl(app_state.inner(), id)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Unit tests for the D2 control round-trip handler glue. The
    //! engine-side `dispatch_and_commit_into → apply_event` chain is covered by
    //! `engine/runtime` tests; these tests pin the SHELL-side
    //! `Mutex::lock → dispatch_and_commit_into → readback` chain that
    //! is otherwise only exercised end-to-end through Tauri.

    use super::{get_control_impl, set_control_impl};
    use crate::state::AppState;
    use chromaticity_runtime::{Command, CommandBus};
    use chromaticity_ui_bridge::{ControlId, ControlValue};

    #[test]
    fn set_control_then_get_control_round_trip() {
        let app = AppState::new();
        set_control_impl(&app, ControlId(0), ControlValue(0.5)).unwrap();
        assert_eq!(
            get_control_impl(&app, ControlId(0)).unwrap(),
            Some(ControlValue(0.5)),
        );
    }

    #[test]
    fn get_control_unknown_id_returns_none() {
        let app = AppState::new();
        assert_eq!(get_control_impl(&app, ControlId(99)).unwrap(), None);
    }

    #[test]
    fn set_control_overwrites_previous_value() {
        let app = AppState::new();
        set_control_impl(&app, ControlId(7), ControlValue(0.25)).unwrap();
        set_control_impl(&app, ControlId(7), ControlValue(0.75)).unwrap();
        assert_eq!(
            get_control_impl(&app, ControlId(7)).unwrap(),
            Some(ControlValue(0.75)),
        );
    }

    #[test]
    fn set_control_readback_includes_own_command_after_queued_work() {
        let app = AppState::new();
        app.bus
            .dispatch(Command::ControlSet {
                id: ControlId(7).into(),
                value: ControlValue(0.25).into(),
            })
            .unwrap();

        set_control_impl(&app, ControlId(7), ControlValue(0.75)).unwrap();

        assert_eq!(
            get_control_impl(&app, ControlId(7)).unwrap(),
            Some(ControlValue(0.75)),
        );
    }
}

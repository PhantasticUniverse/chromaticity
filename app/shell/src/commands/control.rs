// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Control set/get IPC handlers — scalar and 2-component (XY) variants.
//!
//! Frontend → [`set_control`] dispatches a
//! [`chromaticity_runtime::Command::ControlSet`] through the bus held
//! in [`crate::state::AppState`], drains older queued commands, commits
//! its own resulting event into canonical state, and returns. A subsequent
//! [`get_control`] reads the post-commit value back through the same
//! `AppState`. The parallel [`set_control_2d`] / [`get_control_2d`] pair
//! mirrors this pattern exactly using
//! [`chromaticity_runtime::Command::ControlSet2D`] and
//! [`chromaticity_runtime::State::xy_control_value`] for XY controls.
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
use chromaticity_ui_bridge::{ControlId, ControlValue, ControlValue2};
use tauri::State;

use crate::state::AppState;

/// Inner implementation invoked by both the Tauri wrapper and unit
/// tests; takes a borrowed [`AppState`] directly.
fn set_control_impl(app: &AppState, id: ControlId, value: ControlValue) -> Result<(), String> {
    let runtime_id: chromaticity_runtime::ControlId = id.into();
    let runtime_val: chromaticity_runtime::ControlValue = value.into();
    let prev_val = {
        let mut state = app
            .state
            .lock()
            .map_err(|err| format!("state mutex poisoned: {err}"))?;
        let prev = state.control_value(runtime_id).map_or(0.0, |v| v.0);
        app.bus
            .dispatch_and_commit_into(
                Command::ControlSet {
                    id: runtime_id,
                    value: runtime_val,
                },
                &mut state,
            )
            .map_err(|err| err.to_string())?;
        prev
    };
    let control_name = app
        .loaded_scene
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .as_ref()
                .and_then(|rec| rec.scene.controls.get(runtime_id.0 as usize))
                .map(|c| c.name.clone())
        })
        .unwrap_or_else(|| format!("ctrl_{}", runtime_id.0));
    if let Ok(mut host) = app.script_host.lock() {
        let _ = host.notify_control_change(&control_name, prev_val, runtime_val.0);
    }
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

/// Inner implementation for 2-component (XY) control dispatch.
///
/// Mirrors [`set_control_impl`] exactly — same locking, same
/// `dispatch_and_commit_into` pattern — but dispatches
/// [`Command::ControlSet2D`] and carries a `[f32; 2]` value pair.
fn set_control_2d_impl(app: &AppState, id: ControlId, value: ControlValue2) -> Result<(), String> {
    let mut state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    app.bus
        .dispatch_and_commit_into(
            Command::ControlSet2D {
                id: id.into(),
                value: value.into(),
            },
            &mut state,
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}

/// Inner implementation for reading a 2-component (XY) control value.
///
/// Mirrors [`get_control_impl`] exactly — same locking, same readback
/// pattern — but reads from [`State::xy_control_value`], which is
/// populated by the [`Command::ControlSet2D`] dispatch path.
fn get_control_2d_impl(app: &AppState, id: ControlId) -> Result<Option<ControlValue2>, String> {
    let state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    Ok(state.xy_control_value(id.into()).map(Into::into))
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

/// Set the 2-component (XY) control identified by `id` to `value`,
/// dispatching [`Command::ControlSet2D`] and committing the resulting
/// event to canonical state synchronously.
#[tauri::command]
pub fn set_control_2d(
    app_state: State<'_, AppState>,
    id: ControlId,
    value: ControlValue2,
) -> Result<(), String> {
    set_control_2d_impl(app_state.inner(), id, value)
}

/// Read the current 2-component (XY) value of the control identified by
/// `id`. Returns `None` when the control has never been set via
/// `set_control_2d`.
#[tauri::command]
pub fn get_control_2d(
    app_state: State<'_, AppState>,
    id: ControlId,
) -> Result<Option<ControlValue2>, String> {
    get_control_2d_impl(app_state.inner(), id)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Unit tests for the D2 control round-trip handler glue. The
    //! engine-side `dispatch_and_commit_into → apply_event` chain is covered by
    //! `engine/runtime` tests; these tests pin the SHELL-side
    //! `Mutex::lock → dispatch_and_commit_into → readback` chain that
    //! is otherwise only exercised end-to-end through Tauri.

    use super::{get_control_2d_impl, get_control_impl, set_control_2d_impl, set_control_impl};
    use crate::state::AppState;
    use chromaticity_runtime::{Command, CommandBus};
    use chromaticity_ui_bridge::{ControlId, ControlValue, ControlValue2};

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

    #[test]
    fn set_control_2d_then_get_control_2d_round_trip() {
        let app = AppState::new();
        set_control_2d_impl(&app, ControlId(0), ControlValue2([0.3, -0.7])).unwrap();
        assert_eq!(
            get_control_2d_impl(&app, ControlId(0)).unwrap(),
            Some(ControlValue2([0.3, -0.7])),
        );
    }

    #[test]
    fn get_control_2d_unknown_id_returns_none() {
        let app = AppState::new();
        assert_eq!(get_control_2d_impl(&app, ControlId(99)).unwrap(), None);
    }

    #[test]
    fn set_control_2d_overwrites_previous_value() {
        let app = AppState::new();
        set_control_2d_impl(&app, ControlId(12), ControlValue2([0.1, 0.2])).unwrap();
        set_control_2d_impl(&app, ControlId(12), ControlValue2([0.5, 0.6])).unwrap();
        assert_eq!(
            get_control_2d_impl(&app, ControlId(12)).unwrap(),
            Some(ControlValue2([0.5, 0.6])),
        );
    }

    #[test]
    fn set_control_2d_readback_includes_own_command_after_queued_work() {
        let app = AppState::new();
        app.bus
            .dispatch(Command::ControlSet2D {
                id: ControlId(12).into(),
                value: [0.1, 0.2],
            })
            .unwrap();

        set_control_2d_impl(&app, ControlId(12), ControlValue2([0.5, 0.6])).unwrap();

        assert_eq!(
            get_control_2d_impl(&app, ControlId(12)).unwrap(),
            Some(ControlValue2([0.5, 0.6])),
        );
    }
}

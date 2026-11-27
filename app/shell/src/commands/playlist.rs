// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Playlist advance IPC handlers — Phase-1 D4.
//!
//! Mirrors the D2 control and D3 Snap round-trip patterns: the mutator
//! ([`advance_playlist`]) dispatches a
//! [`chromaticity_runtime::Command::PlaylistAdvance`] through the bus
//! held in [`crate::state::AppState`], drains older queued commands,
//! and applies its own committed event to canonical state via
//! [`chromaticity_runtime::CommandBus::dispatch_and_commit_into`]. The reader
//! ([`playlist_position`]) projects the resulting cursor position back
//! across the IPC boundary as `i64`.
//!
//! Errors collapse to `Result<_, String>` so the Tauri serializer
//! carries a human-readable failure across the IPC boundary;
//! [`chromaticity_runtime::CommandBusError`] feeds [`std::fmt::Display`]
//! into the wrapper.
//!
//! Each Tauri-facing handler is a thin wrapper over an inner
//! `*_impl(&AppState, ...)` function so the round-trip glue is unit
//! testable without a Tauri runtime.

use chromaticity_runtime::{Command, CommandBus};
use tauri::State;

use crate::state::AppState;

fn advance_playlist_impl(app: &AppState, step: i32) -> Result<(), String> {
    let mut state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    app.bus
        .dispatch_and_commit_into(Command::PlaylistAdvance { step }, &mut state)
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn playlist_position_impl(app: &AppState) -> Result<i64, String> {
    let state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    Ok(state.playlist_position())
}

/// Advance the active playlist by `step` entries (negative values move
/// backward) and apply the committed event to canonical state
/// synchronously.
#[tauri::command]
pub fn advance_playlist(app_state: State<'_, AppState>, step: i32) -> Result<(), String> {
    advance_playlist_impl(app_state.inner(), step)
}

/// Return the current playlist cursor position. Defaults to 0 before
/// any advance has been applied.
#[tauri::command]
pub fn playlist_position(app_state: State<'_, AppState>) -> Result<i64, String> {
    playlist_position_impl(app_state.inner())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Unit tests for the D4 playlist round-trip handler glue. Pins the
    //! SHELL-side `dispatch_and_commit_into → readback` chain.
    //!
    //! The saturating-add overflow guard is pinned at the runtime
    //! layer in `engine/runtime/src/state.rs::tests::playlist_advance_saturates_at_i64_max`
    //! and cannot be re-pinned here without reaching past
    //! `pub(crate)` runtime internals — visibility is the slice
    //! boundary signal, not friction to route around. The shell
    //! handler is a thin pass-through and inherits the runtime
    //! invariant unchanged.

    use super::{advance_playlist_impl, playlist_position_impl};
    use crate::state::AppState;
    use chromaticity_runtime::{Command, CommandBus};

    #[test]
    fn playlist_position_initial_is_zero() {
        let app = AppState::new();
        assert_eq!(playlist_position_impl(&app).unwrap(), 0);
    }

    #[test]
    fn advance_playlist_increments_position() {
        let app = AppState::new();
        advance_playlist_impl(&app, 3).unwrap();
        assert_eq!(playlist_position_impl(&app).unwrap(), 3);
    }

    #[test]
    fn advance_playlist_negative_step_moves_backward() {
        let app = AppState::new();
        advance_playlist_impl(&app, 5).unwrap();
        advance_playlist_impl(&app, -2).unwrap();
        assert_eq!(playlist_position_impl(&app).unwrap(), 3);
    }

    #[test]
    fn advance_playlist_readback_includes_own_command_after_queued_work() {
        let app = AppState::new();
        app.bus
            .dispatch(Command::PlaylistAdvance { step: 10 })
            .unwrap();

        advance_playlist_impl(&app, 3).unwrap();

        assert_eq!(playlist_position_impl(&app).unwrap(), 13);
    }
}

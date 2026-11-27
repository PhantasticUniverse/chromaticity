// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Preset recall IPC handlers — Phase-1 D4.
//!
//! Mirrors the D2 control and D3 Snap round-trip patterns: the mutator
//! ([`recall_preset`]) wraps the raw `u32` into a
//! [`chromaticity_runtime::PresetId`], dispatches a
//! [`chromaticity_runtime::Command::PresetRecall`] through the bus held
//! in [`crate::state::AppState`], drains older queued commands, and
//! applies its own committed event to canonical state via
//! [`chromaticity_runtime::CommandBus::dispatch_and_commit_into`]. The reader
//! ([`active_preset`]) projects the resulting state back across the IPC
//! boundary as `Option<u32>`.
//!
//! Errors collapse to `Result<_, String>` so the Tauri serializer
//! carries a human-readable failure across the IPC boundary;
//! [`chromaticity_runtime::CommandBusError`] feeds [`std::fmt::Display`]
//! into the wrapper.
//!
//! Each Tauri-facing handler is a thin wrapper over an inner
//! `*_impl(&AppState, ...)` function so the round-trip glue is unit
//! testable without a Tauri runtime.

use chromaticity_runtime::{Command, CommandBus, PresetId};
use tauri::State;

use crate::state::AppState;

fn recall_preset_impl(app: &AppState, id: u32) -> Result<(), String> {
    let mut state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    app.bus
        .dispatch_and_commit_into(Command::PresetRecall { id: PresetId(id) }, &mut state)
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn save_preset_impl(app: &AppState, id: u32) -> Result<(), String> {
    // Wave 2.4 PERF.C: dispatch `Command::PresetSave { id }` and let
    // the canonical state commit thread snapshot the current
    // `ControlStore` into the preset bank via `State::apply_event`.
    // Mirrors `recall_preset_impl`'s
    // `lock → dispatch_and_commit_into` chain.
    let mut state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    app.bus
        .dispatch_and_commit_into(Command::PresetSave { id: PresetId(id) }, &mut state)
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn active_preset_impl(app: &AppState) -> Result<Option<u32>, String> {
    let state = app
        .state
        .lock()
        .map_err(|err| format!("state mutex poisoned: {err}"))?;
    Ok(state.active_preset().map(|p| p.0))
}

/// Recall the preset identified by `id` and apply the committed event
/// to canonical state synchronously.
#[tauri::command]
pub fn recall_preset(app_state: State<'_, AppState>, id: u32) -> Result<(), String> {
    recall_preset_impl(app_state.inner(), id)
}

/// Wave 2.4 PERF.C — save the current `ControlStore` snapshot into the
/// preset bank keyed by `id`. The save also activates the preset
/// (matching the `SnapSave` precedent of setting `active = Some(slot)`
/// at apply time). In-process only; preset-bank durability + on-disk
/// persistence remain Wave 2.5+ scope per the Phase-3
/// `EV-PRESET-PLAYLIST` narrowing.
#[tauri::command]
pub fn save_preset(app_state: State<'_, AppState>, id: u32) -> Result<(), String> {
    save_preset_impl(app_state.inner(), id)
}

/// Return the active preset id, or `None` when no preset has been
/// recalled.
#[tauri::command]
pub fn active_preset(app_state: State<'_, AppState>) -> Result<Option<u32>, String> {
    active_preset_impl(app_state.inner())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Unit tests for the D4 preset round-trip handler glue. Pins the
    //! SHELL-side `dispatch_and_commit_into → readback` chain.
    //! `PresetId` is a transparent `u32` newtype with no validator, so
    //! there is no invalid-input test to write.

    use super::{active_preset_impl, recall_preset_impl, save_preset_impl};
    use crate::state::AppState;
    use chromaticity_runtime::{Command, CommandBus, PresetId};

    #[test]
    fn recall_preset_then_active_preset_round_trip() {
        let app = AppState::new();
        recall_preset_impl(&app, 42).unwrap();
        assert_eq!(active_preset_impl(&app).unwrap(), Some(42));
    }

    #[test]
    fn active_preset_initial_is_none() {
        let app = AppState::new();
        assert_eq!(active_preset_impl(&app).unwrap(), None);
    }

    #[test]
    fn recall_preset_overwrites_active() {
        let app = AppState::new();
        recall_preset_impl(&app, 1).unwrap();
        recall_preset_impl(&app, 2).unwrap();
        assert_eq!(active_preset_impl(&app).unwrap(), Some(2));
    }

    #[test]
    fn save_preset_then_active_preset_round_trip() {
        // Wave 2.4 PERF.C — saving a preset activates it on the
        // canonical-state side. Mirrors `recall_preset_then_active`
        // shape.
        let app = AppState::new();
        save_preset_impl(&app, 5).unwrap();
        assert_eq!(active_preset_impl(&app).unwrap(), Some(5));
    }

    #[test]
    fn recall_preset_readback_includes_own_command_after_queued_work() {
        let app = AppState::new();
        app.bus
            .dispatch(Command::PresetRecall { id: PresetId(1) })
            .unwrap();

        recall_preset_impl(&app, 2).unwrap();

        assert_eq!(active_preset_impl(&app).unwrap(), Some(2));
    }

    #[test]
    fn save_preset_readback_includes_own_command_after_queued_work() {
        let app = AppState::new();
        app.bus
            .dispatch(Command::PresetRecall { id: PresetId(1) })
            .unwrap();

        save_preset_impl(&app, 5).unwrap();

        assert_eq!(active_preset_impl(&app).unwrap(), Some(5));
    }
}

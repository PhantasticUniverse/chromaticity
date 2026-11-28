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

use chromaticity_runtime::{Command, CommandBus, ControlId, ControlValue, PresetId};
use std::collections::BTreeMap;
use std::path::Path;
use tauri::{Manager, State};

use crate::state::AppState;

const PRESETS_FILENAME: &str = "presets.json";
const MAX_PERSISTED_CONTROL_ID: u32 = 64;
/// Neutral shader default (`0.5`) matching `commands::render::render_gpu_frame`
/// so startup preset hydration never clobbers unset controls to `0.0`.
const NEUTRAL_STARTUP_CONTROL_DEFAULT: f32 = 0.5;

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
struct PresetSnapshotDisk {
    #[serde(default)]
    scalars: BTreeMap<u32, f32>,
    #[serde(default)]
    xy: BTreeMap<u32, [f32; 2]>,
}

type PresetBankDisk = BTreeMap<u32, PresetSnapshotDisk>;

fn persist_preset_snapshot(app: &AppState, data_dir: &Path, id: u32) {
    let snapshot: PresetSnapshotDisk = {
        let state = match app.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut scalars = BTreeMap::new();
        let mut xy = BTreeMap::new();
        for cid in 0..MAX_PERSISTED_CONTROL_ID {
            if let Some(val) = state.control_value(ControlId(cid)) {
                scalars.insert(cid, val.0);
            }
            if let Some(pair) = state.xy_control_value(ControlId(cid)) {
                xy.insert(cid, pair);
            }
        }
        PresetSnapshotDisk { scalars, xy }
    };

    let path = data_dir.join(PRESETS_FILENAME);
    if let Err(err) = std::fs::create_dir_all(data_dir) {
        eprintln!("[chromaticity][preset] create_dir_all failed: {err}");
        return;
    }
    let mut bank: PresetBankDisk = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    bank.insert(id, snapshot);
    if let Ok(json) = serde_json::to_string_pretty(&bank) {
        let tmp = data_dir.join(format!(".{PRESETS_FILENAME}.{}.tmp", std::process::id()));
        if std::fs::write(&tmp, json.as_bytes()).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

/// Hydrate previously persisted preset control snapshots (both scalar and 2D XY)
/// from `<data_dir>/presets.json` into canonical [`AppState::state`] on startup,
/// isolating each preset so keys never leak across preset slots.
pub fn hydrate_presets_from_disk(app: &AppState, data_dir: &Path) {
    let path = data_dir.join(PRESETS_FILENAME);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(bank) = serde_json::from_str::<PresetBankDisk>(&raw) else {
        return;
    };
    if bank.is_empty() {
        return;
    }
    let mut state = match app.state.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let mut prior_scalars: Vec<(u32, f32)> = Vec::new();
    let mut prior_xy: Vec<(u32, [f32; 2])> = Vec::new();
    for cid in 0..MAX_PERSISTED_CONTROL_ID {
        if let Some(val) = state.control_value(ControlId(cid)) {
            prior_scalars.push((cid, val.0));
        }
        if let Some(pair) = state.xy_control_value(ControlId(cid)) {
            prior_xy.push((cid, pair));
        }
    }
    for (preset_id, snap) in bank {
        // Reset controls before applying each preset snapshot so previous presets
        // do not bleed values into subsequent slots.
        for cid in 0..MAX_PERSISTED_CONTROL_ID {
            if state.control_value(ControlId(cid)).is_some() && !snap.scalars.contains_key(&cid) {
                let _ = app.bus.dispatch_and_commit_into(
                    Command::ControlSet {
                        id: ControlId(cid),
                        value: ControlValue(0.0),
                    },
                    &mut state,
                );
            }
            if state.xy_control_value(ControlId(cid)).is_some() && !snap.xy.contains_key(&cid) {
                let _ = app.bus.dispatch_and_commit_into(
                    Command::ControlSet2D {
                        id: ControlId(cid),
                        value: [0.0, 0.0],
                    },
                    &mut state,
                );
            }
        }
        for (cid, val) in snap.scalars {
            let _ = app.bus.dispatch_and_commit_into(
                Command::ControlSet {
                    id: ControlId(cid),
                    value: ControlValue(val),
                },
                &mut state,
            );
        }
        for (cid, pair) in snap.xy {
            let _ = app.bus.dispatch_and_commit_into(
                Command::ControlSet2D {
                    id: ControlId(cid),
                    value: pair,
                },
                &mut state,
            );
        }
        let _ = app.bus.dispatch_and_commit_into(
            Command::PresetSave {
                id: PresetId(preset_id),
            },
            &mut state,
        );
    }
    // Reset live controls after hydrating presets so the last preset in
    // `presets.json` never leaks onto fresh startup state, while preserving
    // the neutral `0.5` shader default (`[0.5, 0.5]` for XY) on fresh startup.
    if prior_scalars.is_empty() && prior_xy.is_empty() {
        for cid in 0..MAX_PERSISTED_CONTROL_ID {
            if state.control_value(ControlId(cid)).is_some() {
                let _ = app.bus.dispatch_and_commit_into(
                    Command::ControlSet {
                        id: ControlId(cid),
                        value: ControlValue(NEUTRAL_STARTUP_CONTROL_DEFAULT),
                    },
                    &mut state,
                );
            }
            if state.xy_control_value(ControlId(cid)).is_some() {
                let _ = app.bus.dispatch_and_commit_into(
                    Command::ControlSet2D {
                        id: ControlId(cid),
                        value: [
                            NEUTRAL_STARTUP_CONTROL_DEFAULT,
                            NEUTRAL_STARTUP_CONTROL_DEFAULT,
                        ],
                    },
                    &mut state,
                );
            }
        }
    } else {
        for (cid, val) in prior_scalars {
            let _ = app.bus.dispatch_and_commit_into(
                Command::ControlSet {
                    id: ControlId(cid),
                    value: ControlValue(val),
                },
                &mut state,
            );
        }
        for (cid, pair) in prior_xy {
            let _ = app.bus.dispatch_and_commit_into(
                Command::ControlSet2D {
                    id: ControlId(cid),
                    value: pair,
                },
                &mut state,
            );
        }
    }
}

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

/// Save the current `ControlStore` snapshot into the preset bank keyed by `id`
/// and persist the full control snapshot atomically to `<app_data_dir>/presets.json`.
#[tauri::command]
pub fn save_preset(
    app: tauri::AppHandle,
    app_state: State<'_, AppState>,
    id: u32,
) -> Result<(), String> {
    save_preset_impl(app_state.inner(), id)?;
    if let Ok(dir) = app.path().app_data_dir() {
        persist_preset_snapshot(app_state.inner(), &dir, id);
    }
    Ok(())
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

    #[test]
    fn persist_and_hydrate_preserves_scalars_and_xy_without_cross_preset_bleed() {
        use super::{hydrate_presets_from_disk, persist_preset_snapshot};
        use chromaticity_runtime::{ControlId, ControlValue};

        let tmp_dir =
            std::env::temp_dir().join(format!("chromaticity-preset-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp_dir);

        // Save preset 1 with scalar control 0 = 0.75 and XY control 2 = [0.25, 0.85]
        let app1 = AppState::new();
        {
            let mut st = app1.state.lock().unwrap();
            app1.bus
                .dispatch_and_commit_into(
                    Command::ControlSet {
                        id: ControlId(0),
                        value: ControlValue(0.75),
                    },
                    &mut st,
                )
                .unwrap();
            app1.bus
                .dispatch_and_commit_into(
                    Command::ControlSet2D {
                        id: ControlId(2),
                        value: [0.25, 0.85],
                    },
                    &mut st,
                )
                .unwrap();
        }
        save_preset_impl(&app1, 1).unwrap();
        persist_preset_snapshot(&app1, &tmp_dir, 1);

        // Save preset 2 on a fresh state with only scalar control 1 = 0.42 (no control 0 or XY 2)
        let app2 = AppState::new();
        {
            let mut st = app2.state.lock().unwrap();
            app2.bus
                .dispatch_and_commit_into(
                    Command::ControlSet {
                        id: ControlId(1),
                        value: ControlValue(0.42),
                    },
                    &mut st,
                )
                .unwrap();
        }
        save_preset_impl(&app2, 2).unwrap();
        persist_preset_snapshot(&app2, &tmp_dir, 2);

        // Hydrate into a brand-new AppState and verify neutral startup defaults
        // before recalling any preset, then verify both presets recall cleanly.
        let hydrated = AppState::new();
        hydrate_presets_from_disk(&hydrated, &tmp_dir);
        {
            let st = hydrated.state.lock().unwrap();
            assert!((st.control_value(ControlId(0)).unwrap().0 - 0.5).abs() < 1e-5);
            assert!((st.control_value(ControlId(1)).unwrap().0 - 0.5).abs() < 1e-5);
            let xy = st.xy_control_value(ControlId(2)).unwrap();
            assert!((xy[0] - 0.5).abs() < 1e-5 && (xy[1] - 0.5).abs() < 1e-5);
        }

        recall_preset_impl(&hydrated, 1).unwrap();
        {
            let st = hydrated.state.lock().unwrap();
            assert!((st.control_value(ControlId(0)).unwrap().0 - 0.75).abs() < 1e-5);
            let xy = st.xy_control_value(ControlId(2)).unwrap();
            assert!((xy[0] - 0.25).abs() < 1e-5 && (xy[1] - 0.85).abs() < 1e-5);
        }

        recall_preset_impl(&hydrated, 2).unwrap();
        {
            let st = hydrated.state.lock().unwrap();
            assert!((st.control_value(ControlId(1)).unwrap().0 - 0.42).abs() < 1e-5);
            // Control 0 and XY 2 must be reset to 0.0 rather than bleeding 0.75 / [0.25, 0.85]
            assert!(st.control_value(ControlId(0)).unwrap().0.abs() < 1e-5);
            let xy = st.xy_control_value(ControlId(2)).unwrap();
            assert!(xy[0].abs() < 1e-5 && xy[1].abs() < 1e-5);
        }

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}

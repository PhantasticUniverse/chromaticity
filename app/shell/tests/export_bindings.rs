// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Re-export ts-rs bindings for the IPC types into the frontend tree.
//!
//! Test-target export (the conventional ts-rs 10 pattern): running
//! `cargo test -p chromaticity-app` regenerates
//! `src/lib/ipc/bindings/<TypeName>.ts` so committed bindings stay in
//! lockstep with the bridge crate's serde/ts-rs surface. CI's
//! `cargo test --workspace --all-targets` step fires this test;
//! drift surfaces locally to whoever runs `cargo test` and via review
//! diff on PR pushes.
//!
//! The frontend index file at `src/lib/ipc/bindings/index.ts` is hand-
//! authored and re-exports the per-type files generated here.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_ui_bridge::{
    AudioAxisAvailability, AudioDeviceKindPayload, AudioDevicePayload, AudioSettingsPayload,
    AudioUniformSnapshotPayload, CameraDeviceKindPayload, CameraDevicePayload,
    ControlDefaultValuePayload, ControlId, ControlKindPayload, ControlSchemaPayload, ControlValue,
    GeneralSettingsPayload, GraphicsSettingsPayload, LiveSourcesSettingsPayload,
    LoadedScenePayload, MidiDeviceKindPayload, MidiDevicePayload, MidiLearnEventPayload,
    MidiLearnKindPayload, OscDirectionPayload, OscHealthSnapshot, OscHealthState,
    OscRouteFamilyPayload, OscRoutePayload, OscSettingsPayload, PresetId, SceneCard, SceneSource,
    SettingsPayload, SnapSlotId, StorageSettingsPayload, SyphonOutputStatusPayload,
    SyphonServerPayload, TempoSettingsPayload, ThemePayload,
};
use std::path::PathBuf;
use ts_rs::TS;

#[test]
fn export_bindings_to_frontend_tree() {
    // `cargo test -p chromaticity-app` runs with cwd = `app/shell/`,
    // so the relative path resolves to the repo-root frontend dir.
    let out_dir = PathBuf::from("../../src/lib/ipc/bindings");
    std::fs::create_dir_all(&out_dir).expect("create bindings out dir");

    ControlId::export_all_to(&out_dir).expect("export ControlId bindings");
    ControlValue::export_all_to(&out_dir).expect("export ControlValue bindings");
    SnapSlotId::export_all_to(&out_dir).expect("export SnapSlotId bindings");
    PresetId::export_all_to(&out_dir).expect("export PresetId bindings");
    SceneCard::export_all_to(&out_dir).expect("export SceneCard bindings");
    SceneSource::export_all_to(&out_dir).expect("export SceneSource bindings");
    OscHealthState::export_all_to(&out_dir).expect("export OscHealthState bindings");
    OscHealthSnapshot::export_all_to(&out_dir).expect("export OscHealthSnapshot bindings");
    AudioAxisAvailability::export_all_to(&out_dir).expect("export AudioAxisAvailability bindings");
    AudioUniformSnapshotPayload::export_all_to(&out_dir)
        .expect("export AudioUniformSnapshotPayload bindings");
    // Wave 2.6 Lane AUD.A: live cpal input-device enumeration bridge.
    AudioDeviceKindPayload::export_all_to(&out_dir)
        .expect("export AudioDeviceKindPayload bindings");
    AudioDevicePayload::export_all_to(&out_dir).expect("export AudioDevicePayload bindings");
    CameraDeviceKindPayload::export_all_to(&out_dir)
        .expect("export CameraDeviceKindPayload bindings");
    CameraDevicePayload::export_all_to(&out_dir).expect("export CameraDevicePayload bindings");
    MidiDeviceKindPayload::export_all_to(&out_dir).expect("export MidiDeviceKindPayload bindings");
    MidiDevicePayload::export_all_to(&out_dir).expect("export MidiDevicePayload bindings");
    MidiLearnKindPayload::export_all_to(&out_dir).expect("export MidiLearnKindPayload bindings");
    MidiLearnEventPayload::export_all_to(&out_dir).expect("export MidiLearnEventPayload bindings");
    SyphonServerPayload::export_all_to(&out_dir).expect("export SyphonServerPayload bindings");
    SyphonOutputStatusPayload::export_all_to(&out_dir)
        .expect("export SyphonOutputStatusPayload bindings");
    LoadedScenePayload::export_all_to(&out_dir).expect("export LoadedScenePayload bindings");
    ControlSchemaPayload::export_all_to(&out_dir).expect("export ControlSchemaPayload bindings");
    ControlKindPayload::export_all_to(&out_dir).expect("export ControlKindPayload bindings");
    ControlDefaultValuePayload::export_all_to(&out_dir)
        .expect("export ControlDefaultValuePayload bindings");
    // Wave 2.1 Lane C.3 settings persistence — SettingsPayload + its
    // four section sub-payloads + the ThemePayload discriminant. The
    // typed-newtype cluster carries the operator-preferences IPC
    // surface backing the `get_settings` / `set_settings` Tauri
    // commands.
    ThemePayload::export_all_to(&out_dir).expect("export ThemePayload bindings");
    GeneralSettingsPayload::export_all_to(&out_dir)
        .expect("export GeneralSettingsPayload bindings");
    AudioSettingsPayload::export_all_to(&out_dir).expect("export AudioSettingsPayload bindings");
    GraphicsSettingsPayload::export_all_to(&out_dir)
        .expect("export GraphicsSettingsPayload bindings");
    StorageSettingsPayload::export_all_to(&out_dir)
        .expect("export StorageSettingsPayload bindings");
    OscSettingsPayload::export_all_to(&out_dir).expect("export OscSettingsPayload bindings");
    // Wave 2.7 Lane SET.A — TempoSettingsPayload mirrors the Settings
    // view's Tempo & Sync section (W22 PR1 + W40 expansion).
    TempoSettingsPayload::export_all_to(&out_dir).expect("export TempoSettingsPayload bindings");
    // Wave 2.7 SET.B Live Sources persistence — the six-field
    // `LiveSourcesSettingsPayload` extends `SettingsPayload`.
    LiveSourcesSettingsPayload::export_all_to(&out_dir)
        .expect("export LiveSourcesSettingsPayload bindings");
    SettingsPayload::export_all_to(&out_dir).expect("export SettingsPayload bindings");
    // Wave 2.1 Lane D.3 OSC route binding — registry payloads.
    OscDirectionPayload::export_all_to(&out_dir).expect("export OscDirectionPayload bindings");
    OscRouteFamilyPayload::export_all_to(&out_dir).expect("export OscRouteFamilyPayload bindings");
    OscRoutePayload::export_all_to(&out_dir).expect("export OscRoutePayload bindings");
}

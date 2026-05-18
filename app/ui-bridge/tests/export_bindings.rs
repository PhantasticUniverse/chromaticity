// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Re-export ts-rs bindings for the IPC types into the frontend tree.
//!
//! This bridge-local target mirrors the app-shell export harness so
//! payload-only widenings can be regenerated with
//! `cargo test -p chromaticity-ui-bridge --test export_bindings`. The
//! frontend index file at `src/lib/ipc/bindings/index.ts` is hand-
//! authored and re-exports the per-type files generated here.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_ui_bridge::{
    AudioAxisAvailability, AudioDeviceKindPayload, AudioDevicePayload, AudioSettingsPayload,
    AudioUniformSnapshotPayload, CameraDeviceKindPayload, CameraDevicePayload,
    ControlDefaultValuePayload, ControlId, ControlKindPayload, ControlSchemaPayload, ControlValue,
    CrashResumeStatusPayload, GeneralSettingsPayload, GraphicsSettingsPayload,
    LiveSourcesSettingsPayload, LoadedScenePayload, MidiDeviceKindPayload, MidiDevicePayload,
    MidiLearnEventPayload, MidiLearnKindPayload, OscDirectionPayload, OscHealthSnapshot,
    OscHealthState, OscRouteFamilyPayload, OscRoutePayload, OscSettingsPayload, PresetId,
    ResumeStatusPayload, SceneCard, SceneSource, SettingsPayload, SnapSlotId,
    StorageSettingsPayload, SyphonOutputStatusPayload, SyphonServerPayload, TempoSettingsPayload,
    ThemePayload,
};
use std::path::PathBuf;
use ts_rs::TS;

#[test]
fn export_bindings_to_frontend_tree() {
    // `cargo test -p chromaticity-ui-bridge` runs with cwd =
    // `app/ui-bridge/`, so the relative path resolves to the
    // repo-root frontend dir.
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
    ResumeStatusPayload::export_all_to(&out_dir).expect("export ResumeStatusPayload bindings");
    CrashResumeStatusPayload::export_all_to(&out_dir)
        .expect("export CrashResumeStatusPayload bindings");
    AudioUniformSnapshotPayload::export_all_to(&out_dir)
        .expect("export AudioUniformSnapshotPayload bindings");
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
    ThemePayload::export_all_to(&out_dir).expect("export ThemePayload bindings");
    GeneralSettingsPayload::export_all_to(&out_dir)
        .expect("export GeneralSettingsPayload bindings");
    AudioSettingsPayload::export_all_to(&out_dir).expect("export AudioSettingsPayload bindings");
    GraphicsSettingsPayload::export_all_to(&out_dir)
        .expect("export GraphicsSettingsPayload bindings");
    StorageSettingsPayload::export_all_to(&out_dir)
        .expect("export StorageSettingsPayload bindings");
    OscSettingsPayload::export_all_to(&out_dir).expect("export OscSettingsPayload bindings");
    TempoSettingsPayload::export_all_to(&out_dir).expect("export TempoSettingsPayload bindings");
    LiveSourcesSettingsPayload::export_all_to(&out_dir)
        .expect("export LiveSourcesSettingsPayload bindings");
    SettingsPayload::export_all_to(&out_dir).expect("export SettingsPayload bindings");
    OscDirectionPayload::export_all_to(&out_dir).expect("export OscDirectionPayload bindings");
    OscRouteFamilyPayload::export_all_to(&out_dir).expect("export OscRouteFamilyPayload bindings");
    OscRoutePayload::export_all_to(&out_dir).expect("export OscRoutePayload bindings");
}

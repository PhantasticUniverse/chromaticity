// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Evidence: EV-IMPORT-REFERENCE-SHOW (T2) — Phase-4b Wave 4b.7 PR 3
//! of N synthetic-corpus fixtures for the MIDI mappings, OSC alias
//! tables, and hotkey-remappings importers per ADR-015 §Mechanism +
//! §Diagnostics and `docs/26-evidence-artifact-registry.md` row 85.
//!
//! T2 per `docs/43-development-standards.md` §15.5: synthetic
//! end-to-end fixtures pairing each new per-class importer against
//! synthetic reference user-data root layouts. Five fixtures cover:
//! per-scene MIDI CC round-trip, MIDI missing-device-id diagnostic,
//! OSC chroma-prefix collision diagnostic, hotkey V1-locked dormant
//! status, and a combined three-class dry run that confirms
//! diagnostics from each side land independently.
//!
//! Clean-room posture per ADR-015 line 20 +
//! `docs/29-cleanroom-legal-policy-baseline.md` Brand/Trademark
//! Review Checklist: no reference-app product name appears in any
//! fixture content, doc comment, or assertion message. The
//! source-side directory + filename literals (`MidiMappings`,
//! `OscAliases.json`, `HotkeyRemaps.json`) are read-side path data
//! per ADR-015 line 9 and do NOT name a reference-app product.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use chromaticity_authoring::import_reference::{
    ArtifactClass, HotkeyImporter, ImportDiagnosticCode, ImportedHotkeyStatus, ImportedMidiSource,
    MidiImporter, OscImporter, override_reference_user_data_root,
};

fn unique_scratch_dir(label: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let idx = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "chromaticity-import-reference-w4b7-pr3-corpus-{label}-{pid}-{nanos}-{idx}"
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn cleanup(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: synthetic MidiMappings/<scene>.json
// with a single CC binding round-trips through MidiImporter::dry_run
// with channel + cc + target_control + soft_takeover + invert + the
// explicit min/max preserved per ADR-015 line 25 (MIDI-CONF-01..10
// surface).
#[test]
fn synthetic_midi_per_scene_cc_round_trip() {
    let dir = unique_scratch_dir("midi-cc");
    let midi_dir = dir.join("MidiMappings");
    std::fs::create_dir_all(&midi_dir).unwrap();
    std::fs::write(
        midi_dir.join("scene-alpha.json"),
        r#"{
            "scene": "scene-alpha",
            "bindings": [
                {
                    "device_id": "controller-1",
                    "channel": 2,
                    "cc": 32,
                    "target_control": "fader.master",
                    "soft_takeover": true,
                    "invert": true,
                    "min": 0.05,
                    "max": 0.95
                }
            ]
        }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let (imported, diagnostics) = MidiImporter::dry_run(&root).unwrap();
    assert!(diagnostics.is_empty());

    let scene = imported.per_scene.get("scene-alpha").unwrap();
    assert_eq!(scene.len(), 1);
    let b = &scene[0];
    assert_eq!(b.device_id.as_deref(), Some("controller-1"));
    assert_eq!(b.channel, 2);
    assert_eq!(b.source, ImportedMidiSource::ControlChange { cc: 32 });
    assert_eq!(b.target_control, "fader.master");
    assert!(b.soft_takeover);
    assert!(b.invert);
    assert!((b.min - 0.05).abs() < 1e-6);
    assert!((b.max - 0.95).abs() < 1e-6);

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: synthetic MidiMappings/<scene>.json
// missing the device_id field emits exactly one
// MidiMappingDeviceMissing diagnostic and the binding is preserved
// per ADR-015 line 68 (auto-bind-when-device-appears semantics).
#[test]
fn synthetic_midi_missing_device_id_emits_diagnostic() {
    let dir = unique_scratch_dir("midi-no-device");
    let midi_dir = dir.join("MidiMappings");
    std::fs::create_dir_all(&midi_dir).unwrap();
    std::fs::write(
        midi_dir.join("scene-beta.json"),
        r#"{
            "scene": "scene-beta",
            "bindings": [
                { "channel": 0, "note": 48, "target_control": "trigger.kick" }
            ]
        }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let (imported, diagnostics) = MidiImporter::dry_run(&root).unwrap();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].class, ArtifactClass::Midi);
    assert_eq!(
        diagnostics[0].code,
        ImportDiagnosticCode::MidiMappingDeviceMissing
    );

    let scene = imported.per_scene.get("scene-beta").unwrap();
    assert_eq!(scene.len(), 1);
    assert!(scene[0].device_id.is_none());
    assert_eq!(scene[0].source, ImportedMidiSource::Note { note: 48 });

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: synthetic OscAliases.json containing
// an alias whose source_route literally starts with `/chroma/` emits
// a single OscAliasCollision diagnostic AND the alias is still
// imported.
#[test]
fn synthetic_osc_alias_chroma_prefix_emits_collision_diagnostic() {
    let dir = unique_scratch_dir("osc-chroma");
    std::fs::write(
        dir.join("OscAliases.json"),
        r#"{
            "aliases": [
                { "source_route": "/chroma/master/intensity",
                  "target_route": "/legacy/intensity",
                  "source_ip_allowlist": ["192.168.1.10"] }
            ]
        }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let (imported, diagnostics) = OscImporter::dry_run(&root).unwrap();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].class, ArtifactClass::Osc);
    assert_eq!(diagnostics[0].code, ImportDiagnosticCode::OscAliasCollision);

    assert_eq!(imported.aliases.len(), 1);
    assert_eq!(imported.aliases[0].source_route, "/chroma/master/intensity");
    assert_eq!(
        imported.aliases[0].source_ip_allowlist,
        vec!["192.168.1.10"]
    );

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: synthetic HotkeyRemaps.json with a
// `chroma.v1_locked.*` target produces status =
// V1LockedDormantPendingV2 plus a HotkeyV1Locked diagnostic per
// ADR-015 line 27 (hotkey V1-locked posture pending V2 rebinding UI).
#[test]
fn synthetic_hotkey_v1_locked_yields_dormant_status() {
    let dir = unique_scratch_dir("hotkey-v1");
    std::fs::write(
        dir.join("HotkeyRemaps.json"),
        r#"{
            "remaps": [
                { "source_key": "shift+enter",
                  "target_action": "chroma.v1_locked.cue.fire" }
            ]
        }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let (imported, diagnostics) = HotkeyImporter::dry_run(&root).unwrap();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].class, ArtifactClass::Hotkey);
    assert_eq!(diagnostics[0].code, ImportDiagnosticCode::HotkeyV1Locked);
    assert_eq!(imported.remaps.len(), 1);
    assert_eq!(
        imported.remaps[0].status,
        ImportedHotkeyStatus::V1LockedDormantPendingV2
    );

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: a synthetic root containing all three
// per-class artifacts simultaneously yields independent diagnostic
// streams with the correct artifact class on each entry; cross-class
// imports do not interfere with each other's parsing or diagnostic
// emission.
#[test]
fn combined_three_class_dry_run_emits_independent_diagnostics() {
    let dir = unique_scratch_dir("combined");

    let midi_dir = dir.join("MidiMappings");
    std::fs::create_dir_all(&midi_dir).unwrap();
    std::fs::write(
        midi_dir.join("scene-mix.json"),
        r#"{
            "scene": "scene-mix",
            "bindings": [
                { "channel": 0, "cc": 1, "target_control": "x" }
            ]
        }"#,
    )
    .unwrap();

    std::fs::write(
        dir.join("OscAliases.json"),
        r#"{
            "aliases": [
                { "source_route": "/chroma/x", "target_route": "/y" }
            ]
        }"#,
    )
    .unwrap();

    std::fs::write(
        dir.join("HotkeyRemaps.json"),
        r#"{
            "remaps": [
                { "source_key": "ctrl+f1", "target_action": "control.disable_focus" }
            ]
        }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();

    let (midi_imported, midi_diags) = MidiImporter::dry_run(&root).unwrap();
    let (osc_imported, osc_diags) = OscImporter::dry_run(&root).unwrap();
    let (hk_imported, hk_diags) = HotkeyImporter::dry_run(&root).unwrap();

    // MIDI: missing device id only.
    assert_eq!(midi_diags.len(), 1);
    assert_eq!(midi_diags[0].class, ArtifactClass::Midi);
    assert_eq!(
        midi_diags[0].code,
        ImportDiagnosticCode::MidiMappingDeviceMissing
    );
    assert!(midi_imported.per_scene.contains_key("scene-mix"));

    // OSC: chroma-prefix collision only.
    assert_eq!(osc_diags.len(), 1);
    assert_eq!(osc_diags[0].class, ArtifactClass::Osc);
    assert_eq!(osc_diags[0].code, ImportDiagnosticCode::OscAliasCollision);
    assert_eq!(osc_imported.aliases.len(), 1);

    // Hotkey: control-focus-locked only.
    assert_eq!(hk_diags.len(), 1);
    assert_eq!(hk_diags[0].class, ArtifactClass::Hotkey);
    assert_eq!(
        hk_diags[0].code,
        ImportDiagnosticCode::HotkeyControlFocusLocked
    );
    assert_eq!(
        hk_imported.remaps[0].status,
        ImportedHotkeyStatus::ControlFocusLocked
    );

    cleanup(&dir);
}

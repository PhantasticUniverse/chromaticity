// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Evidence: EV-IMPORT-REFERENCE-SHOW (T2) — Phase-4b Wave 4b.7 PR 4
//! of N synthetic-corpus fixtures composing all eight per-class
//! importers via the drill harness per ADR-015 §Mechanism §2-§6 and
//! `docs/26-evidence-artifact-registry.md` row 85.
//!
//! T2 per `docs/43-development-standards.md` §15.5: synthetic
//! end-to-end fixtures pairing the [`ReferenceImportDriver`] surface
//! against tempdir-rooted reference user-data layouts spanning all
//! eight artifact classes (settings + scene-specific presets + Snap
//! slots + playlists + meta + media + MIDI + OSC + hotkey). Six
//! fixtures cover: full-eight-class dry-run aggregation, full-
//! selection commit round-trip, partial-selection commit shape,
//! synthetic-no-op rollback, mixed-class diagnostic ordering, and
//! empty-root zero-counts no-diagnostics posture.
//!
//! Clean-room posture per ADR-015 line 20 +
//! `docs/29-cleanroom-legal-policy-baseline.md` Brand/Trademark
//! Review Checklist: no reference-app product name appears in any
//! fixture content, doc comment, or assertion message. The source-
//! side directory + filename literals (`Settings.json`, `Presets/`,
//! `Favslots/`, `Playlists/`, `MetaPresets/`, `MediaPresets/`,
//! `MidiMappings/`, `OscAliases.json`, `HotkeyRemaps.json`) are
//! read-side path data per ADR-015 line 9 and do NOT name a
//! reference-app product.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chromaticity_authoring::import_reference::{
    ArtifactClass, ImportSelection, ReferenceImportDriver, override_reference_user_data_root,
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
        "chromaticity-import-reference-w4b7-pr4-corpus-{label}-{pid}-{nanos}-{idx}"
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn cleanup(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

/// Seed every artifact class with a single minimal valid payload at
/// `dir`. Used by the multi-fixture corpus to drive the drill
/// harness against a non-empty synthetic root.
fn seed_eight_class_root(dir: &Path) {
    std::fs::write(
        dir.join("Settings.json"),
        r#"{
            "audioDevice": "device-x",
            "outputDisplay": "display-y",
            "uiScaleFactor": 1.25
        }"#,
    )
    .unwrap();

    let presets_dir = dir.join("Presets");
    std::fs::create_dir_all(&presets_dir).unwrap();
    std::fs::write(
        presets_dir.join("scene-a.json"),
        r#"{
            "scene": "scene-a",
            "presets": [
                { "name": "p1", "controls": { "x": 0.5 } },
                { "name": "p2", "controls": { "x": 0.75 } }
            ]
        }"#,
    )
    .unwrap();

    let snap_dir = dir.join("Favslots");
    std::fs::create_dir_all(&snap_dir).unwrap();
    std::fs::write(
        snap_dir.join("scene-a.json"),
        r#"{
            "scene": "scene-a",
            "slots": [
                { "index": 0, "controls": { "x": 0.25 } },
                { "index": 5, "controls": { "x": 0.5 } }
            ]
        }"#,
    )
    .unwrap();

    let playlists_dir = dir.join("Playlists");
    std::fs::create_dir_all(&playlists_dir).unwrap();
    std::fs::write(
        playlists_dir.join("set.json"),
        r#"{ "name": "set-1", "scenes": ["scene-a", "scene-b"] }"#,
    )
    .unwrap();

    let meta_dir = dir.join("MetaPresets");
    std::fs::create_dir_all(&meta_dir).unwrap();
    std::fs::write(
        meta_dir.join("preset.json"),
        r#"{ "name": "meta-1", "controls": { "y": 0.1 } }"#,
    )
    .unwrap();

    let media_dir = dir.join("MediaPresets");
    std::fs::create_dir_all(&media_dir).unwrap();
    std::fs::write(
        media_dir.join("preset.json"),
        r#"{ "name": "media-1", "settings": { "fade": 0.2 } }"#,
    )
    .unwrap();

    let midi_dir = dir.join("MidiMappings");
    std::fs::create_dir_all(&midi_dir).unwrap();
    std::fs::write(
        midi_dir.join("scene-a.json"),
        r#"{
            "scene": "scene-a",
            "bindings": [{
                "device_id": "controller-1",
                "channel": 0,
                "cc": 1,
                "target_control": "fader.master"
            }]
        }"#,
    )
    .unwrap();

    std::fs::write(
        dir.join("OscAliases.json"),
        r#"{ "aliases": [{ "source_route": "/legacy/x", "target_route": "/canonical/y" }] }"#,
    )
    .unwrap();

    std::fs::write(
        dir.join("HotkeyRemaps.json"),
        r#"{ "remaps": [{ "source_key": "ctrl+1", "target_action": "scene.next" }] }"#,
    )
    .unwrap();
}

// EV-IMPORT-REFERENCE-SHOW T2: a synthetic root carrying a payload
// for each of the eight artifact classes dry-runs cleanly through
// the drill harness; per-class imports + per-class counts populate
// as expected; counts are non-zero across every class.
#[test]
fn synthetic_eight_class_root_dry_run_succeeds() {
    let dir = unique_scratch_dir("dry-run-success");
    seed_eight_class_root(&dir);
    let root = override_reference_user_data_root(dir.clone()).unwrap();

    let report = ReferenceImportDriver::dry_run(&root).unwrap();

    assert_eq!(report.settings.audio_device_id.as_deref(), Some("device-x"));
    assert_eq!(report.class_counts.preset_count, 2);
    assert_eq!(report.class_counts.snap_slot_count, 2);
    assert_eq!(report.class_counts.playlist_count, 1);
    assert_eq!(report.class_counts.meta_preset_count, 1);
    assert_eq!(report.class_counts.media_preset_count, 1);
    assert_eq!(report.class_counts.midi_binding_count, 1);
    assert_eq!(report.class_counts.osc_alias_count, 1);
    assert_eq!(report.class_counts.hotkey_remap_count, 1);
    assert_eq!(report.class_counts.settings_known_keys, 2);
    assert_eq!(report.class_counts.settings_extension_keys, 1);

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: a full-selection commit on an
// eight-class synthetic root round-trips through the drill harness:
// every dry-run diagnostic is re-emitted at commit time with
// `dry_run = false`; the rollback snapshot's pre_commit_state JSON
// payload contains an entry for every class.
#[test]
fn synthetic_eight_class_root_commit_full_selection_round_trips() {
    let dir = unique_scratch_dir("commit-full-rt");
    seed_eight_class_root(&dir);
    let root = override_reference_user_data_root(dir.clone()).unwrap();

    let report = ReferenceImportDriver::dry_run(&root).unwrap();
    let dry_run_diag_count = report.diagnostics.len();
    let outcome = ReferenceImportDriver::commit(&report, &ImportSelection::all()).unwrap();

    assert_eq!(outcome.committed_diagnostics.len(), dry_run_diag_count);
    assert!(outcome.committed_diagnostics.iter().all(|d| !d.dry_run));
    assert_eq!(outcome.committed_classes, ImportSelection::all());

    let serde_json::Value::Object(ref payload) = outcome.rollback_snapshot.pre_commit_state else {
        panic!("expected pre_commit_state to be a JSON object");
    };
    for class_key in [
        "settings",
        "presets",
        "snap",
        "playlists",
        "meta_media",
        "midi",
        "osc",
        "hotkey",
    ] {
        assert!(payload.contains_key(class_key), "missing key {class_key}");
    }

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: a partial-selection commit (Settings
// + MIDI only) on an eight-class synthetic root omits the
// unselected-class diagnostics from the committed outcome, and the
// rollback snapshot's pre_commit_state payload carries entries for
// only the selected classes.
#[test]
fn synthetic_eight_class_root_commit_partial_selection_omits_unselected() {
    let dir = unique_scratch_dir("commit-partial");

    // Plant diagnostics in three classes (Settings, Preset, Hotkey)
    // so partial-selection has unselected diagnostics to omit.
    std::fs::write(dir.join("Settings.json"), b"not actually json").unwrap();
    let presets_dir = dir.join("Presets");
    std::fs::create_dir_all(&presets_dir).unwrap();
    std::fs::write(
        presets_dir.join("scene-a.json"),
        r#"{ "scene": "scene-a", "presets": [{ "name": "p", "controls": {} }] }"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("HotkeyRemaps.json"),
        r#"{ "remaps": [{ "source_key": "ctrl+9", "target_action": "chroma.v1_locked.foo" }] }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let report = ReferenceImportDriver::dry_run(&root).unwrap();

    let selection = ImportSelection {
        settings: true,
        midi: true,
        ..ImportSelection::none()
    };
    let outcome = ReferenceImportDriver::commit(&report, &selection).unwrap();

    // Diagnostics from selected classes only.
    for diag in &outcome.committed_diagnostics {
        assert!(matches!(
            diag.class,
            ArtifactClass::Settings | ArtifactClass::Midi
        ));
    }

    // Pre-commit state payload contains only Settings + MIDI keys.
    let serde_json::Value::Object(ref payload) = outcome.rollback_snapshot.pre_commit_state else {
        panic!("expected pre_commit_state to be a JSON object");
    };
    assert!(payload.contains_key("settings"));
    assert!(payload.contains_key("midi"));
    assert!(!payload.contains_key("hotkey"));
    assert!(!payload.contains_key("presets"));
    assert!(!payload.contains_key("snap"));
    assert!(!payload.contains_key("playlists"));
    assert!(!payload.contains_key("meta_media"));
    assert!(!payload.contains_key("osc"));

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: rollback against a snapshot produced
// by commit completes per PR 4 documented synthetic-no-op
// semantics. Wave 4b.7 PR 4 ships only the contract-shape harness;
// the V1 follow-on PR substitutes the real
// `32-state-preset-playlist-migration-policy.md` restoration.
#[test]
fn synthetic_eight_class_root_rollback_completes_per_pr_4_synthetic_semantics() {
    let dir = unique_scratch_dir("rollback");
    seed_eight_class_root(&dir);
    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let report = ReferenceImportDriver::dry_run(&root).unwrap();
    let outcome = ReferenceImportDriver::commit(&report, &ImportSelection::all()).unwrap();

    let result = ReferenceImportDriver::rollback(&outcome.rollback_snapshot);
    assert!(result.is_ok());

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: a synthetic root planting diagnostics
// in three distinct classes (Settings malformed-shape, Preset
// unknown-scene, OSC chroma-prefix collision) aggregates the three
// diagnostics in canonical artifact-class order through the drill
// harness's flat diagnostics vector.
#[test]
fn synthetic_root_with_diagnostics_in_three_classes_aggregates_in_order() {
    let dir = unique_scratch_dir("aggregation-order");

    std::fs::write(dir.join("Settings.json"), b"not actually json {{").unwrap();

    let presets_dir = dir.join("Presets");
    std::fs::create_dir_all(&presets_dir).unwrap();
    std::fs::write(
        presets_dir.join("a.json"),
        r#"{ "scene": "scene-z", "presets": [{ "name": "p", "controls": {} }] }"#,
    )
    .unwrap();

    std::fs::write(
        dir.join("OscAliases.json"),
        r#"{ "aliases": [{ "source_route": "/chroma/foo", "target_route": "/x" }] }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let report = ReferenceImportDriver::dry_run(&root).unwrap();

    let observed: Vec<ArtifactClass> = report
        .diagnostics
        .iter()
        .map(|d| d.class)
        .filter(|c| {
            matches!(
                c,
                ArtifactClass::Settings | ArtifactClass::Preset | ArtifactClass::Osc
            )
        })
        .collect();
    assert_eq!(
        observed,
        vec![
            ArtifactClass::Settings,
            ArtifactClass::Preset,
            ArtifactClass::Osc,
        ]
    );

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: a synthetic root carrying no
// reference-side artifacts whatsoever dry-runs through the drill
// harness with zero per-class counts and zero diagnostics — the
// "operator pointed the importer at an empty folder" case is not an
// error.
#[test]
fn synthetic_empty_root_dry_run_returns_zero_counts_no_diagnostics() {
    let dir = unique_scratch_dir("empty-root");
    let root = override_reference_user_data_root(dir.clone()).unwrap();

    let report = ReferenceImportDriver::dry_run(&root).unwrap();

    assert!(report.diagnostics.is_empty());
    let c = &report.class_counts;
    assert_eq!(c.settings_known_keys, 0);
    assert_eq!(c.settings_extension_keys, 0);
    assert_eq!(c.preset_count, 0);
    assert_eq!(c.snap_slot_count, 0);
    assert_eq!(c.playlist_count, 0);
    assert_eq!(c.meta_preset_count, 0);
    assert_eq!(c.media_preset_count, 0);
    assert_eq!(c.midi_binding_count, 0);
    assert_eq!(c.osc_alias_count, 0);
    assert_eq!(c.hotkey_remap_count, 0);

    cleanup(&dir);
}

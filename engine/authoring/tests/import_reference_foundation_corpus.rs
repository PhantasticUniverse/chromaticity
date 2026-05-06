// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Evidence: EV-IMPORT-REFERENCE-SHOW (T2) — Phase-4b Wave 4b.7 PR 1
//! of N foundation corpus for the reference-app user-data importer
//! per ADR-015 §Mechanism + §Diagnostics and
//! `docs/26-evidence-artifact-registry.md` row 85.
//!
//! T2 per `docs/43-development-standards.md` §15.5: synthetic
//! end-to-end fixtures pairing the discovery surface with the
//! settings + presets per-class importers against synthetic
//! reference user-data root layouts. Subsequent PRs in this wave
//! extend the corpus with Snap slots / playlists / MIDI / OSC /
//! hotkey-remap fixtures and the dry-run + commit + rollback drill
//! harness.
//!
//! Clean-room posture per ADR-015 line 20 +
//! `docs/29-cleanroom-legal-policy-baseline.md` Brand/Trademark
//! Review Checklist: no reference-app product name appears in any
//! fixture content, doc comment, or assertion message.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use chromaticity_authoring::import_reference::{
    ArtifactClass, DiscoveryError, ImportDiagnosticCode, PresetsImporter, SettingsImporter,
    discover_reference_user_data_root, override_reference_user_data_root,
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
        "chromaticity-import-reference-corpus-{label}-{pid}-{nanos}-{idx}"
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn cleanup(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: synthetic Settings.json with one
// known key and three unknown keys round-trips through
// SettingsImporter::dry_run with correct field-mapping +
// extensions_legacy preservation per ADR-015 line 20.
#[test]
fn synthetic_settings_with_unknown_keys_preserves_extensions_legacy() {
    let dir = unique_scratch_dir("settings-unknown-keys");
    std::fs::write(
        dir.join("Settings.json"),
        r#"{
            "audioDevice": "Built-in Output",
            "outputDisplay": "DisplayPort-1",
            "oscInputPort": 9001,
            "oscOutputPort": 9002,
            "uiScaleFactor": 1.5,
            "lastShowFolder": "/Users/operator/shows",
            "telemetryOptIn": false
        }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let (imported, diagnostics) = SettingsImporter::dry_run(&root).unwrap();

    assert_eq!(imported.audio_device_id.as_deref(), Some("Built-in Output"));
    assert_eq!(imported.output_display.as_deref(), Some("DisplayPort-1"));
    assert_eq!(imported.osc_input_port, Some(9001));
    assert_eq!(imported.osc_output_port, Some(9002));

    let serde_json::Value::Object(ref leftovers) = imported.extensions_legacy else {
        panic!("expected JSON object in extensions_legacy");
    };
    assert!(leftovers.contains_key("uiScaleFactor"));
    assert!(leftovers.contains_key("lastShowFolder"));
    assert!(leftovers.contains_key("telemetryOptIn"));
    assert_eq!(leftovers.len(), 3);

    assert!(diagnostics.is_empty());

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: synthetic preset file references a
// scene name not in the supplied known-scenes set; importer emits
// an UnknownScene diagnostic per ADR-015 §Diagnostics line 58 and
// still records the preset in the imports surface.
#[test]
fn synthetic_presets_with_unknown_scene_emits_diagnostic() {
    let dir = unique_scratch_dir("presets-unknown-scene");
    let presets_dir = dir.join("Presets");
    std::fs::create_dir_all(&presets_dir).unwrap();
    std::fs::write(
        presets_dir.join("scene-x.json"),
        r#"{
            "scene": "scene-x",
            "presets": [
                { "name": "first", "controls": { "intensity": 0.3 } },
                { "name": "second", "controls": { "intensity": 0.7 } }
            ],
            "startup_pointer": "first"
        }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let known: HashSet<String> = HashSet::new();
    let (imported, diagnostics) = PresetsImporter::dry_run(&root, &known).unwrap();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].class, ArtifactClass::Preset);
    assert_eq!(diagnostics[0].code, ImportDiagnosticCode::UnknownScene);
    assert!(diagnostics[0].dry_run);

    let scene_x = imported
        .per_scene
        .get("scene-x")
        .expect("scene-x records still recorded under unknown-scene");
    assert_eq!(scene_x.len(), 2);
    assert_eq!(
        imported.startup_pointers.get("scene-x").map(String::as_str),
        Some("first")
    );

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: auto-discovery returns Ok(None) on
// platforms or installs without a canonical reference-app
// user-data path present, OR returns
// HomeDirectoryUnavailable on sandboxed CI environments. Either
// outcome is the not-an-error case.
#[test]
fn discovery_returns_none_when_root_absent() {
    match discover_reference_user_data_root() {
        Ok(None) => {
            // Either non-mac/non-windows, OR mac/windows but the
            // canonical path doesn't exist on this machine.
        }
        Ok(Some(root)) => {
            // mac/windows where the canonical path happens to
            // exist (developer machine running the reference app);
            // the path returned MUST be a directory.
            assert!(root.path().is_dir());
        }
        Err(DiscoveryError::HomeDirectoryUnavailable) => {
            // Sandboxed CI environments without HOME / LOCALAPPDATA
            // resolution; not-an-error per the discovery contract.
        }
        Err(other) => panic!("unexpected discovery error variant: {other:?}"),
    }
}

// EV-IMPORT-REFERENCE-SHOW T2: explicit operator override at a
// freshly-created scratch directory succeeds and round-trips a
// full empty-presets-empty-settings dry run with no diagnostics.
#[test]
fn discovery_via_explicit_override_succeeds() {
    let dir = unique_scratch_dir("override-success");
    let root = override_reference_user_data_root(dir.clone()).unwrap();
    assert_eq!(root.path(), dir.as_path());

    let (settings, settings_diags) = SettingsImporter::dry_run(&root).unwrap();
    assert_eq!(settings.audio_device_id, None);
    assert_eq!(settings.output_display, None);
    assert!(settings_diags.is_empty());

    let known: HashSet<String> = HashSet::new();
    let (presets, presets_diags) = PresetsImporter::dry_run(&root, &known).unwrap();
    assert!(presets.per_scene.is_empty());
    assert!(presets.startup_pointers.is_empty());
    assert!(presets_diags.is_empty());

    cleanup(&dir);
}

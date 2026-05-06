// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Evidence: EV-IMPORT-REFERENCE-SHOW (T2) — Phase-4b Wave 4b.7 PR 2
//! of N synthetic-corpus fixtures for the Snap-slots, playlists, and
//! meta+media preset channel importers per ADR-015 §Mechanism +
//! §Diagnostics and `docs/26-evidence-artifact-registry.md` row 85.
//!
//! T2 per `docs/43-development-standards.md` §15.5: synthetic
//! end-to-end fixtures pairing each new per-class importer against
//! synthetic reference user-data root layouts. Five fixtures cover:
//! Snap gold-slot-pointer round-trip, playlist unknown-iteration-
//! mode diagnostic, meta-preset round-trip, media-preset
//! preserves-unknown-settings round-trip, and a combined
//! three-class dry run that confirms diagnostics from each side
//! land independently.
//!
//! Clean-room posture per ADR-015 line 20 +
//! `docs/29-cleanroom-legal-policy-baseline.md` Brand/Trademark
//! Review Checklist: no reference-app product name appears in any
//! fixture content, doc comment, or assertion message. The
//! source-side directory literal `Favslots` is read-side path data
//! per ADR-015 line 9 and does NOT name a reference-app product.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use chromaticity_authoring::import_reference::{
    ArtifactClass, ImportDiagnosticCode, ImportedIterationMode, MetaMediaImporter,
    PlaylistsImporter, SnapImporter, override_reference_user_data_root,
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
        "chromaticity-import-reference-w4b7-pr2-corpus-{label}-{pid}-{nanos}-{idx}"
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn cleanup(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: synthetic Favslots/<scene>.json with
// an explicit gold_slot_pointer round-trips through
// SnapImporter::dry_run with the explicit value preserved (not the
// Some(0) default), and per-slot user_name + control_values are
// captured per ADR-015 line 22.
#[test]
fn synthetic_favslots_with_gold_slot_pointer_round_trips() {
    let dir = unique_scratch_dir("favslots-gold-pointer");
    let snap_dir = dir.join("Favslots");
    std::fs::create_dir_all(&snap_dir).unwrap();
    std::fs::write(
        snap_dir.join("scene-pulse.json"),
        r#"{
            "scene": "scene-pulse",
            "gold_slot_pointer": 5,
            "slots": [
                { "index": 0, "user_name": "intro",  "controls": { "x": 0.10 } },
                { "index": 5, "user_name": "drop",   "controls": { "x": 0.95, "y": 0.2 } },
                { "index": 7,                          "controls": { "x": 0.42 } }
            ]
        }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let (imported, diagnostics) = SnapImporter::dry_run(&root).unwrap();

    assert!(diagnostics.is_empty());
    let scene = imported.per_scene.get("scene-pulse").unwrap();
    assert_eq!(scene.gold_slot_pointer, Some(5));

    let slot0 = scene.slots[0].as_ref().unwrap();
    assert_eq!(slot0.user_name.as_deref(), Some("intro"));
    let slot5 = scene.slots[5].as_ref().unwrap();
    assert_eq!(slot5.user_name.as_deref(), Some("drop"));
    assert_eq!(slot5.control_values.len(), 2);
    let slot7 = scene.slots[7].as_ref().unwrap();
    assert!(slot7.user_name.is_none());

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: synthetic playlist file with an
// unknown iteration_mode emits an UnsupportedSchemaVersion
// diagnostic per ADR-015 §Diagnostics line 76 fallback inspection
// posture; the playlist still imports with Forward as the safe
// default so the operator can see the would-land state.
#[test]
fn synthetic_playlist_with_unknown_iteration_mode_emits_diagnostic() {
    let dir = unique_scratch_dir("playlist-unknown-iter");
    let pl_dir = dir.join("Playlists");
    std::fs::create_dir_all(&pl_dir).unwrap();
    let path = pl_dir.join("set-mystery.json");
    std::fs::write(
        &path,
        r#"{
            "name": "set mystery",
            "scenes": ["scene-a", "scene-b"],
            "iteration_mode": "spiral",
            "transition": { "duration_ms": 750, "curve": "ease-out" }
        }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let (imported, diagnostics) = PlaylistsImporter::dry_run(&root).unwrap();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].class, ArtifactClass::Playlist);
    assert_eq!(
        diagnostics[0].code,
        ImportDiagnosticCode::UnsupportedSchemaVersion
    );
    assert_eq!(diagnostics[0].source_path, path);
    assert!(diagnostics[0].dry_run);
    assert!(diagnostics[0].recommended_action.contains("spiral"));

    let pl = &imported.playlists[0];
    assert_eq!(pl.iteration_mode, ImportedIterationMode::Forward);
    assert_eq!(pl.transition.duration_ms, 750);
    assert_eq!(pl.transition.curve, "ease-out");

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: synthetic meta-channel preset
// round-trips through MetaMediaImporter::dry_run with name +
// control_values preserved per ADR-015 line 23 CP-05 channel
// model.
#[test]
fn synthetic_meta_preset_round_trips() {
    let dir = unique_scratch_dir("meta-roundtrip");
    let meta_dir = dir.join("MetaPresets");
    std::fs::create_dir_all(&meta_dir).unwrap();
    std::fs::write(
        meta_dir.join("master-bus.json"),
        r#"{
            "name": "master bus",
            "controls": {
                "gain":   0.80,
                "tilt":   0.30,
                "lookup": 0.15
            }
        }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let (imported, diagnostics) = MetaMediaImporter::dry_run(&root).unwrap();

    assert!(diagnostics.is_empty());
    let preset = imported.meta.get("master bus").unwrap();
    assert_eq!(preset.name, "master bus");
    assert_eq!(preset.control_values.len(), 3);
    assert!((preset.control_values.get("gain").copied().unwrap() - 0.80).abs() < 1e-6);
    assert!((preset.control_values.get("tilt").copied().unwrap() - 0.30).abs() < 1e-6);
    assert!((preset.control_values.get("lookup").copied().unwrap() - 0.15).abs() < 1e-6);

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: synthetic media-channel preset
// preserves the full settings object verbatim as
// serde_json::Value, including unknown nested keys per ADR-015
// line 23 (media settings vary widely; preserve as JSON until the
// commit harness lands).
#[test]
fn synthetic_media_preset_preserves_unknown_settings() {
    let dir = unique_scratch_dir("media-preserve");
    let media_dir = dir.join("MediaPresets");
    std::fs::create_dir_all(&media_dir).unwrap();
    std::fs::write(
        media_dir.join("clip-rotation.json"),
        r#"{
            "name": "clip rotation",
            "settings": {
                "clips": ["intro.mp4", "loop.mp4", "outro.mp4"],
                "loop_mode": "ping_pong",
                "playback_rate": 0.75,
                "audio_pull": true,
                "experimental": {
                    "codec_hint": "av1",
                    "preroll_ms": 250
                }
            }
        }"#,
    )
    .unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();
    let (imported, diagnostics) = MetaMediaImporter::dry_run(&root).unwrap();

    assert!(diagnostics.is_empty());
    let preset = imported.media.get("clip rotation").unwrap();
    let settings = preset.settings.as_object().unwrap();
    assert_eq!(settings.len(), 5);

    let clips = settings.get("clips").and_then(|v| v.as_array()).unwrap();
    assert_eq!(clips.len(), 3);

    let experimental = settings
        .get("experimental")
        .and_then(|v| v.as_object())
        .unwrap();
    assert_eq!(
        experimental.get("codec_hint").and_then(|v| v.as_str()),
        Some("av1")
    );
    assert_eq!(
        experimental.get("preroll_ms").and_then(|v| v.as_i64()),
        Some(250)
    );

    cleanup(&dir);
}

// EV-IMPORT-REFERENCE-SHOW T2: combined dry run across all three
// new classes (Snap + playlists + meta/media) confirms diagnostics
// from each side land independently and per-class output stays
// scoped to its own importer surface. Exercises the common
// pattern the PR 4 commit + rollback drill harness will compose.
#[test]
fn combined_three_class_dry_run_emits_independent_diagnostics() {
    let dir = unique_scratch_dir("combined-three-class");

    // Snap: well-formed scene + a slot-out-of-range diagnostic case.
    let snap_dir = dir.join("Favslots");
    std::fs::create_dir_all(&snap_dir).unwrap();
    let bad_snap = snap_dir.join("scene-bad.json");
    std::fs::write(
        &bad_snap,
        r#"{
            "scene": "scene-bad",
            "slots": [
                { "index": 0,  "controls": { "x": 0.0 } },
                { "index": 99, "controls": { "x": 0.5 } }
            ]
        }"#,
    )
    .unwrap();

    // Playlists: well-formed playlist + an unknown-iteration-mode
    // diagnostic case.
    let pl_dir = dir.join("Playlists");
    std::fs::create_dir_all(&pl_dir).unwrap();
    let bad_pl = pl_dir.join("set-bad.json");
    std::fs::write(
        &bad_pl,
        r#"{
            "name": "set bad",
            "scenes": ["scene-a"],
            "iteration_mode": "warp"
        }"#,
    )
    .unwrap();

    // Meta+Media: well-formed sibling + a malformed-JSON diagnostic
    // case in the media side only.
    let meta_dir = dir.join("MetaPresets");
    std::fs::create_dir_all(&meta_dir).unwrap();
    std::fs::write(
        meta_dir.join("ok.json"),
        r#"{ "name": "ok", "controls": { "x": 0.1 } }"#,
    )
    .unwrap();

    let media_dir = dir.join("MediaPresets");
    std::fs::create_dir_all(&media_dir).unwrap();
    let bad_media = media_dir.join("broken.json");
    std::fs::write(&bad_media, "{ not json").unwrap();

    let root = override_reference_user_data_root(dir.clone()).unwrap();

    let (snap_imp, snap_diags) = SnapImporter::dry_run(&root).unwrap();
    let (pl_imp, pl_diags) = PlaylistsImporter::dry_run(&root).unwrap();
    let (mm_imp, mm_diags) = MetaMediaImporter::dry_run(&root).unwrap();

    // Each importer surfaces exactly one diagnostic, scoped to its
    // own class.
    assert_eq!(snap_diags.len(), 1);
    assert_eq!(snap_diags[0].class, ArtifactClass::Snap);
    assert_eq!(snap_diags[0].source_path, bad_snap);

    assert_eq!(pl_diags.len(), 1);
    assert_eq!(pl_diags[0].class, ArtifactClass::Playlist);
    assert_eq!(pl_diags[0].source_path, bad_pl);

    assert_eq!(mm_diags.len(), 1);
    assert_eq!(mm_diags[0].class, ArtifactClass::Preset);
    assert_eq!(mm_diags[0].source_path, bad_media);

    // Outputs from each side stay independently populated.
    assert!(snap_imp.per_scene.contains_key("scene-bad"));
    assert_eq!(pl_imp.playlists.len(), 1);
    assert!(mm_imp.meta.contains_key("ok"));
    assert!(mm_imp.media.is_empty());

    cleanup(&dir);
}

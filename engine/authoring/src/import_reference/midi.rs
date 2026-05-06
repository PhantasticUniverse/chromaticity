// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! MIDI mappings importer (ADR-015 §Scope line 25 + §Mechanism §3).
//!
//! Phase-4b Wave 4b.7 PR 3 of N — dry-run-only MIDI-mappings importer.
//! Reads `<root>/MidiMappings/*.json` (per-scene file pattern) plus an
//! optional `<root>/MidiMappings/global.json` for the global bindings.
//! The on-disk directory + filename literals are observable file-
//! system structure per ADR-015 line 9 and are permitted; no
//! reference-app product name appears in any operator-facing string.
//!
//! Per ADR-015 line 25 the importer captures the MIDI-CONF-01..10
//! surface for each binding: device id, channel, source (CC or note),
//! target Chromaticity control, soft-takeover flag, invert flag, and
//! `MIN` / `MAX` per-mapping range. The dry-run output is keyed by
//! scene name in [`ImportedMidi::per_scene`] for per-scene bindings,
//! and the optional global file populates [`ImportedMidi::global`].
//!
//! # Per-binding wire shape
//!
//! Each per-scene file contains:
//!
//! - `scene` — scene name the bindings target (per-scene file only;
//!   the global file omits this field).
//! - `bindings` — array of per-binding `{ device_id, channel, cc?,
//!   note?, target_control, soft_takeover?, invert?, min?, max? }`
//!   entries.
//!
//! Defaults when absent in the source file:
//! - `soft_takeover` -> `false`,
//! - `invert` -> `false`,
//! - `min` -> `0.0`,
//! - `max` -> `1.0`.
//!
//! # Diagnostics
//!
//! - Missing `device_id` field → preserve the binding (the
//!   Chromaticity runtime auto-binds when the device appears per
//!   ADR-015 line 68 + `08-io-integrations.md` health state) and
//!   emit a [`ImportDiagnosticCode::MidiMappingDeviceMissing`]
//!   diagnostic.
//! - Channel outside `0..=15` → emit
//!   [`ImportDiagnosticCode::OutlierControlType`] (the closest
//!   variant in the existing eleven-code vocabulary; the wave
//!   intentionally does not widen the diagnostic enum mid-pass) and
//!   skip the binding.
//! - Both `cc` and `note` set on the same binding → emit
//!   [`ImportDiagnosticCode::OutlierControlType`] and prefer `cc`
//!   for the imported source.
//! - Malformed JSON → emit a single
//!   [`ImportDiagnosticCode::UnsupportedSchemaVersion`] diagnostic
//!   per ADR-015 §Versioning line 76.
//! - Missing `<root>/MidiMappings/` directory → returns an empty
//!   [`ImportedMidi`] with no diagnostics; not-an-error case.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::diagnostics::{ArtifactClass, ImportDiagnostic, ImportDiagnosticCode};
use super::discovery::ReferenceUserDataRoot;

/// MIDI source for a single binding per ADR-015 line 25.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportedMidiSource {
    /// MIDI Control-Change source addressed by CC number `0..=127`.
    ControlChange { cc: u8 },
    /// MIDI note source addressed by note number `0..=127`.
    Note { note: u8 },
}

/// Single imported MIDI binding per ADR-015 line 25 (MIDI-CONF-01..10
/// surface).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedMidiBinding {
    /// Device id captured at binding time. `None` when the source
    /// file did not provide one — the diagnostic surface flags this
    /// case but the binding is preserved per ADR-015 line 68.
    pub device_id: Option<String>,
    /// MIDI channel `0..=15`.
    pub channel: u8,
    /// Source side of the binding (CC or note).
    pub source: ImportedMidiSource,
    /// Target Chromaticity control name verbatim from the source.
    pub target_control: String,
    /// Soft-takeover flag (MIDI-CONF-04). Defaults to `false`.
    pub soft_takeover: bool,
    /// Invert flag (MIDI-CONF-05). Defaults to `false`.
    pub invert: bool,
    /// Per-mapping minimum (MIDI-CONF-06). Defaults to `0.0`.
    pub min: f32,
    /// Per-mapping maximum (MIDI-CONF-07). Defaults to `1.0`.
    pub max: f32,
}

/// Aggregate output of the MIDI-mappings importer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportedMidi {
    /// Per-scene bindings keyed by scene name.
    pub per_scene: HashMap<String, Vec<ImportedMidiBinding>>,
    /// Global (scene-agnostic) bindings drawn from
    /// `<root>/MidiMappings/global.json` when present.
    pub global: Vec<ImportedMidiBinding>,
}

/// Errors produced by the MIDI-mappings importer.
#[derive(Debug)]
pub enum ImporterError {
    /// Filesystem error while reading the source directory.
    IoError(std::io::Error),
}

impl std::fmt::Display for ImporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "midi importer I/O error: {e}"),
        }
    }
}

impl std::error::Error for ImporterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
        }
    }
}

/// Wire-shape of a single MIDI mappings file under
/// `<root>/MidiMappings/`. Per-scene files carry `scene`; the global
/// file omits it.
#[derive(Deserialize)]
struct MidiFileShape {
    #[serde(default)]
    scene: Option<String>,
    #[serde(default)]
    bindings: Vec<MidiBindingShape>,
}

#[derive(Deserialize)]
struct MidiBindingShape {
    #[serde(default)]
    device_id: Option<String>,
    channel: i64,
    #[serde(default)]
    cc: Option<u8>,
    #[serde(default)]
    note: Option<u8>,
    target_control: String,
    #[serde(default)]
    soft_takeover: Option<bool>,
    #[serde(default)]
    invert: Option<bool>,
    #[serde(default)]
    min: Option<f32>,
    #[serde(default)]
    max: Option<f32>,
}

/// MIDI-mappings importer surface.
pub struct MidiImporter;

impl MidiImporter {
    fn midi_dir(root: &ReferenceUserDataRoot) -> PathBuf {
        root.path().join("MidiMappings")
    }

    /// Dry-run a MIDI-mappings import against `root`. Walks
    /// `<root>/MidiMappings/*.json` (per-scene files) plus the
    /// optional `<root>/MidiMappings/global.json`. Never mutates the
    /// source root.
    pub fn dry_run(
        root: &ReferenceUserDataRoot,
    ) -> Result<(ImportedMidi, Vec<ImportDiagnostic>), ImporterError> {
        let dir = Self::midi_dir(root);
        let mut imported = ImportedMidi::default();
        let mut diagnostics: Vec<ImportDiagnostic> = Vec::new();

        match dir.try_exists() {
            Ok(false) => return Ok((imported, diagnostics)),
            Err(e) => return Err(ImporterError::IoError(e)),
            Ok(true) => {}
        }

        let read_dir = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(e) => return Err(ImporterError::IoError(e)),
        };

        // Sort entries for deterministic diagnostic ordering across
        // platforms (read_dir order is not specified).
        let mut entries: Vec<PathBuf> = Vec::new();
        for entry in read_dir {
            let entry = entry.map_err(ImporterError::IoError)?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                entries.push(path);
            }
        }
        entries.sort();

        for path in entries {
            let raw = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => return Err(ImporterError::IoError(e)),
            };

            let parsed: MidiFileShape = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(_) => {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Midi,
                        code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                        recommended_action:
                            "midi mappings file shape did not match the supported midi \
                             schema; review the reference user-data midi mappings file"
                                .to_string(),
                        dry_run: true,
                    });
                    continue;
                }
            };

            let is_global = path.file_name().and_then(|s| s.to_str()) == Some("global.json");

            let mut converted: Vec<ImportedMidiBinding> = Vec::new();
            for binding in parsed.bindings {
                let Ok(channel_u8) = u8::try_from(binding.channel) else {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Midi,
                        code: ImportDiagnosticCode::OutlierControlType,
                        recommended_action: format!(
                            "midi binding channel `{}` is outside the supported 0..=15 \
                             range; binding dropped",
                            binding.channel
                        ),
                        dry_run: true,
                    });
                    continue;
                };
                if channel_u8 > 15 {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Midi,
                        code: ImportDiagnosticCode::OutlierControlType,
                        recommended_action: format!(
                            "midi binding channel `{channel_u8}` is outside the supported \
                             0..=15 range; binding dropped"
                        ),
                        dry_run: true,
                    });
                    continue;
                }

                let source = match (binding.cc, binding.note) {
                    (Some(cc), None) => ImportedMidiSource::ControlChange { cc },
                    (None, Some(note)) => ImportedMidiSource::Note { note },
                    (Some(cc), Some(note)) => {
                        diagnostics.push(ImportDiagnostic {
                            source_path: path.clone(),
                            class: ArtifactClass::Midi,
                            code: ImportDiagnosticCode::OutlierControlType,
                            recommended_action: format!(
                                "midi binding carried both `cc` ({cc}) and `note` ({note}); \
                                 preferring cc per import policy"
                            ),
                            dry_run: true,
                        });
                        ImportedMidiSource::ControlChange { cc }
                    }
                    (None, None) => {
                        diagnostics.push(ImportDiagnostic {
                            source_path: path.clone(),
                            class: ArtifactClass::Midi,
                            code: ImportDiagnosticCode::OutlierControlType,
                            recommended_action:
                                "midi binding carried neither `cc` nor `note` source; \
                                 binding dropped"
                                    .to_string(),
                            dry_run: true,
                        });
                        continue;
                    }
                };

                if binding.device_id.is_none() {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Midi,
                        code: ImportDiagnosticCode::MidiMappingDeviceMissing,
                        recommended_action:
                            "midi binding did not carry a device id; binding preserved \
                             and will auto-bind when the device appears on the importing \
                             machine"
                                .to_string(),
                        dry_run: true,
                    });
                }

                converted.push(ImportedMidiBinding {
                    device_id: binding.device_id,
                    channel: channel_u8,
                    source,
                    target_control: binding.target_control,
                    soft_takeover: binding.soft_takeover.unwrap_or(false),
                    invert: binding.invert.unwrap_or(false),
                    min: binding.min.unwrap_or(0.0),
                    max: binding.max.unwrap_or(1.0),
                });
            }

            if is_global {
                imported.global.extend(converted);
            } else if let Some(scene_name) = parsed.scene {
                imported
                    .per_scene
                    .entry(scene_name)
                    .or_default()
                    .extend(converted);
            } else {
                // Per-scene file with no `scene` field — flag as
                // unsupported schema and skip.
                diagnostics.push(ImportDiagnostic {
                    source_path: path.clone(),
                    class: ArtifactClass::Midi,
                    code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                    recommended_action:
                        "per-scene midi mappings file did not carry a `scene` field; \
                         file dropped"
                            .to_string(),
                    dry_run: true,
                });
            }
        }

        Ok((imported, diagnostics))
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-REFERENCE-SHOW (T1) — Phase-4b Wave 4b.7
    //! PR 3 of N MIDI-mappings importer per ADR-015 §Scope line 25 +
    //! §Mechanism §3 and `docs/26-evidence-artifact-registry.md` row
    //! 85.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! pins covering per-scene CC and global note round-trips,
    //! missing-device-id diagnostic posture, channel-out-of-range
    //! drop, MIN/MAX default semantics, and soft-takeover + invert
    //! preservation. The multi-fixture corpus pairs MIDI with the
    //! sibling OSC + hotkey importers in
    //! `tests/import_reference_midi_osc_hotkey_corpus.rs` at T2.

    use super::*;
    use crate::import_reference::override_reference_user_data_root;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_scratch_dir() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let idx = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chromaticity-import-reference-midi-{pid}-{nanos}-{idx}"
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a per-scene file with one CC
    // binding round-trips with channel + cc + target_control + the
    // explicit soft_takeover / invert / min / max preserved.
    #[test]
    fn roundtrip_per_scene_cc_binding() {
        let dir = unique_scratch_dir();
        let midi_dir = dir.join("MidiMappings");
        std::fs::create_dir_all(&midi_dir).unwrap();
        std::fs::write(
            midi_dir.join("scene-a.json"),
            r#"{
                "scene": "scene-a",
                "bindings": [
                    {
                        "device_id": "dev-1",
                        "channel": 3,
                        "cc": 17,
                        "target_control": "intensity",
                        "soft_takeover": true,
                        "invert": false,
                        "min": 0.1,
                        "max": 0.9
                    }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = MidiImporter::dry_run(&root).unwrap();

        assert!(diagnostics.is_empty());
        let scene = imported.per_scene.get("scene-a").unwrap();
        assert_eq!(scene.len(), 1);
        let b = &scene[0];
        assert_eq!(b.device_id.as_deref(), Some("dev-1"));
        assert_eq!(b.channel, 3);
        assert_eq!(b.source, ImportedMidiSource::ControlChange { cc: 17 });
        assert_eq!(b.target_control, "intensity");
        assert!(b.soft_takeover);
        assert!(!b.invert);
        assert!((b.min - 0.1).abs() < 1e-6);
        assert!((b.max - 0.9).abs() < 1e-6);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a `global.json` file populates
    // ImportedMidi::global, preserving note bindings.
    #[test]
    fn roundtrip_global_note_binding() {
        let dir = unique_scratch_dir();
        let midi_dir = dir.join("MidiMappings");
        std::fs::create_dir_all(&midi_dir).unwrap();
        std::fs::write(
            midi_dir.join("global.json"),
            r#"{
                "bindings": [
                    {
                        "device_id": "dev-2",
                        "channel": 0,
                        "note": 60,
                        "target_control": "global.tap"
                    }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = MidiImporter::dry_run(&root).unwrap();

        assert!(diagnostics.is_empty());
        assert!(imported.per_scene.is_empty());
        assert_eq!(imported.global.len(), 1);
        let b = &imported.global[0];
        assert_eq!(b.source, ImportedMidiSource::Note { note: 60 });
        assert_eq!(b.target_control, "global.tap");

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a binding without a device_id
    // emits a MidiMappingDeviceMissing diagnostic AND the binding is
    // still imported (auto-bind-when-device-appears semantics per
    // ADR-015 line 68).
    #[test]
    fn missing_device_id_emits_diagnostic_but_preserves_binding() {
        let dir = unique_scratch_dir();
        let midi_dir = dir.join("MidiMappings");
        std::fs::create_dir_all(&midi_dir).unwrap();
        let path = midi_dir.join("scene-b.json");
        std::fs::write(
            &path,
            r#"{
                "scene": "scene-b",
                "bindings": [
                    {
                        "channel": 1,
                        "cc": 5,
                        "target_control": "x"
                    }
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
        assert_eq!(diagnostics[0].source_path, path);
        assert!(
            diagnostics[0]
                .recommended_action
                .contains("auto-bind when the device appears")
        );

        let scene = imported.per_scene.get("scene-b").unwrap();
        assert_eq!(scene.len(), 1);
        assert!(scene[0].device_id.is_none());

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a binding with channel outside
    // 0..=15 emits an OutlierControlType diagnostic and is dropped;
    // valid sibling bindings remain.
    #[test]
    fn out_of_range_channel_emits_diagnostic_and_skips() {
        let dir = unique_scratch_dir();
        let midi_dir = dir.join("MidiMappings");
        std::fs::create_dir_all(&midi_dir).unwrap();
        let path = midi_dir.join("scene-c.json");
        std::fs::write(
            &path,
            r#"{
                "scene": "scene-c",
                "bindings": [
                    { "device_id": "d", "channel": 0,  "cc": 1, "target_control": "ok" },
                    { "device_id": "d", "channel": 16, "cc": 2, "target_control": "bad" },
                    { "device_id": "d", "channel": -1, "cc": 3, "target_control": "neg" }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = MidiImporter::dry_run(&root).unwrap();

        assert_eq!(diagnostics.len(), 2);
        for diag in &diagnostics {
            assert_eq!(diag.class, ArtifactClass::Midi);
            assert_eq!(diag.code, ImportDiagnosticCode::OutlierControlType);
            assert_eq!(diag.source_path, path);
        }

        let scene = imported.per_scene.get("scene-c").unwrap();
        assert_eq!(scene.len(), 1);
        assert_eq!(scene[0].target_control, "ok");

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: when min / max are absent in the
    // source file the importer fills them with the documented
    // defaults (0.0 / 1.0) per ADR-015 line 25.
    #[test]
    fn min_max_default_to_zero_one_when_absent() {
        let dir = unique_scratch_dir();
        let midi_dir = dir.join("MidiMappings");
        std::fs::create_dir_all(&midi_dir).unwrap();
        std::fs::write(
            midi_dir.join("scene-d.json"),
            r#"{
                "scene": "scene-d",
                "bindings": [
                    {
                        "device_id": "dev-x",
                        "channel": 4,
                        "cc": 7,
                        "target_control": "y"
                    }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = MidiImporter::dry_run(&root).unwrap();
        assert!(diagnostics.is_empty());
        let b = &imported.per_scene.get("scene-d").unwrap()[0];
        assert!((b.min - 0.0).abs() < f32::EPSILON);
        assert!((b.max - 1.0).abs() < f32::EPSILON);
        assert!(!b.soft_takeover);
        assert!(!b.invert);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: soft_takeover + invert flags
    // round-trip true and false explicitly.
    #[test]
    fn soft_takeover_invert_round_trip() {
        let dir = unique_scratch_dir();
        let midi_dir = dir.join("MidiMappings");
        std::fs::create_dir_all(&midi_dir).unwrap();
        std::fs::write(
            midi_dir.join("scene-e.json"),
            r#"{
                "scene": "scene-e",
                "bindings": [
                    {
                        "device_id": "dev-1",
                        "channel": 0, "cc": 1, "target_control": "a",
                        "soft_takeover": true, "invert": true
                    },
                    {
                        "device_id": "dev-1",
                        "channel": 0, "cc": 2, "target_control": "b",
                        "soft_takeover": false, "invert": false
                    }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = MidiImporter::dry_run(&root).unwrap();
        assert!(diagnostics.is_empty());
        let scene = imported.per_scene.get("scene-e").unwrap();
        assert_eq!(scene.len(), 2);
        assert!(scene[0].soft_takeover);
        assert!(scene[0].invert);
        assert!(!scene[1].soft_takeover);
        assert!(!scene[1].invert);

        cleanup(&dir);
    }
}

// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Scene-specific presets importer (ADR-015 §Scope line 21 +
//! §Mechanism line 43).
//!
//! Phase-4b Wave 4b.7 PR 1 of N — dry-run-only presets importer.
//! Reads `<root>/Presets/*.json` (canonical reference layout — an
//! observable file-system structure, not source-code-derivative).
//! Each per-scene preset file contains:
//!
//! - `scene` — the scene name the presets target.
//! - `presets` — array of `{ name, controls: { <name>: <value> } }`.
//! - `startup_pointer` — optional name of the startup preset for
//!   the scene.
//!
//! Output is keyed by scene name in [`ImportedPresets::per_scene`];
//! startup pointers are aggregated in [`ImportedPresets::startup_pointers`].
//! The importer is dry-run only and never mutates the source root
//! per ADR-015 §Mechanism §7.
//!
//! # Library-cross-check is deferred
//!
//! ADR-015 §Diagnostics line 58 defines the `unknown-scene`
//! diagnostic as "preset or Snap references a scene name not
//! present in the Chromaticity library". PR 1 of this wave does
//! NOT cross-check against the live `SceneIndex` (the
//! `app/shell/src/commands/library_index.rs` surface) — the
//! library cross-check requires plumbing a `SceneIndex` (or a
//! library-name set abstraction) through the importer surface,
//! which lands in a later PR of this wave when the commit harness
//! composes per-class importers against a real library snapshot.
//!
//! For PR 1 the cross-check accepts a caller-provided
//! `&HashSet<String>` of "known scene names" (empty in PR 1 tests
//! — meaning every scene name is "unknown" — and the diagnostic
//! semantics still exercise correctly). When the live cross-check
//! lands, the call sites flip from the empty set to the real
//! library snapshot without changing the importer's interface.
//!
//! # Coercion
//!
//! ADR-015 §Mechanism line 43 references CP-01 normalization. The
//! PR 1 surface includes a minimal coercion-applied diagnostic
//! emission for control values that fall outside the unit
//! interval `[0.0, 1.0]` (the only normalization rule a Wave 4b.7
//! PR 1 importer can exercise without the live scene schema; full
//! CR-01..04 coercion handling lands in a later PR). Out-of-range
//! values are clamped to the unit interval and the diagnostic
//! fires once per offending control name.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::diagnostics::{ArtifactClass, ImportDiagnostic, ImportDiagnosticCode};
use super::discovery::ReferenceUserDataRoot;

/// Single named preset for a scene per ADR-015 line 21.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedPreset {
    /// Operator-supplied preset name.
    pub name: String,
    /// Per-control values, keyed by control name. Values are
    /// clamped to the unit interval `[0.0, 1.0]` per the PR 1
    /// minimal coercion rule documented at module scope.
    pub control_values: HashMap<String, f32>,
}

/// Aggregate output of the presets importer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportedPresets {
    /// Per-scene preset list keyed by scene name.
    pub per_scene: HashMap<String, Vec<ImportedPreset>>,
    /// Startup preset pointers keyed by scene name → preset name.
    pub startup_pointers: HashMap<String, String>,
}

/// Errors produced by the presets importer.
#[derive(Debug)]
pub enum PresetsImporterError {
    /// Filesystem error while reading the presets directory.
    IoError(std::io::Error),
}

impl std::fmt::Display for PresetsImporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "presets importer I/O error: {e}"),
        }
    }
}

impl std::error::Error for PresetsImporterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
        }
    }
}

/// Wire-shape of a single preset file inside `<root>/Presets/`.
#[derive(Deserialize)]
struct PresetFileShape {
    scene: String,
    presets: Vec<PresetEntryShape>,
    #[serde(default)]
    startup_pointer: Option<String>,
}

#[derive(Deserialize)]
struct PresetEntryShape {
    name: String,
    controls: HashMap<String, f32>,
}

/// Presets importer surface.
///
/// PR 1 of N ships [`PresetsImporter::dry_run`] only. The commit
/// harness composes this with sibling per-class importers in PR 4
/// of this wave.
pub struct PresetsImporter;

impl PresetsImporter {
    fn presets_dir(root: &ReferenceUserDataRoot) -> PathBuf {
        root.path().join("Presets")
    }

    /// Dry-run a presets import against `root`. Walks
    /// `<root>/Presets/*.json` and parses each file. Cross-checks
    /// scene names against `known_scenes` (empty set means every
    /// scene name is "unknown"; see module-level prose for the
    /// library-cross-check deferral). Never mutates the source
    /// root.
    pub fn dry_run(
        root: &ReferenceUserDataRoot,
        known_scenes: &HashSet<String>,
    ) -> Result<(ImportedPresets, Vec<ImportDiagnostic>), PresetsImporterError> {
        let dir = Self::presets_dir(root);
        let mut imported = ImportedPresets::default();
        let mut diagnostics: Vec<ImportDiagnostic> = Vec::new();

        match dir.try_exists() {
            Ok(false) => return Ok((imported, diagnostics)),
            Err(e) => return Err(PresetsImporterError::IoError(e)),
            Ok(true) => {}
        }

        let read_dir = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(e) => return Err(PresetsImporterError::IoError(e)),
        };

        // Sort entries for deterministic diagnostic ordering across
        // platforms (read_dir order is not specified).
        let mut entries: Vec<PathBuf> = Vec::new();
        for entry in read_dir {
            let entry = entry.map_err(PresetsImporterError::IoError)?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                entries.push(path);
            }
        }
        entries.sort();

        for path in entries {
            let raw = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => return Err(PresetsImporterError::IoError(e)),
            };

            let parsed: PresetFileShape = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(_) => {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Preset,
                        code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                        recommended_action:
                            "preset file shape did not match the supported preset schema; \
                             review the reference user-data preset file"
                                .to_string(),
                        dry_run: true,
                    });
                    continue;
                }
            };

            let scene_name = parsed.scene;
            let scene_known = known_scenes.contains(&scene_name);
            if !scene_known {
                diagnostics.push(ImportDiagnostic {
                    source_path: path.clone(),
                    class: ArtifactClass::Preset,
                    code: ImportDiagnosticCode::UnknownScene,
                    recommended_action: format!(
                        "preset file references scene `{scene_name}` not present in the \
                         Chromaticity library; import the matching scene first or move \
                         the presets manually"
                    ),
                    dry_run: true,
                });
                // Still record the imports so the operator can see
                // what would land.
            }

            let mut entries_for_scene: Vec<ImportedPreset> = Vec::new();
            for entry in parsed.presets {
                let mut clamped: HashMap<String, f32> = HashMap::new();
                for (control_name, raw_value) in entry.controls {
                    let clamped_value = raw_value.clamp(0.0_f32, 1.0_f32);
                    let coerced = (clamped_value - raw_value).abs() > f32::EPSILON;
                    if coerced {
                        diagnostics.push(ImportDiagnostic {
                            source_path: path.clone(),
                            class: ArtifactClass::Preset,
                            code: ImportDiagnosticCode::CoercionApplied,
                            recommended_action: format!(
                                "control `{control_name}` value out of unit interval; clamped \
                                 to [0.0, 1.0] per CP-01 normalization rules"
                            ),
                            dry_run: true,
                        });
                    }
                    clamped.insert(control_name, clamped_value);
                }
                entries_for_scene.push(ImportedPreset {
                    name: entry.name,
                    control_values: clamped,
                });
            }

            imported
                .per_scene
                .entry(scene_name.clone())
                .or_default()
                .extend(entries_for_scene);

            if let Some(startup) = parsed.startup_pointer {
                imported.startup_pointers.insert(scene_name, startup);
            }
        }

        Ok((imported, diagnostics))
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-REFERENCE-SHOW (T1) — Phase-4b Wave 4b.7
    //! PR 1 of N foundation presets importer per ADR-015 §Scope
    //! line 21 + §Mechanism line 43 and
    //! `docs/26-evidence-artifact-registry.md` row 85.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! pins covering missing-presets-dir no-diagnostic posture,
    //! unknown-scene diagnostic emission against an empty
    //! known-scenes set, coercion-applied emission for outlier
    //! control values, and a small two-scene round-trip. The
    //! multi-fixture corpus pairing diagnostics with end-to-end
    //! synthetic preset shapes lives in
    //! `tests/import_reference_foundation_corpus.rs` at T2.

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
            "chromaticity-import-reference-presets-{pid}-{nanos}-{idx}"
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: missing presets dir is
    // not-an-error and produces empty imports + no diagnostics.
    #[test]
    fn presets_dir_missing_returns_empty_imports() {
        let dir = unique_scratch_dir();
        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let known: HashSet<String> = HashSet::new();
        let (imported, diagnostics) = PresetsImporter::dry_run(&root, &known).unwrap();

        assert!(imported.per_scene.is_empty());
        assert!(imported.startup_pointers.is_empty());
        assert!(diagnostics.is_empty());

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a preset referencing a scene
    // not in the known set emits an UnknownScene diagnostic per
    // ADR-015 §Diagnostics line 58. The import still records the
    // preset entries so the operator sees the would-land state.
    #[test]
    fn unknown_scene_emits_diagnostic() {
        let dir = unique_scratch_dir();
        let presets_dir = dir.join("Presets");
        std::fs::create_dir_all(&presets_dir).unwrap();
        let path = presets_dir.join("scene-a.json");
        std::fs::write(
            &path,
            r#"{
                "scene": "scene-not-in-library",
                "presets": [
                    { "name": "calm", "controls": { "intensity": 0.25 } }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let known: HashSet<String> = HashSet::new();
        let (imported, diagnostics) = PresetsImporter::dry_run(&root, &known).unwrap();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].class, ArtifactClass::Preset);
        assert_eq!(diagnostics[0].code, ImportDiagnosticCode::UnknownScene);
        assert_eq!(diagnostics[0].source_path, path);
        assert!(diagnostics[0].dry_run);
        assert_eq!(
            imported.per_scene.get("scene-not-in-library").map(Vec::len),
            Some(1)
        );

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: out-of-unit-interval control
    // values emit CoercionApplied diagnostics and clamp.
    #[test]
    fn coercion_applied_diagnostic_for_outlier_value() {
        let dir = unique_scratch_dir();
        let presets_dir = dir.join("Presets");
        std::fs::create_dir_all(&presets_dir).unwrap();
        let path = presets_dir.join("scene-a.json");
        std::fs::write(
            &path,
            r#"{
                "scene": "scene-a",
                "presets": [
                    { "name": "loud", "controls": { "intensity": 1.5 } }
                ]
            }"#,
        )
        .unwrap();

        let mut known: HashSet<String> = HashSet::new();
        known.insert("scene-a".to_string());

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = PresetsImporter::dry_run(&root, &known).unwrap();

        let coercion_count = diagnostics
            .iter()
            .filter(|d| d.code == ImportDiagnosticCode::CoercionApplied)
            .count();
        assert_eq!(coercion_count, 1);

        let scene_presets = imported
            .per_scene
            .get("scene-a")
            .expect("scene-a should be present");
        let preset = &scene_presets[0];
        let clamped = preset.control_values.get("intensity").copied().unwrap();
        assert!((clamped - 1.0).abs() < f32::EPSILON);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: two-scene three-preset round-
    // trip exercises per_scene aggregation + startup pointer
    // capture without any diagnostic emissions when scenes are
    // known and values are in-range.
    #[test]
    fn roundtrip_two_scenes_three_presets_each() {
        let dir = unique_scratch_dir();
        let presets_dir = dir.join("Presets");
        std::fs::create_dir_all(&presets_dir).unwrap();

        std::fs::write(
            presets_dir.join("scene-a.json"),
            r#"{
                "scene": "scene-a",
                "presets": [
                    { "name": "calm",  "controls": { "intensity": 0.1 } },
                    { "name": "med",   "controls": { "intensity": 0.5 } },
                    { "name": "loud",  "controls": { "intensity": 0.9 } }
                ],
                "startup_pointer": "calm"
            }"#,
        )
        .unwrap();

        std::fs::write(
            presets_dir.join("scene-b.json"),
            r#"{
                "scene": "scene-b",
                "presets": [
                    { "name": "p1", "controls": { "x": 0.0 } },
                    { "name": "p2", "controls": { "x": 0.5 } },
                    { "name": "p3", "controls": { "x": 1.0 } }
                ]
            }"#,
        )
        .unwrap();

        let mut known: HashSet<String> = HashSet::new();
        known.insert("scene-a".to_string());
        known.insert("scene-b".to_string());

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = PresetsImporter::dry_run(&root, &known).unwrap();

        assert!(diagnostics.is_empty());
        assert_eq!(imported.per_scene.get("scene-a").map(Vec::len), Some(3));
        assert_eq!(imported.per_scene.get("scene-b").map(Vec::len), Some(3));
        assert_eq!(
            imported.startup_pointers.get("scene-a").map(String::as_str),
            Some("calm")
        );
        assert!(!imported.startup_pointers.contains_key("scene-b"));

        cleanup(&dir);
    }
}

// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Snap slots importer (ADR-015 §Scope line 22 + §Mechanism §3).
//!
//! Phase-4b Wave 4b.7 PR 2 of N — dry-run-only Snap-slots importer.
//! Reads `<root>/Favslots/*.json` — the source-side directory
//! literal stays `Favslots` because that is the on-disk name to
//! read; the user-facing Chromaticity surface continues to use the
//! renamed "Snap" vocabulary (the bundle v0.2-beta sweep). This
//! module does not name any reference-app product; the read-side
//! path literal is observable file-system data per ADR-015 line 9
//! and is permitted.
//!
//! Each per-scene Snap file contains:
//!
//! - `scene` — the scene name the slots target.
//! - `slots` — an array of zero or more
//!   `{ "index": <0..=15>, "user_name": <string?>,
//!      "controls": { <name>: <value> } }` entries.
//! - `gold_slot_pointer` — optional 0..=15 pointer to the gold slot
//!   for the scene; defaults to `Some(0)` per ADR-015 line 22 when
//!   absent (gold-slot-0 default semantics).
//!
//! Output is keyed by scene name in [`ImportedSnap::per_scene`];
//! per-scene shape is [`ImportedSceneSnap`] with a fixed-size
//! `[Option<ImportedSlot>; 16]` array indexed 0-15. The importer is
//! dry-run only and never mutates the source root per ADR-015
//! §Mechanism §7.
//!
//! # Diagnostics
//!
//! - Slot index outside `0..=15` → emits an
//!   [`ImportDiagnosticCode::OutlierControlType`] diagnostic. The
//!   ADR-015 §Diagnostics line 56 vocabulary does not have a
//!   dedicated "out-of-range slot index" code; `OutlierControlType`
//!   is the closest matching variant (a malformed shape that does
//!   not fit the canonical CP-01-style structural set), which keeps
//!   the diagnostic surface in the existing eleven-variant
//!   vocabulary without widening the enum mid-wave. The full slot
//!   entry is dropped from the imports surface; the diagnostic
//!   carries the offending index in `recommended_action`.
//! - Malformed JSON → emits a single
//!   [`ImportDiagnosticCode::UnsupportedSchemaVersion`] diagnostic
//!   per ADR-015 §Versioning line 76.
//! - Missing `<root>/Favslots/` directory → returns an empty
//!   [`ImportedSnap`] with no diagnostics; this is the
//!   not-an-error case (the operator may not have used Snap slots).
//!
//! # Gold-slot pointer default
//!
//! Per ADR-015 line 22 the gold-slot pointer defaults to slot 0 when
//! absent. The importer sets `gold_slot_pointer = Some(0)` for files
//! that do not carry the field, preserving the explicit-vs-default
//! posture round-trip.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::diagnostics::{ArtifactClass, ImportDiagnostic, ImportDiagnosticCode};
use super::discovery::ReferenceUserDataRoot;

/// Single Snap slot entry per ADR-015 line 22.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedSlot {
    /// Operator-supplied per-slot user name when present.
    pub user_name: Option<String>,
    /// Per-control values captured at slot-store time.
    pub control_values: HashMap<String, f32>,
}

/// Per-scene Snap-slot collection per ADR-015 line 22.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedSceneSnap {
    /// Fixed 16-slot array indexed 0-15. `None` indicates the slot
    /// was empty in the reference user-data file.
    pub slots: [Option<ImportedSlot>; 16],
    /// Gold-slot pointer per ADR-015 line 22. Defaults to `Some(0)`
    /// when absent in the source file.
    pub gold_slot_pointer: Option<u8>,
}

impl Default for ImportedSceneSnap {
    fn default() -> Self {
        Self {
            slots: Default::default(),
            gold_slot_pointer: Some(0),
        }
    }
}

/// Aggregate output of the Snap importer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportedSnap {
    /// Per-scene Snap-slot collection keyed by scene name.
    pub per_scene: HashMap<String, ImportedSceneSnap>,
}

/// Errors produced by the Snap importer.
#[derive(Debug)]
pub enum ImporterError {
    /// Filesystem error while reading the source directory.
    IoError(std::io::Error),
}

impl std::fmt::Display for ImporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "snap importer I/O error: {e}"),
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

/// Wire-shape of a single Snap file under `<root>/Favslots/`.
#[derive(Deserialize)]
struct SnapFileShape {
    scene: String,
    #[serde(default)]
    slots: Vec<SnapSlotShape>,
    #[serde(default)]
    gold_slot_pointer: Option<u8>,
}

#[derive(Deserialize)]
struct SnapSlotShape {
    index: i64,
    #[serde(default)]
    user_name: Option<String>,
    #[serde(default)]
    controls: HashMap<String, f32>,
}

/// Snap importer surface.
pub struct SnapImporter;

impl SnapImporter {
    fn snap_dir(root: &ReferenceUserDataRoot) -> PathBuf {
        // Source-side on-disk directory literal per ADR-015 line 22;
        // see module-level prose for the read-side-only naming
        // posture (Chromaticity UI continues to use the renamed
        // "Snap" vocabulary).
        root.path().join("Favslots")
    }

    /// Dry-run a Snap import against `root`. Walks
    /// `<root>/Favslots/*.json` and parses each per-scene file.
    /// Never mutates the source root.
    pub fn dry_run(
        root: &ReferenceUserDataRoot,
    ) -> Result<(ImportedSnap, Vec<ImportDiagnostic>), ImporterError> {
        let dir = Self::snap_dir(root);
        let mut imported = ImportedSnap::default();
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

            let parsed: SnapFileShape = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(_) => {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Snap,
                        code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                        recommended_action:
                            "snap file shape did not match the supported snap schema; \
                             review the reference user-data snap file"
                                .to_string(),
                        dry_run: true,
                    });
                    continue;
                }
            };

            let scene_name = parsed.scene;
            let mut scene_snap = ImportedSceneSnap::default();
            // `default()` set gold_slot_pointer to Some(0); override
            // only if the source file carried an explicit value.
            if let Some(ptr) = parsed.gold_slot_pointer {
                scene_snap.gold_slot_pointer = Some(ptr);
            }

            for slot in parsed.slots {
                let Ok(idx_u8) = u8::try_from(slot.index) else {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Snap,
                        code: ImportDiagnosticCode::OutlierControlType,
                        recommended_action: format!(
                            "snap slot index `{}` is outside the supported 0..=15 range; \
                             slot dropped",
                            slot.index
                        ),
                        dry_run: true,
                    });
                    continue;
                };
                if idx_u8 > 15 {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Snap,
                        code: ImportDiagnosticCode::OutlierControlType,
                        recommended_action: format!(
                            "snap slot index `{idx_u8}` is outside the supported 0..=15 \
                             range; slot dropped"
                        ),
                        dry_run: true,
                    });
                    continue;
                }
                let imported_slot = ImportedSlot {
                    user_name: slot.user_name,
                    control_values: slot.controls,
                };
                scene_snap.slots[idx_u8 as usize] = Some(imported_slot);
            }

            imported.per_scene.insert(scene_name, scene_snap);
        }

        Ok((imported, diagnostics))
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-REFERENCE-SHOW (T1) — Phase-4b Wave 4b.7
    //! PR 2 of N Snap-slots importer per ADR-015 §Scope line 22 +
    //! §Mechanism §3 and `docs/26-evidence-artifact-registry.md` row
    //! 85.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! pins covering gold-slot-pointer default round-trip, slot
    //! index range validation, per-slot user-name preservation, and
    //! missing-dir no-diagnostic posture. The multi-fixture corpus
    //! pairing diagnostics with end-to-end synthetic snap shapes
    //! lives in
    //! `tests/import_reference_snap_playlists_meta_corpus.rs` at T2.

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
            "chromaticity-import-reference-snap-{pid}-{nanos}-{idx}"
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a snap file omitting the
    // gold_slot_pointer field round-trips with gold_slot_pointer =
    // Some(0) per ADR-015 line 22 default semantics; an explicit
    // value overrides the default.
    #[test]
    fn gold_slot_pointer_zero_default_round_trip() {
        let dir = unique_scratch_dir();
        let snap_dir = dir.join("Favslots");
        std::fs::create_dir_all(&snap_dir).unwrap();
        std::fs::write(
            snap_dir.join("scene-default.json"),
            r#"{
                "scene": "scene-default",
                "slots": [
                    { "index": 0, "controls": { "intensity": 0.4 } }
                ]
            }"#,
        )
        .unwrap();
        std::fs::write(
            snap_dir.join("scene-explicit.json"),
            r#"{
                "scene": "scene-explicit",
                "slots": [],
                "gold_slot_pointer": 7
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = SnapImporter::dry_run(&root).unwrap();
        assert!(diagnostics.is_empty());

        let default_scene = imported.per_scene.get("scene-default").unwrap();
        assert_eq!(default_scene.gold_slot_pointer, Some(0));
        assert!(default_scene.slots[0].is_some());

        let explicit_scene = imported.per_scene.get("scene-explicit").unwrap();
        assert_eq!(explicit_scene.gold_slot_pointer, Some(7));

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: slot index outside 0..=15 emits
    // an OutlierControlType diagnostic and the offending entry is
    // dropped; valid sibling slots remain.
    #[test]
    fn slot_index_out_of_range_emits_diagnostic() {
        let dir = unique_scratch_dir();
        let snap_dir = dir.join("Favslots");
        std::fs::create_dir_all(&snap_dir).unwrap();
        let path = snap_dir.join("scene-out-of-range.json");
        std::fs::write(
            &path,
            r#"{
                "scene": "scene-out-of-range",
                "slots": [
                    { "index": 0,  "controls": { "x": 0.1 } },
                    { "index": 16, "controls": { "x": 0.2 } },
                    { "index": -1, "controls": { "x": 0.3 } }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = SnapImporter::dry_run(&root).unwrap();

        assert_eq!(diagnostics.len(), 2);
        for diag in &diagnostics {
            assert_eq!(diag.class, ArtifactClass::Snap);
            assert_eq!(diag.code, ImportDiagnosticCode::OutlierControlType);
            assert!(diag.dry_run);
            assert_eq!(diag.source_path, path);
        }

        let scene = imported.per_scene.get("scene-out-of-range").unwrap();
        assert!(scene.slots[0].is_some());
        // The 16-slot array couldn't accept index 16 or -1; both
        // dropped.
        let occupied = scene.slots.iter().filter(|s| s.is_some()).count();
        assert_eq!(occupied, 1);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: per-slot user_name field is
    // preserved verbatim when present; absent user_name yields
    // None.
    #[test]
    fn per_slot_user_name_preserved_when_present() {
        let dir = unique_scratch_dir();
        let snap_dir = dir.join("Favslots");
        std::fs::create_dir_all(&snap_dir).unwrap();
        std::fs::write(
            snap_dir.join("scene-named.json"),
            r#"{
                "scene": "scene-named",
                "slots": [
                    { "index": 0, "user_name": "calm",  "controls": { "x": 0.0 } },
                    { "index": 1,                       "controls": { "x": 0.5 } }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = SnapImporter::dry_run(&root).unwrap();
        assert!(diagnostics.is_empty());

        let scene = imported.per_scene.get("scene-named").unwrap();
        let slot0 = scene.slots[0].as_ref().unwrap();
        assert_eq!(slot0.user_name.as_deref(), Some("calm"));
        let slot1 = scene.slots[1].as_ref().unwrap();
        assert_eq!(slot1.user_name, None);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: missing Favslots/ dir is
    // not-an-error and produces empty imports + no diagnostics.
    #[test]
    fn missing_favslots_dir_returns_empty_imports() {
        let dir = unique_scratch_dir();
        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = SnapImporter::dry_run(&root).unwrap();

        assert!(imported.per_scene.is_empty());
        assert!(diagnostics.is_empty());

        cleanup(&dir);
    }
}

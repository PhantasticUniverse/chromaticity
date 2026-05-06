// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Meta + Media preset channels importer (ADR-015 §Scope line 23 +
//! §Mechanism §3).
//!
//! Phase-4b Wave 4b.7 PR 2 of N — dry-run-only meta+media preset
//! channel importer. Reads two sibling directories under the
//! reference user-data root:
//!
//! - `<root>/MetaPresets/*.json` — meta-control preset payloads.
//!   Each file carries `{ "name": <string>, "controls": { <name>:
//!   <f32> } }`. Maps directly to [`ImportedMetaPreset`].
//! - `<root>/MediaPresets/*.json` — media settings preset payloads.
//!   Media settings vary widely across reference-app installs (clip
//!   pointers, playback parameters, file paths, codec preferences,
//!   et al), so the importer preserves the body verbatim as
//!   `serde_json::Value` in [`ImportedMediaPreset::settings`].
//!   Each file carries `{ "name": <string>, "settings": <object> }`.
//!
//! Meta + Media presets are channel-scoped (CP-05 channel model);
//! they are NOT keyed by scene like the per-scene Snap and preset
//! collections. The output is two named-entry HashMaps keyed by
//! preset name.
//!
//! The importer is dry-run only and never mutates the source root
//! per ADR-015 §Mechanism §7.
//!
//! # Diagnostics
//!
//! - Malformed JSON in either directory → emits a single
//!   [`ImportDiagnosticCode::UnsupportedSchemaVersion`] diagnostic
//!   for the offending file (per-file scope; sibling files in the
//!   same directory continue to import). Diagnostics for meta and
//!   media files share the same diagnostic code but are
//!   distinguished by their `source_path`.
//! - Missing `<root>/MetaPresets/` or `<root>/MediaPresets/` → no
//!   diagnostic; the corresponding side of the output stays empty.
//!   This is the not-an-error case (the operator may have used
//!   only one of the two channels).

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::diagnostics::{ArtifactClass, ImportDiagnostic, ImportDiagnosticCode};
use super::discovery::ReferenceUserDataRoot;

/// Single meta-channel preset per ADR-015 line 23.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedMetaPreset {
    /// Operator-supplied preset name (also the HashMap key).
    pub name: String,
    /// Per-control values captured at preset-store time.
    pub control_values: HashMap<String, f32>,
}

/// Single media-channel preset per ADR-015 line 23.
///
/// Media settings vary widely; the body is preserved as
/// `serde_json::Value` so the commit harness can transform on
/// cross-side schema understanding rather than the importer
/// pre-collapsing fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedMediaPreset {
    /// Operator-supplied preset name (also the HashMap key).
    pub name: String,
    /// Verbatim media-settings object.
    pub settings: serde_json::Value,
}

/// Aggregate output of the Meta+Media importer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportedMetaMedia {
    /// Meta-channel presets keyed by preset name.
    pub meta: HashMap<String, ImportedMetaPreset>,
    /// Media-channel presets keyed by preset name.
    pub media: HashMap<String, ImportedMediaPreset>,
}

/// Errors produced by the Meta+Media importer.
#[derive(Debug)]
pub enum ImporterError {
    /// Filesystem error while reading either source directory.
    IoError(std::io::Error),
}

impl std::fmt::Display for ImporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "meta/media importer I/O error: {e}"),
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

#[derive(Deserialize)]
struct MetaFileShape {
    name: String,
    #[serde(default)]
    controls: HashMap<String, f32>,
}

#[derive(Deserialize)]
struct MediaFileShape {
    name: String,
    #[serde(default)]
    settings: serde_json::Value,
}

/// Meta+Media importer surface.
pub struct MetaMediaImporter;

impl MetaMediaImporter {
    fn meta_dir(root: &ReferenceUserDataRoot) -> PathBuf {
        root.path().join("MetaPresets")
    }

    fn media_dir(root: &ReferenceUserDataRoot) -> PathBuf {
        root.path().join("MediaPresets")
    }

    /// Dry-run a Meta+Media import against `root`. Walks both
    /// `<root>/MetaPresets/*.json` and `<root>/MediaPresets/*.json`.
    /// Diagnostics from each side are independent; a malformed file
    /// in one side does not affect the other side. Never mutates
    /// the source root.
    pub fn dry_run(
        root: &ReferenceUserDataRoot,
    ) -> Result<(ImportedMetaMedia, Vec<ImportDiagnostic>), ImporterError> {
        let mut imported = ImportedMetaMedia::default();
        let mut diagnostics: Vec<ImportDiagnostic> = Vec::new();

        Self::walk_meta(&Self::meta_dir(root), &mut imported, &mut diagnostics)?;
        Self::walk_media(&Self::media_dir(root), &mut imported, &mut diagnostics)?;

        Ok((imported, diagnostics))
    }

    fn walk_meta(
        dir: &PathBuf,
        imported: &mut ImportedMetaMedia,
        diagnostics: &mut Vec<ImportDiagnostic>,
    ) -> Result<(), ImporterError> {
        match dir.try_exists() {
            Ok(false) => return Ok(()),
            Err(e) => return Err(ImporterError::IoError(e)),
            Ok(true) => {}
        }

        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(e) => return Err(ImporterError::IoError(e)),
        };

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
            let parsed: MetaFileShape = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(_) => {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Preset,
                        code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                        recommended_action:
                            "meta-channel preset file shape did not match the supported \
                             schema; review the reference user-data meta preset file"
                                .to_string(),
                        dry_run: true,
                    });
                    continue;
                }
            };
            imported.meta.insert(
                parsed.name.clone(),
                ImportedMetaPreset {
                    name: parsed.name,
                    control_values: parsed.controls,
                },
            );
        }

        Ok(())
    }

    fn walk_media(
        dir: &PathBuf,
        imported: &mut ImportedMetaMedia,
        diagnostics: &mut Vec<ImportDiagnostic>,
    ) -> Result<(), ImporterError> {
        match dir.try_exists() {
            Ok(false) => return Ok(()),
            Err(e) => return Err(ImporterError::IoError(e)),
            Ok(true) => {}
        }

        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(e) => return Err(ImporterError::IoError(e)),
        };

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
            let parsed: MediaFileShape = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(_) => {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Preset,
                        code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                        recommended_action:
                            "media-channel preset file shape did not match the supported \
                             schema; review the reference user-data media preset file"
                                .to_string(),
                        dry_run: true,
                    });
                    continue;
                }
            };
            imported.media.insert(
                parsed.name.clone(),
                ImportedMediaPreset {
                    name: parsed.name,
                    settings: parsed.settings,
                },
            );
        }

        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-REFERENCE-SHOW (T1) — Phase-4b Wave 4b.7
    //! PR 2 of N meta + media preset channel importer per ADR-015
    //! §Scope line 23 + §Mechanism §3 and
    //! `docs/26-evidence-artifact-registry.md` row 85.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! pins covering meta-preset round-trip, media-preset
    //! preserves-unknown-settings posture, missing-dirs
    //! no-diagnostic posture, and malformed-JSON
    //! UnsupportedSchemaVersion emission. The multi-fixture corpus
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
            "chromaticity-import-reference-meta-media-{pid}-{nanos}-{idx}"
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: meta-channel preset round-trips
    // through the importer with name + control values preserved.
    #[test]
    fn meta_preset_round_trip() {
        let dir = unique_scratch_dir();
        let meta_dir = dir.join("MetaPresets");
        std::fs::create_dir_all(&meta_dir).unwrap();
        std::fs::write(
            meta_dir.join("master.json"),
            r#"{
                "name": "master",
                "controls": { "gain": 0.7, "tilt": 0.25 }
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = MetaMediaImporter::dry_run(&root).unwrap();

        assert!(diagnostics.is_empty());
        let preset = imported.meta.get("master").unwrap();
        assert_eq!(preset.name, "master");
        assert!((preset.control_values.get("gain").copied().unwrap() - 0.7).abs() < 1e-6);
        assert!((preset.control_values.get("tilt").copied().unwrap() - 0.25).abs() < 1e-6);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: media-channel preset preserves
    // the full settings object verbatim as serde_json::Value (clip
    // pointers, codec preferences, et al land in the commit
    // harness).
    #[test]
    fn media_preset_preserves_unknown_settings_as_json() {
        let dir = unique_scratch_dir();
        let media_dir = dir.join("MediaPresets");
        std::fs::create_dir_all(&media_dir).unwrap();
        std::fs::write(
            media_dir.join("clip-bank.json"),
            r#"{
                "name": "clip-bank",
                "settings": {
                    "clips": ["a.mp4", "b.mp4"],
                    "loop": true,
                    "playback_rate": 1.0,
                    "extra_unknown": { "nested": 42 }
                }
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = MetaMediaImporter::dry_run(&root).unwrap();

        assert!(diagnostics.is_empty());
        let preset = imported.media.get("clip-bank").unwrap();
        assert_eq!(preset.name, "clip-bank");

        let settings = preset.settings.as_object().unwrap();
        assert!(settings.contains_key("clips"));
        assert!(settings.contains_key("loop"));
        assert!(settings.contains_key("playback_rate"));
        assert!(settings.contains_key("extra_unknown"));

        let extra = settings
            .get("extra_unknown")
            .and_then(|v| v.as_object())
            .unwrap();
        assert_eq!(extra.get("nested").and_then(|v| v.as_i64()), Some(42));

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: missing MetaPresets/ +
    // MediaPresets/ dirs produce empty imports + no diagnostics on
    // each side. Operators using only one channel still get a clean
    // import.
    #[test]
    fn missing_dirs_return_empty_imports() {
        let dir = unique_scratch_dir();
        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = MetaMediaImporter::dry_run(&root).unwrap();

        assert!(imported.meta.is_empty());
        assert!(imported.media.is_empty());
        assert!(diagnostics.is_empty());

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: malformed JSON in either side
    // emits an UnsupportedSchemaVersion diagnostic against the
    // offending file; sibling well-formed files continue to import.
    #[test]
    fn malformed_json_emits_unsupported_schema_version_diagnostic() {
        let dir = unique_scratch_dir();
        let meta_dir = dir.join("MetaPresets");
        std::fs::create_dir_all(&meta_dir).unwrap();
        let bad_path = meta_dir.join("bad.json");
        std::fs::write(&bad_path, "{ not valid json").unwrap();
        std::fs::write(
            meta_dir.join("good.json"),
            r#"{
                "name": "good",
                "controls": { "x": 0.1 }
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = MetaMediaImporter::dry_run(&root).unwrap();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].source_path, bad_path);
        assert_eq!(
            diagnostics[0].code,
            ImportDiagnosticCode::UnsupportedSchemaVersion
        );
        assert!(imported.meta.contains_key("good"));

        cleanup(&dir);
    }
}

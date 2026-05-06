// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Settings importer (ADR-015 §Scope line 20 + §Mechanism line 42).
//!
//! Phase-4b Wave 4b.7 PR 1 of N — dry-run-only settings importer.
//! Reads `<root>/Settings.json` (canonical reference filename — an
//! observable file-system location, not source-code-derivative)
//! and maps known keys into the [`ImportedSettings`] shape:
//!
//! | reference key   | Chromaticity field   |
//! |-----------------|----------------------|
//! | `audioDevice`   | `audio_device_id`    |
//! | `outputDisplay` | `output_display`     |
//! | `oscInputPort`  | `osc_input_port`     |
//! | `oscOutputPort` | `osc_output_port`    |
//!
//! All other keys are preserved verbatim in `extensions_legacy`
//! per ADR-015 line 20 + line 42 ("Unknown keys preserved in
//! `settings.extensions.reference-app-legacy` namespace"). The
//! transform table is authored clean-room from observable shape
//! plus the Chromaticity schema; no reference source code is
//! consulted.
//!
//! # Clean-room neutral wording
//!
//! Per ADR-015 line 20 +
//! `docs/29-cleanroom-legal-policy-baseline.md` Brand/Trademark
//! Review Checklist, no reference-app product name appears in any
//! field name, log message, doc comment, JSON key, or
//! operator-facing copy emitted from this module. The neutral
//! phrasing is "reference-app" / "reference user data".
//!
//! # Diagnostics
//!
//! - Missing `<root>/Settings.json` → returns
//!   `ImportedSettings::default()` with no diagnostics; this is
//!   the not-an-error case (the reference install may not have
//!   ever been launched).
//! - Malformed JSON → emits a single
//!   [`ImportDiagnosticCode::UnsupportedSchemaVersion`] diagnostic
//!   per ADR-015 §Versioning line 76 ("shape heuristics ... fallback
//!   inspection mode").
//!
//! Coercion-applied / unknown-control diagnostics live with the
//! presets importer; settings keys are flat and do not exercise
//! those code paths.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::diagnostics::{ArtifactClass, ImportDiagnostic, ImportDiagnosticCode};
use super::discovery::ReferenceUserDataRoot;

/// Imported settings shape per ADR-015 line 20.
///
/// Only the four field-mapped keys (audio device, output display,
/// OSC input/output ports) are surfaced as typed fields; everything
/// else is preserved verbatim in `extensions_legacy` so a
/// commit-time merger can fold them into Chromaticity's
/// `settings.extensions.reference-app-legacy` namespace per ADR-015
/// line 42.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedSettings {
    /// Reference `audioDevice` value if present.
    pub audio_device_id: Option<String>,
    /// Reference `outputDisplay` value if present.
    pub output_display: Option<String>,
    /// Reference `oscInputPort` value if present (clamped to
    /// `u16::MAX`; out-of-range emits CoercionApplied via a future
    /// PR widening when range-checking lands).
    pub osc_input_port: Option<u16>,
    /// Reference `oscOutputPort` value if present.
    pub osc_output_port: Option<u16>,
    /// Verbatim catch-all for keys that have no Chromaticity-side
    /// field mapping. Preserved at commit time under
    /// `settings.extensions.reference-app-legacy`.
    pub extensions_legacy: serde_json::Value,
}

/// Errors produced by the settings importer.
#[derive(Debug)]
pub enum SettingsImporterError {
    /// Filesystem error while reading `<root>/Settings.json`.
    IoError(std::io::Error),
}

impl std::fmt::Display for SettingsImporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "settings importer I/O error: {e}"),
        }
    }
}

impl std::error::Error for SettingsImporterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
        }
    }
}

/// Settings importer surface.
///
/// PR 1 of N ships [`SettingsImporter::dry_run`] only. The commit
/// plus rollback harness composes this with sibling per-class
/// importers in PR 4 of this wave.
pub struct SettingsImporter;

impl SettingsImporter {
    /// Path of the canonical settings file under the reference
    /// user-data root.
    fn settings_path(root: &ReferenceUserDataRoot) -> PathBuf {
        root.path().join("Settings.json")
    }

    /// Dry-run a settings import against `root`. Reads
    /// `<root>/Settings.json`, maps known keys, preserves unknown
    /// keys in `extensions_legacy`. Returns
    /// `(ImportedSettings, Vec<ImportDiagnostic>)`. Never mutates
    /// the source root per ADR-015 §Mechanism §7.
    pub fn dry_run(
        root: &ReferenceUserDataRoot,
    ) -> Result<(ImportedSettings, Vec<ImportDiagnostic>), SettingsImporterError> {
        let path = Self::settings_path(root);
        let mut diagnostics = Vec::new();

        // Missing file is the not-an-error case.
        match path.try_exists() {
            Ok(false) => return Ok((ImportedSettings::default(), diagnostics)),
            Err(e) => return Err(SettingsImporterError::IoError(e)),
            Ok(true) => {}
        }

        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => return Err(SettingsImporterError::IoError(e)),
        };

        let parsed: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => {
                diagnostics.push(ImportDiagnostic {
                    source_path: path.clone(),
                    class: ArtifactClass::Settings,
                    code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                    recommended_action: "review the reference user-data settings file shape; \
                                         re-run import after correcting"
                        .to_string(),
                    dry_run: true,
                });
                return Ok((ImportedSettings::default(), diagnostics));
            }
        };

        let serde_json::Value::Object(map) = parsed else {
            diagnostics.push(ImportDiagnostic {
                source_path: path.clone(),
                class: ArtifactClass::Settings,
                code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                recommended_action: "expected a JSON object at the top level of the reference \
                                     user-data settings file"
                    .to_string(),
                dry_run: true,
            });
            return Ok((ImportedSettings::default(), diagnostics));
        };

        let mut imported = ImportedSettings::default();
        let mut leftovers = serde_json::Map::new();

        for (key, value) in map {
            match key.as_str() {
                "audioDevice" => {
                    imported.audio_device_id = value.as_str().map(str::to_string);
                }
                "outputDisplay" => {
                    imported.output_display = value.as_str().map(str::to_string);
                }
                "oscInputPort" => {
                    imported.osc_input_port = value.as_u64().and_then(|v| u16::try_from(v).ok());
                }
                "oscOutputPort" => {
                    imported.osc_output_port = value.as_u64().and_then(|v| u16::try_from(v).ok());
                }
                _ => {
                    leftovers.insert(key, value);
                }
            }
        }

        imported.extensions_legacy = if leftovers.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::Value::Object(leftovers)
        };

        Ok((imported, diagnostics))
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-REFERENCE-SHOW (T1) — Phase-4b Wave 4b.7
    //! PR 1 of N foundation settings importer per ADR-015 §Scope
    //! line 20 + §Mechanism line 42 and
    //! `docs/26-evidence-artifact-registry.md` row 85.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! pins covering known-key mapping, unknown-key preservation,
    //! missing-file no-diagnostic posture, and malformed-JSON
    //! UnsupportedSchemaVersion emission. The multi-fixture corpus
    //! exercising real synthetic Settings.json shapes lives in
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
            "chromaticity-import-reference-settings-{pid}-{nanos}-{idx}"
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: known keys map and unknown keys
    // are preserved verbatim in extensions_legacy per ADR-015
    // line 20 + line 42.
    #[test]
    fn unknown_keys_preserved_in_extensions_legacy_namespace() {
        let dir = unique_scratch_dir();
        let settings_path = dir.join("Settings.json");
        std::fs::write(
            &settings_path,
            r#"{
                "audioDevice": "Built-in Output",
                "uiScaleFactor": 1.5,
                "lastShowFolder": "/Users/operator/shows",
                "telemetryOptIn": false
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = SettingsImporter::dry_run(&root).unwrap();

        assert_eq!(imported.audio_device_id.as_deref(), Some("Built-in Output"));
        assert_eq!(imported.output_display, None);

        let serde_json::Value::Object(ref leftovers) = imported.extensions_legacy else {
            panic!("expected extensions_legacy to be a JSON object");
        };
        assert!(leftovers.contains_key("uiScaleFactor"));
        assert!(leftovers.contains_key("lastShowFolder"));
        assert!(leftovers.contains_key("telemetryOptIn"));
        assert!(!leftovers.contains_key("audioDevice"));

        assert!(diagnostics.is_empty());

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: missing Settings.json is the
    // not-an-error case — the reference install may not have ever
    // been launched.
    #[test]
    fn missing_settings_json_returns_empty_with_no_diagnostics() {
        let dir = unique_scratch_dir();
        let root = override_reference_user_data_root(dir.clone()).unwrap();

        let (imported, diagnostics) = SettingsImporter::dry_run(&root).unwrap();
        assert_eq!(imported, ImportedSettings::default());
        assert!(diagnostics.is_empty());

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: malformed JSON emits a single
    // UnsupportedSchemaVersion diagnostic per ADR-015 §Versioning
    // line 76 fallback-inspection-mode posture.
    #[test]
    fn malformed_json_emits_unsupported_schema_version_diagnostic() {
        let dir = unique_scratch_dir();
        let settings_path = dir.join("Settings.json");
        std::fs::write(&settings_path, b"not actually json {{{").unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = SettingsImporter::dry_run(&root).unwrap();

        assert_eq!(imported, ImportedSettings::default());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].class, ArtifactClass::Settings);
        assert_eq!(
            diagnostics[0].code,
            ImportDiagnosticCode::UnsupportedSchemaVersion
        );
        assert!(diagnostics[0].dry_run);
        assert_eq!(diagnostics[0].source_path, settings_path);

        cleanup(&dir);
    }
}

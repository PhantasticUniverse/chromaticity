// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Hotkey remappings importer (ADR-015 §Scope line 27 + §Mechanism §3).
//!
//! Phase-4b Wave 4b.7 PR 3 of N — dry-run-only hotkey-remappings
//! importer. Reads a single `<root>/HotkeyRemaps.json` file. The
//! on-disk filename literal is permitted per ADR-015 line 9; no
//! reference-app product name appears in any operator-facing string.
//!
//! Per ADR-015 line 27 + §Mechanism §3 the importer surfaces four
//! status states matching the V1 hotkey policy in
//! `42-operator-hotkey-map.md` (V1 ships no rebinding UI; imported
//! rebinds either run as `Active`, sit dormant pending V2 if they
//! collide with the V1 locked set, are rejected outright if they
//! target a control-focused D/R/P/L action, or are flagged as
//! conflicting with another imported binding):
//!
//! - [`ImportedHotkeyStatus::Active`]
//! - [`ImportedHotkeyStatus::V1LockedDormantPendingV2`]
//! - [`ImportedHotkeyStatus::ControlFocusLocked`]
//! - [`ImportedHotkeyStatus::Conflict`]
//!
//! # Heuristics in this PR
//!
//! The Chromaticity-side hotkey-action vocabulary lookup (mapping
//! source `target_action` strings to the canonical action set in
//! `42-operator-hotkey-map.md`) is **deferred** to a future PR
//! composing the hotkey registry into the importer. This PR uses
//! literal-prefix and explicit-name heuristics so the diagnostic +
//! status surface compiles + tests cleanly:
//!
//! - `target_action` literally starts with `chroma.v1_locked.` →
//!   treated as a V1-locked stand-in; status =
//!   `V1LockedDormantPendingV2`, diagnostic =
//!   [`ImportDiagnosticCode::HotkeyV1Locked`].
//! - `target_action` literally one of `control.disable_focus`,
//!   `control.reset_focus`, `control.previous_focus`,
//!   `control.lock_focus` → treated as a D/R/P/L control-focus
//!   target (FR-17); status = `ControlFocusLocked`, diagnostic =
//!   [`ImportDiagnosticCode::HotkeyControlFocusLocked`].
//! - Two imported remaps share the same `source_key` → second + later
//!   entries get status = `Conflict`, diagnostic =
//!   [`ImportDiagnosticCode::HotkeyConflict`]. The first occurrence
//!   keeps its otherwise-evaluated status.
//! - Otherwise → status = `Active`, no diagnostic.
//!
//! # Wire shape
//!
//! ```text
//! { "remaps": [
//!     { "source_key": "ctrl+1",
//!       "target_action": "scene.recall.1" },
//!     ...
//! ] }
//! ```

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::diagnostics::{ArtifactClass, ImportDiagnostic, ImportDiagnosticCode};
use super::discovery::ReferenceUserDataRoot;

/// Status state for a single imported hotkey remap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImportedHotkeyStatus {
    /// Remap is wired into the active hotkey table.
    Active,
    /// Remap collides with a V1 locked binding; stored dormant and
    /// will activate when the V2 rebinding UI ships.
    V1LockedDormantPendingV2,
    /// Remap targets a control-focused D/R/P/L action that is never
    /// rebindable per FR-17; rejected with no side effect.
    ControlFocusLocked,
    /// Another imported remap already claimed this source_key.
    Conflict,
}

/// Single imported hotkey remap per ADR-015 line 27.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedHotkeyRemap {
    /// Source-side key combination string verbatim (e.g. `ctrl+1`).
    pub source_key: String,
    /// Target-side Chromaticity action name verbatim.
    pub target_action: String,
    /// Resolved status state per the heuristics in module-level
    /// prose.
    pub status: ImportedHotkeyStatus,
}

/// Aggregate output of the hotkey-remappings importer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedHotkey {
    /// All discovered remaps, preserving source-file ordering for
    /// determinism.
    pub remaps: Vec<ImportedHotkeyRemap>,
}

/// Errors produced by the hotkey-remappings importer.
#[derive(Debug)]
pub enum ImporterError {
    /// Filesystem error while reading the source file.
    IoError(std::io::Error),
}

impl std::fmt::Display for ImporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "hotkey importer I/O error: {e}"),
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

/// Wire-shape of the `<root>/HotkeyRemaps.json` file.
#[derive(Deserialize)]
struct HotkeyFileShape {
    #[serde(default)]
    remaps: Vec<HotkeyRemapShape>,
}

#[derive(Deserialize)]
struct HotkeyRemapShape {
    source_key: String,
    target_action: String,
}

/// Hotkey-remappings importer surface.
pub struct HotkeyImporter;

impl HotkeyImporter {
    fn hotkey_file(root: &ReferenceUserDataRoot) -> PathBuf {
        root.path().join("HotkeyRemaps.json")
    }

    fn classify(target_action: &str) -> Option<(ImportedHotkeyStatus, ImportDiagnosticCode)> {
        if target_action.starts_with("chroma.v1_locked.") {
            return Some((
                ImportedHotkeyStatus::V1LockedDormantPendingV2,
                ImportDiagnosticCode::HotkeyV1Locked,
            ));
        }
        match target_action {
            "control.disable_focus"
            | "control.reset_focus"
            | "control.previous_focus"
            | "control.lock_focus" => Some((
                ImportedHotkeyStatus::ControlFocusLocked,
                ImportDiagnosticCode::HotkeyControlFocusLocked,
            )),
            _ => None,
        }
    }

    /// Dry-run a hotkey-remappings import against `root`. Reads a
    /// single `<root>/HotkeyRemaps.json` file. Never mutates the
    /// source root.
    pub fn dry_run(
        root: &ReferenceUserDataRoot,
    ) -> Result<(ImportedHotkey, Vec<ImportDiagnostic>), ImporterError> {
        let file = Self::hotkey_file(root);
        let mut imported = ImportedHotkey::default();
        let mut diagnostics: Vec<ImportDiagnostic> = Vec::new();

        match file.try_exists() {
            Ok(false) => return Ok((imported, diagnostics)),
            Err(e) => return Err(ImporterError::IoError(e)),
            Ok(true) => {}
        }

        let raw = match std::fs::read_to_string(&file) {
            Ok(s) => s,
            Err(e) => return Err(ImporterError::IoError(e)),
        };

        let parsed: HotkeyFileShape = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => {
                diagnostics.push(ImportDiagnostic {
                    source_path: file.clone(),
                    class: ArtifactClass::Hotkey,
                    code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                    recommended_action:
                        "hotkey remaps file shape did not match the supported hotkey \
                         schema; review the reference user-data hotkey remaps file"
                            .to_string(),
                    dry_run: true,
                });
                return Ok((imported, diagnostics));
            }
        };

        let mut seen_keys: HashSet<String> = HashSet::new();
        for remap in parsed.remaps {
            let is_conflict = !seen_keys.insert(remap.source_key.clone());
            let (status, diag_code) = if is_conflict {
                (
                    ImportedHotkeyStatus::Conflict,
                    Some(ImportDiagnosticCode::HotkeyConflict),
                )
            } else if let Some((s, c)) = Self::classify(&remap.target_action) {
                (s, Some(c))
            } else {
                (ImportedHotkeyStatus::Active, None)
            };

            if let Some(code) = diag_code {
                let recommended_action = match code {
                    ImportDiagnosticCode::HotkeyV1Locked => format!(
                        "imported hotkey `{}` -> `{}` collides with a v1 locked binding; \
                         stored dormant and will activate when the v2 rebinding ui ships",
                        remap.source_key, remap.target_action
                    ),
                    ImportDiagnosticCode::HotkeyControlFocusLocked => format!(
                        "imported hotkey `{}` targets a control-focused d/r/p/l action \
                         (`{}`) that is never rebindable per fr-17; rejected with no \
                         side effect",
                        remap.source_key, remap.target_action
                    ),
                    ImportDiagnosticCode::HotkeyConflict => format!(
                        "imported hotkey `{}` collides with another imported remap on \
                         the same source key; second occurrence flagged",
                        remap.source_key
                    ),
                    _ => String::new(),
                };
                diagnostics.push(ImportDiagnostic {
                    source_path: file.clone(),
                    class: ArtifactClass::Hotkey,
                    code,
                    recommended_action,
                    dry_run: true,
                });
            }

            imported.remaps.push(ImportedHotkeyRemap {
                source_key: remap.source_key,
                target_action: remap.target_action,
                status,
            });
        }

        Ok((imported, diagnostics))
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-REFERENCE-SHOW (T1) — Phase-4b Wave 4b.7
    //! PR 3 of N hotkey-remappings importer per ADR-015 §Scope line
    //! 27 + §Mechanism §3 and `docs/26-evidence-artifact-registry.md`
    //! row 85.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! pins covering active-remap no-diagnostic round-trip, V1-locked
    //! dormant status emission, control-focus-locked status emission,
    //! conflict on second-with-same-source_key, and missing-file
    //! no-diagnostic posture. The multi-fixture corpus pairs hotkey
    //! with the sibling MIDI + OSC importers in
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
            "chromaticity-import-reference-hotkey-{pid}-{nanos}-{idx}"
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a remap whose target_action does
    // not match any locked-set heuristic round-trips with status =
    // Active and emits no diagnostic.
    #[test]
    fn active_remap_no_diagnostic() {
        let dir = unique_scratch_dir();
        std::fs::write(
            dir.join("HotkeyRemaps.json"),
            r#"{
                "remaps": [
                    { "source_key": "ctrl+1", "target_action": "scene.recall.1" }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = HotkeyImporter::dry_run(&root).unwrap();

        assert!(diagnostics.is_empty());
        assert_eq!(imported.remaps.len(), 1);
        assert_eq!(imported.remaps[0].status, ImportedHotkeyStatus::Active);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a remap whose target_action starts
    // with `chroma.v1_locked.` produces status =
    // V1LockedDormantPendingV2 and emits a HotkeyV1Locked diagnostic.
    #[test]
    fn v1_locked_remap_emits_diagnostic_and_status() {
        let dir = unique_scratch_dir();
        let path = dir.join("HotkeyRemaps.json");
        std::fs::write(
            &path,
            r#"{
                "remaps": [
                    { "source_key": "ctrl+space", "target_action": "chroma.v1_locked.tap" }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = HotkeyImporter::dry_run(&root).unwrap();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].class, ArtifactClass::Hotkey);
        assert_eq!(diagnostics[0].code, ImportDiagnosticCode::HotkeyV1Locked);
        assert_eq!(diagnostics[0].source_path, path);
        assert_eq!(
            imported.remaps[0].status,
            ImportedHotkeyStatus::V1LockedDormantPendingV2
        );

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a remap targeting one of the
    // D/R/P/L control-focus actions produces status =
    // ControlFocusLocked and emits a HotkeyControlFocusLocked
    // diagnostic.
    #[test]
    fn control_focus_remap_emits_diagnostic_and_status() {
        let dir = unique_scratch_dir();
        let path = dir.join("HotkeyRemaps.json");
        std::fs::write(
            &path,
            r#"{
                "remaps": [
                    { "source_key": "alt+d", "target_action": "control.disable_focus" }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = HotkeyImporter::dry_run(&root).unwrap();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].class, ArtifactClass::Hotkey);
        assert_eq!(
            diagnostics[0].code,
            ImportDiagnosticCode::HotkeyControlFocusLocked
        );
        assert_eq!(
            imported.remaps[0].status,
            ImportedHotkeyStatus::ControlFocusLocked
        );

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: two remaps sharing the same
    // source_key produce a HotkeyConflict diagnostic on the second
    // entry; the second remap's status = Conflict; first remap is
    // unaffected.
    #[test]
    fn conflicting_remaps_emit_diagnostic_for_second() {
        let dir = unique_scratch_dir();
        let path = dir.join("HotkeyRemaps.json");
        std::fs::write(
            &path,
            r#"{
                "remaps": [
                    { "source_key": "ctrl+2", "target_action": "scene.recall.2" },
                    { "source_key": "ctrl+2", "target_action": "scene.recall.3" }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = HotkeyImporter::dry_run(&root).unwrap();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].class, ArtifactClass::Hotkey);
        assert_eq!(diagnostics[0].code, ImportDiagnosticCode::HotkeyConflict);
        assert_eq!(diagnostics[0].source_path, path);

        assert_eq!(imported.remaps.len(), 2);
        assert_eq!(imported.remaps[0].status, ImportedHotkeyStatus::Active);
        assert_eq!(imported.remaps[1].status, ImportedHotkeyStatus::Conflict);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: missing HotkeyRemaps.json is
    // not-an-error and produces empty imports + no diagnostics.
    #[test]
    fn missing_hotkey_remaps_file_returns_empty_imports() {
        let dir = unique_scratch_dir();
        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = HotkeyImporter::dry_run(&root).unwrap();
        assert!(imported.remaps.is_empty());
        assert!(diagnostics.is_empty());

        cleanup(&dir);
    }
}

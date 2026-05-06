// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Playlists importer (ADR-015 §Scope line 24 + §Mechanism §3).
//!
//! Phase-4b Wave 4b.7 PR 2 of N — dry-run-only playlists importer.
//! Reads `<root>/Playlists/*.json` (canonical reference layout — an
//! observable file-system structure, not source-code-derivative).
//! Each per-playlist file contains:
//!
//! - `name` — the operator-supplied playlist name.
//! - `scenes` — ordered array of scene names that comprise the
//!   playlist.
//! - `iteration_mode` — one of `forward` / `reverse` / `random` /
//!   `one_shot` (mirrors Chromaticity's runtime iteration semantics
//!   per the playlist runtime). Unknown iteration values emit an
//!   [`ImportDiagnosticCode::UnsupportedSchemaVersion`] diagnostic
//!   with the offending value preserved in `recommended_action`.
//!   The playlist still imports with [`ImportedIterationMode::Forward`]
//!   as the safe default so the operator can still see the
//!   would-land state.
//! - `transition` — optional `{ "duration_ms": <u32>, "curve": <string> }`
//!   capturing the per-playlist transition settings. When absent the
//!   importer falls back to a zero-duration linear transition; full
//!   transition-curve composition is a later concern (PR 4 of this
//!   wave introduces the dry-run + commit + rollback drill harness).
//!
//! Output is the [`ImportedPlaylists::playlists`] vector preserving
//! source ordering. The importer is dry-run only and never mutates
//! the source root per ADR-015 §Mechanism §7.
//!
//! # Iteration-mode mapping
//!
//! The four iteration variants ([`ImportedIterationMode`]) mirror
//! the runtime semantics already exercised by
//! `engine/runtime`'s playlist scheduler:
//!
//! | source value | imported variant            |
//! |--------------|-----------------------------|
//! | `forward`    | [`ImportedIterationMode::Forward`]   |
//! | `reverse`    | [`ImportedIterationMode::Reverse`]   |
//! | `random`     | [`ImportedIterationMode::Random`]    |
//! | `one_shot`   | [`ImportedIterationMode::OneShot`]   |
//!
//! Unknown values fall back to `Forward` plus emit the
//! `UnsupportedSchemaVersion` diagnostic; the operator-facing
//! recommended-action string carries the offending source value
//! verbatim so the operator can correct upstream and re-run.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::diagnostics::{ArtifactClass, ImportDiagnostic, ImportDiagnosticCode};
use super::discovery::ReferenceUserDataRoot;

/// Playlist iteration semantics per the runtime playlist scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportedIterationMode {
    /// Step forward through `scenes` and wrap at the end.
    Forward,
    /// Step backward through `scenes` and wrap at the start.
    Reverse,
    /// Pick a random next scene, never repeating the current one.
    Random,
    /// Step forward through `scenes` and stop at the end.
    OneShot,
}

impl Default for ImportedIterationMode {
    fn default() -> Self {
        Self::Forward
    }
}

/// Per-playlist transition settings per ADR-015 line 24.
///
/// Minimal shape — `duration_ms` + `curve` name. Full transition-
/// curve composition (catmull, bezier, etc) is a later concern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedTransition {
    /// Transition duration in milliseconds. `0` means an
    /// instantaneous cut.
    pub duration_ms: u32,
    /// Transition curve name (verbatim from the source). Defaults
    /// to `"linear"` when the field is absent in the source.
    pub curve: String,
}

impl Default for ImportedTransition {
    fn default() -> Self {
        Self {
            duration_ms: 0,
            curve: "linear".to_string(),
        }
    }
}

/// Single imported playlist per ADR-015 line 24.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedPlaylist {
    /// Operator-supplied playlist name.
    pub name: String,
    /// Ordered scene names comprising the playlist.
    pub scenes: Vec<String>,
    /// Iteration semantics at end-of-playlist.
    pub iteration_mode: ImportedIterationMode,
    /// Per-playlist transition settings.
    pub transition: ImportedTransition,
}

/// Aggregate output of the playlists importer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedPlaylists {
    /// All discovered playlists, preserving file-name source
    /// ordering for determinism.
    pub playlists: Vec<ImportedPlaylist>,
}

/// Errors produced by the playlists importer.
#[derive(Debug)]
pub enum ImporterError {
    /// Filesystem error while reading the source directory.
    IoError(std::io::Error),
}

impl std::fmt::Display for ImporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "playlists importer I/O error: {e}"),
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

/// Wire-shape of a single playlist file.
#[derive(Deserialize)]
struct PlaylistFileShape {
    name: String,
    #[serde(default)]
    scenes: Vec<String>,
    #[serde(default)]
    iteration_mode: Option<String>,
    #[serde(default)]
    transition: Option<TransitionShape>,
}

#[derive(Deserialize)]
struct TransitionShape {
    #[serde(default)]
    duration_ms: u32,
    #[serde(default)]
    curve: Option<String>,
}

/// Playlists importer surface.
pub struct PlaylistsImporter;

impl PlaylistsImporter {
    fn playlists_dir(root: &ReferenceUserDataRoot) -> PathBuf {
        root.path().join("Playlists")
    }

    /// Dry-run a playlists import against `root`. Walks
    /// `<root>/Playlists/*.json` and parses each file. Never mutates
    /// the source root.
    pub fn dry_run(
        root: &ReferenceUserDataRoot,
    ) -> Result<(ImportedPlaylists, Vec<ImportDiagnostic>), ImporterError> {
        let dir = Self::playlists_dir(root);
        let mut imported = ImportedPlaylists::default();
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

            let parsed: PlaylistFileShape = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(_) => {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Playlist,
                        code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                        recommended_action:
                            "playlist file shape did not match the supported playlist \
                             schema; review the reference user-data playlist file"
                                .to_string(),
                        dry_run: true,
                    });
                    continue;
                }
            };

            let iteration_mode = match parsed.iteration_mode.as_deref() {
                None => ImportedIterationMode::Forward,
                Some("forward") => ImportedIterationMode::Forward,
                Some("reverse") => ImportedIterationMode::Reverse,
                Some("random") => ImportedIterationMode::Random,
                Some("one_shot") => ImportedIterationMode::OneShot,
                Some(other) => {
                    diagnostics.push(ImportDiagnostic {
                        source_path: path.clone(),
                        class: ArtifactClass::Playlist,
                        code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                        recommended_action: format!(
                            "playlist iteration_mode `{other}` is not in the supported \
                             set (forward / reverse / random / one_shot); falling back \
                             to forward"
                        ),
                        dry_run: true,
                    });
                    ImportedIterationMode::Forward
                }
            };

            let transition = match parsed.transition {
                None => ImportedTransition::default(),
                Some(t) => ImportedTransition {
                    duration_ms: t.duration_ms,
                    curve: t.curve.unwrap_or_else(|| "linear".to_string()),
                },
            };

            imported.playlists.push(ImportedPlaylist {
                name: parsed.name,
                scenes: parsed.scenes,
                iteration_mode,
                transition,
            });
        }

        Ok((imported, diagnostics))
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-REFERENCE-SHOW (T1) — Phase-4b Wave 4b.7
    //! PR 2 of N playlists importer per ADR-015 §Scope line 24 +
    //! §Mechanism §3 and `docs/26-evidence-artifact-registry.md` row
    //! 85.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! pins covering three-scene round-trip, unknown iteration mode
    //! diagnostic emission, missing-dir no-diagnostic posture, and
    //! default-transition-on-absent-field. The multi-fixture corpus
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
            "chromaticity-import-reference-playlists-{pid}-{nanos}-{idx}"
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: three-scene playlist round-trips
    // through the importer with iteration_mode + transition
    // captured.
    #[test]
    fn roundtrip_playlist_with_three_scenes() {
        let dir = unique_scratch_dir();
        let pl_dir = dir.join("Playlists");
        std::fs::create_dir_all(&pl_dir).unwrap();
        std::fs::write(
            pl_dir.join("set-a.json"),
            r#"{
                "name": "set a",
                "scenes": ["scene-a", "scene-b", "scene-c"],
                "iteration_mode": "reverse",
                "transition": { "duration_ms": 1500, "curve": "ease-in-out" }
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = PlaylistsImporter::dry_run(&root).unwrap();

        assert!(diagnostics.is_empty());
        assert_eq!(imported.playlists.len(), 1);
        let pl = &imported.playlists[0];
        assert_eq!(pl.name, "set a");
        assert_eq!(pl.scenes, vec!["scene-a", "scene-b", "scene-c"]);
        assert_eq!(pl.iteration_mode, ImportedIterationMode::Reverse);
        assert_eq!(pl.transition.duration_ms, 1500);
        assert_eq!(pl.transition.curve, "ease-in-out");

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: an unknown iteration_mode emits
    // an UnsupportedSchemaVersion diagnostic and falls back to
    // Forward; the playlist still imports.
    #[test]
    fn unknown_iteration_mode_emits_diagnostic() {
        let dir = unique_scratch_dir();
        let pl_dir = dir.join("Playlists");
        std::fs::create_dir_all(&pl_dir).unwrap();
        let path = pl_dir.join("set-b.json");
        std::fs::write(
            &path,
            r#"{
                "name": "set b",
                "scenes": ["scene-x"],
                "iteration_mode": "ping_pong"
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
        assert!(diagnostics[0].recommended_action.contains("ping_pong"));

        let pl = &imported.playlists[0];
        assert_eq!(pl.iteration_mode, ImportedIterationMode::Forward);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: missing Playlists/ dir is
    // not-an-error and produces empty imports + no diagnostics.
    #[test]
    fn missing_playlists_dir_returns_empty_imports() {
        let dir = unique_scratch_dir();
        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = PlaylistsImporter::dry_run(&root).unwrap();

        assert!(imported.playlists.is_empty());
        assert!(diagnostics.is_empty());

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a playlist with no transition
    // field captures the default transition (duration_ms = 0,
    // curve = "linear").
    #[test]
    fn default_transition_when_field_absent() {
        let dir = unique_scratch_dir();
        let pl_dir = dir.join("Playlists");
        std::fs::create_dir_all(&pl_dir).unwrap();
        std::fs::write(
            pl_dir.join("set-c.json"),
            r#"{
                "name": "set c",
                "scenes": ["scene-z"]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = PlaylistsImporter::dry_run(&root).unwrap();

        assert!(diagnostics.is_empty());
        let pl = &imported.playlists[0];
        assert_eq!(pl.transition.duration_ms, 0);
        assert_eq!(pl.transition.curve, "linear");
        assert_eq!(pl.iteration_mode, ImportedIterationMode::Forward);

        cleanup(&dir);
    }
}

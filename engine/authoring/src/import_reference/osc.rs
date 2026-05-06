// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! OSC mapping aliases importer (ADR-015 §Scope line 26 + §Mechanism §3).
//!
//! Phase-4b Wave 4b.7 PR 3 of N — dry-run-only OSC alias importer.
//! Reads a single `<root>/OscAliases.json` file (ADR-015 does not pin
//! a filename for the OSC alias artifact; the importer adopts the
//! most-natural single-file `OscAliases.json` layout against the
//! observable file-system structure). The on-disk filename literal
//! is permitted per ADR-015 line 9; no reference-app product name
//! appears in any operator-facing string.
//!
//! Per ADR-015 line 26 the importer captures the route alias table
//! plus per-mapping source-IP allowlist entries.
//!
//! # Wire shape
//!
//! ```text
//! { "aliases": [
//!     { "source_route": "/foo/bar",
//!       "target_route": "/chroma/intensity",
//!       "source_ip_allowlist": ["10.0.0.1", "10.0.0.2"] },
//!     ...
//! ] }
//! ```
//!
//! Defaults when absent: `source_ip_allowlist` -> empty vec.
//!
//! # Diagnostics
//!
//! - Imported alias whose `source_route` literally starts with
//!   `/chroma/` → emit
//!   [`ImportDiagnosticCode::OscAliasCollision`]. The full
//!   Chromaticity-side canonical route registry is not available at
//!   this layer; this PR uses the `/chroma/` prefix as a stand-in
//!   heuristic and the alias is still imported so the operator can
//!   review. The full registry-lookup pass is deferred to a future
//!   PR composing the integration-hub registry into the importer.
//! - Malformed JSON → emit a single
//!   [`ImportDiagnosticCode::UnsupportedSchemaVersion`] diagnostic
//!   per ADR-015 §Versioning line 76.
//! - Missing `<root>/OscAliases.json` → returns an empty
//!   [`ImportedOsc`] with no diagnostics; not-an-error case.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::diagnostics::{ArtifactClass, ImportDiagnostic, ImportDiagnosticCode};
use super::discovery::ReferenceUserDataRoot;

/// Single imported OSC alias per ADR-015 line 26.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedOscAlias {
    /// Source-side OSC route address as captured in the source file.
    pub source_route: String,
    /// Target-side OSC route address (the Chromaticity-side route
    /// the alias re-points to).
    pub target_route: String,
    /// Per-mapping source-IP allowlist; empty vec when not provided.
    pub source_ip_allowlist: Vec<String>,
}

/// Aggregate output of the OSC aliases importer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedOsc {
    /// All discovered aliases, preserving source-file ordering for
    /// determinism.
    pub aliases: Vec<ImportedOscAlias>,
}

/// Errors produced by the OSC aliases importer.
#[derive(Debug)]
pub enum ImporterError {
    /// Filesystem error while reading the source file.
    IoError(std::io::Error),
}

impl std::fmt::Display for ImporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "osc importer I/O error: {e}"),
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

/// Wire-shape of the `<root>/OscAliases.json` file.
#[derive(Deserialize)]
struct OscFileShape {
    #[serde(default)]
    aliases: Vec<OscAliasShape>,
}

#[derive(Deserialize)]
struct OscAliasShape {
    source_route: String,
    target_route: String,
    #[serde(default)]
    source_ip_allowlist: Vec<String>,
}

/// OSC aliases importer surface.
pub struct OscImporter;

impl OscImporter {
    fn osc_file(root: &ReferenceUserDataRoot) -> PathBuf {
        root.path().join("OscAliases.json")
    }

    /// Dry-run an OSC aliases import against `root`. Reads a single
    /// `<root>/OscAliases.json` file. Never mutates the source root.
    pub fn dry_run(
        root: &ReferenceUserDataRoot,
    ) -> Result<(ImportedOsc, Vec<ImportDiagnostic>), ImporterError> {
        let file = Self::osc_file(root);
        let mut imported = ImportedOsc::default();
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

        let parsed: OscFileShape = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => {
                diagnostics.push(ImportDiagnostic {
                    source_path: file.clone(),
                    class: ArtifactClass::Osc,
                    code: ImportDiagnosticCode::UnsupportedSchemaVersion,
                    recommended_action:
                        "osc aliases file shape did not match the supported osc schema; \
                         review the reference user-data osc aliases file"
                            .to_string(),
                    dry_run: true,
                });
                return Ok((imported, diagnostics));
            }
        };

        for alias in parsed.aliases {
            if alias.source_route.starts_with("/chroma/") {
                diagnostics.push(ImportDiagnostic {
                    source_path: file.clone(),
                    class: ArtifactClass::Osc,
                    code: ImportDiagnosticCode::OscAliasCollision,
                    recommended_action: format!(
                        "imported osc alias source route `{}` would shadow a canonical \
                         chromaticity route; review before commit",
                        alias.source_route
                    ),
                    dry_run: true,
                });
            }

            imported.aliases.push(ImportedOscAlias {
                source_route: alias.source_route,
                target_route: alias.target_route,
                source_ip_allowlist: alias.source_ip_allowlist,
            });
        }

        Ok((imported, diagnostics))
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-REFERENCE-SHOW (T1) — Phase-4b Wave 4b.7
    //! PR 3 of N OSC-aliases importer per ADR-015 §Scope line 26 +
    //! §Mechanism §3 and `docs/26-evidence-artifact-registry.md` row
    //! 85.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! pins covering alias round-trip with empty + populated
    //! allowlists, chroma-prefix collision diagnostic, missing-file
    //! no-diagnostic posture, and malformed-JSON
    //! UnsupportedSchemaVersion diagnostic. The multi-fixture corpus
    //! pairs OSC with the sibling MIDI + hotkey importers in
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
            "chromaticity-import-reference-osc-{pid}-{nanos}-{idx}"
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(dir: &PathBuf) {
        let _ = std::fs::remove_dir_all(dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: a single-alias file with no
    // source_ip_allowlist round-trips with the alias and an empty
    // allowlist vec.
    #[test]
    fn roundtrip_alias_with_no_allowlist() {
        let dir = unique_scratch_dir();
        std::fs::write(
            dir.join("OscAliases.json"),
            r#"{
                "aliases": [
                    { "source_route": "/foo/bar",
                      "target_route": "/intensity" }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = OscImporter::dry_run(&root).unwrap();
        assert!(diagnostics.is_empty());
        assert_eq!(imported.aliases.len(), 1);
        let a = &imported.aliases[0];
        assert_eq!(a.source_route, "/foo/bar");
        assert_eq!(a.target_route, "/intensity");
        assert!(a.source_ip_allowlist.is_empty());

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: alias round-trips with two
    // source-IP allowlist entries preserved verbatim.
    #[test]
    fn roundtrip_alias_with_two_ip_allowlist_entries() {
        let dir = unique_scratch_dir();
        std::fs::write(
            dir.join("OscAliases.json"),
            r#"{
                "aliases": [
                    { "source_route": "/foo/two",
                      "target_route": "/two",
                      "source_ip_allowlist": ["10.0.0.1", "10.0.0.2"] }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = OscImporter::dry_run(&root).unwrap();
        assert!(diagnostics.is_empty());
        assert_eq!(
            imported.aliases[0].source_ip_allowlist,
            vec!["10.0.0.1", "10.0.0.2"]
        );

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: an alias whose source_route starts
    // with `/chroma/` emits an OscAliasCollision diagnostic AND the
    // alias is still imported (operator-review posture).
    #[test]
    fn chroma_prefix_emits_collision_diagnostic() {
        let dir = unique_scratch_dir();
        let path = dir.join("OscAliases.json");
        std::fs::write(
            &path,
            r#"{
                "aliases": [
                    { "source_route": "/chroma/intensity",
                      "target_route": "/elsewhere" }
                ]
            }"#,
        )
        .unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = OscImporter::dry_run(&root).unwrap();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].class, ArtifactClass::Osc);
        assert_eq!(diagnostics[0].code, ImportDiagnosticCode::OscAliasCollision);
        assert_eq!(diagnostics[0].source_path, path);
        assert!(
            diagnostics[0]
                .recommended_action
                .contains("/chroma/intensity")
        );
        assert_eq!(imported.aliases.len(), 1);

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: missing OscAliases.json is
    // not-an-error and produces empty imports + no diagnostics.
    #[test]
    fn missing_osc_aliases_file_returns_empty_imports() {
        let dir = unique_scratch_dir();
        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = OscImporter::dry_run(&root).unwrap();
        assert!(imported.aliases.is_empty());
        assert!(diagnostics.is_empty());

        cleanup(&dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: malformed JSON in OscAliases.json
    // emits a single UnsupportedSchemaVersion diagnostic and yields
    // empty imports.
    #[test]
    fn malformed_json_emits_unsupported_schema_version_diagnostic() {
        let dir = unique_scratch_dir();
        let path = dir.join("OscAliases.json");
        std::fs::write(&path, "{this is not json").unwrap();

        let root = override_reference_user_data_root(dir.clone()).unwrap();
        let (imported, diagnostics) = OscImporter::dry_run(&root).unwrap();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].class, ArtifactClass::Osc);
        assert_eq!(
            diagnostics[0].code,
            ImportDiagnosticCode::UnsupportedSchemaVersion
        );
        assert_eq!(diagnostics[0].source_path, path);
        assert!(imported.aliases.is_empty());

        cleanup(&dir);
    }
}

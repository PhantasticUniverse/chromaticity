// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler

//! Reference-app user-data root discovery.
//!
//! Phase-4b Wave 4b.7 PR 1 of N — auto-detection of the canonical
//! reference-app user-data folder per ADR-015 lines 10-11, plus an
//! operator-provided override path for installs in non-default
//! locations. The path itself is observable (a file-system location,
//! not source code, not copyrightable per ADR-015 line 9); discovery
//! consults `dirs::home_dir` on macOS or `dirs::data_local_dir` on
//! Windows and joins the canonical relative components.
//!
//! # Discovery vs override
//!
//! [`discover_reference_user_data_root`] returns `Ok(None)` when the
//! canonical path does not exist on disk — this is the not-an-error
//! case (the operator may not have the reference app installed). It
//! returns `Err(DiscoveryError::HomeDirectoryUnavailable)` when the
//! platform cannot resolve a home / local-data directory at all
//! (rare; sandboxed CI environments).
//!
//! [`override_reference_user_data_root`] takes an operator-provided
//! path and validates that it (a) exists, (b) is a directory.
//! Anything else surfaces as a [`DiscoveryError`] for the operator.
//!
//! # Read-only posture
//!
//! The returned [`ReferenceUserDataRoot`] is a borrow-only handle;
//! per ADR-015 §Mechanism §7 the importer never mutates the source
//! folder. Per-class importers in sibling modules accept
//! `&ReferenceUserDataRoot` and read-only-walk the contents.

use std::path::{Path, PathBuf};

/// Discovered or operator-provided root of the reference-app
/// user-data folder.
///
/// Newtype around `PathBuf` to constrain the API to "reference user
/// data root" semantics; per-class importers accept this type rather
/// than a bare path so the type system carries the discovery
/// invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceUserDataRoot {
    root: PathBuf,
}

impl ReferenceUserDataRoot {
    /// Read-only access to the underlying root path. Per-class
    /// importers compose subpaths against this for parsing.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.root
    }
}

/// Errors produced during discovery / override-validation.
#[derive(Debug)]
pub enum DiscoveryError {
    /// Operator-supplied path does not exist on disk.
    PathDoesNotExist(PathBuf),
    /// Operator-supplied path exists but is not a directory.
    PathNotADirectory(PathBuf),
    /// Platform did not surface a home / local-data directory.
    HomeDirectoryUnavailable,
    /// Filesystem error during stat / metadata.
    IoError(std::io::Error),
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathDoesNotExist(p) => {
                write!(
                    f,
                    "reference user-data path does not exist: {}",
                    p.display()
                )
            }
            Self::PathNotADirectory(p) => {
                write!(
                    f,
                    "reference user-data path is not a directory: {}",
                    p.display()
                )
            }
            Self::HomeDirectoryUnavailable => {
                write!(f, "platform did not provide a home / local-data directory")
            }
            Self::IoError(e) => write!(f, "filesystem error during discovery: {e}"),
        }
    }
}

impl std::error::Error for DiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for DiscoveryError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

/// Auto-detect the canonical reference user-data path for the
/// current OS per ADR-015 lines 10-11.
///
/// Returns:
/// - `Ok(Some(root))` — canonical path resolved and exists on disk.
/// - `Ok(None)` — canonical path resolved but does not exist (the
///   operator may not have the reference app installed); not an
///   error.
/// - `Err(DiscoveryError::HomeDirectoryUnavailable)` — platform did
///   not surface a home / local-data directory.
///
/// Other platforms (Linux et al) return `Ok(None)` — there is no
/// canonical reference-app user-data path defined.
pub fn discover_reference_user_data_root() -> Result<Option<ReferenceUserDataRoot>, DiscoveryError>
{
    let candidate = canonical_root_for_current_platform();
    let Some(path) = candidate else {
        return Ok(None);
    };

    match path.try_exists() {
        Ok(true) if path.is_dir() => Ok(Some(ReferenceUserDataRoot { root: path })),
        Ok(true) => Ok(None),
        Ok(false) => Ok(None),
        Err(e) => Err(DiscoveryError::IoError(e)),
    }
}

/// Validate and wrap an operator-supplied path. Used when the
/// reference install lives outside the canonical user-data location.
pub fn override_reference_user_data_root(
    path: PathBuf,
) -> Result<ReferenceUserDataRoot, DiscoveryError> {
    match path.try_exists() {
        Ok(true) => {
            if path.is_dir() {
                Ok(ReferenceUserDataRoot { root: path })
            } else {
                Err(DiscoveryError::PathNotADirectory(path))
            }
        }
        Ok(false) => Err(DiscoveryError::PathDoesNotExist(path)),
        Err(e) => Err(DiscoveryError::IoError(e)),
    }
}

/// Returns the canonical reference user-data path for the current
/// platform per ADR-015 lines 10-11. Returns `None` on platforms
/// for which no canonical path is defined, or when the platform's
/// home / local-data directory cannot be resolved.
#[cfg(target_os = "macos")]
fn canonical_root_for_current_platform() -> Option<PathBuf> {
    // ADR-015 line 10: ~/Library/Application Support/com.gravitycurrent.synesthesia/Data
    // (path per ADR-015; observable file-system location, not copyrightable code)
    let home = dirs::home_dir()?;
    Some(
        home.join("Library")
            .join("Application Support")
            .join("com.gravitycurrent.synesthesia")
            .join("Data"),
    )
}

#[cfg(target_os = "windows")]
fn canonical_root_for_current_platform() -> Option<PathBuf> {
    // ADR-015 line 11: %LOCALAPPDATA%\Synesthesia\Data
    let local_data = dirs::data_local_dir()?;
    Some(local_data.join("Synesthesia").join("Data"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn canonical_root_for_current_platform() -> Option<PathBuf> {
    // ADR-015 line 10-11 only define macOS + Windows canonical
    // paths; other platforms surface no canonical path. Operator
    // override remains available.
    None
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-IMPORT-REFERENCE-SHOW (T1) — Phase-4b Wave 4b.7
    //! PR 1 of N foundation discovery surface for the reference-app
    //! user-data import per ADR-015 §Mechanism and
    //! `docs/26-evidence-artifact-registry.md` row 85.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! pins against the producer surface (auto-discovery,
    //! operator-override path, error-variant routing). The
    //! multi-fixture corpus exercising the discovery surface
    //! against synthetic root layouts lives in
    //! `tests/import_reference_foundation_corpus.rs` at T2.

    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_scratch_path() -> PathBuf {
        // Per-test-unique scratch path under std::env::temp_dir()
        // (no tempfile dep in the workspace; pattern mirrors
        // engine/authoring/src/live_reload.rs and scaffolder.rs).
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let idx = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chromaticity-import-reference-discovery-{pid}-{nanos}-{idx}"
        ));
        path
    }

    // EV-IMPORT-REFERENCE-SHOW T1: discovery returns `None` cleanly
    // when an operator-supplied candidate path does not exist on
    // disk (the not-an-error case per ADR-015 §Mechanism §1).
    #[test]
    fn discovery_returns_none_on_unknown_path() {
        let scratch = unique_scratch_path();
        // The override path goes through the same try_exists guard
        // path that auto-detect uses, so this exercises the same
        // code surface.
        let result = override_reference_user_data_root(scratch.clone());
        match result {
            Err(DiscoveryError::PathDoesNotExist(p)) => assert_eq!(p, scratch),
            other => panic!("expected PathDoesNotExist, got {other:?}"),
        }
    }

    // EV-IMPORT-REFERENCE-SHOW T1: operator-override rejects a
    // regular file (must be a directory).
    #[test]
    fn override_rejects_non_directory() {
        let scratch_dir = unique_scratch_path();
        std::fs::create_dir_all(&scratch_dir).unwrap();
        let file_path = scratch_dir.join("not-a-dir.txt");
        std::fs::write(&file_path, b"placeholder").unwrap();

        let result = override_reference_user_data_root(file_path.clone());
        match result {
            Err(DiscoveryError::PathNotADirectory(p)) => assert_eq!(p, file_path),
            other => panic!("expected PathNotADirectory, got {other:?}"),
        }

        // Cleanup
        let _ = std::fs::remove_file(&file_path);
        let _ = std::fs::remove_dir(&scratch_dir);
    }

    // EV-IMPORT-REFERENCE-SHOW T1: operator-override accepts an
    // existing directory and returns a `ReferenceUserDataRoot`
    // wrapping the supplied path verbatim.
    #[test]
    fn override_accepts_existing_directory() {
        let scratch_dir = unique_scratch_path();
        std::fs::create_dir_all(&scratch_dir).unwrap();

        let result = override_reference_user_data_root(scratch_dir.clone());
        match result {
            Ok(root) => assert_eq!(root.path(), scratch_dir.as_path()),
            Err(e) => panic!("expected Ok, got {e:?}"),
        }

        let _ = std::fs::remove_dir(&scratch_dir);
    }
}

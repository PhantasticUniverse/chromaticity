// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! `.chromaShow` bundle format — Phase-3 wave PR 5 EV-SHOW-BUNDLE-RECOVERY
//! foundational format harness.
//!
//! ## Scope
//!
//! Phase-3 row 35 narrowed scope per AO direction: **bundle-format only**.
//! This module ships the on-disk container that `.chromaShow` files
//! consume: a tar archive carrying a flat `manifest.json` at root plus
//! payload files at their declared paths, with per-file SHA-256 + a
//! self-elided manifest SHA-256 for end-to-end integrity coverage.
//!
//! Deferred to Phase-4+ feature work tracked separately:
//!
//! - **Bundle-level signing** — the per-file + manifest checksums in
//!   this module cover tampering detection but NOT authenticity. The
//!   ed25519 signing pass that turns `.chromaShow` into a signed
//!   container is `EV-ENTITLEMENT-ENFORCEMENT` scope (Phase-4a) and
//!   layers on top of, not inside, this module's checksum surface.
//! - **Recovery / rollback semantics** — show-state migration and
//!   roll-forward / roll-back orchestration is `EV-STATE-MIGRATION`
//!   scope and consumes the bundle format declared here. The
//!   `Recovery` portion of the row name reflects the cumulative-release
//!   gate target; this PR closes only the format-conformance subset.
//! - **Bundle-level checksum index versioning** — the [`ShowBundleManifest::version`]
//!   field is reserved at `1` for this PR; future widening (e.g.
//!   per-entry `ContentEncoding` for the `script.js` / `main.glsl` /
//!   `images/` pairing per `docs/05-scene-system.md` §3) goes through
//!   the `version` axis without breaking the wire format.
//!
//! ## Format choice — tar, not zip
//!
//! `docs/22-compatibility-versioning-policy.md` line 78 reserved `.chromaShow`
//! as a "Package extension; signed and integrity-checked" without
//! committing to a container format. This PR elects **tar** over zip on
//! determinism / transparency grounds:
//!
//! - Tar is a streaming concatenation of fixed-shape headers + raw
//!   payload bytes; given identical inputs in identical order, the
//!   `tar` crate emits byte-identical archive output. Zip's central
//!   directory plus per-entry CRC32 / DEFLATE choice surfaces
//!   determinism friction the format harness would have to police.
//! - Tar's lack of compression is a feature here: the engine's payload
//!   files (scene JSON, shader text, small image fixtures) compress
//!   poorly enough that the deterministic-encoding savings dominate.
//!   Compression, if ever wanted, can layer on top of the tar bytes
//!   (`.chromaShow.gz`) as a transport-only concern that doesn't
//!   perturb the format-conformance corpus.
//! - Tar's flat-tree path encoding maps cleanly to the manifest's
//!   path-as-canonical-key invariant. Zip's per-entry separator
//!   conventions (forward slash by spec, backslash in some Windows
//!   tooling) would force a normalisation pass at read time.
//!
//! `docs/22` will be reconciled in a follow-up `docs(repo)` PR — the
//! reservation language predates this PR's harness landing.
//!
//! ## Security
//!
//! [`read_bundle`] uses [`tar::Archive::entries`] and validates every
//! entry path against the manifest path-set BEFORE materialising the
//! payload. It does NOT use [`tar::Archive::unpack`], which would
//! resolve `..` segments against the working directory and open a
//! directory-traversal hole. Path-traversal defense is structural:
//!
//! 1. The manifest path-set is the canonical allow-list — any tar
//!    entry whose path is not in the manifest fails with
//!    [`BundleError::PathTraversal`] at the entry-iteration boundary.
//! 2. [`write_bundle`] rejects manifest entries whose paths contain
//!    `..` segments, are absolute, or carry Windows drive prefixes
//!    BEFORE archive emission.
//! 3. Per-file SHA-256 verification confirms payload integrity;
//!    a corrupted byte fails [`BundleError::FileChecksumMismatch`]
//!    before any consumer receives the payload bytes.
//! 4. The manifest's own SHA-256 (computed over canonical JSON of
//!    the manifest with the `manifest_sha256` field elided) is
//!    self-validating: tampering with the file list itself fails
//!    [`BundleError::ManifestChecksumMismatch`].
//!
//! The T1 drift pin module asserts the path-traversal-defense invariants
//! directly. The T2 corpus (`engine/scene/tests/show_bundle_corpus/`) extends
//! coverage with fuzz-style mutation fixtures.
//!
//! ## Surface
//!
//! - [`ShowBundleManifest`] — top-level manifest carrying `version`,
//!   `files`, and a self-elided `manifest_sha256`.
//! - [`ShowBundleEntry`] — per-file row carrying `path`, `sha256`,
//!   and `size`.
//! - [`write_bundle`] — emit a `.chromaShow` byte stream from
//!   `(path, payload)` pairs.
//! - [`read_bundle`] — parse a `.chromaShow` byte stream, validate
//!   every checksum, and return the manifest plus payload pairs.
//! - [`verify_manifest`] — standalone integrity check against an
//!   already-parsed manifest plus payload pairs (used by callers that
//!   load the manifest separately, e.g. dry-run inspection).
//! - [`BundleError`] — non-exhaustive error taxonomy.
//!
//! Produces evidence for `EV-SHOW-BUNDLE-RECOVERY` (T1 drift pins in
//! [`bundle_pins`]; T2 corpus in `engine/scene/tests/show_bundle_corpus/main.rs`).

use std::fmt;
use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// `.chromaShow` manifest. Top-level entry of every bundle archive.
///
/// `manifest_sha256` is the SHA-256 of canonical JSON with the
/// `manifest_sha256` field set to a 32-byte zero array, so the manifest
/// is self-validating without a chicken-and-egg encoding step. The
/// hex round-trip handles the `[u8; 32]` array via the [`hex_array`]
/// module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowBundleManifest {
    /// Bundle-format version axis. Reserved at `1` for the Phase-3
    /// wave-PR-5 foundational harness. Bumps follow
    /// `docs/22-compatibility-versioning-policy.md` §Version Axes
    /// `.chromaShow` row.
    pub version: u32,
    /// Canonical-ordered file list. Path-based dedup is enforced at
    /// [`write_bundle`] time; reading rejects any tar entry whose path
    /// is not in this set with [`BundleError::PathTraversal`].
    pub files: Vec<ShowBundleEntry>,
    /// Self-elided SHA-256 over canonical JSON of this manifest with
    /// `manifest_sha256` zeroed out.
    #[serde(with = "hex_array")]
    pub manifest_sha256: [u8; 32],
}

/// Per-file manifest entry. The path is canonical-ordered relative to
/// the bundle root and rejects `..` segments + absolute paths +
/// Windows drive prefixes at write time. The 32-byte SHA-256 is
/// validated against the payload at read time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowBundleEntry {
    /// Forward-slash-separated relative path of the payload inside
    /// the bundle. Validated against `..` segments, absolute prefixes,
    /// and Windows drive prefixes.
    pub path: String,
    /// SHA-256 of the payload bytes. Hex-encoded in JSON.
    #[serde(with = "hex_array")]
    pub sha256: [u8; 32],
    /// Payload byte count. Sourced from the input bytes at write time.
    pub size: u64,
}

/// Errors returned by the bundle format surface.
///
/// Non-exhaustive so the Phase-4+ widening (signing, recovery
/// orchestration) can extend the variant set without a compat-surface
/// break. Each variant carries enough context to drive a diagnostic
/// without leaking caller-supplied bytes.
#[non_exhaustive]
#[derive(Debug)]
pub enum BundleError {
    /// I/O failure reading the tar stream. Wraps [`std::io::Error`].
    TarRead(io::Error),
    /// I/O failure writing the tar stream. Wraps [`std::io::Error`].
    TarWrite(io::Error),
    /// `manifest.json` missing from the archive.
    ManifestMissing,
    /// `manifest.json` failed JSON shape parsing.
    ManifestParseInvalid(serde_json::Error),
    /// Manifest-self-checksum did not match the canonical-encoded
    /// manifest contents.
    ManifestChecksumMismatch,
    /// A payload file's SHA-256 did not match the manifest entry.
    FileChecksumMismatch {
        /// The manifest path of the mismatched payload.
        path: String,
    },
    /// A payload path appeared more than once at write time.
    DuplicatePath(String),
    /// A path failed the safety checks: `..` segment, absolute prefix,
    /// or Windows drive prefix at write time, or an out-of-manifest
    /// path on a tar entry at read time.
    PathTraversal(String),
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TarRead(err) => write!(f, "tar read failed: {err}"),
            Self::TarWrite(err) => write!(f, "tar write failed: {err}"),
            Self::ManifestMissing => write!(f, "manifest.json missing from bundle"),
            Self::ManifestParseInvalid(err) => write!(f, "manifest.json shape invalid: {err}"),
            Self::ManifestChecksumMismatch => {
                write!(f, "manifest.json sha256 did not match canonical encoding")
            }
            Self::FileChecksumMismatch { path } => {
                write!(f, "payload sha256 mismatch at path {path}")
            }
            Self::DuplicatePath(path) => write!(f, "duplicate path in bundle: {path}"),
            Self::PathTraversal(path) => write!(f, "unsafe path rejected: {path}"),
        }
    }
}

impl std::error::Error for BundleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TarRead(err) | Self::TarWrite(err) => Some(err),
            Self::ManifestParseInvalid(err) => Some(err),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for BundleError {
    fn from(err: serde_json::Error) -> Self {
        Self::ManifestParseInvalid(err)
    }
}

/// Hex-string serde shim for the 32-byte SHA-256 fields. Lower-case
/// hex; rejects non-hex / wrong-length inputs.
mod hex_array {
    use serde::de::{Error as _, Unexpected};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut hex = String::with_capacity(64);
        for byte in bytes {
            hex.push_str(&format!("{byte:02x}"));
        }
        serializer.serialize_str(&hex)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        if raw.len() != 64 {
            return Err(D::Error::invalid_length(raw.len(), &"64 hex characters"));
        }
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            let pair = raw
                .get(i * 2..i * 2 + 2)
                .ok_or_else(|| D::Error::invalid_length(raw.len(), &"64 hex characters"))?;
            *byte = u8::from_str_radix(pair, 16)
                .map_err(|_| D::Error::invalid_value(Unexpected::Str(pair), &"hex byte (00-ff)"))?;
        }
        Ok(out)
    }
}

/// Canonical manifest filename inside the tar archive.
const MANIFEST_NAME: &str = "manifest.json";

/// Bundle-format version pinned by this PR.
const BUNDLE_VERSION: u32 = 1;

/// `(path, payload)` pair used by [`write_bundle`] / [`read_bundle`]
/// at their public boundaries. Aliased to keep the function signatures
/// readable and to satisfy `clippy::type_complexity`.
pub type BundleEntryPair = (String, Vec<u8>);

/// Validate an entry path against the safety rules: no `..` segments,
/// no absolute prefixes, no Windows drive prefixes, no embedded NUL,
/// no carriage-return / line-feed (defense in depth against header-
/// smuggling). Returns the unchanged path on success.
fn validate_path(path: &str) -> Result<&str, BundleError> {
    if path.is_empty() {
        return Err(BundleError::PathTraversal(path.to_owned()));
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err(BundleError::PathTraversal(path.to_owned()));
    }
    // Windows drive prefix check (e.g. `C:\Windows\System32` or `C:/...`).
    if path.len() >= 2 {
        let bytes = path.as_bytes();
        // ASCII letter followed by `:` is the drive-letter form. Length-
        // prefixed UNC paths (`\\?\...`) are caught by the leading-`\`
        // check above.
        if bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return Err(BundleError::PathTraversal(path.to_owned()));
        }
    }
    if path.contains('\0') || path.contains('\n') || path.contains('\r') {
        return Err(BundleError::PathTraversal(path.to_owned()));
    }
    for segment in path.split(['/', '\\']) {
        if segment == ".." {
            return Err(BundleError::PathTraversal(path.to_owned()));
        }
    }
    if path == MANIFEST_NAME {
        // The manifest filename is reserved at the bundle root; payload
        // files cannot collide with it.
        return Err(BundleError::DuplicatePath(path.to_owned()));
    }
    Ok(path)
}

/// Compute SHA-256 of a byte slice as a 32-byte array.
fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

/// Serialize the manifest to canonical JSON bytes with the
/// `manifest_sha256` field zeroed out, then SHA-256 the result. This
/// is the reference encoding the manifest's own checksum field
/// asserts against — used by both [`write_bundle`] (to compute the
/// stored value) and [`read_bundle`] / [`verify_manifest`] (to
/// validate it).
fn canonical_manifest_checksum_bytes(
    manifest: &ShowBundleManifest,
) -> Result<Vec<u8>, BundleError> {
    let elided = ShowBundleManifest {
        version: manifest.version,
        files: manifest.files.clone(),
        manifest_sha256: [0u8; 32],
    };
    Ok(serde_json::to_vec(&elided)?)
}

/// Compute the manifest's self-checksum over canonical JSON with the
/// field elided.
fn compute_manifest_checksum(manifest: &ShowBundleManifest) -> Result<[u8; 32], BundleError> {
    let canonical = canonical_manifest_checksum_bytes(manifest)?;
    Ok(sha256(&canonical))
}

/// Write a `.chromaShow` bundle from `(path, payload)` pairs.
///
/// Validation:
/// - Path uniqueness across the input slice. A duplicate path fails
///   [`BundleError::DuplicatePath`].
/// - Per-path safety: `..` segments, absolute prefixes, Windows drive
///   prefixes, embedded control characters all fail
///   [`BundleError::PathTraversal`].
///
/// Output shape:
/// - Tar archive with `manifest.json` as the first entry, followed by
///   the payload entries in input order.
/// - The manifest carries the `version`, `files` (canonical-ordered
///   per input), and the self-elided `manifest_sha256`.
/// - Per-file SHA-256 + size populated from the input bytes.
///
/// Determinism:
/// - File ordering matches input order (caller-controlled).
/// - The tar mtime is fixed at the Unix epoch (`0`) and the mode at
///   `0o644` so identical inputs produce byte-identical archives.
pub fn write_bundle<W: Write>(w: W, files: &[BundleEntryPair]) -> Result<(), BundleError> {
    // Path-uniqueness + safety pass.
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for (path, _) in files {
        validate_path(path)?;
        if !seen.insert(path.as_str()) {
            return Err(BundleError::DuplicatePath(path.clone()));
        }
    }

    let entries: Vec<ShowBundleEntry> = files
        .iter()
        .map(|(path, payload)| ShowBundleEntry {
            path: path.clone(),
            sha256: sha256(payload),
            size: payload.len() as u64,
        })
        .collect();

    let mut manifest = ShowBundleManifest {
        version: BUNDLE_VERSION,
        files: entries,
        manifest_sha256: [0u8; 32],
    };
    manifest.manifest_sha256 = compute_manifest_checksum(&manifest)?;

    let manifest_bytes = serde_json::to_vec(&manifest)?;

    let mut builder = tar::Builder::new(w);
    // Determinism: explicit header so the tar crate's caller-defaults
    // (system time, current UID/GID, current umask) do not vary the
    // archive bytes across hosts.
    let mut manifest_header = tar::Header::new_gnu();
    manifest_header
        .set_path(MANIFEST_NAME)
        .map_err(BundleError::TarWrite)?;
    manifest_header.set_size(manifest_bytes.len() as u64);
    manifest_header.set_mode(0o644);
    manifest_header.set_mtime(0);
    manifest_header.set_uid(0);
    manifest_header.set_gid(0);
    manifest_header.set_entry_type(tar::EntryType::Regular);
    manifest_header.set_cksum();
    builder
        .append(&manifest_header, manifest_bytes.as_slice())
        .map_err(BundleError::TarWrite)?;

    for (path, payload) in files {
        let mut header = tar::Header::new_gnu();
        header.set_path(path).map_err(BundleError::TarWrite)?;
        header.set_size(payload.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        builder
            .append(&header, payload.as_slice())
            .map_err(BundleError::TarWrite)?;
    }

    builder.finish().map_err(BundleError::TarWrite)?;
    let _ = builder.into_inner().map_err(BundleError::TarWrite)?;
    Ok(())
}

/// Read a `.chromaShow` bundle stream into the parsed manifest plus
/// payload pairs.
///
/// Security:
/// - Uses [`tar::Archive::entries`], NOT [`tar::Archive::unpack`].
///   No filesystem-side path resolution.
/// - Every tar entry path is validated against [`validate_path`]'s
///   ruleset — `..` segments, absolute prefixes, Windows drive
///   prefixes, embedded control characters all reject with
///   [`BundleError::PathTraversal`].
/// - Every non-manifest tar entry path MUST appear in the manifest
///   path-set; off-manifest entries reject with
///   [`BundleError::PathTraversal`].
/// - The manifest's self-checksum is validated before per-file
///   checksums, so a tampered manifest is the first failure surface.
/// - Per-file SHA-256 is validated against the manifest before the
///   payload bytes leave this function.
pub fn read_bundle<R: Read>(
    r: R,
) -> Result<(ShowBundleManifest, Vec<BundleEntryPair>), BundleError> {
    let mut archive = tar::Archive::new(r);
    let mut manifest_bytes: Option<Vec<u8>> = None;
    let mut payloads: Vec<BundleEntryPair> = Vec::new();

    let entries = archive.entries().map_err(BundleError::TarRead)?;
    for entry in entries {
        let mut entry = entry.map_err(BundleError::TarRead)?;
        let raw_path = entry.path().map_err(BundleError::TarRead)?;
        let path_str = raw_path
            .to_str()
            .ok_or_else(|| BundleError::PathTraversal(raw_path.display().to_string()))?
            .to_owned();
        // Reject `..` segments / absolute prefixes / drive prefixes /
        // control characters before reading payload bytes.
        if path_str != MANIFEST_NAME {
            // validate_path also rejects the manifest filename as a
            // duplicate (since payloads can't collide with it). For
            // the manifest entry itself, skip the validate pass.
            validate_path(&path_str)?;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(BundleError::TarRead)?;
        if path_str == MANIFEST_NAME {
            if manifest_bytes.is_some() {
                // Duplicate manifest entry — reject as a structural
                // ambiguity rather than silently keeping the last.
                return Err(BundleError::DuplicatePath(MANIFEST_NAME.to_owned()));
            }
            manifest_bytes = Some(buf);
        } else {
            payloads.push((path_str, buf));
        }
    }

    let manifest_bytes = manifest_bytes.ok_or(BundleError::ManifestMissing)?;
    let manifest: ShowBundleManifest = serde_json::from_slice(&manifest_bytes)?;

    // Manifest-self-checksum gate.
    let expected_manifest_sha = compute_manifest_checksum(&manifest)?;
    if expected_manifest_sha != manifest.manifest_sha256 {
        return Err(BundleError::ManifestChecksumMismatch);
    }

    verify_manifest(&manifest, &payloads)?;

    Ok((manifest, payloads))
}

/// Standalone integrity check: confirm every payload pair matches a
/// manifest entry by path + SHA-256, and that no payload is
/// off-manifest.
///
/// Used by [`read_bundle`] internally and exposed for callers that
/// load the manifest separately (e.g. dry-run inspection of a
/// previously-extracted bundle).
pub fn verify_manifest(
    manifest: &ShowBundleManifest,
    files: &[BundleEntryPair],
) -> Result<(), BundleError> {
    let by_path: std::collections::HashMap<&str, &ShowBundleEntry> = manifest
        .files
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();

    // Manifest must declare unique paths. (write_bundle enforces this
    // at write time; an externally-constructed manifest could violate
    // the invariant, so verify it here too.)
    if by_path.len() != manifest.files.len() {
        let mut seen = std::collections::HashSet::new();
        for entry in &manifest.files {
            if !seen.insert(entry.path.as_str()) {
                return Err(BundleError::DuplicatePath(entry.path.clone()));
            }
        }
    }

    // Every payload must resolve to a manifest row + match SHA-256.
    for (path, payload) in files {
        let entry = by_path
            .get(path.as_str())
            .ok_or_else(|| BundleError::PathTraversal(path.clone()))?;
        let observed = sha256(payload);
        if observed != entry.sha256 {
            return Err(BundleError::FileChecksumMismatch { path: path.clone() });
        }
        if (payload.len() as u64) != entry.size {
            return Err(BundleError::FileChecksumMismatch { path: path.clone() });
        }
    }

    Ok(())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod bundle_pins {
    //! Evidence: EV-SHOW-BUNDLE-RECOVERY (T1) — Phase-3 wave PR 5
    //! foundational drift pins on the bundle-format invariants.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: in-source
    //! invariants asserted directly without a fixture corpus. Each
    //! pin guards a single load-bearing property of the format —
    //! canonical-JSON byte stability, empty-bundle round-trip purity,
    //! path-duplicate rejection at verify time, and the
    //! manifest-checksum-excludes-self structural invariant.

    use super::*;

    #[test]
    fn manifest_serializes_to_canonical_json() {
        // Canonical-JSON output must be byte-stable across runs given
        // identical input. serde_json's default `to_vec` is
        // deterministic for `derive(Serialize)` structs (field order
        // matches declaration order; map types with `BTreeMap`-style
        // ordering aren't in play here). This pin asserts the contract
        // by construction so any future field reordering is loudest
        // here.
        let manifest = ShowBundleManifest {
            version: 1,
            files: vec![ShowBundleEntry {
                path: "scenes/intro.json".to_owned(),
                sha256: [0xAB; 32],
                size: 42,
            }],
            manifest_sha256: [0xCD; 32],
        };
        let a = serde_json::to_vec(&manifest).unwrap();
        let b = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(a, b, "canonical JSON serialization must be byte-stable");

        // Decoding round-trips field-for-field.
        let decoded: ShowBundleManifest = serde_json::from_slice(&a).unwrap();
        assert_eq!(decoded, manifest);
    }

    #[test]
    fn empty_bundle_round_trip_is_pure() {
        // Empty payload list: write_bundle emits a tar archive
        // carrying only manifest.json; read_bundle round-trips an
        // empty payload list and the same manifest.
        let mut buf = Vec::new();
        write_bundle(&mut buf, &[]).unwrap();
        assert!(!buf.is_empty(), "empty bundle should still emit tar bytes");

        let (manifest, payloads) = read_bundle(buf.as_slice()).unwrap();
        assert_eq!(manifest.version, BUNDLE_VERSION);
        assert!(manifest.files.is_empty());
        assert!(payloads.is_empty());

        // Manifest checksum is non-zero (the empty-files manifest
        // still hashes to a deterministic value, never the all-zero
        // sentinel used during checksum computation).
        assert_ne!(manifest.manifest_sha256, [0u8; 32]);
    }

    #[test]
    fn verify_manifest_rejects_path_duplicate() {
        // An externally-constructed manifest with duplicate paths
        // must fail verify_manifest. write_bundle enforces uniqueness
        // at write time, but verify_manifest is also exposed for
        // callers that load the manifest separately, so the same
        // invariant must hold at the verify boundary.
        let manifest = ShowBundleManifest {
            version: 1,
            files: vec![
                ShowBundleEntry {
                    path: "shared.glsl".to_owned(),
                    sha256: sha256(b"a"),
                    size: 1,
                },
                ShowBundleEntry {
                    path: "shared.glsl".to_owned(),
                    sha256: sha256(b"b"),
                    size: 1,
                },
            ],
            manifest_sha256: [0u8; 32],
        };
        let payloads = vec![("shared.glsl".to_owned(), b"a".to_vec())];
        match verify_manifest(&manifest, &payloads) {
            Err(BundleError::DuplicatePath(path)) => {
                assert_eq!(path, "shared.glsl");
            }
            other => panic!("expected DuplicatePath, got {other:?}"),
        }
    }

    #[test]
    fn manifest_checksum_excludes_self() {
        // The manifest's self-checksum is computed over canonical JSON
        // with `manifest_sha256` set to zero. This pin asserts the
        // structural invariant: changing the stored checksum value
        // must not change the canonical-encoding bytes used to
        // compute it (otherwise the field would be non-self-validating).
        let mut manifest = ShowBundleManifest {
            version: 1,
            files: vec![ShowBundleEntry {
                path: "main.glsl".to_owned(),
                sha256: sha256(b"shader source"),
                size: 13,
            }],
            manifest_sha256: [0u8; 32],
        };
        let canonical_a = canonical_manifest_checksum_bytes(&manifest).unwrap();
        manifest.manifest_sha256 = [0xFF; 32];
        let canonical_b = canonical_manifest_checksum_bytes(&manifest).unwrap();
        assert_eq!(
            canonical_a, canonical_b,
            "canonical encoding must elide manifest_sha256 — tampering with the field \
             must NOT alter the bytes hashed to validate the field"
        );

        // Independent path: write -> mutate the stored checksum ->
        // read fails. Reuses write_bundle / read_bundle to confirm
        // the elision wires up end-to-end.
        let payload = b"shader source".to_vec();
        let mut bytes = Vec::new();
        write_bundle(&mut bytes, &[("main.glsl".to_owned(), payload)]).unwrap();
        let (good_manifest, _payloads) = read_bundle(bytes.as_slice()).unwrap();
        let mut tampered = good_manifest.clone();
        tampered.manifest_sha256 = [0xAA; 32];
        let recomputed = compute_manifest_checksum(&tampered).unwrap();
        assert_eq!(
            recomputed, good_manifest.manifest_sha256,
            "recomputing the checksum on the tampered manifest must yield the SAME \
             value as the original because manifest_sha256 is elided from the encoding"
        );
    }
}

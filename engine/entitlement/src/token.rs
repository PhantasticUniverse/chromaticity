// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! `entitlement-v2` token schema and canonical CBOR helpers.
//!
//! The schema below is a verbatim transcription of the JSON example
//! in `docs/adr/ADR-009-activation-and-entitlements.md` §Decision
//! lines 21-36, plus the optionality semantics from line 38 ("All
//! claims other than `schema`, `subject`, `issued_at`,
//! `capabilities`, and `signature` are optional. Unknown claim keys
//! are forward-compat-ignored, not rejected").
//!
//! # Required vs optional claims (ADR-009 line 38)
//!
//! - **Required:** `schema`, `subject`, `issued_at`, `capabilities`,
//!   `signature`.
//! - **Optional:** `not_before`, `expires_at`, `issuer`, `kid`,
//!   `flags`, `machine_binding`, `session`.
//! - **Forward-compat:** any other claim key is preserved through
//!   round-trip without rejection (the [`Token::extra`] flatten
//!   bag).
//!
//! Note that the JSON example in the ADR uses every optional claim
//! to illustrate the full surface, but the optionality assertion in
//! line 38 is the authoritative rule. The schema here therefore
//! marks `not_before`, `expires_at`, `issuer`, `kid`, and `flags` as
//! `Option<…>` and the fixture pack covers both
//! present-and-absent shapes.
//!
//! Trial-mode invariants from ADR-009 §Token lifetimes and trial mode
//! (line 85: `expires_at - issued_at <= 14 days`, `machine_binding`
//! required, `not_before == issued_at`) are RUNTIME-enforcement
//! invariants checked at the entitlement-evaluator boundary
//! (`EV-ENTITLEMENT-ENFORCEMENT`, Phase 4a). The schema-shape
//! contract pinned here does not validate them; a trial fixture in
//! [`crate`] tests carries them as an example of well-formed input
//! but the deserializer accepts a trial token without
//! `machine_binding` so the Phase-4a verifier has the full
//! input space to police.
//!
//! # Canonical CBOR
//!
//! ADR-009 line 37 mandates Ed25519 signature over the canonical
//! CBOR encoding of the payload excluding `signature`. Phase 0
//! pins the encoding determinism property — encode → decode → re-
//! encode is byte-identical — so the Phase-4a verifier can reuse
//! the same path. ciborium's serde-derived struct emission gives
//! this property naturally for the `entitlement-v2` shape: every
//! map key is a string, no map key collides with another, and
//! Vec-encoded `capabilities` maintains insertion order. The
//! [`canonical_cbor_encode`] / [`canonical_cbor_decode`] helpers
//! wrap the ciborium calls and lower errors into
//! [`CanonicalCborError`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `entitlement-v2` token, modelled after the JSON example in
/// ADR-009 §Decision.
///
/// Every field carries `#[serde(rename = "...")]` mapping the Rust
/// `snake_case` identifier to the on-the-wire spelling. The schema
/// uses snake_case throughout (e.g. `issued_at`, `machine_binding`),
/// so the rename annotations are explicit-but-no-op against the
/// serde default — they're present so a future serde renaming
/// convention shift cannot silently break the wire format.
///
/// Forward-compat: any claim key not enumerated below is absorbed
/// into [`Token::extra`] via `#[serde(flatten)]` and round-trips
/// through both JSON and CBOR without data loss. This realizes
/// ADR-009 line 38's "Unknown claim keys are forward-compat-ignored,
/// not rejected" rule.
///
/// `BTreeMap` (rather than `HashMap`) backs the unknown-claim bag so
/// the canonical CBOR re-encode is deterministic regardless of
/// hasher seed; ciborium emits map entries in BTreeMap iteration
/// order, which is the lexicographic key order BTreeMap guarantees.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    /// Schema discriminator. ADR-009 v2 fixes this to
    /// `"entitlement-v2"`; future revisions bump the suffix.
    #[serde(rename = "schema")]
    pub schema: String,

    /// Opaque user-or-site identifier. ADR-009 line 24.
    #[serde(rename = "subject")]
    pub subject: String,

    /// RFC 3339 issuance timestamp. ADR-009 line 25 / 38 (required).
    /// String-typed; chrono / time conversion is the runtime
    /// evaluator's concern (Phase 4a), not the schema's.
    #[serde(rename = "issued_at")]
    pub issued_at: String,

    /// RFC 3339 not-before timestamp. Optional per ADR-009 line 38.
    #[serde(
        rename = "not_before",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub not_before: Option<String>,

    /// RFC 3339 expiry timestamp. Optional per ADR-009 line 38.
    #[serde(
        rename = "expires_at",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub expires_at: Option<String>,

    /// Issuer identifier (e.g. `"chromaticity-release"`). Optional
    /// per ADR-009 line 38.
    #[serde(rename = "issuer", skip_serializing_if = "Option::is_none", default)]
    pub issuer: Option<String>,

    /// Key identifier selecting which embedded public key verifies
    /// the signature. Optional per ADR-009 line 38; required at
    /// runtime by the Phase-4a verifier. Rotation discipline pinned
    /// in ADR-009 §Key rotation lines 100-103.
    #[serde(rename = "kid", skip_serializing_if = "Option::is_none", default)]
    pub kid: Option<String>,

    /// Capability strings the token entitles. ADR-009 line 30.
    /// Required per line 38. The strings live in
    /// [`crate::capability`].
    #[serde(rename = "capabilities")]
    pub capabilities: Vec<String>,

    /// Trial / beta / enterprise flags. Optional per ADR-009 line
    /// 38; the `Flags::default()` zero value is the paid-token
    /// shape. ADR-009 lines 84-87 enumerate per-flag semantics.
    #[serde(rename = "flags", skip_serializing_if = "Option::is_none", default)]
    pub flags: Option<Flags>,

    /// Machine binding claim — a hash binding the token to a
    /// specific install. Optional per ADR-009 line 38; required for
    /// trial tokens (line 85). The `algorithm` value is currently
    /// `"blake3-hmac"` per the ADR example.
    #[serde(
        rename = "machine_binding",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub machine_binding: Option<MachineBinding>,

    /// Session claim — an opaque session id with a revocability
    /// flag. Optional per ADR-009 line 38; required for the
    /// session-invalidation revocation vector (Phase 4a,
    /// `EV-ENTITLEMENT-REVOCATION`).
    #[serde(rename = "session", skip_serializing_if = "Option::is_none", default)]
    pub session: Option<Session>,

    /// Ed25519 signature over the canonical CBOR encoding of the
    /// payload excluding this field. ADR-009 line 37. The
    /// Phase-0 fixture pack treats the value as opaque
    /// (placeholder bytes are accepted); signature verification is
    /// Phase 4a.
    #[serde(rename = "signature")]
    pub signature: Signature,

    /// Forward-compat unknown-claim bag. Empty under v2 input;
    /// populated when a v3+ token rolls into a v2 install.
    /// `BTreeMap` (vs `HashMap`) keeps the canonical CBOR re-encode
    /// deterministic regardless of hasher seed.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Per-token flags. ADR-009 lines 84-87.
///
/// `default` means a paid token (no trial / beta / enterprise bit
/// set). The `Default` impl gives the paid-token shape directly so
/// a fixture builder can write `Flags { trial: true, ..Flags::default() }`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flags {
    /// Trial mode flag. ADR-009 line 85: `expires_at - issued_at`
    /// MUST be `<= 14 days`, `machine_binding` MUST be present,
    /// `not_before` MUST equal `issued_at`. Schema-shape only here;
    /// the Phase-4a runtime verifier enforces.
    #[serde(rename = "trial", default)]
    pub trial: bool,

    /// Beta mode flag. ADR-009 line 86: longer expiries permitted;
    /// surfaced in support bundles.
    #[serde(rename = "beta", default)]
    pub beta: bool,

    /// Enterprise mode flag. ADR-009 line 87: site-licensed; may
    /// waive `machine_binding`.
    #[serde(rename = "enterprise", default)]
    pub enterprise: bool,
}

/// Machine-binding claim. ADR-009 line 32 example uses
/// `algorithm = "blake3-hmac"`; the schema accepts any string so
/// post-MVP rotation to a different KDF doesn't require a token-
/// format bump.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineBinding {
    /// Hash / MAC algorithm identifier (e.g. `"blake3-hmac"`).
    #[serde(rename = "algorithm")]
    pub algorithm: String,

    /// Hex / base64-encoded digest bytes. The schema is opaque on
    /// encoding; the Phase-4a runtime verifier picks one and
    /// rejects the other.
    #[serde(rename = "digest")]
    pub digest: String,
}

/// Session claim. ADR-009 line 33.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Opaque session identifier.
    #[serde(rename = "id")]
    pub id: String,

    /// `true` iff the server can invalidate this session via
    /// `SESSION_INVALIDATE`-equivalent (ADR-009 line 97).
    #[serde(rename = "revocable")]
    pub revocable: bool,
}

/// Signature claim. ADR-009 line 34 example uses
/// `algorithm = "ed25519"`. The Phase-0 fixture pack treats the
/// value as opaque bytes-as-string (matching the ADR example which
/// uses an elided string placeholder).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// Signature algorithm identifier (`"ed25519"` at v2).
    #[serde(rename = "algorithm")]
    pub algorithm: String,

    /// Encoded signature value. Schema-opaque encoding (the
    /// Phase-4a verifier picks one).
    #[serde(rename = "value")]
    pub value: String,
}

/// Error type for the canonical CBOR encode / decode helpers.
#[derive(Debug)]
pub enum CanonicalCborError {
    /// `ciborium::ser::into_writer` returned an error during encode.
    Encode(String),
    /// `ciborium::de::from_reader` returned an error during decode.
    Decode(String),
}

impl std::fmt::Display for CanonicalCborError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encode(message) => write!(f, "canonical CBOR encode failed: {message}"),
            Self::Decode(message) => write!(f, "canonical CBOR decode failed: {message}"),
        }
    }
}

impl std::error::Error for CanonicalCborError {}

/// Canonical CBOR encode: serialize a serde-derivable value to a
/// CBOR byte vector with deterministic ordering.
///
/// ciborium's struct emission produces map entries in serde struct-
/// field declaration order; the [`Token`] struct's
/// `#[serde(flatten)]` extra bag is a `BTreeMap`, so unknown-claim
/// entries emit in lexicographic key order. Together this gives the
/// byte-identity re-encode property the fixture pack pins.
pub fn canonical_cbor_encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonicalCborError> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes)
        .map_err(|err| CanonicalCborError::Encode(err.to_string()))?;
    Ok(bytes)
}

/// Canonical CBOR decode: deserialize a serde-derivable value from
/// a CBOR byte slice.
pub fn canonical_cbor_decode<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
) -> Result<T, CanonicalCborError> {
    ciborium::de::from_reader(bytes).map_err(|err| CanonicalCborError::Decode(err.to_string()))
}

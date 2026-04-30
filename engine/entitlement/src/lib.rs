// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity entitlement token format fixture pack.
//!
//! Phase-0 evidence harness for `EV-ENTITLEMENT-TOKEN-FORMAT`
//! (`docs/26-evidence-artifact-registry.md` row 88). Realizes the
//! locked `entitlement-v2` token schema from
//! `docs/adr/ADR-009-activation-and-entitlements.md` as Rust types
//! and exercises round-trip serialization over JSON and canonical
//! CBOR.
//!
//! # Scope
//!
//! This crate is **fixture-pack only**. It pins the schema-shape
//! contract (field names, optionality, value types) and the
//! deterministic-CBOR encode round-trip property that the future
//! signature-verification path (Phase 4a, `EV-ENTITLEMENT-ENFORCEMENT`)
//! will sign over. It does NOT implement:
//!
//! - Signature verification (Ed25519 over canonical CBOR; Phase 4a).
//! - Offline 14-day expiry behaviour (`EV-ENTITLEMENT-OFFLINE`,
//!   Phase 4a).
//! - Deny-list / session-revocation evaluation
//!   (`EV-ENTITLEMENT-REVOCATION`, Phase 4a).
//! - The `chromaticity.entitled(capability) -> bool` runtime
//!   surface (`EV-ENTITLEMENT-ENFORCEMENT`, Phase 4a).
//!
//! The capability namespace constants in [`capability`] are a
//! parsing target only — they exist so a token's
//! `capabilities: Vec<String>` field can be populated from a
//! known-good source in fixtures and so the namespace itself is
//! pinned alongside the schema. Wiring any of these strings to a
//! runtime entitlement evaluator is Phase 4a.
//!
//! # Surface summary
//!
//! - [`token::Token`] — the `entitlement-v2` schema as a serde-
//!   derived struct, with the optional `machine_binding` and
//!   `session` claims modelled as `Option<…>` and the unknown-claim
//!   forward-compat bag modelled as a `#[serde(flatten)]`
//!   `BTreeMap<String, serde_json::Value>` (per ADR-009 line 38:
//!   "Unknown claim keys are forward-compat-ignored, not rejected").
//! - [`token::canonical_cbor_encode`] /
//!   [`token::canonical_cbor_decode`] — the canonical CBOR
//!   encode / decode helpers the fixture pack pins for byte-identity
//!   re-encode determinism.
//! - [`capability`] — the MVP capability-namespace constants
//!   enumerated in ADR-009 §Capability namespace, exposed as
//!   `&'static str` constants so fixtures can populate
//!   `Token::capabilities` from a known-good source.
//!
//! # Forward-compat posture
//!
//! ADR-009 line 38 mandates that unknown future claim keys are
//! forward-compat-preserved, not rejected. The [`token::Token`]
//! struct therefore does NOT carry `#[serde(deny_unknown_fields)]`;
//! instead, it has an `extra` field marked
//! `#[serde(flatten)]` that absorbs any claim key not enumerated in
//! the schema. This keeps the fixture pack's unknown-claim
//! round-trip honest — a v3 token issued to a v2 install
//! deserialises without data loss and re-serialises bit-identically
//! through both JSON and CBOR.
//!
//! Produces evidence for `EV-ENTITLEMENT-TOKEN-FORMAT` per the
//! `engine/entitlement/tests/token_fixtures.rs` corpus. Registry
//! row 88 transition from `Planned`/`pending:artifact` to `Pass`
//! lands on a separate `docs(repo)` follow-up commit alongside the
//! Phase-0 gate count update (9-of-19 → 10-of-19 in
//! `docs/EV-GATE-MODEL-AUDIT-checklist.md`).
//!
//! # Governing ADR
//!
//! - ADR-009 — activation and entitlements
//!   (`docs/adr/ADR-009-activation-and-entitlements.md`).

pub mod capability;
pub mod token;

pub use token::{
    CanonicalCborError, Flags, MachineBinding, Session, Signature, Token, canonical_cbor_decode,
    canonical_cbor_encode,
};

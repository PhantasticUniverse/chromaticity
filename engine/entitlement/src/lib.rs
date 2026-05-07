// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity entitlement token format fixture pack.
//!
//! Phase-0 fixture-pack origin for `EV-ENTITLEMENT-TOKEN-FORMAT`
//! (`docs/26-evidence-artifact-registry.md` row 88), widened through
//! Phase-4a Wave 1a + 1b to the runtime enforcement surface
//! ([`enforcement::entitled`] / [`enforcement::is_valid_at`] /
//! [`enforcement::HONORED_TOKEN_CAPABILITIES`]). Realizes the locked
//! `entitlement-v2` token schema from
//! `docs/adr/ADR-009-activation-and-entitlements.md` as Rust types
//! and exercises round-trip serialization over JSON and canonical
//! CBOR.
//!
//! # Scope
//!
//! The schema-shape contract (field names, optionality, value types)
//! and the deterministic-CBOR encode round-trip property are pinned
//! at Phase-0 quality. Phase-4a Wave 1a (anchor run `25213475549`)
//! landed the runtime enforcement surface for `EV-ENTITLEMENT-ENFORCEMENT`
//! (row 87), `EV-ENTITLEMENT-OFFLINE` (row 88), `EV-ENTITLEMENT-REVOCATION`
//! (row 89), and `EV-ENTITLEMENT-FREE-FEATURES` (row 91) under the
//! MVP watermark-only narrowing — all four rows at `Pass`. Phase-4a
//! Wave 1b widened [`enforcement::HONORED_TOKEN_CAPABILITIES`] from
//! the initial 1-cap honored set to 13 entries (anchor run
//! `25219329314`); IO_CAMERA is intentionally excluded — free at
//! MVP per ADR-009.
//!
//! Still-future scope:
//!
//! - Signature verification (Ed25519 over canonical CBOR) — the
//!   token type carries the [`token::Signature`] field but no
//!   verification path is wired; production activation lives outside
//!   this crate.
//! - Deny-list activation for `EV-ENTITLEMENT-REVOCATION` is deferred
//!   post-MVP; the row 89 narrowing covers expiry-based revocation
//!   only.
//!
//! The capability namespace constants in [`capability`] back the
//! `Token::capabilities: Vec<String>` field; the runtime enforcement
//! surface in [`enforcement`] consumes them via the
//! [`enforcement::HONORED_TOKEN_CAPABILITIES`] honored set.
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
pub mod enforcement;
pub mod token;

pub use enforcement::{
    Clock, FREE_CAPABILITIES, FixedClock, HONORED_TOKEN_CAPABILITIES, SystemClock, entitled,
    is_valid_at,
};
pub use token::{
    CanonicalCborError, Flags, MachineBinding, Session, Signature, Token, canonical_cbor_decode,
    canonical_cbor_encode,
};

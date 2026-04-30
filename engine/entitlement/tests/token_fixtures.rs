// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-ENTITLEMENT-TOKEN-FORMAT (T2) — `entitlement-v2`
//! token round-trip corpus over JSON and canonical CBOR.
//!
//! Advances `docs/26-evidence-artifact-registry.md` row 88
//! (`EV-ENTITLEMENT-TOKEN-FORMAT`) past `pending:artifact` toward
//! Phase-0 exit. T2 per `docs/43-development-standards.md` §15.5:
//! corpus conformance over a fixture set with stable golden
//! assertions.
//!
//! Coverage matrix (matches the registry row 88 generation source):
//!
//! - **Paid token (no flags), `machine_binding` absent.** Default
//!   shape per ADR-009 §Token lifetimes: paid lifetime token;
//!   1-year expiry. Round-trips through JSON and CBOR.
//! - **Trial token, `machine_binding` present.** ADR-009 line 85.
//! - **Beta token, `machine_binding` absent.** ADR-009 line 86.
//! - **Enterprise token, `machine_binding` absent.** ADR-009 line
//!   87: enterprise tokens may waive machine binding.
//! - **`kid` rotation.** Two tokens carrying the previous and
//!   current rotation epochs (ADR-009 §Key rotation lines 100-103,
//!   "key rotation cadence: annual + on-incident") both parse and
//!   re-serialize.
//! - **Canonical CBOR determinism.** Every fixture's
//!   encode → decode → re-encode is byte-identical (the gate
//!   property the Phase-4a Ed25519 verifier will rely on per
//!   ADR-009 line 37).
//! - **Capability namespace.** A fixture token enumerating every
//!   ADR-009 §Capability namespace string parses cleanly and the
//!   namespace stays in the [`crate::capability::ALL`] order.
//! - **Unknown-claim forward-compat.** A v3+ token shape (with two
//!   future claim keys not in the v2 schema) round-trips through
//!   both JSON and CBOR without data loss; the future claims land
//!   in [`Token::extra`] and re-serialize at the same byte
//!   positions per the BTreeMap deterministic-iteration property.
//!
//! Out of scope (deferred to Phase 4a per docs/26 rows 85-87):
//!
//! - Signature verification (Ed25519 over canonical CBOR).
//! - Trial-mode invariant enforcement (`expires_at - issued_at <=
//!   14 days`, `machine_binding` required, `not_before == issued_at`).
//! - Deny-list and session-revocation evaluation.
//! - The `chromaticity.entitled(cap) -> bool` runtime evaluator.
//!
//! Anchors:
//!
//! - `docs/adr/ADR-009-activation-and-entitlements.md` §Decision
//!   (token schema, line 37 canonical CBOR, line 38 optionality +
//!   forward-compat).
//! - `docs/26-evidence-artifact-registry.md` row 88 —
//!   `EV-ENTITLEMENT-TOKEN-FORMAT`.
//! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::collections::BTreeMap;

use chromaticity_entitlement::{
    Flags, MachineBinding, Session, Signature, Token, canonical_cbor_decode, canonical_cbor_encode,
    capability,
};
use serde_json::{Value, json};

/// Build a paid-shape token with the canonical claim values from the
/// ADR-009 example (`subject`, `issued_at`, etc. mirror the JSON
/// example values from lines 21-36 where applicable).
fn paid_token_fixture() -> Token {
    Token {
        schema: "entitlement-v2".to_owned(),
        subject: "user-fixture-paid-001".to_owned(),
        issued_at: "2026-04-22T14:30:00Z".to_owned(),
        not_before: Some("2026-04-22T14:30:00Z".to_owned()),
        expires_at: Some("2027-04-22T14:30:00Z".to_owned()),
        issuer: Some("chromaticity-release".to_owned()),
        kid: Some("chromaticity-release-2026-04".to_owned()),
        capabilities: vec![
            capability::RENDER_WATERMARK_OFF.to_owned(),
            capability::IO_OSC_INPUT.to_owned(),
            capability::IO_OSC_OUTPUT.to_owned(),
        ],
        flags: Some(Flags::default()),
        machine_binding: None,
        session: None,
        signature: Signature {
            algorithm: "ed25519".to_owned(),
            value: "AA".repeat(32),
        },
        extra: BTreeMap::new(),
    }
}

fn trial_token_fixture() -> Token {
    Token {
        schema: "entitlement-v2".to_owned(),
        subject: "user-fixture-trial-002".to_owned(),
        issued_at: "2026-04-22T14:30:00Z".to_owned(),
        // ADR-009 line 85: trial requires `not_before == issued_at`.
        not_before: Some("2026-04-22T14:30:00Z".to_owned()),
        // 14-day trial ceiling per ADR-009 line 85.
        expires_at: Some("2026-05-06T14:30:00Z".to_owned()),
        issuer: Some("chromaticity-release".to_owned()),
        kid: Some("chromaticity-release-2026-04".to_owned()),
        capabilities: vec![capability::RENDER_WATERMARK_OFF.to_owned()],
        flags: Some(Flags {
            trial: true,
            ..Flags::default()
        }),
        machine_binding: Some(MachineBinding {
            algorithm: "blake3-hmac".to_owned(),
            // Hex digest placeholder; signature verification is
            // Phase 4a, the schema is opaque on encoding.
            digest: "abcdef0123456789".repeat(4),
        }),
        session: None,
        signature: Signature {
            algorithm: "ed25519".to_owned(),
            value: "BB".repeat(32),
        },
        extra: BTreeMap::new(),
    }
}

fn beta_token_fixture() -> Token {
    Token {
        schema: "entitlement-v2".to_owned(),
        subject: "user-fixture-beta-003".to_owned(),
        issued_at: "2026-04-01T00:00:00Z".to_owned(),
        not_before: Some("2026-04-01T00:00:00Z".to_owned()),
        expires_at: Some("2026-09-01T00:00:00Z".to_owned()),
        issuer: Some("chromaticity-release".to_owned()),
        kid: Some("chromaticity-release-2026-04".to_owned()),
        capabilities: vec![
            capability::RENDER_WATERMARK_OFF.to_owned(),
            capability::AUTHORING_IDE.to_owned(),
        ],
        flags: Some(Flags {
            beta: true,
            ..Flags::default()
        }),
        // ADR-009 line 86: beta tokens have no specific
        // machine_binding requirement; this fixture omits it.
        machine_binding: None,
        // Beta tokens may carry a session for support-bundle
        // surfacing; this fixture exercises that shape.
        session: Some(Session {
            id: "beta-session-fixture-003".to_owned(),
            revocable: true,
        }),
        signature: Signature {
            algorithm: "ed25519".to_owned(),
            value: "CC".repeat(32),
        },
        extra: BTreeMap::new(),
    }
}

fn enterprise_token_fixture() -> Token {
    Token {
        schema: "entitlement-v2".to_owned(),
        subject: "site-fixture-enterprise-004".to_owned(),
        issued_at: "2026-01-01T00:00:00Z".to_owned(),
        not_before: Some("2026-01-01T00:00:00Z".to_owned()),
        expires_at: Some("2027-01-01T00:00:00Z".to_owned()),
        issuer: Some("chromaticity-release".to_owned()),
        kid: Some("chromaticity-release-2026-01".to_owned()),
        capabilities: vec![
            capability::RENDER_WATERMARK_OFF.to_owned(),
            capability::IO_NDI_INPUT.to_owned(),
            capability::IO_NDI_OUTPUT.to_owned(),
            capability::AUTHORING_EDIT.to_owned(),
            capability::AUTHORING_BUILTIN_SCENES.to_owned(),
        ],
        flags: Some(Flags {
            enterprise: true,
            ..Flags::default()
        }),
        // ADR-009 line 87: enterprise tokens may waive
        // machine_binding (site-licensed).
        machine_binding: None,
        session: None,
        signature: Signature {
            algorithm: "ed25519".to_owned(),
            value: "DD".repeat(32),
        },
        extra: BTreeMap::new(),
    }
}

// --- JSON round-trip ------------------------------------------------------

#[test]
fn paid_token_round_trips_json() {
    let token = paid_token_fixture();
    let serialized = serde_json::to_string(&token).expect("paid token JSON serialize");
    let parsed: Token = serde_json::from_str(&serialized).expect("paid token JSON deserialize");
    assert_eq!(token, parsed);
}

#[test]
fn trial_token_with_machine_binding_round_trips_json() {
    let token = trial_token_fixture();
    let serialized = serde_json::to_string(&token).expect("trial token JSON serialize");
    let parsed: Token = serde_json::from_str(&serialized).expect("trial token JSON deserialize");
    assert_eq!(token, parsed);
    // ADR-009 line 85 mandates machine_binding for trial tokens; pin
    // its presence on the round-tripped value as a schema-shape
    // contract pin (runtime invariant enforcement is Phase 4a).
    assert!(parsed.machine_binding.is_some());
    assert_eq!(
        parsed.flags.as_ref().map(|f| f.trial),
        Some(true),
        "trial flag must round-trip"
    );
}

#[test]
fn beta_token_without_machine_binding_round_trips_json() {
    let token = beta_token_fixture();
    let serialized = serde_json::to_string(&token).expect("beta token JSON serialize");
    let parsed: Token = serde_json::from_str(&serialized).expect("beta token JSON deserialize");
    assert_eq!(token, parsed);
    assert!(parsed.machine_binding.is_none());
    assert_eq!(parsed.flags.as_ref().map(|f| f.beta), Some(true));
}

#[test]
fn enterprise_token_without_machine_binding_round_trips_json() {
    let token = enterprise_token_fixture();
    let serialized = serde_json::to_string(&token).expect("enterprise token JSON serialize");
    let parsed: Token =
        serde_json::from_str(&serialized).expect("enterprise token JSON deserialize");
    assert_eq!(token, parsed);
    assert!(parsed.machine_binding.is_none());
    assert_eq!(parsed.flags.as_ref().map(|f| f.enterprise), Some(true));
}

#[test]
fn json_omits_optional_fields_when_absent() {
    // ADR-009 line 38: optional claims are omitted when not present.
    // The schema's `skip_serializing_if = "Option::is_none"` keeps
    // the wire shape compact.
    let mut token = paid_token_fixture();
    token.machine_binding = None;
    token.session = None;
    let serialized = serde_json::to_string(&token).expect("serialize without optionals");
    assert!(
        !serialized.contains("machine_binding"),
        "absent machine_binding leaked into JSON: {serialized}"
    );
    assert!(
        !serialized.contains("\"session\""),
        "absent session leaked into JSON: {serialized}"
    );
}

// --- CBOR round-trip ------------------------------------------------------

#[test]
fn paid_token_round_trips_cbor() {
    let token = paid_token_fixture();
    let bytes = canonical_cbor_encode(&token).expect("paid token CBOR encode");
    let parsed: Token = canonical_cbor_decode(&bytes).expect("paid token CBOR decode");
    assert_eq!(token, parsed);
}

#[test]
fn trial_token_with_machine_binding_round_trips_cbor() {
    let token = trial_token_fixture();
    let bytes = canonical_cbor_encode(&token).expect("trial token CBOR encode");
    let parsed: Token = canonical_cbor_decode(&bytes).expect("trial token CBOR decode");
    assert_eq!(token, parsed);
    assert!(parsed.machine_binding.is_some());
}

#[test]
fn beta_token_without_machine_binding_round_trips_cbor() {
    let token = beta_token_fixture();
    let bytes = canonical_cbor_encode(&token).expect("beta token CBOR encode");
    let parsed: Token = canonical_cbor_decode(&bytes).expect("beta token CBOR decode");
    assert_eq!(token, parsed);
    assert!(parsed.machine_binding.is_none());
}

#[test]
fn enterprise_token_without_machine_binding_round_trips_cbor() {
    let token = enterprise_token_fixture();
    let bytes = canonical_cbor_encode(&token).expect("enterprise token CBOR encode");
    let parsed: Token = canonical_cbor_decode(&bytes).expect("enterprise token CBOR decode");
    assert_eq!(token, parsed);
    assert!(parsed.machine_binding.is_none());
}

// --- Canonical CBOR determinism (the gate test) --------------------------

/// The byte-identity re-encode property ADR-009 line 37 requires for
/// the Phase-4a Ed25519 signature path: encode → decode → re-encode
/// produces identical bytes. Exercises every fixture in the corpus
/// so a determinism regression on any shape (paid / trial / beta /
/// enterprise / unknown-claim) fires.
#[test]
fn canonical_cbor_encode_is_byte_identity_across_round_trip() {
    let fixtures: &[(&str, Token)] = &[
        ("paid", paid_token_fixture()),
        ("trial", trial_token_fixture()),
        ("beta", beta_token_fixture()),
        ("enterprise", enterprise_token_fixture()),
        ("kid_rotation_previous", kid_previous_epoch_fixture()),
        ("kid_rotation_current", kid_current_epoch_fixture()),
        ("forward_compat", forward_compat_token_fixture()),
        ("namespace_full", capability_namespace_fixture()),
    ];
    for (name, token) in fixtures {
        let first = canonical_cbor_encode(token).unwrap_or_else(|_| {
            panic!("canonical_cbor_encode failed for fixture {name}");
        });
        let decoded: Token = canonical_cbor_decode(&first)
            .unwrap_or_else(|_| panic!("canonical_cbor_decode failed for fixture {name}"));
        let second = canonical_cbor_encode(&decoded).unwrap_or_else(|_| {
            panic!("canonical_cbor_encode (re-encode) failed for fixture {name}")
        });
        assert_eq!(
            first, second,
            "canonical CBOR re-encode diverged for fixture {name}: \
             first={first:02x?} second={second:02x?}",
        );
    }
}

// --- `kid` rotation -------------------------------------------------------

/// ADR-009 §Key rotation: previous epoch (annual cadence). Pinned
/// against the kid that the §Decision example uses
/// (`chromaticity-release-2026-01`).
fn kid_previous_epoch_fixture() -> Token {
    let mut token = paid_token_fixture();
    token.subject = "user-fixture-kid-prev-005".to_owned();
    token.kid = Some("chromaticity-release-2026-01".to_owned());
    token
}

/// ADR-009 §Key rotation: current epoch
/// (`chromaticity-release-2026-04`).
fn kid_current_epoch_fixture() -> Token {
    let mut token = paid_token_fixture();
    token.subject = "user-fixture-kid-curr-006".to_owned();
    token.kid = Some("chromaticity-release-2026-04".to_owned());
    token
}

#[test]
fn kid_rotation_round_trips_both_epochs_json() {
    for token in [kid_previous_epoch_fixture(), kid_current_epoch_fixture()] {
        let serialized = serde_json::to_string(&token).expect("kid-rotation JSON serialize");
        let parsed: Token =
            serde_json::from_str(&serialized).expect("kid-rotation JSON deserialize");
        assert_eq!(token, parsed);
        // The kid value must round-trip exactly; rotation discipline
        // depends on it.
        assert_eq!(token.kid, parsed.kid);
    }
}

#[test]
fn kid_rotation_round_trips_both_epochs_cbor() {
    for token in [kid_previous_epoch_fixture(), kid_current_epoch_fixture()] {
        let bytes = canonical_cbor_encode(&token).expect("kid-rotation CBOR encode");
        let parsed: Token = canonical_cbor_decode(&bytes).expect("kid-rotation CBOR decode");
        assert_eq!(token, parsed);
        assert_eq!(token.kid, parsed.kid);
    }
}

// --- Capability namespace -------------------------------------------------

fn capability_namespace_fixture() -> Token {
    let mut token = paid_token_fixture();
    token.subject = "site-fixture-namespace-007".to_owned();
    token.capabilities = capability::ALL.iter().map(|s| (*s).to_owned()).collect();
    token
}

#[test]
fn capability_namespace_round_trips_full_set_json() {
    let token = capability_namespace_fixture();
    let serialized = serde_json::to_string(&token).expect("namespace JSON serialize");
    let parsed: Token = serde_json::from_str(&serialized).expect("namespace JSON deserialize");
    assert_eq!(token, parsed);
    assert_eq!(parsed.capabilities.len(), capability::ALL.len());
    // Insertion order must round-trip — namespace ordering is the
    // ADR-009 declaration order and downstream consumers may rely
    // on it.
    for (i, expected) in capability::ALL.iter().enumerate() {
        assert_eq!(parsed.capabilities[i], **expected);
    }
}

#[test]
fn capability_namespace_round_trips_full_set_cbor() {
    let token = capability_namespace_fixture();
    let bytes = canonical_cbor_encode(&token).expect("namespace CBOR encode");
    let parsed: Token = canonical_cbor_decode(&bytes).expect("namespace CBOR decode");
    assert_eq!(token, parsed);
    assert_eq!(parsed.capabilities.len(), capability::ALL.len());
}

// --- Signature placeholder ------------------------------------------------

#[test]
fn signature_field_accepts_placeholder_value() {
    // Phase 0 scope: signature verification is Phase 4a. The Token
    // schema treats Signature.value as opaque; a fixture with an
    // empty value still parses cleanly.
    let mut token = paid_token_fixture();
    token.signature.value = String::new();
    let serialized = serde_json::to_string(&token).expect("empty-sig JSON serialize");
    let parsed: Token = serde_json::from_str(&serialized).expect("empty-sig JSON deserialize");
    assert_eq!(token, parsed);
    assert!(parsed.signature.value.is_empty());

    let bytes = canonical_cbor_encode(&token).expect("empty-sig CBOR encode");
    let parsed_cbor: Token = canonical_cbor_decode(&bytes).expect("empty-sig CBOR decode");
    assert_eq!(token, parsed_cbor);
}

// --- Forward-compat unknown-claim preservation ---------------------------

/// A v3+ token shape: same v2 fields plus two future claim keys
/// (`device_count`, `cloud_sync`) per ADR-009 line 38's
/// "post-MVP additions (`cloud.sync`, `device.count`, metered
/// quotas) ship as data" note.
fn forward_compat_token_fixture() -> Token {
    let mut token = paid_token_fixture();
    token.subject = "user-fixture-forward-compat-008".to_owned();
    let mut extra = BTreeMap::new();
    extra.insert("cloud_sync".to_owned(), Value::Bool(true));
    extra.insert("device_count".to_owned(), json!(3));
    token.extra = extra;
    token
}

#[test]
fn unknown_claims_round_trip_through_json() {
    let token = forward_compat_token_fixture();
    let serialized = serde_json::to_string(&token).expect("forward-compat JSON serialize");
    // The unknown claims must appear in the on-the-wire JSON.
    assert!(serialized.contains("\"cloud_sync\":true"));
    assert!(serialized.contains("\"device_count\":3"));

    let parsed: Token = serde_json::from_str(&serialized).expect("forward-compat JSON deserialize");
    assert_eq!(token, parsed);
    assert_eq!(parsed.extra.len(), 2);
    assert_eq!(parsed.extra.get("cloud_sync"), Some(&Value::Bool(true)));
    assert_eq!(parsed.extra.get("device_count"), Some(&json!(3)));
}

#[test]
fn unknown_claims_round_trip_through_cbor() {
    let token = forward_compat_token_fixture();
    let bytes = canonical_cbor_encode(&token).expect("forward-compat CBOR encode");
    let parsed: Token = canonical_cbor_decode(&bytes).expect("forward-compat CBOR decode");
    assert_eq!(token, parsed);
    assert_eq!(parsed.extra.len(), 2);
    assert_eq!(parsed.extra.get("cloud_sync"), Some(&Value::Bool(true)));
    assert_eq!(parsed.extra.get("device_count"), Some(&json!(3)));
}

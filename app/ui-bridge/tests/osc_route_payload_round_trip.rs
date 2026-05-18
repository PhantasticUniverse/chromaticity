// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Evidence: EV-INTEGRATION-MATRIX (T2) — engine→UI OSC route binding
//! IPC payload round-trip conformance (OSC arm, Wave 2.1 Lane D.3).
//!
//! T2 per docs/43 §15.5: this corpus drives the
//! [`chromaticity_ui_bridge::OscRoutePayload`] +
//! [`chromaticity_ui_bridge::OscDirectionPayload`] +
//! [`chromaticity_ui_bridge::OscRouteFamilyPayload`] newtypes across
//! the bidirectional
//! `From<chromaticity_osc_adapter::route_registry::OscRouteEntry>`
//! conversion + serde JSON wire shape. Field-by-field equality across
//! the 7-field surface (canonical / compat_alias / type_tag / range /
//! direction / family / bind_target) is the load-bearing assertion:
//! any future payload-shape divergence between the engine entry and
//! the bridge newtype surfaces here before the committed ts-rs
//! binding under `src/lib/ipc/bindings/OscRoutePayload.ts` drifts.
//!
//! Lives under `app/ui-bridge/tests/` (outside the
//! `engine/**`/`integrations/**` evidence-tag-lint coverage scope per
//! `scripts/evidence-tag-lint.sh`); the in-source T1 drift pin under
//! `integrations/osc-adapter/src/route_registry.rs` carries the
//! registry-anchored seed + bind / unbind invariants this test layer
//! composes on top of.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_osc_adapter::route_registry::{
    OscDirection, OscRouteEntry, OscRouteFamily, OscRouteRegistry,
};
use chromaticity_ui_bridge::{OscDirectionPayload, OscRouteFamilyPayload, OscRoutePayload};
use ts_rs::TS;

fn sample_entry() -> OscRouteEntry {
    OscRouteEntry {
        canonical: "/controls/{bank}/{name}".to_string(),
        compat_alias: Some("/controls/{addr}".to_string()),
        type_tag: "f".to_string(),
        range: "0.0 – 1.0".to_string(),
        direction: OscDirection::Both,
        family: OscRouteFamily::Control,
        bind_target: Some("control:scene:fader-1".to_string()),
    }
}

fn sample_payload() -> OscRoutePayload {
    OscRoutePayload {
        canonical: "/controls/{bank}/{name}".to_string(),
        compat_alias: Some("/controls/{addr}".to_string()),
        type_tag: "f".to_string(),
        range: "0.0 – 1.0".to_string(),
        direction: OscDirectionPayload::Both,
        family: OscRouteFamilyPayload::Control,
        bind_target: Some("control:scene:fader-1".to_string()),
    }
}

#[test]
fn engine_to_payload_field_by_field_equality() {
    let entry = sample_entry();
    let payload: OscRoutePayload = entry.clone().into();

    assert_eq!(payload.canonical, entry.canonical);
    assert_eq!(payload.compat_alias, entry.compat_alias);
    assert_eq!(payload.type_tag, entry.type_tag);
    assert_eq!(payload.range, entry.range);
    assert_eq!(payload.direction, OscDirectionPayload::Both);
    assert_eq!(payload.family, OscRouteFamilyPayload::Control);
    assert_eq!(payload.bind_target, entry.bind_target);
}

#[test]
fn payload_to_engine_round_trip_field_by_field_equality() {
    let payload = sample_payload();
    let entry: OscRouteEntry = payload.clone().into();

    assert_eq!(entry.canonical, payload.canonical);
    assert_eq!(entry.compat_alias, payload.compat_alias);
    assert_eq!(entry.type_tag, payload.type_tag);
    assert_eq!(entry.range, payload.range);
    assert_eq!(entry.direction, OscDirection::Both);
    assert_eq!(entry.family, OscRouteFamily::Control);
    assert_eq!(entry.bind_target, payload.bind_target);
}

#[test]
fn payload_json_round_trip_field_by_field_equality() {
    let payload = sample_payload();
    let json = serde_json::to_string(&payload).expect("serialize payload");
    let decoded: OscRoutePayload = serde_json::from_str(&json).expect("deserialize payload");
    assert_eq!(decoded, payload);
}

#[test]
fn direction_payload_renders_as_flat_string() {
    // `OscDirectionPayload` is a unit-variant enum; serde renders it
    // as a bare JSON string. The frontend type-import treats it as a
    // string union, so the wire shape must NOT carry a tag object.
    for (variant, repr) in [
        (OscDirectionPayload::In, "\"In\""),
        (OscDirectionPayload::Out, "\"Out\""),
        (OscDirectionPayload::Both, "\"Both\""),
    ] {
        let json = serde_json::to_string(&variant).expect("serialize variant");
        assert_eq!(json, repr);
        let decoded: OscDirectionPayload = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, variant);
    }
}

#[test]
fn family_payload_renders_as_flat_string() {
    for (variant, repr) in [
        (OscRouteFamilyPayload::Control, "\"Control\""),
        (OscRouteFamilyPayload::Scene, "\"Scene\""),
        (OscRouteFamilyPayload::Preset, "\"Preset\""),
        (OscRouteFamilyPayload::Snap, "\"Snap\""),
        (OscRouteFamilyPayload::Playlist, "\"Playlist\""),
        (OscRouteFamilyPayload::Audio, "\"Audio\""),
        (OscRouteFamilyPayload::Reserved, "\"Reserved\""),
    ] {
        let json = serde_json::to_string(&variant).expect("serialize variant");
        assert_eq!(json, repr);
        let decoded: OscRouteFamilyPayload = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, variant);
    }
}

#[test]
fn default_seed_round_trips_through_payload() {
    // The shell-side handler iterates `OscRouteRegistry::snapshot()`
    // and converts each entry to the bridge payload. Pin that every
    // seeded entry survives the round trip without semantic loss.
    let registry = OscRouteRegistry::with_default_seed();
    for entry in registry.snapshot() {
        let payload: OscRoutePayload = entry.clone().into();
        let back: OscRouteEntry = payload.into();
        assert_eq!(back, entry);
    }
}

#[test]
fn bind_target_none_round_trips_as_serde_null() {
    // The default unbound entry has `bind_target = None`. Verify the
    // JSON renders the option as `null` rather than omitting the
    // field, so the frontend can pattern-match on the presence of
    // the key without a `key in obj` guard.
    let entry = OscRouteEntry {
        bind_target: None,
        ..sample_entry()
    };
    let payload: OscRoutePayload = entry.into();
    let json = serde_json::to_string(&payload).expect("serialize");
    assert!(
        json.contains("\"bind_target\":null"),
        "expected explicit null bind_target field, got {json}"
    );
}

#[test]
fn ts_rs_exported_names_match_committed_bindings() {
    assert_eq!(<OscRoutePayload as TS>::name(), "OscRoutePayload");
    assert_eq!(<OscDirectionPayload as TS>::name(), "OscDirectionPayload");
    assert_eq!(
        <OscRouteFamilyPayload as TS>::name(),
        "OscRouteFamilyPayload"
    );
}

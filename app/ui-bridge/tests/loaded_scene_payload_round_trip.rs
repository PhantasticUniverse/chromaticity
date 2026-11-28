// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-INTEGRATION-MATRIX (T2) — Wave 2.1 Lane A.1 scene-load
//! IPC payload round-trip conformance.
//!
//! T2 per docs/43 §15.5: this corpus drives the
//! [`chromaticity_ui_bridge::LoadedScenePayload`] +
//! [`chromaticity_ui_bridge::ControlSchemaPayload`] +
//! [`chromaticity_ui_bridge::ControlKindPayload`] +
//! [`chromaticity_ui_bridge::ControlDefaultValuePayload`] surfaces
//! across the ts-rs newtype mirror + bidirectional From conversions
//! against [`chromaticity_scene::schema::{Scene, ControlDecl,
//! ControlKind, ControlValue}`] + serde JSON wire shape per the
//! Library → Perform scene-load seam. Mirrors the
//! `audio_uniform_payload_round_trip.rs` Wave 1c-α precedent: any
//! future payload-shape divergence between the engine type and the
//! bridge wrapper surfaces here before the committed ts-rs binding
//! under `src/lib/ipc/bindings/LoadedScenePayload.ts` drifts.
//!
//! Lives under `app/ui-bridge/tests/` (outside the
//! `engine/**`/`integrations/**` evidence-tag-lint coverage scope per
//! `scripts/evidence-tag-lint.sh`); the EV-INTEGRATION-MATRIX
//! row carries the registry-anchored `Evidence:` tag on the engine
//! side already.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_scene::schema::{ControlDecl, ControlKind, ControlValue};
use chromaticity_ui_bridge::{
    ControlDefaultValuePayload, ControlKindPayload, ControlSchemaPayload, LoadedScenePayload,
};
use ts_rs::TS;

#[test]
fn control_kind_payload_round_trips_all_variants() {
    // Every variant of the engine-side ControlKind must round-trip
    // through the bridge wrapper unchanged. The custom catch-all
    // arm (`_ => Self::Slider`) covers any future #[non_exhaustive]
    // addition; the explicit-variant arms below pin the current set.
    let cases = [
        (ControlKind::Slider, ControlKindPayload::Slider),
        (ControlKind::Toggle, ControlKindPayload::Toggle),
        (ControlKind::Color, ControlKindPayload::Color),
        (ControlKind::Xy, ControlKindPayload::Xy),
        (ControlKind::Bang, ControlKindPayload::Bang),
        (ControlKind::Dropdown, ControlKindPayload::Dropdown),
        (ControlKind::Knob, ControlKindPayload::Knob),
    ];
    for (engine, expected) in cases {
        let payload: ControlKindPayload = engine.into();
        assert_eq!(payload, expected);
        let back: ControlKind = payload.into();
        assert_eq!(back, engine);
    }
}

#[test]
fn control_default_value_payload_round_trips_all_variants() {
    let cases = [
        (
            ControlValue::Bool(true),
            ControlDefaultValuePayload::Bool(true),
        ),
        (
            ControlValue::Float(0.5),
            ControlDefaultValuePayload::Float(0.5),
        ),
        (
            ControlValue::Vec2([0.1, 0.2]),
            ControlDefaultValuePayload::Vec2([0.1, 0.2]),
        ),
        (
            ControlValue::Vec3([0.1, 0.2, 0.3]),
            ControlDefaultValuePayload::Vec3([0.1, 0.2, 0.3]),
        ),
        (
            ControlValue::Vec4([0.1, 0.2, 0.3, 1.0]),
            ControlDefaultValuePayload::Vec4([0.1, 0.2, 0.3, 1.0]),
        ),
    ];
    for (engine, expected) in cases {
        let payload: ControlDefaultValuePayload = engine.clone().into();
        assert_eq!(payload, expected);
        let back: ControlValue = payload.into();
        assert_eq!(back, engine);
    }
}

#[test]
fn control_default_value_payload_json_round_trip_uses_untagged_shape() {
    // The serde `#[serde(untagged)]` attribute means JSON encodes the
    // payload variant payload-only — no discriminant wrapper. ts-rs
    // mirrors that as a union of payload types. Pin the wire shape so
    // a future tagged-attribute drift fails this test before the
    // frontend binding drifts.
    let bool_json = serde_json::to_string(&ControlDefaultValuePayload::Bool(true)).unwrap();
    assert_eq!(bool_json, "true");
    let float_json = serde_json::to_string(&ControlDefaultValuePayload::Float(0.5)).unwrap();
    assert_eq!(float_json, "0.5");
    let vec2_json = serde_json::to_string(&ControlDefaultValuePayload::Vec2([0.1, 0.2])).unwrap();
    assert_eq!(vec2_json, "[0.1,0.2]");
}

#[test]
fn control_schema_payload_mirrors_engine_decl_fields() {
    // Field-by-field assertion against the engine-side ControlDecl
    // fields the Perform view binds against. Carries enough metadata
    // for the SceneControls panel to render names, kinds, defaults,
    // ranges, grouping hints, descriptions, and labels; the richer
    // LABELS / VALUES / PARAMS / DEF_COLOR fields ride forward when a
    // future widget retrofit needs them.
    let decl = ControlDecl {
        name: "brightness".to_string(),
        kind: Some(ControlKind::Slider),
        default: ControlValue::Float(0.5),
        min: Some(chromaticity_scene::schema::ControlBound::Scalar(0.0)),
        max: Some(chromaticity_scene::schema::ControlBound::Scalar(1.0)),
        ui_group: Some("routing".to_string()),
        description: Some("Output brightness".to_string()),
        params: None,
        def_color: None,
        labels: Vec::new(),
        values: Vec::new(),
        is_meta: Some(false),
        label: Some("Brightness".to_string()),
        extensions: serde_json::Map::new(),
    };
    let payload: ControlSchemaPayload = decl.into();
    assert_eq!(payload.name, "brightness");
    assert_eq!(payload.kind, Some(ControlKindPayload::Slider));
    assert_eq!(payload.default, ControlDefaultValuePayload::Float(0.5));
    assert_eq!(payload.min, Some(0.0));
    assert_eq!(payload.max, Some(1.0));
    assert_eq!(payload.ui_group.as_deref(), Some("routing"));
    assert_eq!(payload.description.as_deref(), Some("Output brightness"));
    assert_eq!(payload.label.as_deref(), Some("Brightness"));
}

#[test]
fn control_schema_payload_handles_optional_metadata_absence() {
    // Bare minimum ControlDecl — only NAME + DEFAULT — must produce
    // a payload with every optional field at None / empty defaults.
    let decl = ControlDecl {
        name: "x".to_string(),
        kind: None,
        default: ControlValue::Float(0.0),
        min: None,
        max: None,
        ui_group: None,
        description: None,
        params: None,
        def_color: None,
        labels: Vec::new(),
        values: Vec::new(),
        is_meta: None,
        label: None,
        extensions: serde_json::Map::new(),
    };
    let payload: ControlSchemaPayload = decl.into();
    assert_eq!(payload.name, "x");
    // No TYPE: the payload carries the CP-01 coerced kind (DEFAULT 0.0,
    // no MIN/MAX -> toggle, docs/28 §CP-01 step 2) so the UI renders the
    // declared shape instead of a fallback fader (audit 2026-11-28).
    assert_eq!(payload.kind, Some(ControlKindPayload::Toggle));
    assert!(payload.labels.is_none());
    assert!(payload.values.is_none());
    assert_eq!(payload.default, ControlDefaultValuePayload::Float(0.0));
    assert!(payload.min.is_none());
    assert!(payload.max.is_none());
    assert!(payload.ui_group.is_none());
    assert!(payload.description.is_none());
    assert!(payload.label.is_none());
}

#[test]
fn loaded_scene_payload_json_round_trip_field_by_field_equality() {
    // ADR-019-style wire-shape stability check (same precedent as
    // the audio-uniform payload). Tauri's invoke serialiser uses
    // serde_json under the hood, so any field rename or attribute
    // drift between the bridge wrapper and the engine-side struct
    // surfaces here before the frontend committed binding drifts.
    let payload = LoadedScenePayload {
        id: "demo-kaleido".to_string(),
        path: "/tmp/demo-kaleido.chroma".to_string(),
        name: "Demo Kaleido".to_string(),
        credit: "chromaticity demo".to_string(),
        description: "Six-fold radial mirror".to_string(),
        controls: vec![ControlSchemaPayload {
            name: "segments".to_string(),
            kind: Some(ControlKindPayload::Slider),
            default: ControlDefaultValuePayload::Float(6.0),
            min: Some(2.0),
            max: Some(24.0),
            ui_group: None,
            description: None,
            label: None,
            labels: None,
            values: None,
        }],
        schema_version: 1,
    };

    let json = serde_json::to_string(&payload).expect("serialize payload");
    let decoded: LoadedScenePayload = serde_json::from_str(&json).expect("deserialize payload");
    assert_eq!(decoded, payload);
    assert_eq!(decoded.controls.len(), 1);
    assert_eq!(decoded.controls[0].name, "segments");
}

#[test]
fn ts_rs_exported_names_match() {
    // Committed bindings live at `src/lib/ipc/bindings/<TypeName>.ts`.
    // ts-rs's `name()` is the source of truth for the export filename;
    // an accidental rename of the Rust newtype (or a `#[ts(rename = ...)]`
    // attribute) would surface here before the frontend index drifts.
    assert_eq!(<LoadedScenePayload as TS>::name(), "LoadedScenePayload");
    assert_eq!(<ControlSchemaPayload as TS>::name(), "ControlSchemaPayload");
    assert_eq!(<ControlKindPayload as TS>::name(), "ControlKindPayload");
    assert_eq!(
        <ControlDefaultValuePayload as TS>::name(),
        "ControlDefaultValuePayload"
    );
}

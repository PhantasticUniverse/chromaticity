// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-INTEGRATION-MATRIX (T2) — engine→UI MIDI device-listing
//! IPC payload round-trip conformance (MIDI arm, Wave 2.1 Lane D.1).
//!
//! T2 per docs/43 §15.5: this corpus drives the
//! [`chromaticity_ui_bridge::MidiDevicePayload`] +
//! [`chromaticity_ui_bridge::MidiDeviceKindPayload`] newtypes across
//! the bidirectional `From<chromaticity_midi_adapter::MidiDeviceDescriptor>`
//! conversion + serde JSON wire shape. Field-by-field equality across
//! the 3-field surface (one `id`, one `name`, one `kind`) is the
//! load-bearing assertion: any future payload-shape divergence between
//! the engine descriptor and the bridge newtype surfaces here before
//! the committed ts-rs binding under
//! `src/lib/ipc/bindings/MidiDevicePayload.ts` drifts.
//!
//! Lives under `app/ui-bridge/tests/` (outside the
//! `engine/**`/`integrations/**` evidence-tag-lint coverage scope per
//! `scripts/evidence-tag-lint.sh`); the in-source T1 drift pin under
//! `integrations/midi-adapter/src/adapter.rs` carries the registry-
//! anchored `Evidence:` tag the lint enforces.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_midi_adapter::{MidiDeviceDescriptor, MidiDeviceKind};
use chromaticity_ui_bridge::{MidiDeviceKindPayload, MidiDevicePayload};
use ts_rs::TS;

fn sample_descriptor() -> MidiDeviceDescriptor {
    MidiDeviceDescriptor {
        id: "input-0".to_string(),
        name: "Launchpad X".to_string(),
        kind: MidiDeviceKind::Input,
    }
}

fn sample_payload() -> MidiDevicePayload {
    MidiDevicePayload {
        id: "input-0".to_string(),
        name: "Launchpad X".to_string(),
        kind: MidiDeviceKindPayload::Input,
    }
}

#[test]
fn engine_to_payload_field_by_field_equality() {
    // Producer-side conversion: the engine descriptor flows through
    // the `From<MidiDeviceDescriptor>` impl. Field-by-field assertion
    // pins the 3-field surface so a future field-mapping bug (e.g. a
    // hand-rolled `From` impl swapping `id` and `name`) surfaces here.
    let descriptor = sample_descriptor();
    let payload: MidiDevicePayload = descriptor.clone().into();

    assert_eq!(payload.id, descriptor.id);
    assert_eq!(payload.name, descriptor.name);
    assert_eq!(payload.kind, MidiDeviceKindPayload::Input);
}

#[test]
fn payload_to_engine_round_trip_field_by_field_equality() {
    // Reverse direction: a payload constructed on the consumer side
    // (e.g. by deserialising an event payload, or by a test harness)
    // converts back into the engine descriptor with field identity
    // preserved.
    let payload = sample_payload();
    let descriptor: MidiDeviceDescriptor = payload.clone().into();

    assert_eq!(descriptor.id, payload.id);
    assert_eq!(descriptor.name, payload.name);
    assert_eq!(descriptor.kind, MidiDeviceKind::Input);
}

#[test]
fn payload_json_round_trip_field_by_field_equality() {
    // Pin serde JSON round-trip identity so the wire shape stays
    // stable across the engine→Tauri→Svelte path. A future
    // serde-tagged attribute or rename would surface here.
    let payload = sample_payload();
    let json = serde_json::to_string(&payload).expect("serialize payload");
    let decoded: MidiDevicePayload = serde_json::from_str(&json).expect("deserialize payload");

    assert_eq!(decoded.id, payload.id);
    assert_eq!(decoded.name, payload.name);
    assert_eq!(decoded.kind, payload.kind);
    assert_eq!(decoded, payload);
}

#[test]
fn kind_payload_json_renders_as_flat_string() {
    // `MidiDeviceKindPayload` is a unit-variant enum; serde renders it
    // as a bare JSON string. The frontend type-import treats it as
    // a string union, so the wire shape must NOT carry a tag object.
    let json = serde_json::to_string(&MidiDeviceKindPayload::Input).expect("serialize kind");
    assert_eq!(json, "\"Input\"");
    let decoded: MidiDeviceKindPayload = serde_json::from_str(&json).expect("deserialize kind");
    assert_eq!(decoded, MidiDeviceKindPayload::Input);
}

#[test]
fn ts_rs_exported_names_match() {
    // The committed binding lives at
    // `src/lib/ipc/bindings/MidiDevicePayload.ts` +
    // `src/lib/ipc/bindings/MidiDeviceKindPayload.ts`. ts-rs's
    // `name()` is the source of truth for the export filename;
    // an accidental rename of the Rust newtype (or a
    // `#[ts(rename = ...)]` attribute) would surface here before the
    // frontend index drifts.
    assert_eq!(<MidiDevicePayload as TS>::name(), "MidiDevicePayload");
    assert_eq!(
        <MidiDeviceKindPayload as TS>::name(),
        "MidiDeviceKindPayload"
    );
}

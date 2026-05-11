// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-INTEGRATION-MATRIX (T2) — engine→UI MIDI learn-mode
//! event payload round-trip conformance (MIDI arm, Wave 2.1 Lane D.2).
//!
//! T2 per docs/43 §15.5: this corpus drives the
//! [`chromaticity_ui_bridge::MidiLearnEventPayload`] +
//! [`chromaticity_ui_bridge::MidiLearnKindPayload`] newtypes across the
//! engine→payload `from_message` conversion + serde JSON wire shape.
//! Field-by-field equality across the 6-field surface (`port_id`,
//! `raw_bytes`, `kind`, `channel`, `note_or_cc`, `velocity_or_value`)
//! is the load-bearing assertion: any future payload-shape divergence
//! between the engine `MidiMessage` taxonomy and the bridge newtype
//! surfaces here before the committed ts-rs binding under
//! `src/lib/ipc/bindings/MidiLearnEventPayload.ts` drifts.
//!
//! Lives under `app/ui-bridge/tests/` (outside the
//! `engine/**`/`integrations/**` evidence-tag-lint coverage scope per
//! `scripts/evidence-tag-lint.sh`); the in-source T1 drift pin under
//! `integrations/midi-adapter/src/adapter.rs::drift_pins` carries the
//! registry-anchored `Evidence:` tag the lint enforces.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_midi_adapter::MidiMessage;
use chromaticity_ui_bridge::{MidiLearnEventPayload, MidiLearnKindPayload};
use ts_rs::TS;

#[test]
fn note_on_message_decomposes_to_payload_fields() {
    let payload = MidiLearnEventPayload::from_message(
        "input-0".to_string(),
        &MidiMessage::NoteOn {
            channel: 3,
            note: 60,
            velocity: 100,
        },
    );
    assert_eq!(payload.port_id, "input-0");
    assert_eq!(payload.kind, MidiLearnKindPayload::NoteOn);
    assert_eq!(payload.channel, Some(3));
    assert_eq!(payload.note_or_cc, Some(60));
    assert_eq!(payload.velocity_or_value, Some(100));
    assert_eq!(payload.raw_bytes, vec![0x93, 60, 100]);
}

#[test]
fn control_change_message_decomposes_to_payload_fields() {
    let payload = MidiLearnEventPayload::from_message(
        "input-1".to_string(),
        &MidiMessage::ControlChange {
            channel: 0,
            controller: 7,
            value: 42,
        },
    );
    assert_eq!(payload.kind, MidiLearnKindPayload::ControlChange);
    assert_eq!(payload.channel, Some(0));
    assert_eq!(payload.note_or_cc, Some(7));
    assert_eq!(payload.velocity_or_value, Some(42));
    assert_eq!(payload.raw_bytes, vec![0xB0, 7, 42]);
}

#[test]
fn pitch_bend_message_decomposes_to_lsb_msb_payload_fields() {
    // Center pitch bend is value14 = 8192 (0x2000): LSB = 0x00, MSB = 0x40.
    let payload = MidiLearnEventPayload::from_message(
        "input-2".to_string(),
        &MidiMessage::PitchBend {
            channel: 5,
            value14: 8192,
        },
    );
    assert_eq!(payload.kind, MidiLearnKindPayload::PitchBend);
    assert_eq!(payload.channel, Some(5));
    assert_eq!(payload.note_or_cc, Some(0x00));
    assert_eq!(payload.velocity_or_value, Some(0x40));
    assert_eq!(payload.raw_bytes, vec![0xE5, 0x00, 0x40]);
}

#[test]
fn cc14_bit_message_classifies_as_control_change() {
    // 14-bit CC pair-assembled forms are coarse-classified as
    // ControlChange for the operator-facing learn UI; the high 7
    // bits (the pre-pairing MSB byte) flow as `velocity_or_value`.
    let payload = MidiLearnEventPayload::from_message(
        "input-3".to_string(),
        &MidiMessage::Cc14Bit {
            channel: 2,
            controller_msb: 7,
            value14: 0x3FFF, // 16383
        },
    );
    assert_eq!(payload.kind, MidiLearnKindPayload::ControlChange);
    assert_eq!(payload.channel, Some(2));
    assert_eq!(payload.note_or_cc, Some(7));
    assert_eq!(payload.velocity_or_value, Some(0x7F));
}

#[test]
fn system_real_time_messages_classify_as_other() {
    let payload =
        MidiLearnEventPayload::from_message("input-4".to_string(), &MidiMessage::MidiClock);
    assert_eq!(payload.kind, MidiLearnKindPayload::Other);
    assert_eq!(payload.channel, None);
    assert_eq!(payload.note_or_cc, None);
    assert_eq!(payload.velocity_or_value, None);
    assert_eq!(payload.raw_bytes, vec![0xF8]);
}

#[test]
fn payload_json_round_trip_field_by_field_equality() {
    // Pin serde JSON round-trip identity so the wire shape stays
    // stable across the engine→Tauri→Svelte path. A future
    // serde-tagged attribute or rename would surface here.
    let payload = MidiLearnEventPayload {
        port_id: "input-0".to_string(),
        raw_bytes: vec![0x90, 60, 100],
        kind: MidiLearnKindPayload::NoteOn,
        channel: Some(0),
        note_or_cc: Some(60),
        velocity_or_value: Some(100),
    };
    let json = serde_json::to_string(&payload).expect("serialize payload");
    let decoded: MidiLearnEventPayload = serde_json::from_str(&json).expect("deserialize payload");
    assert_eq!(decoded, payload);
}

#[test]
fn kind_payload_json_renders_as_flat_string() {
    let json = serde_json::to_string(&MidiLearnKindPayload::ControlChange).expect("serialize");
    assert_eq!(json, "\"ControlChange\"");
    let decoded: MidiLearnKindPayload = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, MidiLearnKindPayload::ControlChange);
}

#[test]
fn ts_rs_exported_names_match() {
    // The committed binding lives at
    // `src/lib/ipc/bindings/MidiLearnEventPayload.ts` +
    // `src/lib/ipc/bindings/MidiLearnKindPayload.ts`. ts-rs's
    // `name()` is the source of truth for the export filename.
    assert_eq!(
        <MidiLearnEventPayload as TS>::name(),
        "MidiLearnEventPayload"
    );
    assert_eq!(<MidiLearnKindPayload as TS>::name(), "MidiLearnKindPayload");
}

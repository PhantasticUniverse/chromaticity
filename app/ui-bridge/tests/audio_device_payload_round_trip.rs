// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-INTEGRATION-MATRIX (T2) — engine→UI audio device-listing
//! IPC payload round-trip conformance (audio arm, Wave 2.6 Lane AUD.A).
//!
//! T2 per docs/43 §15.5: this corpus drives the
//! [`chromaticity_ui_bridge::AudioDevicePayload`] +
//! [`chromaticity_ui_bridge::AudioDeviceKindPayload`] newtypes across
//! the bidirectional `From<chromaticity_audio::AudioDeviceInfo>`
//! conversion + serde JSON wire shape. Field-by-field equality across
//! the 3-field surface (one `id`, one `name`, one `kind`) is the
//! load-bearing assertion: any future payload-shape divergence between
//! the engine descriptor and the bridge newtype surfaces here before
//! the committed ts-rs binding under
//! `src/lib/ipc/bindings/AudioDevicePayload.ts` drifts.
//!
//! Lives under `app/ui-bridge/tests/` (outside the
//! `engine/**`/`integrations/**` evidence-tag-lint coverage scope per
//! `scripts/evidence-tag-lint.sh`); the in-source T1 drift pin under
//! `engine/audio/src/cpal_source.rs` carries the registry-anchored
//! `Evidence:` tag the lint enforces.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_audio::AudioDeviceInfo;
use chromaticity_ui_bridge::{AudioDeviceKindPayload, AudioDevicePayload};
use ts_rs::TS;

fn sample_info() -> AudioDeviceInfo {
    AudioDeviceInfo {
        id: "input-built-in-microphone".to_string(),
        name: "Built-in Microphone".to_string(),
    }
}

fn sample_payload() -> AudioDevicePayload {
    AudioDevicePayload {
        id: "input-built-in-microphone".to_string(),
        name: "Built-in Microphone".to_string(),
        kind: AudioDeviceKindPayload::Input,
    }
}

#[test]
fn engine_to_payload_field_by_field_equality() {
    // Producer-side conversion: the engine descriptor flows through
    // the `From<AudioDeviceInfo>` impl. Field-by-field assertion
    // pins the 3-field surface so a future field-mapping bug (e.g. a
    // hand-rolled `From` impl swapping `id` and `name`) surfaces here.
    let info = sample_info();
    let payload: AudioDevicePayload = info.clone().into();

    assert_eq!(payload.id, info.id);
    assert_eq!(payload.name, info.name);
    assert_eq!(payload.kind, AudioDeviceKindPayload::Input);
}

#[test]
fn payload_to_engine_round_trip_field_by_field_equality() {
    // Reverse direction: a payload constructed on the consumer side
    // (e.g. by deserialising an event payload, or by a test harness)
    // converts back into the engine descriptor with field identity
    // preserved.
    let payload = sample_payload();
    let info: AudioDeviceInfo = payload.clone().into();

    assert_eq!(info.id, payload.id);
    assert_eq!(info.name, payload.name);
}

#[test]
fn payload_json_round_trip_field_by_field_equality() {
    // Pin serde JSON round-trip identity so the wire shape stays
    // stable across the engine→Tauri→Svelte path. A future
    // serde-tagged attribute or rename would surface here.
    let payload = sample_payload();
    let json = serde_json::to_string(&payload).expect("serialize payload");
    let decoded: AudioDevicePayload = serde_json::from_str(&json).expect("deserialize payload");

    assert_eq!(decoded.id, payload.id);
    assert_eq!(decoded.name, payload.name);
    assert_eq!(decoded.kind, payload.kind);
    assert_eq!(decoded, payload);
}

#[test]
fn kind_payload_json_renders_as_flat_string() {
    // `AudioDeviceKindPayload` is a unit-variant enum; serde renders
    // it as a bare JSON string. The frontend type-import treats it as
    // a string union, so the wire shape must NOT carry a tag object.
    let json_in = serde_json::to_string(&AudioDeviceKindPayload::Input).expect("serialize Input");
    assert_eq!(json_in, "\"Input\"");
    let json_out =
        serde_json::to_string(&AudioDeviceKindPayload::Output).expect("serialize Output");
    assert_eq!(json_out, "\"Output\"");
    let decoded_in: AudioDeviceKindPayload =
        serde_json::from_str(&json_in).expect("deserialize Input");
    assert_eq!(decoded_in, AudioDeviceKindPayload::Input);
    let decoded_out: AudioDeviceKindPayload =
        serde_json::from_str(&json_out).expect("deserialize Output");
    assert_eq!(decoded_out, AudioDeviceKindPayload::Output);
}

#[test]
fn output_variant_round_trips_to_engine_input() {
    // Documented narrowing: the bridge-side `Output` variant has no
    // engine-side peer yet; the safe round-trip on the way back to
    // the engine collapses it to `Input` so the typed-newtype seam
    // stays total. The follow-up output-enumeration wave adds the
    // engine-side `Output` variant and removes this collapse — at
    // which point this pin updates in lockstep.
    let payload = AudioDevicePayload {
        id: "output-built-in-speaker".to_string(),
        name: "Built-in Speaker".to_string(),
        kind: AudioDeviceKindPayload::Output,
    };
    let info: AudioDeviceInfo = payload.clone().into();
    assert_eq!(info.id, payload.id);
    assert_eq!(info.name, payload.name);
}

#[test]
fn ts_rs_exported_names_match() {
    // The committed binding lives at
    // `src/lib/ipc/bindings/AudioDevicePayload.ts` +
    // `src/lib/ipc/bindings/AudioDeviceKindPayload.ts`. ts-rs's
    // `name()` is the source of truth for the export filename;
    // an accidental rename of the Rust newtype (or a
    // `#[ts(rename = ...)]` attribute) would surface here before the
    // frontend index drifts.
    assert_eq!(<AudioDevicePayload as TS>::name(), "AudioDevicePayload");
    assert_eq!(
        <AudioDeviceKindPayload as TS>::name(),
        "AudioDeviceKindPayload"
    );
}

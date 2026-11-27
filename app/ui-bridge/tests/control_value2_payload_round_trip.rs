// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-INTEGRATION-MATRIX (T2) — 2-component (XY) control
//! value IPC payload round-trip conformance (parity-closure wave 2,
//! Lane D PR4).
//!
//! T2 per docs/43 §15.5: this corpus drives the
//! [`chromaticity_ui_bridge::ControlValue2`] newtype across the
//! bidirectional `From<[f32; 2]>` / `From<ControlValue2> for [f32; 2]`
//! conversions + serde JSON wire shape. Field-by-field equality across
//! the 2-element array surface is the load-bearing assertion: any future
//! payload-shape divergence between the engine 2-component value and the
//! bridge newtype surfaces here before the committed ts-rs binding under
//! `src/lib/ipc/bindings/ControlValue2.ts` drifts.
//!
//! Lives under `app/ui-bridge/tests/` (outside the
//! `engine/**`/`integrations/**` evidence-tag-lint coverage scope per
//! `scripts/evidence-tag-lint.sh`). The in-source T1 drift pins under
//! `engine/runtime/src/command_bus.rs` carry the `Command::ControlSet2D`
//! and `EventPayload::ControlChanged2D` invariants this test layer
//! builds on.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_ui_bridge::ControlValue2;
use ts_rs::TS;

#[test]
fn from_array_round_trip_preserves_values() {
    let arr: [f32; 2] = [0.4, -0.6];
    let payload: ControlValue2 = arr.into();
    assert_eq!(payload.0, arr);
    let back: [f32; 2] = payload.into();
    assert_eq!(back, arr);
}

#[test]
fn from_array_round_trip_boundary_values() {
    // Cover canonical XY ranges: (0, 0) centre, (1, 1) max, (-1, -1) min,
    // and a mixed-sign pair representative of real operator gestures.
    for arr in [[0.0_f32, 0.0], [1.0, 1.0], [-1.0, -1.0], [0.5, -0.5]] {
        let payload: ControlValue2 = arr.into();
        assert_eq!(payload.0, arr);
        let back: [f32; 2] = payload.into();
        assert_eq!(back, arr);
    }
}

#[test]
fn json_round_trip_uses_two_element_array() {
    let payload = ControlValue2([0.25, 0.75]);
    let json = serde_json::to_string(&payload).expect("serialize payload");
    // ts-rs renders [f32; 2] as a JSON array; verify the wire shape
    // so the frontend tuple type `[number, number]` parses correctly.
    assert_eq!(json, "[0.25,0.75]");
    let decoded: ControlValue2 = serde_json::from_str(&json).expect("deserialize payload");
    assert_eq!(decoded, payload);
}

#[test]
fn json_round_trip_centre_value() {
    let payload = ControlValue2([0.0, 0.0]);
    let json = serde_json::to_string(&payload).expect("serialize");
    let decoded: ControlValue2 = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, payload);
}

#[test]
fn json_round_trip_negative_components() {
    let payload = ControlValue2([-0.5, -1.0]);
    let json = serde_json::to_string(&payload).expect("serialize");
    let decoded: ControlValue2 = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, payload);
}

#[test]
fn ts_rs_exported_name_matches_committed_binding() {
    assert_eq!(<ControlValue2 as TS>::name(), "ControlValue2");
}

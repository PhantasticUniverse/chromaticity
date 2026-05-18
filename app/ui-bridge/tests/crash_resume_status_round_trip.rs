// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Crash-resume recovery status IPC payload round-trip conformance.
//!
//! Pins the shell setup payload surfaced by `get_crash_resume_status`
//! across the ui-bridge serde + ts-rs contract. The frontend consumes
//! the generated `CrashResumeStatusPayload` binding directly, so this
//! test keeps the camelCase field names and lowercase restoration
//! status values load-bearing.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_ui_bridge::{CrashResumeStatusPayload, ResumeStatusPayload};
use serde_json::json;
use ts_rs::TS;

fn sample_payload() -> CrashResumeStatusPayload {
    CrashResumeStatusPayload {
        show_name: "Recovered runtime state".to_string(),
        controls: ResumeStatusPayload::Restored,
        snaps: ResumeStatusPayload::Restored,
        playlist: ResumeStatusPayload::Restored,
        adapters: ResumeStatusPayload::Restored,
        scene: ResumeStatusPayload::Partial,
        p50: 37,
    }
}

#[test]
fn payload_json_round_trip_field_by_field_equality() {
    let payload = sample_payload();
    let json = serde_json::to_string(&payload).expect("serialize crash-resume status");
    let decoded: CrashResumeStatusPayload =
        serde_json::from_str(&json).expect("deserialize crash-resume status");

    assert_eq!(decoded, payload);
    assert_eq!(decoded.show_name, "Recovered runtime state");
    assert_eq!(decoded.controls, ResumeStatusPayload::Restored);
    assert_eq!(decoded.scene, ResumeStatusPayload::Partial);
    assert_eq!(decoded.p50, 37);
}

#[test]
fn serde_shape_uses_camel_case_fields_and_lowercase_status_values() {
    let value = serde_json::to_value(sample_payload()).expect("serialize to value");

    assert_eq!(value["showName"], json!("Recovered runtime state"));
    assert_eq!(value["controls"], json!("restored"));
    assert_eq!(value["snaps"], json!("restored"));
    assert_eq!(value["playlist"], json!("restored"));
    assert_eq!(value["adapters"], json!("restored"));
    assert_eq!(value["scene"], json!("partial"));
    assert_eq!(value["p50"], json!(37));
    assert!(value.get("show_name").is_none());
}

#[test]
fn resume_status_payload_json_renders_as_flat_lowercase_string() {
    let restored =
        serde_json::to_string(&ResumeStatusPayload::Restored).expect("serialize restored status");
    let partial =
        serde_json::to_string(&ResumeStatusPayload::Partial).expect("serialize partial status");
    let lost = serde_json::to_string(&ResumeStatusPayload::Lost).expect("serialize lost status");

    assert_eq!(restored, "\"restored\"");
    assert_eq!(partial, "\"partial\"");
    assert_eq!(lost, "\"lost\"");
}

#[test]
fn ts_rs_exported_names_match_committed_bindings() {
    assert_eq!(
        <CrashResumeStatusPayload as TS>::name(),
        "CrashResumeStatusPayload"
    );
    assert_eq!(<ResumeStatusPayload as TS>::name(), "ResumeStatusPayload");
}

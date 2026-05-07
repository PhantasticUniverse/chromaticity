// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4d Wave 4d.1 Lane A `EV-CRASH-PAYLOAD-REDACTION` T2 corpus.
//! Tier declaration and evidence tag live on the inner `fixtures`
//! module below, matching the pattern established by
//! `engine/observability/tests/no_network_egress/main.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary counts
//! this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-CRASH-PAYLOAD-REDACTION (T2) — fixture-driven
    //! conformance corpus for the
    //! [`chromaticity_observability::crash_redaction`] primitives.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! evidence-producing behaviour against a fixture corpus. The
    //! "fixture corpus" here is a set of synthetic crash-report-
    //! shaped [`serde_json::Value`] payloads modelled against
    //! ADR-016 §Crash reporting line 28-31 (the redaction +
    //! inclusion list) and the wireframe payload-shape candidates
    //! plausibly emitted by future crash-payload-construction code.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md`
    //! row 93 (`EV-CRASH-PAYLOAD-REDACTION`) carries Generation
    //! Source "Crash-injection + payload inspection" and Phase 4d
    //! exit gating. This T2 corpus is the **redaction-primitives
    //! conformance half** of that row's contract; the row stays
    //! at `Planned` until the future crash-payload-construction PR
    //! composes these primitives with an end-to-end injected-crash
    //! payload-inspection corpus + the closing-docs PR transitions
    //! the row.
    //!
    //! Defense-in-depth design.
    //!
    //! - **Per-category arm (fixtures 1-5).** One fixture per
    //!   [`RedactCategory`] arm exercising the field-name pattern
    //!   set against a synthetic crash payload.
    //! - **Cross-category interaction (fixture 6).** Single payload
    //!   with all five categories present; redactor configured for
    //!   all five; assert all five fields land on the redacted
    //!   placeholder.
    //! - **Conservative-by-default (fixture 7).** Unknown sensitive
    //!   field name (`super_secret_user_data`) is NOT redacted — a
    //!   future-proofing pin that documents the registry-extension
    //!   path when a new payload field shape lands.
    //! - **Diagnostic preservation (fixture 8).** ADR-016 §Crash
    //!   reporting line 30 inclusion list (`build_version`,
    //!   `gpu_vendor`, frame-time bucket, etc.) survives the
    //!   redactor verbatim so the transmitted payload still carries
    //!   diagnostic value.

    use chromaticity_observability::crash_redaction::{
        CrashPayloadRedactor, REDACTED_PLACEHOLDER, RedactCategory,
    };
    use serde_json::{Value, json};

    fn placeholder() -> Value {
        Value::String(REDACTED_PLACEHOLDER.to_string())
    }

    /// Synthetic crash-report payload modelled against ADR-016
    /// §Crash reporting. Carries (a) the four redaction categories
    /// the ADR names verbatim, plus (b) the diagnostic inclusion
    /// list line 30 says SHOULD survive transmission.
    fn synthetic_full_crash_payload() -> Value {
        json!({
            // ADR-016 §Crash reporting line 30 INCLUDED diagnostic
            // surface — must survive verbatim.
            "build_version": "0.0.0+phase-4d",
            "profile_class": "native-chroma-v1",
            "subsystem": "render",
            "stack_trace": [
                { "file": "engine/render/src/scene_adapter.rs", "line": 437 },
                { "file": "engine/runtime/src/scheduler/render.rs", "line": 92 },
            ],
            "gpu_vendor": "Apple",
            "gpu_driver": "Metal 3.0",
            "os_version": "macOS 25.3.0",
            "frame_time_p99_ms": 11.4,
            "error_kind": "panic",
            "frame_count": 17_452_u64,
            "timestamp": "2026-05-03T14:32:01Z",

            // ADR-016 §Crash reporting line 29 EXCLUDED — every
            // category arm.
            "main_glsl": "void main() { gl_FragColor = vec4(secret); }",
            "script_js": "function tick(t) { return SECRET_API_CALL(t); }",
            "media_bytes": [0xde, 0xad, 0xbe, 0xef, 0x42, 0x42],
            "audio_pcm": [0.1, 0.2, 0.3, 0.4],
            "scene_name": "Operator's secret midnight set",
            "preset_name": "venue-name-do-not-leak",
            "comment": "private operator note about cue order",
            "license_token": "secret-entitlement-token-bytes",
            "activation_signature": "cryptographic-signature-leak",
        })
    }

    #[test]
    fn fixture_1_glsl_arm_redacts_glsl_only() {
        let redactor = CrashPayloadRedactor::new().with_category(RedactCategory::Glsl);
        let payload = synthetic_full_crash_payload();
        let out = redactor.redact(payload.clone());

        assert_eq!(out["main_glsl"], placeholder());
        // Other category arms NOT enabled — those fields pass
        // through verbatim.
        assert_eq!(out["script_js"], payload["script_js"]);
        assert_eq!(out["media_bytes"], payload["media_bytes"]);
        assert_eq!(out["scene_name"], payload["scene_name"]);
        assert_eq!(out["license_token"], payload["license_token"]);
    }

    #[test]
    fn fixture_2_script_arm_redacts_script_only() {
        let redactor = CrashPayloadRedactor::new().with_category(RedactCategory::Script);
        let payload = synthetic_full_crash_payload();
        let out = redactor.redact(payload.clone());

        assert_eq!(out["script_js"], placeholder());
        assert_eq!(out["main_glsl"], payload["main_glsl"]);
        assert_eq!(out["media_bytes"], payload["media_bytes"]);
        assert_eq!(out["scene_name"], payload["scene_name"]);
        assert_eq!(out["license_token"], payload["license_token"]);
    }

    #[test]
    fn fixture_3_media_buffer_arm_redacts_media_only() {
        let redactor = CrashPayloadRedactor::new().with_category(RedactCategory::MediaBuffer);
        let payload = synthetic_full_crash_payload();
        let out = redactor.redact(payload.clone());

        assert_eq!(out["media_bytes"], placeholder());
        assert_eq!(out["audio_pcm"], placeholder());
        assert_eq!(out["main_glsl"], payload["main_glsl"]);
        assert_eq!(out["script_js"], payload["script_js"]);
        assert_eq!(out["scene_name"], payload["scene_name"]);
    }

    #[test]
    fn fixture_4_free_text_arm_redacts_free_text_only() {
        let redactor = CrashPayloadRedactor::new().with_category(RedactCategory::FreeText);
        let payload = synthetic_full_crash_payload();
        let out = redactor.redact(payload.clone());

        assert_eq!(out["scene_name"], placeholder());
        assert_eq!(out["preset_name"], placeholder());
        assert_eq!(out["comment"], placeholder());
        assert_eq!(out["main_glsl"], payload["main_glsl"]);
        assert_eq!(out["license_token"], payload["license_token"]);
    }

    #[test]
    fn fixture_5_license_token_arm_redacts_token_only() {
        let redactor = CrashPayloadRedactor::new().with_category(RedactCategory::LicenseToken);
        let payload = synthetic_full_crash_payload();
        let out = redactor.redact(payload.clone());

        assert_eq!(out["license_token"], placeholder());
        assert_eq!(out["activation_signature"], placeholder());
        assert_eq!(out["main_glsl"], payload["main_glsl"]);
        assert_eq!(out["scene_name"], payload["scene_name"]);
    }

    #[test]
    fn fixture_6_all_categories_redact_full_excluded_set() {
        // Cross-category interaction: configure the redactor for
        // every category and assert all of ADR-016's excluded
        // fields land on the placeholder while diagnostic fields
        // survive verbatim.
        let redactor = CrashPayloadRedactor::all();
        let out = redactor.redact(synthetic_full_crash_payload());

        // Every excluded field per ADR-016 §Crash reporting line
        // 29 + ADR-009 token-residency:
        assert_eq!(out["main_glsl"], placeholder());
        assert_eq!(out["script_js"], placeholder());
        assert_eq!(out["media_bytes"], placeholder());
        assert_eq!(out["audio_pcm"], placeholder());
        assert_eq!(out["scene_name"], placeholder());
        assert_eq!(out["preset_name"], placeholder());
        assert_eq!(out["comment"], placeholder());
        assert_eq!(out["license_token"], placeholder());
        assert_eq!(out["activation_signature"], placeholder());

        // Every diagnostic field per ADR-016 §Crash reporting line
        // 30 — these must survive transmission for the crash
        // report to retain triage value:
        assert_eq!(out["build_version"], json!("0.0.0+phase-4d"));
        assert_eq!(out["profile_class"], json!("native-chroma-v1"));
        assert_eq!(out["subsystem"], json!("render"));
        assert_eq!(out["gpu_vendor"], json!("Apple"));
        assert_eq!(out["gpu_driver"], json!("Metal 3.0"));
        assert_eq!(out["os_version"], json!("macOS 25.3.0"));
        assert_eq!(out["frame_time_p99_ms"], json!(11.4));
        assert_eq!(out["error_kind"], json!("panic"));
        assert_eq!(out["frame_count"], json!(17_452_u64));
        assert_eq!(out["timestamp"], json!("2026-05-03T14:32:01Z"));
        // Stack-trace structure (file + line only per line 30) is
        // NOT redacted — this is the explicit "no surrounding
        // source" inclusion shape.
        assert_eq!(
            out["stack_trace"][0]["file"],
            json!("engine/render/src/scene_adapter.rs")
        );
        assert_eq!(out["stack_trace"][0]["line"], json!(437));
    }

    #[test]
    fn fixture_7_unknown_field_name_is_not_redacted_by_default() {
        // Conservative-by-default posture: a plausibly-sensitive
        // field name that no registered pattern names is NOT
        // redacted. The future crash-payload-construction PR is
        // expected to extend the pattern registry as concrete
        // payload shapes land. The intent: surfacing this as a
        // visible regression at construction-PR review time is
        // safer than silently allowlisting everything-not-known.
        let redactor = CrashPayloadRedactor::all();
        let payload = json!({
            "super_secret_user_data": "this is sensitive but not registered",
            "scene_name": "this IS registered",
        });
        let out = redactor.redact(payload);
        assert_eq!(
            out["super_secret_user_data"],
            json!("this is sensitive but not registered")
        );
        // Sanity-pin the registered arm still fires.
        assert_eq!(out["scene_name"], placeholder());
    }

    // -----------------------------------------------------------------
    // Wave 4d.1 PR 2 — synthetic crash-payload validation harness
    // -----------------------------------------------------------------
    //
    // The fixtures below exercise the
    // [`chromaticity_observability::crash_harness`] surface the row 93
    // PR 2 closing-docs section transitions to Pass-narrowed. Each
    // fixture constructs a synthetic payload for a simulated crash
    // class, runs it through `CrashPayloadRedactor::all`, and asserts
    // no sentinel from the sensitive-field set survives in the
    // serialised redacted output.

    use chromaticity_observability::crash_harness::{
        CrashPayloadValidationReport, SimulatedCrashClass, SyntheticCrashPayload,
        validate_synthetic_crash_payload,
    };

    fn assert_zero_leak(report: &CrashPayloadValidationReport, expected_sentinels: usize) {
        assert!(
            report.sentinels_leaked.is_empty(),
            "{:?} leaked sentinels: {:?}",
            report.crash_class,
            report.sentinels_leaked
        );
        assert_eq!(report.sentinels_redacted, expected_sentinels);
    }

    #[test]
    fn fixture_9_sigsegv_class_redacts_all_five_categories() {
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::Sigsegv);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
        assert_zero_leak(&report, 5);
        assert_eq!(report.crash_class, SimulatedCrashClass::Sigsegv);
        // The five known field-name keys land on the placeholder.
        assert_eq!(report.redacted_payload["main_glsl"], placeholder());
        assert_eq!(report.redacted_payload["script_js"], placeholder());
        assert_eq!(report.redacted_payload["media_buffer"], placeholder());
        assert_eq!(report.redacted_payload["comment"], placeholder());
        assert_eq!(report.redacted_payload["license_token"], placeholder());
    }

    #[test]
    fn fixture_10_panic_in_render_thread_class_redacts_all_five_categories() {
        let payload =
            SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::PanicInRenderThread);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
        assert_zero_leak(&report, 5);
        assert_eq!(report.crash_class, SimulatedCrashClass::PanicInRenderThread);
    }

    #[test]
    fn fixture_11_oom_class_redacts_all_five_categories() {
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::OutOfMemory);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
        assert_zero_leak(&report, 5);
        assert_eq!(report.crash_class, SimulatedCrashClass::OutOfMemory);
    }

    #[test]
    fn fixture_12_assertion_failure_class_redacts_all_five_categories() {
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::AssertionFailure);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
        assert_zero_leak(&report, 5);
        assert_eq!(report.crash_class, SimulatedCrashClass::AssertionFailure);
    }

    #[test]
    fn fixture_13_safe_telemetry_field_passes_through_post_redaction() {
        // ADR-016 §Crash reporting line 30 inclusion-list survives the
        // harness's redactor pass with every value byte-for-byte
        // preserved.
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::Sigsegv);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
        let telemetry = &report.redacted_payload["telemetry"];
        assert_eq!(telemetry["build_version"], json!("0.0.0+phase-4d"));
        assert_eq!(telemetry["profile_class"], json!("native-chroma-v1"));
        assert_eq!(telemetry["subsystem"], json!("render"));
        assert_eq!(telemetry["gpu_vendor"], json!("Apple"));
        assert_eq!(telemetry["gpu_driver"], json!("Metal 3.0"));
        assert_eq!(telemetry["os_version"], json!("macOS 25.3.0"));
        assert_eq!(telemetry["frame_time_p99_ms"], json!(11.4));
        assert_eq!(telemetry["frame_count"], json!(17_452_u64));
        assert_eq!(telemetry["timestamp"], json!("2026-05-07T12:00:00Z"));
        // Backtrace frames (file + line only) survive verbatim.
        let backtrace = report.redacted_payload["backtrace"]
            .as_array()
            .expect("backtrace array");
        assert_eq!(backtrace.len(), 3);
        assert_eq!(
            backtrace[0],
            json!("engine/render/src/scene_adapter.rs:437")
        );
    }

    #[test]
    fn fixture_14_redactor_default_categories_redact_all_synthetic_classes() {
        // Sweep every [`SimulatedCrashClass`] variant through the
        // default-all redactor. Pin: NO sentinel survives across the
        // full crash-class surface.
        for class in SimulatedCrashClass::all() {
            let payload = SyntheticCrashPayload::synthetic_for(*class);
            let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
            assert_zero_leak(&report, 5);
            assert_eq!(report.crash_class, *class);
        }
    }

    #[test]
    fn fixture_15_multi_class_validation_report_serializes_round_trip_via_json() {
        // The redacted_payload is a `serde_json::Value` and must
        // round-trip through `to_string` / `from_str` byte-for-byte.
        // This pins the harness's "redacted output is itself
        // serialisable" property — what a downstream Sentry
        // transport composing on top of the redactor would rely on.
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::OutOfMemory);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
        let serialised = serde_json::to_string(&report.redacted_payload).unwrap();
        let parsed: Value = serde_json::from_str(&serialised).unwrap();
        assert_eq!(parsed, report.redacted_payload);
        // None of the per-class sentinels must appear in the
        // serialised string form.
        assert!(!serialised.contains("oom-glsl-sentinel-do-not-leak"));
        assert!(!serialised.contains("oom-script-sentinel-do-not-leak"));
        assert!(!serialised.contains("oom-media-sentinel-do-not-leak"));
        assert!(!serialised.contains("oom-free-text-sentinel-do-not-leak"));
        assert!(!serialised.contains("oom-license-sentinel-do-not-leak"));
    }

    #[test]
    fn fixture_8_diagnostic_inclusion_list_round_trips_verbatim() {
        // Lift-only assertion: a payload containing ONLY ADR-016
        // §Crash reporting line 30 inclusion fields round-trips
        // through the redactor unchanged. Pins that the redactor
        // doesn't accidentally redact a diagnostic field via a
        // false-positive substring match.
        let redactor = CrashPayloadRedactor::all();
        let diagnostic_only = json!({
            "build_version": "0.0.0+phase-4d",
            "profile_class": "native-chroma-v1",
            "subsystem": "runtime",
            "gpu_vendor": "Apple",
            "gpu_driver": "Metal 3.0",
            "os_version": "macOS 25.3.0",
            "frame_time_p99_ms": 11.4,
            "frame_time_p50_ms": 4.2,
            "error_kind": "panic",
            "frame_count": 17_452_u64,
            "timestamp": "2026-05-03T14:32:01Z",
            "stack_trace": [
                { "file": "engine/runtime/src/scheduler/mod.rs", "line": 142 },
            ],
        });
        let out = redactor.redact(diagnostic_only.clone());
        assert_eq!(out, diagnostic_only);
    }
}

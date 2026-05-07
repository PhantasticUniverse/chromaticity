// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Crash-payload construction harness validating
//! `EV-CRASH-PAYLOAD-REDACTION` per ADR-016 §Crash payloads.
//! Composes on top of [`super::crash_redaction`]'s
//! [`CrashPayloadRedactor`] + [`RedactCategory`] surface.
//!
//! This module ships the **synthetic** crash-payload construction
//! half of the row 93 contract. The construction emits crash-shaped
//! [`serde_json::Value`] payloads representing each of four simulated
//! crash classes ([`SimulatedCrashClass`]) — `SIGSEGV`,
//! panic-in-render-thread, OOM, and assertion-failure — with each of
//! the five [`RedactCategory`] arms populated with realistic-shape
//! sentinel string values. The harness then runs the payload through
//! [`CrashPayloadRedactor::redact`] and asserts no sentinel from the
//! sensitive-field set survives in the redacted output.
//!
//! ## Wave 4d.1 narrowing — synthetic-harness scope
//!
//! This is a **synthetic** harness, not a real crash-injection
//! harness. Real platform-specific crash-handler injection (POSIX
//! signal handlers feeding `SIGSEGV` minidumps, panic-hook wiring on
//! the render thread, OOM-killer integration) requires platform-
//! specific infrastructure that doesn't ship at MVP and isn't
//! reachable from a deterministic per-PR CI surface in any case. The
//! narrowing matches the same shape as `EV-SOAK-CHAOS` row 40
//! (synthetic chaos extension under Wave 4d.4 narrowing per
//! [`super::soak`] module prose) and `EV-PERF-BUDGETS` (Phase-3
//! Path B nightly-only criterion harness).
//!
//! The synthetic harness establishes the **redaction contract at the
//! engine layer** and a future-wave full-injection harness composes
//! on top of these primitives — the deferred-composition framing
//! identical to the [`super::crash_redaction`] module's PR 1 framing.
//!
//! ## Design
//!
//! [`SyntheticCrashPayload`] is the construction surface. Each
//! crash-class variant of [`SimulatedCrashClass`] is paired with a
//! `synthetic_for(class)` constructor that produces a fully-populated
//! payload — every redact category arm carries a class-distinct
//! sentinel string, every diagnostic field per ADR-016 §Crash
//! reporting line 30 carries a representative value. The
//! [`SyntheticCrashPayload::sentinel_strings`] accessor returns the
//! ground-truth set of sentinels the redactor must scrub from the
//! output.
//!
//! [`validate_synthetic_crash_payload`] is the integration point:
//! given a constructed payload + a configured redactor, it runs the
//! redaction transform, serialises the result to a string, scans for
//! every ground-truth sentinel, and produces a
//! [`CrashPayloadValidationReport`] cataloguing the
//! sentinels-redacted vs sentinels-leaked counts. A `0`-leak result
//! is the contract: under [`CrashPayloadRedactor::all`] no sentinel
//! survives.
//!
//! ## Sentinel-string detection rationale
//!
//! Substring-search over the serialised redacted output catches both
//! direct-field leaks (a sensitive value preserved verbatim) AND
//! transitive leaks (the same byte sequence reappearing elsewhere in
//! the payload — e.g. a stack-trace frame quoting a redacted source
//! line). The redactor's pattern set covers the field-name surface;
//! the harness's sentinel scan covers the value-residency surface.
//!
//! Sentinels are deliberately constructed as **distinctive,
//! per-class, per-category** strings (e.g.
//! `"sigsegv-glsl-sentinel-do-not-leak"`) rather than realistic-
//! shape decoys. The realistic shape (e.g. an actual GLSL source
//! string) is preserved in a separate field so the field's payload
//! shape is documented, but the sentinel-search target is a
//! known-unique byte sequence that cannot accidentally match a
//! diagnostic field's content.
//!
//! ## Registry binding
//!
//! `docs/26-evidence-artifact-registry.md` row 93
//! (`EV-CRASH-PAYLOAD-REDACTION`) carries Generation Source "Crash-
//! injection + payload inspection". This harness + the sibling T2
//! corpus at `engine/observability/tests/crash_payload_redaction_corpus.rs`
//! together establish the row's `Planned -> Pass` transition under
//! the synthetic-harness narrowing recorded in row 93's description
//! column.
//!
//! T1 evidence: EV-CRASH-PAYLOAD-REDACTION.

use crate::crash_redaction::CrashPayloadRedactor;
use serde_json::{Value, json};

/// Simulated crash classes the harness exercises. Per the Wave 4d.1
/// synthetic-harness narrowing, no platform-specific signal handler
/// or panic hook is invoked; the variants name the **shape** of the
/// crash report a real-world handler would produce, and the harness
/// constructs a payload of that shape so the redactor's contract is
/// validated against every shape in the ADR-016 surface.
///
/// `#[non_exhaustive]` so the future-wave full-injection harness can
/// add classes (e.g. GPU-reset, audio-thread-deadlock) without
/// breaking callers that match on this enum.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimulatedCrashClass {
    /// `SIGSEGV` — segmentation fault, typically from native code
    /// (render adapter, GPU driver shim). Carries a backtrace with
    /// signal-handler frames.
    Sigsegv,
    /// Rust `panic!` reaching the panic hook on the render thread.
    /// Carries a panic-message string + a Rust-style backtrace.
    PanicInRenderThread,
    /// Allocation failure / OOM. Carries an allocation-size hint +
    /// a backtrace from the allocation site.
    OutOfMemory,
    /// Debug-build assertion failure (`assert!`/`debug_assert!`).
    /// Carries the assertion-expression text + the source location.
    AssertionFailure,
}

impl SimulatedCrashClass {
    /// Stable slug used in sentinel strings and report telemetry.
    /// Lower-case, hyphen-separated; never re-spelled across
    /// releases (downstream payload-inspection corpora pin these
    /// exact bytes).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            SimulatedCrashClass::Sigsegv => "sigsegv",
            SimulatedCrashClass::PanicInRenderThread => "panic-in-render-thread",
            SimulatedCrashClass::OutOfMemory => "oom",
            SimulatedCrashClass::AssertionFailure => "assertion-failure",
        }
    }

    /// Every variant in declaration order. Used by the T2 corpus to
    /// iterate the full crash-class surface.
    #[must_use]
    pub const fn all() -> &'static [SimulatedCrashClass] {
        &[
            SimulatedCrashClass::Sigsegv,
            SimulatedCrashClass::PanicInRenderThread,
            SimulatedCrashClass::OutOfMemory,
            SimulatedCrashClass::AssertionFailure,
        ]
    }
}

/// A synthetic crash-shaped payload populated with sentinel values
/// across every [`crate::crash_redaction::RedactCategory`] arm.
///
/// Construct via [`SyntheticCrashPayload::synthetic_for`]; serialise
/// via [`SyntheticCrashPayload::into_json`]; recover the
/// ground-truth sentinel set via
/// [`SyntheticCrashPayload::sentinel_strings`].
#[derive(Debug, Clone)]
pub struct SyntheticCrashPayload {
    /// The simulated crash class this payload represents.
    pub crash_class: SimulatedCrashClass,
    /// Thread name the simulated crash originated on (e.g.
    /// `"render-thread"`, `"audio-runtime"`). Diagnostic, not
    /// redacted.
    pub thread_name: String,
    /// Synthetic backtrace. File-and-line frames per ADR-016 §Crash
    /// reporting line 30; no surrounding source. Diagnostic, not
    /// redacted.
    pub backtrace: Vec<String>,
    /// Scene fragment-shader source — [`crate::crash_redaction::
    /// RedactCategory::Glsl`]. Carries a sentinel value the harness
    /// asserts is scrubbed.
    pub scene_glsl_source: Option<String>,
    /// Scene scripting source — [`crate::crash_redaction::
    /// RedactCategory::Script`].
    pub script_js_source: Option<String>,
    /// Media payload bytes — [`crate::crash_redaction::
    /// RedactCategory::MediaBuffer`]. Encoded as a sentinel-tagged
    /// string when serialised so the substring-search harness can
    /// detect a leak; the underlying type would be a `Vec<u8>` /
    /// base64 in production.
    pub media_buffer_data: Option<Vec<u8>>,
    /// Operator-entered free-text — [`crate::crash_redaction::
    /// RedactCategory::FreeText`].
    pub free_text_log: Option<String>,
    /// Activation-token bytes — [`crate::crash_redaction::
    /// RedactCategory::LicenseToken`].
    pub license_token: Option<String>,
    /// Sample of allowed-through diagnostic fields per ADR-016
    /// §Crash reporting line 30. Round-trips through the redactor
    /// verbatim.
    pub safe_telemetry: Value,
}

impl SyntheticCrashPayload {
    /// Construct a fully-populated synthetic crash payload for
    /// `class`. Every [`crate::crash_redaction::RedactCategory`] arm
    /// is populated with a class-distinct sentinel string; every
    /// diagnostic field per ADR-016 §Crash reporting line 30 carries
    /// a representative value.
    ///
    /// Sentinel format: `"<class-label>-<category>-sentinel-do-not-
    /// leak"`. The harness searches for every produced sentinel in
    /// the redactor's serialised output; the
    /// `do-not-leak` suffix makes accidental matches against
    /// diagnostic content vanishingly unlikely.
    #[must_use]
    pub fn synthetic_for(class: SimulatedCrashClass) -> Self {
        let label = class.label();
        Self {
            crash_class: class,
            thread_name: match class {
                SimulatedCrashClass::Sigsegv => "render-thread".to_string(),
                SimulatedCrashClass::PanicInRenderThread => "render-thread".to_string(),
                SimulatedCrashClass::OutOfMemory => "audio-runtime".to_string(),
                SimulatedCrashClass::AssertionFailure => "main".to_string(),
            },
            backtrace: vec![
                format!("engine/render/src/scene_adapter.rs:{}", 437),
                format!("engine/runtime/src/scheduler/render.rs:{}", 92),
                format!("engine/runtime/src/scheduler/mod.rs:{}", 142),
            ],
            scene_glsl_source: Some(format!(
                "void main() {{ /* {label}-glsl-sentinel-do-not-leak */ \
                 gl_FragColor = vec4(1.0); }}"
            )),
            script_js_source: Some(format!(
                "function tick(t) {{ /* {label}-script-sentinel-do-not-leak */ \
                 return t * 2; }}"
            )),
            media_buffer_data: Some(format!("{label}-media-sentinel-do-not-leak").into_bytes()),
            free_text_log: Some(format!(
                "operator note: {label}-free-text-sentinel-do-not-leak"
            )),
            license_token: Some(format!("{label}-license-sentinel-do-not-leak")),
            safe_telemetry: json!({
                "build_version": "0.0.0+phase-4d",
                "profile_class": "native-chroma-v1",
                "subsystem": "render",
                "gpu_vendor": "Apple",
                "gpu_driver": "Metal 3.0",
                "os_version": "macOS 25.3.0",
                "frame_time_p99_ms": 11.4,
                "error_kind": label,
                "frame_count": 17_452_u64,
                "timestamp": "2026-05-07T12:00:00Z",
            }),
        }
    }

    /// Serialise this payload to a JSON [`Value`] in the shape a
    /// real crash reporter would produce. Top-level keys mirror the
    /// pattern set in [`crate::crash_redaction::RedactCategory::patterns`]
    /// so the redactor's field-name match fires for every
    /// category arm.
    #[must_use]
    pub fn into_json(self) -> Value {
        // Encode the media buffer as a string so the substring
        // search can detect a sentinel leak. A real crash reporter
        // would emit base64; the substring-search invariant is
        // preserved either way.
        let media_string = self.media_buffer_data.map(|bytes| {
            String::from_utf8(bytes).unwrap_or_else(|err| {
                // The harness only emits UTF-8 bytes via
                // `synthetic_for`. The fallback path keeps clippy's
                // `unwrap_used` lint clean and surfaces an obvious
                // sentinel if the construction-side ever pivots to
                // a binary buffer.
                format!("media-buffer-non-utf8-len-{}", err.as_bytes().len())
            })
        });

        let mut map = serde_json::Map::new();
        map.insert(
            "crash_class".to_string(),
            Value::String(self.crash_class.label().to_string()),
        );
        map.insert("thread".to_string(), Value::String(self.thread_name));
        map.insert(
            "backtrace".to_string(),
            Value::Array(self.backtrace.into_iter().map(Value::String).collect()),
        );
        if let Some(glsl) = self.scene_glsl_source {
            map.insert("main_glsl".to_string(), Value::String(glsl));
        }
        if let Some(script) = self.script_js_source {
            map.insert("script_js".to_string(), Value::String(script));
        }
        if let Some(media) = media_string {
            map.insert("media_buffer".to_string(), Value::String(media));
        }
        if let Some(free_text) = self.free_text_log {
            map.insert("comment".to_string(), Value::String(free_text));
        }
        if let Some(token) = self.license_token {
            map.insert("license_token".to_string(), Value::String(token));
        }
        map.insert("telemetry".to_string(), self.safe_telemetry);
        Value::Object(map)
    }

    /// Returns every sentinel string this payload places in a
    /// sensitive-category field. The harness uses this as the
    /// ground-truth set: NONE of these byte sequences may survive
    /// in the redacted output.
    ///
    /// Order: declaration order on
    /// [`crate::crash_redaction::RedactCategory`] (Glsl, Script,
    /// MediaBuffer, FreeText, LicenseToken).
    #[must_use]
    pub fn sentinel_strings(&self) -> Vec<String> {
        let label = self.crash_class.label();
        let mut out = Vec::new();
        if self.scene_glsl_source.is_some() {
            out.push(format!("{label}-glsl-sentinel-do-not-leak"));
        }
        if self.script_js_source.is_some() {
            out.push(format!("{label}-script-sentinel-do-not-leak"));
        }
        if self.media_buffer_data.is_some() {
            out.push(format!("{label}-media-sentinel-do-not-leak"));
        }
        if self.free_text_log.is_some() {
            out.push(format!("{label}-free-text-sentinel-do-not-leak"));
        }
        if self.license_token.is_some() {
            out.push(format!("{label}-license-sentinel-do-not-leak"));
        }
        out
    }
}

/// Outcome report from running a [`SyntheticCrashPayload`] through
/// [`validate_synthetic_crash_payload`].
///
/// The contract: under [`CrashPayloadRedactor::all`] every payload
/// produces `sentinels_leaked.is_empty()`. A non-empty leak vector
/// names the exact byte sequences that survived redaction —
/// actionable diagnostic for the future-wave full-injection harness
/// extending the pattern registry.
#[derive(Debug, Clone)]
pub struct CrashPayloadValidationReport {
    /// The simulated crash class the validated payload represented.
    pub crash_class: SimulatedCrashClass,
    /// Redacted output, retained for downstream inspection (e.g.
    /// the T2 corpus's diagnostic-preservation assertions).
    pub redacted_payload: Value,
    /// Count of sentinels that were correctly scrubbed from the
    /// output. Equals
    /// `total_sentinels - sentinels_leaked.len()`.
    pub sentinels_redacted: usize,
    /// Sentinels (verbatim byte sequences) that survived in the
    /// serialised redacted output. `is_empty()` is the contract
    /// under [`CrashPayloadRedactor::all`].
    pub sentinels_leaked: Vec<String>,
}

/// Runs `payload` through `redactor` and produces a
/// [`CrashPayloadValidationReport`] cataloguing the
/// sentinels-redacted vs sentinels-leaked counts.
///
/// The serialised redacted output is scanned via substring search
/// for every sentinel returned from
/// [`SyntheticCrashPayload::sentinel_strings`]. Any survivor is
/// recorded in [`CrashPayloadValidationReport::sentinels_leaked`].
///
/// # Panics
///
/// Panics if `serde_json::to_string` fails on the redacted payload.
/// The redactor produces only [`serde_json::Value`] trees built from
/// stable JSON-compatible types, so this is not a reachable path in
/// practice; the panic preserves the harness's clippy posture
/// (`unwrap_used`/`expect_used` denied) by surfacing a serde-internal
/// invariant violation as a hard failure rather than silently
/// dropping diagnostic.
#[must_use]
pub fn validate_synthetic_crash_payload(
    payload: SyntheticCrashPayload,
    redactor: &CrashPayloadRedactor,
) -> CrashPayloadValidationReport {
    let crash_class = payload.crash_class;
    let sentinels = payload.sentinel_strings();
    let json = payload.into_json();
    let redacted = redactor.redact(json);
    let redacted_str = match serde_json::to_string(&redacted) {
        Ok(s) => s,
        Err(err) => panic!(
            "validate_synthetic_crash_payload: serde_json::to_string failed on a \
             Value tree composed of JSON-stable types — this should be unreachable; \
             err = {err}"
        ),
    };
    let leaked: Vec<String> = sentinels
        .iter()
        .filter(|s| redacted_str.contains(s.as_str()))
        .cloned()
        .collect();
    let total = sentinels.len();
    let leaked_count = leaked.len();
    CrashPayloadValidationReport {
        crash_class,
        redacted_payload: redacted,
        sentinels_redacted: total - leaked_count,
        sentinels_leaked: leaked,
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-CRASH-PAYLOAD-REDACTION (T1) — in-source drift
    //! pins for the synthetic crash-payload validation harness.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: each pin
    //! exercises a discrete behavioural property of the harness
    //! against a small in-source fixture. The T2 corpus at
    //! `engine/observability/tests/crash_payload_redaction_corpus.rs`
    //! exercises the harness across the full crash-class × redact-
    //! category surface.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md`
    //! row 93 (`EV-CRASH-PAYLOAD-REDACTION`). The harness's
    //! `Planned -> Pass-narrowed` transition lands in the row 93 PR 2
    //! closing-docs section.
    use super::{
        CrashPayloadRedactor, SimulatedCrashClass, SyntheticCrashPayload,
        validate_synthetic_crash_payload,
    };

    #[test]
    fn synthetic_payload_constructor_populates_all_five_redact_categories() {
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::Sigsegv);
        assert!(payload.scene_glsl_source.is_some());
        assert!(payload.script_js_source.is_some());
        assert!(payload.media_buffer_data.is_some());
        assert!(payload.free_text_log.is_some());
        assert!(payload.license_token.is_some());
    }

    #[test]
    fn into_json_round_trips_via_serde() {
        let payload =
            SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::PanicInRenderThread);
        let json = payload.into_json();
        let serialised = serde_json::to_string(&json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&serialised).unwrap();
        assert_eq!(json, parsed);
        // Top-level keys we promise.
        assert!(json.get("crash_class").is_some());
        assert!(json.get("thread").is_some());
        assert!(json.get("backtrace").is_some());
        assert!(json.get("main_glsl").is_some());
        assert!(json.get("script_js").is_some());
        assert!(json.get("media_buffer").is_some());
        assert!(json.get("comment").is_some());
        assert!(json.get("license_token").is_some());
        assert!(json.get("telemetry").is_some());
    }

    #[test]
    fn sentinel_strings_returns_unique_sentinel_per_redact_category() {
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::OutOfMemory);
        let sentinels = payload.sentinel_strings();
        assert_eq!(sentinels.len(), 5);
        // Each sentinel must be unique (substring search depends on
        // it).
        let unique: std::collections::HashSet<_> = sentinels.iter().cloned().collect();
        assert_eq!(unique.len(), 5);
        // Each carries the class label.
        for sentinel in &sentinels {
            assert!(sentinel.contains("oom"));
            assert!(sentinel.contains("sentinel-do-not-leak"));
        }
    }

    #[test]
    fn validate_redacts_all_sentinels_for_sigsegv_class() {
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::Sigsegv);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
        assert_eq!(report.crash_class, SimulatedCrashClass::Sigsegv);
        assert_eq!(report.sentinels_redacted, 5);
        assert!(
            report.sentinels_leaked.is_empty(),
            "leaked sentinels: {:?}",
            report.sentinels_leaked
        );
    }

    #[test]
    fn validate_redacts_all_sentinels_for_panic_class() {
        let payload =
            SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::PanicInRenderThread);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
        assert_eq!(report.crash_class, SimulatedCrashClass::PanicInRenderThread);
        assert_eq!(report.sentinels_redacted, 5);
        assert!(report.sentinels_leaked.is_empty());
    }

    #[test]
    fn validate_redacts_all_sentinels_for_oom_class() {
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::OutOfMemory);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
        assert_eq!(report.crash_class, SimulatedCrashClass::OutOfMemory);
        assert_eq!(report.sentinels_redacted, 5);
        assert!(report.sentinels_leaked.is_empty());
    }

    #[test]
    fn validate_redacts_all_sentinels_for_assertion_class() {
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::AssertionFailure);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
        assert_eq!(report.crash_class, SimulatedCrashClass::AssertionFailure);
        assert_eq!(report.sentinels_redacted, 5);
        assert!(report.sentinels_leaked.is_empty());
    }

    #[test]
    fn report_distinguishes_redacted_vs_leaked_counts() {
        // A no-op redactor (no categories enabled) produces 5 leaked
        // sentinels; this pin documents the report's accounting
        // contract by exercising the explicit-leak path.
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::Sigsegv);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::new());
        assert_eq!(report.sentinels_redacted, 0);
        assert_eq!(report.sentinels_leaked.len(), 5);
    }

    #[test]
    fn safe_telemetry_passes_through_unredacted() {
        let payload = SyntheticCrashPayload::synthetic_for(SimulatedCrashClass::Sigsegv);
        let report = validate_synthetic_crash_payload(payload, &CrashPayloadRedactor::all());
        // ADR-016 §Crash reporting line 30 inclusion list survives.
        let telemetry = &report.redacted_payload["telemetry"];
        assert_eq!(
            telemetry["build_version"],
            serde_json::json!("0.0.0+phase-4d")
        );
        assert_eq!(
            telemetry["profile_class"],
            serde_json::json!("native-chroma-v1")
        );
        assert_eq!(telemetry["gpu_vendor"], serde_json::json!("Apple"));
        assert_eq!(telemetry["frame_time_p99_ms"], serde_json::json!(11.4));
        assert_eq!(telemetry["error_kind"], serde_json::json!("sigsegv"));
    }

    #[test]
    fn simulated_crash_class_label_is_stable() {
        // Pin the exact slug bytes downstream payload-inspection
        // corpora rely on.
        assert_eq!(SimulatedCrashClass::Sigsegv.label(), "sigsegv");
        assert_eq!(
            SimulatedCrashClass::PanicInRenderThread.label(),
            "panic-in-render-thread"
        );
        assert_eq!(SimulatedCrashClass::OutOfMemory.label(), "oom");
        assert_eq!(
            SimulatedCrashClass::AssertionFailure.label(),
            "assertion-failure"
        );
    }
}

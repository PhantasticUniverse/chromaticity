// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Crash-report payload redaction primitives (Phase-4d Wave 4d.1
//! Lane A scope).
//!
//! ADR-016 §Crash reporting line 28-29 declares the redaction
//! contract for crash-report payloads transmitted to the self-hosted
//! Sentry instance: scene source code (`main.glsl`, `script.js`),
//! media payload content, recent audio buffers, operator-entered
//! free-text (preset names, playlist names) MUST be excluded before
//! any transmission. ADR-016 §Data sources line 60 extends the same
//! exclusion set to bytes returned from any `DATA_SOURCES` fetch
//! (carried here under the [`RedactCategory::MediaBuffer`] arm; the
//! data-source-specific arm lands when post-MVP data sources ship).
//!
//! ADR-009 §Activation tokens carries the entitlement-token surface;
//! crash payloads that quote token bytes are a token-residency leak.
//! The [`RedactCategory::LicenseToken`] arm pins this contract.
//!
//! ## Why primitives ship now
//!
//! There is no crash-report-construction code in the engine today.
//! The Phase-1 observability surface is in-process metric emission
//! only ([`crate::emit`] + [`crate::catalog`]); the panic-hook
//! wiring + minidump capture + Sentry transport land in a future
//! Phase-4d PR ("crash-payload construction") that this module's
//! primitives are designed to compose into. Shipping the redaction
//! contract early bakes the ADR-016 exclusion set into the type
//! surface ahead of the construction code, so the future PR composes
//! a known-good filter rather than re-deriving the exclusion set
//! from prose.
//!
//! ## Design: value-driven, not type-driven
//!
//! [`CrashPayloadRedactor::redact`] takes a [`serde_json::Value`]
//! and returns a [`serde_json::Value`] with matched keys replaced
//! by the string `"[REDACTED]"`. A type-driven `trait Redact`
//! alternative was considered and rejected: future crash-payload
//! shapes are unknown at this module's author time, and a trait-
//! based contract would couple every payload struct (or every new
//! payload field) to a `Redact` impl, which is exactly the coupling
//! the deferred-composition framing avoids. The [`serde_json::Value`]
//! pivot lets future construction code serialise its
//! [`serde::Serialize`]-derived payload, run it through this
//! redactor, and emit the redacted JSON without the redactor caring
//! about the payload's static type.
//!
//! ## Conservative-by-default posture
//!
//! Only field names that match a registered pattern are redacted.
//! An unknown field name with sensitive content (e.g. a future
//! `super_secret_user_data` key that no pattern lists) is preserved
//! verbatim. This is the deliberately conservative posture: rather
//! than redacting everything-except-allowlisted (which would lose
//! diagnostic information silently as the catalog evolves), the
//! registry MUST be extended when a new sensitive field shape lands.
//! The `redactor_unknown_field_name_is_not_redacted_by_default`
//! pin in `tests/crash_payload_redaction_corpus` documents this as
//! the intended-failure-mode-for-future-work signal.
//!
//! ## Recursion semantics
//!
//! Object values descend recursively (every key tested against the
//! configured pattern set; every value descended into). Array values
//! descend element-wise. Scalars (strings, numbers, booleans, null)
//! are passed through verbatim — patterns match field NAMES, not
//! field VALUES. This is intentional: a UUID embedded in a
//! `frame_id` field is not redacted; only fields whose KEY matches
//! a registered pattern lose their value to `"[REDACTED]"`.
//!
//! ## Pass-transition gating (registry row 93)
//!
//! `docs/26-evidence-artifact-registry.md` row 94 (in the registry,
//! the `EV-CRASH-PAYLOAD-REDACTION` row) carries Generation Source
//! "Crash-injection + payload inspection". This module is the
//! primitives half — necessary but not sufficient. The
//! `Planned -> Pass` transition lands in a future closing-docs PR
//! after the crash-payload-construction PR composes these primitives
//! with an end-to-end payload-inspection corpus actually exercising
//! the redaction against an injected crash. The crash-injection +
//! Sentry transport surface is out of scope here; the redaction
//! contract is in scope here.
//!
//! ## Governing documents
//!
//! - `docs/adr/ADR-016-observability-and-telemetry-posture.md`
//!   §Crash reporting (redaction contract).
//! - `docs/adr/ADR-009-activation-and-entitlements.md`
//!   (license-token residency baseline).
//! - `docs/26-evidence-artifact-registry.md` row 93
//!   (`EV-CRASH-PAYLOAD-REDACTION`, Phase-4d exit).
//! - `docs/31-supportability-privacy-ops-policy.md` §Telemetry
//!   redaction (the broader telemetry redaction posture this
//!   crash-specific contract narrows from).

use serde_json::{Map, Value};

/// Sentinel string substituted for any redacted field's value.
///
/// Stable across the public API; downstream payload-inspection
/// corpora pin this exact byte sequence.
pub const REDACTED_PLACEHOLDER: &str = "[REDACTED]";

/// Categories of payload field that ADR-016 §Crash reporting line
/// 28-29 (extended by §Data sources line 60 + ADR-009 token-
/// residency) requires excluded from a crash-report payload.
///
/// `#[non_exhaustive]` so adding a future category (e.g. when a
/// post-MVP data-source surface lands) is not a breaking change for
/// callers that match on this enum.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedactCategory {
    /// Scene fragment-shader source content (`main.glsl` text).
    Glsl,
    /// Scene scripting source content (`script.js` text).
    Script,
    /// Media payload bytes — image / audio / video buffers + recent
    /// audio analysis buffers. Per ADR-016 §Data sources line 60,
    /// this category also covers bytes returned from any
    /// `DATA_SOURCES` fetch (post-MVP).
    MediaBuffer,
    /// Operator-entered free-text — preset names, scene names,
    /// playlist names, label / title / description / comment fields.
    /// Anything a human typed into the UI.
    FreeText,
    /// Activation token bytes per ADR-009. Quoting token material
    /// in a transmitted payload is a residency leak.
    LicenseToken,
}

impl RedactCategory {
    /// Field-name substring patterns this category redacts. A
    /// field-name match is case-insensitive and substring-based:
    /// any object key whose lowercased form contains any pattern
    /// string is treated as belonging to this category.
    ///
    /// The patterns are curated against the wireframe payload
    /// shapes seen in `chromaticity-planning/ui-predev-bundle-v0.2-
    /// beta/views/_data.jsx` plus the engine-side type names that
    /// would plausibly appear in a serialised crash payload. The
    /// future crash-payload-construction PR is expected to extend
    /// this registry as concrete payload field names land — the
    /// `redactor_unknown_field_name_is_not_redacted_by_default`
    /// pin is the breadcrumb for that extension cycle.
    #[must_use]
    pub const fn patterns(self) -> &'static [&'static str] {
        match self {
            // GLSL fragment-shader source. Covers the `main.glsl`
            // scene-author surface plus the engine-side render-
            // adapter shader-source field name candidates.
            RedactCategory::Glsl => &[
                "main_glsl",
                "main.glsl",
                "fragment_shader_src",
                "fragment_shader_source",
                "shader_source",
                "shader_src",
                "glsl_source",
                "glsl_src",
            ],
            // JS scripting layer source. Covers the `script.js`
            // scene-author surface plus engine-side script-host
            // field name candidates.
            RedactCategory::Script => &[
                "script_js",
                "script.js",
                "script_source",
                "script_src",
                "user_script",
                "user_script_source",
            ],
            // Media payload buffers (image / audio / video) plus
            // recent audio-analysis buffers per ADR-016 §Crash
            // reporting line 29.
            RedactCategory::MediaBuffer => &[
                "media_bytes",
                "media_buffer",
                "image_buffer",
                "image_bytes",
                "video_frame",
                "video_bytes",
                "audio_pcm",
                "audio_buffer",
                "audio_samples",
                "pcm_buffer",
                "frame_buffer",
            ],
            // Operator-entered free-text. Anything a human typed
            // into the authoring or playlist surface.
            RedactCategory::FreeText => &[
                "scene_name",
                "scene_title",
                "preset_name",
                "preset_title",
                "playlist_name",
                "playlist_title",
                "comment",
                "description",
                "label",
                "title",
                "note",
                "notes",
            ],
            // Entitlement-token material per ADR-009.
            RedactCategory::LicenseToken => &[
                "license_token",
                "entitlement_token",
                "activation_signature",
                "activation_token",
                "token_bytes",
                "license_signature",
            ],
        }
    }
}

/// Redactor for crash-report payload [`serde_json::Value`] trees.
///
/// Construct via [`CrashPayloadRedactor::new`] (no categories
/// configured; redacts nothing) and opt into categories via
/// [`CrashPayloadRedactor::with_category`]. The
/// [`CrashPayloadRedactor::all`] convenience constructor opts in to
/// every category — the expected default for future crash-payload
/// construction code, which should redact every category ADR-016
/// names; per-category opt-in exists so test corpora can exercise
/// individual category arms in isolation.
#[derive(Debug, Clone)]
pub struct CrashPayloadRedactor {
    /// Per-category opt-in flags. A `true` cell means "this
    /// category's patterns are active for the current redactor".
    glsl: bool,
    script: bool,
    media_buffer: bool,
    free_text: bool,
    license_token: bool,
}

impl Default for CrashPayloadRedactor {
    fn default() -> Self {
        Self::new()
    }
}

impl CrashPayloadRedactor {
    /// Construct a redactor with no categories configured. Calling
    /// [`Self::redact`] on this state is a no-op (the input value
    /// is returned verbatim). Compose categories via
    /// [`Self::with_category`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            glsl: false,
            script: false,
            media_buffer: false,
            free_text: false,
            license_token: false,
        }
    }

    /// Construct a redactor with every [`RedactCategory`] enabled.
    /// This is the expected default for future crash-payload
    /// construction code (ADR-016 names every category as
    /// required).
    #[must_use]
    pub const fn all() -> Self {
        Self {
            glsl: true,
            script: true,
            media_buffer: true,
            free_text: true,
            license_token: true,
        }
    }

    /// Builder method that opts in to redacting fields matching
    /// `category`'s pattern set.
    #[must_use]
    pub const fn with_category(mut self, category: RedactCategory) -> Self {
        match category {
            RedactCategory::Glsl => self.glsl = true,
            RedactCategory::Script => self.script = true,
            RedactCategory::MediaBuffer => self.media_buffer = true,
            RedactCategory::FreeText => self.free_text = true,
            RedactCategory::LicenseToken => self.license_token = true,
        }
        self
    }

    /// Test-visible accessor for the currently-enabled category
    /// set. Returns each [`RedactCategory`] this redactor will act
    /// on. Order: declaration order on [`RedactCategory`].
    #[must_use]
    pub fn enabled_categories(&self) -> Vec<RedactCategory> {
        let mut out = Vec::new();
        if self.glsl {
            out.push(RedactCategory::Glsl);
        }
        if self.script {
            out.push(RedactCategory::Script);
        }
        if self.media_buffer {
            out.push(RedactCategory::MediaBuffer);
        }
        if self.free_text {
            out.push(RedactCategory::FreeText);
        }
        if self.license_token {
            out.push(RedactCategory::LicenseToken);
        }
        out
    }

    /// Redact `value` per the configured categories.
    ///
    /// Object keys are tested against every enabled category's
    /// pattern set; a matched key has its value replaced with the
    /// [`REDACTED_PLACEHOLDER`] string. Unmatched keys descend
    /// recursively. Arrays descend element-wise. Scalars pass
    /// through verbatim.
    #[must_use]
    pub fn redact(&self, value: Value) -> Value {
        match value {
            Value::Object(map) => Value::Object(self.redact_map(map)),
            Value::Array(items) => {
                Value::Array(items.into_iter().map(|item| self.redact(item)).collect())
            }
            // Scalars pass through; patterns match field NAMES,
            // not field VALUES.
            other => other,
        }
    }

    /// Inner recursion over a JSON object. Each key is tested for
    /// category match; matched keys map to the redacted-placeholder
    /// string, unmatched keys recurse into their value.
    fn redact_map(&self, map: Map<String, Value>) -> Map<String, Value> {
        let mut out = Map::with_capacity(map.len());
        for (key, value) in map {
            if self.key_matches_any_enabled_category(&key) {
                out.insert(key, Value::String(REDACTED_PLACEHOLDER.to_string()));
            } else {
                out.insert(key, self.redact(value));
            }
        }
        out
    }

    /// Returns `true` iff `key`'s lowercased form contains a
    /// substring listed in any enabled category's pattern set.
    fn key_matches_any_enabled_category(&self, key: &str) -> bool {
        let lowered = key.to_ascii_lowercase();
        for category in self.enabled_categories() {
            for pattern in category.patterns() {
                if lowered.contains(pattern) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-CRASH-PAYLOAD-REDACTION (T1) — in-source drift
    //! pins for the redaction primitives.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: exercises a
    //! discrete behavioural property of the unit under test against
    //! a small in-source fixture set. The T2 corpus fixture-driven
    //! cross-category round-trip lives at
    //! `engine/observability/tests/crash_payload_redaction_corpus.rs`.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md`
    //! row 93 (`EV-CRASH-PAYLOAD-REDACTION`) declares Generation
    //! Source "Crash-injection + payload inspection". This T1 +
    //! the sibling T2 corpus together pin the redaction-primitives
    //! contract; the row's `Planned -> Pass` transition lands when
    //! the future crash-payload-construction PR composes these
    //! primitives with an end-to-end injected-crash inspection
    //! corpus.

    use super::{CrashPayloadRedactor, REDACTED_PLACEHOLDER, RedactCategory};
    use serde_json::{Value, json};

    fn placeholder() -> Value {
        Value::String(REDACTED_PLACEHOLDER.to_string())
    }

    #[test]
    fn redactor_replaces_glsl_field_with_placeholder() {
        let redactor = CrashPayloadRedactor::all();
        let input = json!({
            "main_glsl": "void main() { gl_FragColor = vec4(1.0); }",
        });
        let out = redactor.redact(input);
        assert_eq!(out["main_glsl"], placeholder());
    }

    #[test]
    fn redactor_replaces_script_field_with_placeholder() {
        let redactor = CrashPayloadRedactor::all();
        let input = json!({
            "script_js": "function tick(t) { return t * 2; }",
        });
        let out = redactor.redact(input);
        assert_eq!(out["script_js"], placeholder());
    }

    #[test]
    fn redactor_replaces_media_buffer_field_with_placeholder() {
        let redactor = CrashPayloadRedactor::all();
        let input = json!({
            "media_bytes": [0, 1, 2, 3, 4, 5],
            "audio_pcm": [0.1, 0.2, 0.3],
        });
        let out = redactor.redact(input);
        assert_eq!(out["media_bytes"], placeholder());
        assert_eq!(out["audio_pcm"], placeholder());
    }

    #[test]
    fn redactor_replaces_free_text_fields_with_placeholder() {
        let redactor = CrashPayloadRedactor::all();
        let input = json!({
            "scene_name": "Operator's secret scene",
            "preset_name": "midnight-jam-3",
            "comment": "remember to fix the curve at bar 32",
        });
        let out = redactor.redact(input);
        assert_eq!(out["scene_name"], placeholder());
        assert_eq!(out["preset_name"], placeholder());
        assert_eq!(out["comment"], placeholder());
    }

    #[test]
    fn redactor_replaces_license_token_with_placeholder() {
        let redactor = CrashPayloadRedactor::all();
        let input = json!({
            "license_token": "secret-token-bytes-do-not-leak",
            "activation_signature": "another-residency-leak-vector",
        });
        let out = redactor.redact(input);
        assert_eq!(out["license_token"], placeholder());
        assert_eq!(out["activation_signature"], placeholder());
    }

    #[test]
    fn redactor_preserves_non_matching_fields_verbatim() {
        // Diagnostic fields that ADR-016 §Crash reporting line 30
        // explicitly INCLUDES in the transmitted payload — these
        // must pass through unchanged.
        let redactor = CrashPayloadRedactor::all();
        let input = json!({
            "frame_count": 17_452_u64,
            "error_kind": "panic",
            "timestamp": "2026-05-03T14:32:01Z",
            "build_version": "0.0.0+phase-4d",
            "gpu_vendor": "Apple",
            "frame_time_p99_ms": 11.4,
        });
        let out = redactor.redact(input.clone());
        // None of the diagnostic fields are touched.
        assert_eq!(out, input);
    }

    #[test]
    fn redactor_handles_nested_objects_recursively() {
        let redactor = CrashPayloadRedactor::all();
        let input = json!({
            "frame_count": 100,
            "scene": {
                "id": "abc-123",
                "scene_name": "leak-me-please",
                "passes": {
                    "fragment": {
                        "shader_source": "highly sensitive",
                    },
                },
            },
        });
        let out = redactor.redact(input);
        assert_eq!(out["frame_count"], json!(100));
        assert_eq!(out["scene"]["id"], json!("abc-123"));
        assert_eq!(out["scene"]["scene_name"], placeholder());
        assert_eq!(
            out["scene"]["passes"]["fragment"]["shader_source"],
            placeholder()
        );
    }

    #[test]
    fn redactor_handles_arrays_recursively() {
        let redactor = CrashPayloadRedactor::all();
        let input = json!({
            "presets": [
                { "preset_name": "alpha", "value": 1 },
                { "preset_name": "beta", "value": 2 },
                { "preset_name": "gamma", "value": 3 },
            ],
        });
        let out = redactor.redact(input);
        let presets = out["presets"].as_array().unwrap();
        assert_eq!(presets.len(), 3);
        for (idx, expected_value) in [1, 2, 3].iter().enumerate() {
            assert_eq!(presets[idx]["preset_name"], placeholder());
            assert_eq!(presets[idx]["value"], json!(*expected_value));
        }
    }

    #[test]
    fn redactor_with_only_glsl_category_leaves_other_categories_alone() {
        // Per-category opt-in: only Glsl is enabled, so a payload
        // carrying free-text + license-token + media-buffer field
        // names passes through verbatim except the GLSL field.
        let redactor = CrashPayloadRedactor::new().with_category(RedactCategory::Glsl);
        let input = json!({
            "main_glsl": "void main() {}",
            "scene_name": "not redacted in this config",
            "license_token": "also not redacted",
            "media_bytes": [1, 2, 3],
        });
        let out = redactor.redact(input);
        assert_eq!(out["main_glsl"], placeholder());
        assert_eq!(out["scene_name"], json!("not redacted in this config"));
        assert_eq!(out["license_token"], json!("also not redacted"));
        assert_eq!(out["media_bytes"], json!([1, 2, 3]));
    }

    #[test]
    fn redactor_default_is_no_op() {
        // `CrashPayloadRedactor::new()` (no categories enabled) is
        // a pass-through — the deferred-composition framing means
        // the future crash-payload-construction PR explicitly opts
        // in to categories rather than getting them by accident.
        let redactor = CrashPayloadRedactor::new();
        let input = json!({
            "main_glsl": "leaked",
            "scene_name": "leaked",
        });
        let out = redactor.redact(input.clone());
        assert_eq!(out, input);
    }

    #[test]
    fn enabled_categories_reports_in_declaration_order() {
        let redactor = CrashPayloadRedactor::all();
        assert_eq!(
            redactor.enabled_categories(),
            vec![
                RedactCategory::Glsl,
                RedactCategory::Script,
                RedactCategory::MediaBuffer,
                RedactCategory::FreeText,
                RedactCategory::LicenseToken,
            ]
        );
    }

    #[test]
    fn redactor_is_case_insensitive_on_field_names() {
        // ADR-016 doesn't constrain payload-field naming case.
        // Substring match runs against the lowercased key, so an
        // upper- or mixed-case rendering of a registered pattern
        // still matches. Patterns themselves are snake_case (the
        // engine-side convention); the case-insensitivity contract
        // applies to the KEY casing, not to alternative naming
        // conventions. A future construction PR that emits
        // camelCase keys (`sceneName`) would need a registry
        // extension — the conservative-by-default posture.
        let redactor = CrashPayloadRedactor::all();
        let input = json!({
            "Main_GLSL": "uppercased GLSL key",
            "SCENE_NAME": "screaming snake-case key",
        });
        let out = redactor.redact(input);
        assert_eq!(out["Main_GLSL"], placeholder());
        assert_eq!(out["SCENE_NAME"], placeholder());
    }
}

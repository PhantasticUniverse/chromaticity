// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Scene-schema lint warnings.
//!
//! Owns the compat-warning taxonomy defined in
//! `docs/05-scene-system.md` §5 (Validation and Tooling Requirements)
//! lines 79–83: `legacy-key`, `unknown-control-type`,
//! `deprecated-script-api`, `coercion-applied`. The enum lives here —
//! not in `crate::validator` — so the slice-3 lint extensions have a
//! single lexical home for the warning-class taxonomy. The validator
//! imports [`Warning`] and emits variants in compat mode; lint-only
//! producers (this module's [`lint_scene`] harness) emit the same
//! variants outside the validate path.
//!
//! Slice-3 scope: the enum declaration, variant-construction smoke
//! coverage, and the [`lint_scene`] classification harness. The
//! harness walks an unnormalized `serde_json::Value` and emits the
//! four [`Warning`] classes pre-deserialization — the lint vantage
//! point is distinct from the validator's, which walks the typed
//! [`crate::schema::Scene`] post-deserialization. Lint detects classes
//! that disappear after deserialize (legacy-key rewrites absorbed by
//! serde aliases / `#[serde(flatten)]`, deprecated-script-api access
//! in raw script source) where the validator cannot.
//!
//! # Detection passes
//!
//! - `LegacyKey` — top-level keys with a documented canonical pair.
//!   The slice-3 map is intentionally minimal: only `IMAGE` (canonical
//!   `IMAGE_PATH`, per `docs/02-synesthesia-contracts.md` §Observed
//!   root keys lines 20-22). `CATEGORIES` and the rare control-level
//!   `LABEL` are listed as legacy in `docs/02` but lack a documented
//!   canonical replacement so they parse-reserve through
//!   `Scene::extensions` / `ControlDecl::label` without a
//!   `legacy-key` warning. The map is extensible — future CR-03
//!   work-package PRs adding a documented canonical pair for these
//!   or other observed legacy keys widen the table without an enum
//!   change.
//!
//! - `UnknownControlType` — walks `CONTROLS[].TYPE` and emits when
//!   the first whitespace-split token is not in the
//!   [`crate::schema::ControlKind`] taxonomy
//!   (`slider`/`toggle`/`color`/`xy`/`bang`/`dropdown`/`knob`).
//!   Distinct from `CoercionApplied { from: "missing-type" }`: here
//!   `TYPE` was spelled but unrecognized.
//!
//! - `DeprecatedScriptApi` — scans top-level `SCRIPT` / `script` /
//!   `MAIN_SCRIPT` source strings for `inputs.<name>` and
//!   `uniforms.<name>` access patterns per `docs/28` §CP-07 + CP-01
//!   CR-04. The current scene corpus does not embed inline script
//!   content (scripts arrive via the `script.js` package entry that
//!   is a later-slice loader concern), so the harness fires zero
//!   warnings on the slice-3 fixture set; the lint-shape coverage is
//!   what closes `EV-SCHEMA-LINT` T2. Future scene fixtures carrying
//!   raw script in `scene.json` will exercise this branch without a
//!   harness change.
//!
//! - `CoercionApplied` — walks `CONTROLS[]` and predicts the CP-01
//!   missing-`TYPE` fallback per `docs/28` §CP-01 CR-01 (steps 1-4
//!   match emit `coercion-applied`; step 5 emits no warning — the
//!   strict-reject is the validator's responsibility). Parallels the
//!   validator's emission from a different vantage point (raw JSON
//!   vs deserialized `Scene`); both emissions key off the same
//!   warning vocabulary.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Compatibility warning emitted by the scene validator / linter.
///
/// Variants map 1:1 to the classes named in
/// `docs/05-scene-system.md` §5: `LegacyKey` = `legacy-key`,
/// `UnknownControlType` = `unknown-control-type`,
/// `DeprecatedScriptApi` = `deprecated-script-api`,
/// `CoercionApplied` = `coercion-applied`.
///
/// Fields carry only the minimum diagnostic context slice-2 needs.
/// Slice-3 may widen payloads (e.g. source-span information, file
/// / key pointers per `docs/05` §5 "human-readable diagnostics with
/// file and key context") without a compat bump — the enum
/// currently has no external reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Warning {
    /// `legacy-key`: a legacy key was accepted and rewritten to its
    /// canonical spelling per CP-01 step 2.
    LegacyKey {
        /// Canonical key the legacy key was rewritten to.
        canonical: String,
        /// Legacy key as it appeared in the input.
        legacy: String,
    },

    /// `unknown-control-type`: a control declared a `TYPE` that is
    /// not part of the canonical taxonomy and could not be coerced.
    /// Distinct from `CoercionApplied { to: "…" }`: here the
    /// original type was spelled but unrecognized.
    UnknownControlType {
        /// Author-declared control name (`ControlDecl::name`).
        control_name: String,
    },

    /// `deprecated-script-api`: a scene script invoked an API that
    /// ships a compatibility shim under `compat-syn-v1` but is
    /// rejected under `native-chroma-v1` per CP-01 CR-04.
    DeprecatedScriptApi {
        /// Fully qualified API path as authored (e.g. `inputs.foo`).
        api: String,
    },

    /// `coercion-applied`: the CP-01 missing-TYPE fallback selected
    /// a control kind for a control that did not declare one. In
    /// compat mode the coerced kind is applied silently (warning
    /// logged, not shown mid-performance per `docs/28` CP-01).
    CoercionApplied {
        /// Author-declared control name (`ControlDecl::name`).
        control_name: String,
        /// Source spelling: `"missing-type"` for the CP-01 CR-01
        /// fallback; future coercion classes (`"inverted-range"`
        /// for CR-02, `"legacy-key"` collision) reuse this field.
        from: String,
        /// Target spelling: `"slider"`, `"toggle"`, `"color"`, or
        /// `"xy"` for the CP-01 CR-01 fallback.
        to: String,
    },
}

/// Canonical control-kind taxonomy as kebab-lowercase first-token
/// strings. Mirrors [`crate::schema::ControlKind`]; the lint harness
/// keeps a private copy because it walks raw JSON and can't import
/// the typed enum without coupling lint to the deserializer's
/// custom impl.
const KNOWN_CONTROL_KINDS: &[&str] = &[
    "bang", "color", "dropdown", "knob", "slider", "toggle", "xy",
];

/// Documented legacy → canonical key map for the top-level
/// `scene.json` namespace. Slice-3 ships only the documented pair
/// from `docs/02-synesthesia-contracts.md` §Observed root keys; the
/// table is extensible (see module doc).
const LEGACY_ROOT_KEY_MAP: &[(&str, &str)] = &[("IMAGE", "IMAGE_PATH")];

/// Top-level keys whose values may carry inline script source the
/// `DeprecatedScriptApi` pass should scan. The reference-app wire
/// format binds scripts via the `script.js` package entry that the
/// scene loader does not yet handle, but a future scene fixture or
/// Chromaticity-native scene may inline source via these keys; the
/// pass is shape-coverage-ready for that case.
const SCRIPT_SOURCE_KEYS: &[&str] = &["SCRIPT", "script", "MAIN_SCRIPT"];

/// Walk an unnormalized `scene.json` value and emit
/// pre-deserialization compat warnings.
///
/// This is the slice-3 lint harness — distinct from
/// [`crate::validator::validate`], which walks a deserialized
/// [`crate::schema::Scene`]. Both emit the same [`Warning`]
/// vocabulary; lint sees raw keys, validator sees the canonical
/// model. The harness operates on `serde_json::Value` so it can
/// detect classes that disappear post-deserialization (legacy-key
/// rewrites absorbed by serde, deprecated-script-api access in raw
/// script source).
///
/// Returns warnings in deterministic order: legacy-key passes first
/// (top-level key scan), then per-control passes
/// (`unknown-control-type`, `coercion-applied`) in `CONTROLS[]` array
/// order, then deprecated-script-api scans of any inline script
/// keys. The order is not part of any compat surface; downstream
/// callers should treat the result as a set.
///
/// # Non-emissions
///
/// - A non-object input (`null`, array, scalar, string) emits zero
///   warnings — the lint pass is a best-effort pre-walk that defers
///   shape errors to the validator / deserializer.
/// - A control whose missing `TYPE` matches CP-01 step 5
///   (strict-reject) emits zero warnings; the validator owns the
///   strict-reject diagnostic.
pub fn lint_scene(unnormalized: &Value) -> Vec<Warning> {
    let mut warnings: Vec<Warning> = Vec::new();

    let Some(obj) = unnormalized.as_object() else {
        return warnings;
    };

    // Pass 1 — legacy-key rewrite per CP-01 CR-03 / docs/02 §Observed
    // root keys.
    for (legacy, canonical) in LEGACY_ROOT_KEY_MAP {
        if obj.contains_key(*legacy) {
            warnings.push(Warning::LegacyKey {
                canonical: (*canonical).to_string(),
                legacy: (*legacy).to_string(),
            });
        }
    }

    // Pass 2 — per-control unknown-control-type + coercion-applied.
    if let Some(controls) = obj.get("CONTROLS").and_then(Value::as_array) {
        for control in controls {
            let Some(co) = control.as_object() else {
                continue;
            };
            let name = co
                .get("NAME")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            match co.get("TYPE") {
                Some(Value::String(t)) => {
                    let first = t.split_ascii_whitespace().next().unwrap_or("");
                    let lower = first.to_ascii_lowercase();
                    if !KNOWN_CONTROL_KINDS.contains(&lower.as_str()) {
                        warnings.push(Warning::UnknownControlType {
                            control_name: name.clone(),
                        });
                    }
                }
                Some(_) => {
                    // TYPE present but not a string — treat as
                    // unknown rather than missing.
                    warnings.push(Warning::UnknownControlType {
                        control_name: name.clone(),
                    });
                }
                None => {
                    if let Some(to) = predict_cp01_fallback(co) {
                        warnings.push(Warning::CoercionApplied {
                            control_name: name.clone(),
                            from: "missing-type".to_string(),
                            to: to.to_string(),
                        });
                    }
                    // CP-01 step 5 (strict-reject) emits no
                    // CoercionApplied warning — the validator owns
                    // the diagnostic.
                }
            }
        }
    }

    // Pass 3 — deprecated-script-api scan over any inline script
    // source. Reference-app wire format binds scripts via the
    // `script.js` package entry, so this pass is shape-coverage-
    // ready against future fixtures rather than triggered by the
    // current corpus.
    for key in SCRIPT_SOURCE_KEYS {
        if let Some(Value::String(src)) = obj.get(*key) {
            collect_deprecated_script_apis(src, &mut warnings);
        }
    }

    warnings
}

/// Predict the CP-01 missing-`TYPE` fallback target as the
/// kebab-lowercase tag the validator's `CoercionApplied` payload
/// carries. Returns `None` for step 5 (strict-reject).
///
/// Mirrors the canonical order from
/// `docs/28-implementation-contract-pack.md` §CP-01 lines 38-42:
///
/// 1. `slider` — MIN, MAX, numeric DEFAULT, DEFAULT∈[MIN,MAX].
/// 2. `toggle` — DEFAULT∈{0,1} (bool or numeric) and no MIN / no MAX.
/// 3. `color`  — DEF_COLOR is present OR DEFAULT is a 3- or 4-float array.
/// 4. `xy`     — DEFAULT is a 2-float array.
/// 5. None     — strict-reject.
fn predict_cp01_fallback(control: &serde_json::Map<String, Value>) -> Option<&'static str> {
    let default = control.get("DEFAULT");
    let min = control.get("MIN").and_then(Value::as_f64);
    let max = control.get("MAX").and_then(Value::as_f64);
    let def_color_present = control.contains_key("DEF_COLOR");

    // Step 1 — slider.
    if let (Some(lo), Some(hi), Some(d)) = (min, max, default.and_then(Value::as_f64)) {
        if lo <= hi && d >= lo && d <= hi {
            return Some("slider");
        }
    }

    // Step 2 — toggle. Requires no MIN / no MAX.
    if min.is_none() && max.is_none() {
        match default {
            Some(Value::Bool(_)) => return Some("toggle"),
            Some(Value::Number(n)) => {
                if let Some(d) = n.as_f64() {
                    if d == 0.0 || d == 1.0 {
                        return Some("toggle");
                    }
                }
            }
            _ => {}
        }
    }

    // Step 3 — color.
    if def_color_present {
        return Some("color");
    }
    if let Some(Value::Array(arr)) = default {
        let len = arr.len();
        if (len == 3 || len == 4) && arr.iter().all(Value::is_number) {
            return Some("color");
        }
    }

    // Step 4 — xy.
    if let Some(Value::Array(arr)) = default {
        if arr.len() == 2 && arr.iter().all(Value::is_number) {
            return Some("xy");
        }
    }

    // Step 5 — strict-reject.
    None
}

/// Scan a raw script-source string for the deprecated `inputs.<name>`
/// and `uniforms.<name>` access patterns named in
/// `docs/28-implementation-contract-pack.md` §CP-01 CR-04 + §CP-07
/// parity matrix line 167.
///
/// Emits one `DeprecatedScriptApi` warning per match — duplicate
/// occurrences of the same access are reported once each so callers
/// can tally per-call-site usage. The pass is intentionally
/// substring-based: a real script-source linter would parse the
/// surface syntax, but slice-3 lint scope is shape coverage. False
/// positives on commented-out source are accepted as a known
/// limitation; a JS-aware deprecated-API pass lands with the script
/// host work-package.
fn collect_deprecated_script_apis(source: &str, out: &mut Vec<Warning>) {
    for prefix in ["inputs.", "uniforms."] {
        let mut rest = source;
        while let Some(idx) = rest.find(prefix) {
            let after = &rest[idx + prefix.len()..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                out.push(Warning::DeprecatedScriptApi {
                    api: format!("{prefix}{name}"),
                });
            }
            rest = &rest[idx + prefix.len()..];
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod scene_lint_classification {
    //! Evidence: EV-SCHEMA-LINT (T1) — scene-lint classification
    //! pass-by-pass drift pins for `lint_scene`.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins each of
    //! the four detection passes against a hand-built
    //! `serde_json::Value` exercising one positive emission and one
    //! negative non-emission. The T2 corpus conformance suite lives
    //! in `engine/scene/tests/lint_corpus.rs`; together they advance
    //! `EV-SCHEMA-LINT` past `pending:artifact` to Pass.

    use super::{Warning, lint_scene};
    use serde_json::json;

    #[test]
    fn lint_emits_legacy_key_for_image_alias() {
        let v = json!({"IMAGE": "/tmp/foo.png"});
        let warnings = lint_scene(&v);
        assert_eq!(warnings.len(), 1, "expected exactly one LegacyKey warning");
        match &warnings[0] {
            Warning::LegacyKey { canonical, legacy } => {
                assert_eq!(canonical, "IMAGE_PATH");
                assert_eq!(legacy, "IMAGE");
            }
            other => panic!("expected LegacyKey, got {other:?}"),
        }
    }

    #[test]
    fn lint_emits_no_legacy_key_for_canonical_image_path() {
        let v = json!({"IMAGE_PATH": "/tmp/foo.png"});
        assert!(lint_scene(&v).is_empty());
    }

    #[test]
    fn lint_emits_unknown_control_type_for_unrecognized_first_token() {
        let v = json!({
            "CONTROLS": [
                {"NAME": "wat", "TYPE": "nonsense", "DEFAULT": 0.0}
            ]
        });
        let warnings = lint_scene(&v);
        assert_eq!(warnings.len(), 1);
        match &warnings[0] {
            Warning::UnknownControlType { control_name } => {
                assert_eq!(control_name, "wat");
            }
            other => panic!("expected UnknownControlType, got {other:?}"),
        }
    }

    #[test]
    fn lint_accepts_canonical_first_token_with_compound_type() {
        // docs/02 §Observed control types: compound spellings like
        // `slider traveler` and `xy traveler` keep the canonical base
        // kind in the first whitespace token.
        let v = json!({
            "CONTROLS": [
                {"NAME": "a", "TYPE": "slider traveler", "DEFAULT": 0.0},
                {"NAME": "b", "TYPE": "xy traveler", "DEFAULT": [0.0, 0.0]},
                {"NAME": "c", "TYPE": "bang counter", "DEFAULT": 0.0}
            ]
        });
        assert!(lint_scene(&v).is_empty());
    }

    #[test]
    fn lint_emits_coercion_applied_for_missing_type_slider() {
        let v = json!({
            "CONTROLS": [
                {"NAME": "gain", "DEFAULT": 0.5, "MIN": 0.0, "MAX": 1.0}
            ]
        });
        let warnings = lint_scene(&v);
        assert_eq!(warnings.len(), 1);
        match &warnings[0] {
            Warning::CoercionApplied {
                control_name,
                from,
                to,
            } => {
                assert_eq!(control_name, "gain");
                assert_eq!(from, "missing-type");
                assert_eq!(to, "slider");
            }
            other => panic!("expected CoercionApplied, got {other:?}"),
        }
    }

    #[test]
    fn lint_emits_coercion_applied_for_missing_type_toggle_bool() {
        let v = json!({
            "CONTROLS": [
                {"NAME": "enabled", "DEFAULT": true}
            ]
        });
        let warnings = lint_scene(&v);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            Warning::CoercionApplied { to, .. } if to == "toggle"
        ));
    }

    #[test]
    fn lint_emits_coercion_applied_for_missing_type_color_def_color() {
        // DEFAULT must be outside {0, 1} so step 2 (toggle) does NOT
        // capture this case; step 2 takes priority over step 3 in the
        // CP-01 fallback order. Mirrors the validator's
        // `cp01_step3_def_color_coerces_in_compat_rejects_in_strict`
        // pattern (DEFAULT = 0.5).
        let v = json!({
            "CONTROLS": [
                {"NAME": "tint", "DEFAULT": 0.5, "DEF_COLOR": [0.1, 0.2, 0.3, 1.0]}
            ]
        });
        let warnings = lint_scene(&v);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            Warning::CoercionApplied { to, .. } if to == "color"
        ));
    }

    #[test]
    fn lint_emits_coercion_applied_for_missing_type_color_vec3_default() {
        // DEFAULT is a 3-float array; step 3 color path that does
        // not depend on DEF_COLOR.
        let v = json!({
            "CONTROLS": [
                {"NAME": "tint", "DEFAULT": [0.1, 0.2, 0.3]}
            ]
        });
        let warnings = lint_scene(&v);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            Warning::CoercionApplied { to, .. } if to == "color"
        ));
    }

    #[test]
    fn lint_emits_coercion_applied_for_missing_type_xy() {
        let v = json!({
            "CONTROLS": [
                {"NAME": "anchor", "DEFAULT": [0.5, 0.5]}
            ]
        });
        let warnings = lint_scene(&v);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            Warning::CoercionApplied { to, .. } if to == "xy"
        ));
    }

    #[test]
    fn lint_emits_no_warning_for_cp01_step5_strict_reject() {
        // CP-01 step 5: a missing-TYPE control whose shape fails
        // every fallback predicate. Lint emits NO warning — the
        // validator owns the strict-reject diagnostic.
        let v = json!({
            "CONTROLS": [
                {"NAME": "out_of_range", "DEFAULT": 5.0, "MIN": 0.0, "MAX": 1.0}
            ]
        });
        assert!(
            lint_scene(&v).is_empty(),
            "step-5 strict-reject must not emit a CoercionApplied warning",
        );
    }

    #[test]
    fn lint_emits_no_warning_for_canonical_typed_control() {
        let v = json!({
            "CONTROLS": [
                {"NAME": "gain", "TYPE": "slider", "DEFAULT": 0.5, "MIN": 0.0, "MAX": 1.0}
            ]
        });
        assert!(lint_scene(&v).is_empty());
    }

    #[test]
    fn lint_emits_deprecated_script_api_for_inputs_access() {
        let v = json!({
            "SCRIPT": "function setup() { var x = inputs.brightness; }"
        });
        let warnings = lint_scene(&v);
        assert_eq!(warnings.len(), 1);
        match &warnings[0] {
            Warning::DeprecatedScriptApi { api } => {
                assert_eq!(api, "inputs.brightness");
            }
            other => panic!("expected DeprecatedScriptApi, got {other:?}"),
        }
    }

    #[test]
    fn lint_emits_deprecated_script_api_for_uniforms_access() {
        let v = json!({
            "SCRIPT": "uniforms.tempo = 120;"
        });
        let warnings = lint_scene(&v);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            Warning::DeprecatedScriptApi { api } if api == "uniforms.tempo"
        ));
    }

    #[test]
    fn lint_returns_empty_for_non_object_input() {
        // Defensive: lint_scene on a non-object value emits zero
        // warnings rather than panicking. The validator / loader
        // owns shape-error diagnostics.
        for v in [
            serde_json::Value::Null,
            serde_json::json!([]),
            serde_json::json!(42),
            serde_json::json!("scene"),
        ] {
            assert!(
                lint_scene(&v).is_empty(),
                "non-object input must emit zero warnings: {v:?}"
            );
        }
    }

    #[test]
    fn lint_returns_empty_for_empty_object() {
        assert!(lint_scene(&serde_json::json!({})).is_empty());
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-SCHEMA-LINT (T0) — lint warning class baseline.
    //!
    //! T0 per `docs/43-development-standards.md` §15.5: the subject
    //! module exposes only a data-shape declaration (the `Warning`
    //! enum) with no non-`#[derive]`-generated behaviour. The
    //! variant-constructibility round-trip pins the enum shape
    //! against `docs/05-scene-system.md` §5 lines 79–83 so that
    //! silently dropping or renaming a variant fails in review.
    //!
    //! This was the first EV-SCHEMA-LINT claim anywhere in the
    //! tree; it reserved the EV row for the slice-3 lint harness.
    //! The harness has now landed (`super::lint_scene`), with T1
    //! classification pins in `super::scene_lint_classification` and
    //! T2 corpus conformance in `engine/scene/tests/lint_corpus.rs`.
    //! This T0 baseline is retained per
    //! `docs/43-development-standards.md` §15.5 grandfather rule —
    //! the variant-constructibility shape pin is still load-bearing
    //! against silent enum-variant drops or renames.

    use super::Warning;

    #[test]
    fn every_warning_class_is_constructible() {
        // Round-trip each of the four `docs/05` §5 variants. Clone
        // + equality comes from the `#[derive(Clone, PartialEq)]`
        // on `Warning`; the point of the test is that the variants
        // exist with the shape the docs name, not that derive
        // works.
        let legacy = Warning::LegacyKey {
            canonical: "IMAGE_PATH".to_string(),
            legacy: "IMAGE".to_string(),
        };
        let unknown = Warning::UnknownControlType {
            control_name: "brightness".to_string(),
        };
        let deprecated = Warning::DeprecatedScriptApi {
            api: "inputs.brightness".to_string(),
        };
        let coerced = Warning::CoercionApplied {
            control_name: "brightness".to_string(),
            from: "missing-type".to_string(),
            to: "slider".to_string(),
        };

        for w in [legacy, unknown, deprecated, coerced] {
            // Round-trip via clone + equality pins derive output
            // against the variant set.
            assert_eq!(w.clone(), w);
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod warning_class_taxonomy {
    //! Evidence: EV-SCHEMA-LINT (T1) — warning-class taxonomy pin against
    //! docs/05-scene-system.md §5 lines 79-83.
    //!
    //! T1 per docs/43-development-standards.md §15.5 — pins external-spec
    //! constants (the four kebab-case classes). Compile-time drift is
    //! caught by the exhaustive match in `class_name`; test-time drift is
    //! caught by the `EXPECTED_CLASSES` table.
    //!
    //! Row 26 (`EV-SCHEMA-LINT`) advances to `Pass` via the slice-3
    //! T2 corpus conformance suite in
    //! `engine/scene/tests/lint_corpus.rs`; this T1 taxonomy pin is
    //! complementary baseline coverage (kebab-case spellings + enum
    //! exhaustiveness) and does not by itself transition the row.

    use super::Warning;

    /// Sorted ascending for stable assertion.
    const EXPECTED_CLASSES: &[&str] = &[
        "coercion-applied",
        "deprecated-script-api",
        "legacy-key",
        "unknown-control-type",
    ];

    /// Maps a `Warning` variant to its kebab-case class name. Exhaustive
    /// match with NO `_` arm — adding a variant to `Warning` without
    /// extending this function fails at rustc level.
    fn class_name(w: &Warning) -> &'static str {
        match w {
            Warning::LegacyKey { .. } => "legacy-key",
            Warning::UnknownControlType { .. } => "unknown-control-type",
            Warning::DeprecatedScriptApi { .. } => "deprecated-script-api",
            Warning::CoercionApplied { .. } => "coercion-applied",
        }
    }

    fn sample_variants() -> [Warning; 4] {
        [
            Warning::LegacyKey {
                canonical: "IMAGE_PATH".to_string(),
                legacy: "IMAGE".to_string(),
            },
            Warning::UnknownControlType {
                control_name: "brightness".to_string(),
            },
            Warning::DeprecatedScriptApi {
                api: "inputs.brightness".to_string(),
            },
            Warning::CoercionApplied {
                control_name: "brightness".to_string(),
                from: "missing-type".to_string(),
                to: "slider".to_string(),
            },
        ]
    }

    #[test]
    fn every_warning_variant_has_a_kebab_class_name() {
        for w in sample_variants() {
            let name = class_name(&w);
            assert!(!name.is_empty(), "variant produced empty class name");
            assert!(
                name.chars().all(|c| c == '-' || c.is_ascii_lowercase()),
                "class name is not kebab-lower-ascii: {name}",
            );
        }
    }

    #[test]
    fn every_docs05_class_has_a_warning_variant() {
        // Catches docs/05-only additions — if docs/05 gains a 5th class
        // without a matching Warning variant, the collected set won't
        // cover EXPECTED_CLASSES.
        let produced: std::collections::BTreeSet<&'static str> =
            sample_variants().iter().map(class_name).collect();
        for expected in EXPECTED_CLASSES {
            assert!(
                produced.contains(expected),
                "docs/05 §5 lists `{expected}` but no Warning variant emits it",
            );
        }
    }

    #[test]
    fn every_warning_variant_class_is_in_docs05() {
        // Catches Rust-only additions — if Warning gains a 5th variant
        // whose class name isn't in docs/05, rust-class-not-in-docs fires.
        for w in sample_variants() {
            let name = class_name(&w);
            assert!(
                EXPECTED_CLASSES.contains(&name),
                "Warning variant emits class `{name}` not listed in docs/05 §5",
            );
        }
    }

    #[test]
    fn taxonomy_count_is_exactly_four() {
        assert_eq!(EXPECTED_CLASSES.len(), 4);
    }
}

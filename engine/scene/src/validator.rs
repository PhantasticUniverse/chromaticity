// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Scene validator — slice-2 real implementation.
//!
//! Replaces the slice-1 always-`Ok(())` stub with:
//!
//! - A [`ValidationMode`] enum selecting `Compat` (compat-syn-v1
//!   semantics per `docs/22-compatibility-versioning-policy.md`) vs
//!   `Strict` (native-chroma-v1 semantics).
//! - [`validate`], which cross-checks every
//!   [`crate::schema::UniformBinding`] symbol against a representative
//!   subset of `docs/36-reactive-symbol-catalog.md`, runs the CP-01
//!   missing-TYPE fallback on each [`crate::schema::ControlDecl`], and
//!   enforces node-ID uniqueness across `Scene::passes`.
//!
//! # Symbol catalog — representative subset vs slice-3 full loader
//!
//! Slice-2 ships a deliberately partial catalog loaded into a
//! `LazyLock<HashSet<&'static str>>` ([`KNOWN_SYMBOLS`]) covering
//! seven canonical symbols spanning every docs/36 top-level section:
//!
//! - `TIME` (docs/36 §1.1 Frame / timing, Profile C)
//! - `RENDERSIZE` (docs/36 §1.4 Multipass, Profile C)
//! - `_mouse` (docs/36 §1.3 Mouse, Profile C)
//! - `chroma_FinalPass` (docs/36 §1.4 Multipass, Profile C)
//! - `chroma_AudioTime` (docs/36 §2.4 Audio time clocks, Profile C)
//! - `chroma_Level` (docs/36 §2.2 Audio level, Profile C)
//! - `chroma_Spectrum` (docs/36 §2.1 Audio data textures, Profile C)
//!
//! The **full catalog loader lands with slice-3** alongside the
//! `.chroma` deserializer. The slice-3 loader will read docs/36 (or a
//! generated catalog manifest) at build time and expose the full
//! symbol set — including Profile A (Parity-Advanced) and Profile D
//! (Differentiation) symbols, and compat aliases under the CR-06
//! media-sampler alias rule. Until then, symbols outside the subset
//! above fail validation in both modes; this is a deliberate
//! false-negative the slice-3 loader eliminates.
//!
//! # CP-01 coercion order
//!
//! The missing-TYPE fallback walks the canonical order from
//! `docs/28-implementation-contract-pack.md` §CP-01 lines 38–42:
//!
//! 1. `slider` — MIN, MAX, DEFAULT all present and DEFAULT∈[MIN,MAX]
//! 2. `toggle` — DEFAULT∈{0,1} and no MIN/MAX
//! 3. `color`  — DEF_COLOR is present or DEFAULT is a 3- or 4-float array
//! 4. `xy`     — DEFAULT is a 2-float array
//! 5. strict-reject — `missing-type`
//!
//! Author-supplied MIN/MAX are preserved unchanged per CP-01 line 46
//! ("do not overwrite author-provided range hints").

use chromaticity_observability::catalog::{SceneLintCategory, SceneValidationOutcome};

use crate::lint::Warning;
use crate::observability::validation_counters;
use crate::schema::{ControlBound, ControlDecl, ControlKind, ControlValue, Scene};
use std::collections::HashSet;
use std::sync::LazyLock;

/// Representative subset of `docs/36-reactive-symbol-catalog.md`.
///
/// See the module doc for the provenance of each entry and the
/// slice-3 deferral note. The subset is intentionally partial.
static KNOWN_SYMBOLS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        // §1.1 Frame / timing
        "TIME",
        // §1.3 Mouse
        "_mouse",
        // §1.4 Multipass
        "RENDERSIZE",
        "chroma_FinalPass",
        // §2.1 Audio data textures
        "chroma_Spectrum",
        // §2.2 Audio level
        "chroma_Level",
        // §2.4 Audio time clocks
        "chroma_AudioTime",
    ]
    .into_iter()
    .collect()
});

/// Validator operating mode.
///
/// `Compat` applies the CP-01 coercion fallbacks and emits [`Warning`]
/// diagnostics for recoverable cases. `Strict` rejects the same cases
/// with [`ValidationError`]. The profile-tag strings that
/// `docs/22-compatibility-versioning-policy.md` names
/// (`compat-syn-v1`, `native-chroma-v1`) land with the slice-3 loader
/// that reads them off the scene header.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationMode {
    /// Accept legacy / outlier shapes and coerce per CP-01.
    Compat,
    /// Enforce the canonical schema only. No coercion, no warnings.
    Strict,
}

/// Validation error returned by [`validate`].
///
/// Each variant maps to a hard-fail diagnostic. `ValidationMode`
/// selects whether a recoverable CP-01 case becomes [`Warning`]
/// (compat) or [`ValidationError::MissingControlType`] (strict).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationError {
    /// A [`crate::schema::UniformBinding`]`::symbol` referenced a
    /// symbol not present in [`KNOWN_SYMBOLS`]. Carries the offending
    /// spelling.
    UnknownSymbol(String),
    /// A control declared no `TYPE` and none of the CP-01 fallback
    /// predicates matched — or the mode is [`ValidationMode::Strict`]
    /// and the missing-TYPE case is rejected unconditionally per
    /// CP-01 line 44. Carries the control's `NAME`.
    MissingControlType(String),
    /// Two [`crate::schema::PassSpec`] entries referenced the same
    /// [`crate::schema::NodeRef`]. Node-ID uniqueness is a hard
    /// structural invariant in both modes.
    DuplicateNode(String),
}

/// Validate a [`Scene`] under the given [`ValidationMode`].
///
/// Returns the list of [`Warning`] diagnostics emitted under compat
/// mode (empty under strict mode when validation succeeds), or the
/// first [`ValidationError`] encountered. Validation order:
///
/// 1. Node-ID uniqueness across `scene.passes`.
/// 2. Per-pass uniform-binding symbol cross-check against
///    [`KNOWN_SYMBOLS`].
/// 3. Per-control CP-01 missing-TYPE coercion (compat mode) or
///    rejection (strict mode).
///
/// The order is not yet part of any compat surface; slice-3 may
/// reorder freely.
pub fn validate(scene: &Scene, mode: ValidationMode) -> Result<Vec<Warning>, ValidationError> {
    let counters = validation_counters();
    let mut warnings: Vec<Warning> = Vec::new();

    // 1. Node-ID uniqueness. Strict and compat both reject.
    let mut seen_nodes: HashSet<&str> = HashSet::new();
    for pass in &scene.passes {
        let name = pass.node.0.as_str();
        if !seen_nodes.insert(name) {
            counters.record_validation(SceneValidationOutcome::Fail);
            return Err(ValidationError::DuplicateNode(name.to_string()));
        }
    }

    // 2. Uniform-binding symbol cross-check. Symbol unknowns are a
    //    hard error in both modes — the representative subset is
    //    authoritative for slice-2 (see module doc).
    for pass in &scene.passes {
        for binding in &pass.uniform_bindings {
            if !KNOWN_SYMBOLS.contains(binding.symbol.as_str()) {
                counters.record_validation(SceneValidationOutcome::Fail);
                return Err(ValidationError::UnknownSymbol(binding.symbol.clone()));
            }
        }
    }

    // 3. Per-control CP-01 missing-TYPE coercion.
    for control in &scene.controls {
        if control.kind.is_none() {
            match mode {
                ValidationMode::Strict => {
                    // CP-01 line 44: strict mode rejects
                    // unconditionally with a pointer to the first
                    // offending NAME.
                    counters.record_validation(SceneValidationOutcome::Fail);
                    return Err(ValidationError::MissingControlType(control.name.clone()));
                }
                ValidationMode::Compat => match coerce_missing_type(control) {
                    Some(kind) => {
                        counters.record_lint_warning(SceneLintCategory::CoercionApplied);
                        warnings.push(Warning::CoercionApplied {
                            control_name: control.name.clone(),
                            from: "missing-type".to_string(),
                            to: kind_tag(kind).to_string(),
                        });
                    }
                    None => {
                        // CP-01 step 5: strict-reject applies in
                        // compat mode too when no fallback matched.
                        counters.record_validation(SceneValidationOutcome::Fail);
                        return Err(ValidationError::MissingControlType(control.name.clone()));
                    }
                },
            }
        }
    }

    counters.record_validation(SceneValidationOutcome::Pass);
    Ok(warnings)
}

/// Walk the CP-01 missing-TYPE fallback order and return the coerced
/// [`ControlKind`], or `None` if none matched (strict-reject).
///
/// Author-supplied MIN/MAX are never overwritten — this function is
/// read-only against `decl` per `docs/28` §CP-01 line 46.
fn coerce_missing_type(decl: &ControlDecl) -> Option<ControlKind> {
    // Step 1 — slider: scalar MIN, MAX, numeric DEFAULT, DEFAULT∈[MIN,MAX].
    if let (Some(min), Some(max), ControlValue::Float(default)) = (
        decl.min.as_ref().and_then(ControlBound::as_scalar),
        decl.max.as_ref().and_then(ControlBound::as_scalar),
        &decl.default,
    ) {
        let d = *default;
        if min <= max && d >= min && d <= max {
            return Some(ControlKind::Slider);
        }
    }

    // Step 2 — toggle: DEFAULT∈{0,1} and no MIN / no MAX.
    if decl.min.is_none() && decl.max.is_none() {
        match &decl.default {
            ControlValue::Bool(_) => return Some(ControlKind::Toggle),
            ControlValue::Float(d) => {
                if *d == 0.0 || *d == 1.0 {
                    return Some(ControlKind::Toggle);
                }
            }
            _ => {}
        }
    }

    // Step 3 — color: DEF_COLOR is present OR DEFAULT is a 3- or
    // 4-float array.
    if decl.def_color.is_some()
        || matches!(&decl.default, ControlValue::Vec3(_) | ControlValue::Vec4(_))
    {
        return Some(ControlKind::Color);
    }

    // Step 4 — xy: DEFAULT is a 2-float array.
    if matches!(&decl.default, ControlValue::Vec2(_)) {
        return Some(ControlKind::Xy);
    }

    // Step 5 — strict-reject / `missing-type`.
    None
}

/// Map [`ControlKind`] to its canonical lowercase spelling as named
/// in `docs/28-implementation-contract-pack.md` §CP-01.
fn kind_tag(kind: ControlKind) -> &'static str {
    match kind {
        ControlKind::Slider => "slider",
        ControlKind::Toggle => "toggle",
        ControlKind::Color => "color",
        ControlKind::Xy => "xy",
        ControlKind::Bang => "bang",
        ControlKind::Dropdown => "dropdown",
        ControlKind::Knob => "knob",
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod ctor_placeholder {
    //! Evidence: EV-SCENE-IMPORT (T0) — slice-1 baseline placeholder
    //! retained per `docs/43-development-standards.md` §15.5.
    //!
    //! These tests were the slice-1 T0 claim against EV-SCENE-IMPORT
    //! (default-scene validate round-trip + error-variant
    //! constructibility smoke test). Slice-2 retains them exactly so
    //! that the §15.5 grandfather rule — "every subsequent slice is
    //! expected to upgrade existing T0 claims to T1 or T2 as real
    //! behaviour lands" — is satisfied by the new `drift_pins` T1
    //! module below rather than by deleting the T0 baseline.
    //!
    //! T0 remains the correct tier for these two tests: neither
    //! exercises the real validator behaviour. They pin the
    //! default-scene no-op and the error-variant shape, and nothing
    //! more.

    use super::{Scene, ValidationError, ValidationMode, validate};

    #[test]
    fn validate_default_scene_is_ok() {
        let scene = Scene::default();
        assert_eq!(validate(&scene, ValidationMode::Compat), Ok(Vec::new()));
    }

    #[test]
    fn unknown_symbol_error_is_constructible() {
        // Smoke-test the error shape so later work lands against a
        // known-good variant surface.
        let err = ValidationError::UnknownSymbol("chroma_DoesNotExist".to_string());
        match err {
            ValidationError::UnknownSymbol(name) => {
                assert_eq!(name, "chroma_DoesNotExist");
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-SCENE-IMPORT (T1) — validator drift pins for the
    //! slice-2 real implementation.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins
    //! external-spec constants (`docs/28` §CP-01 fallback order,
    //! CP-01 line 46 author-range preservation, `docs/36` symbol
    //! set) against a minimal real implementation. Any drift in the
    //! pinned order or preserved-range invariant fails at build.
    //! Upgrades from the slice-1 T0 baseline in `ctor_placeholder`
    //! per §15.5: "every subsequent slice is expected to upgrade
    //! existing T0 claims to T1 or T2 as real behaviour lands."
    //!
    //! Phase-1 exit still requires T2 (conformance over the baseline
    //! scene corpus); the T2 suite lands with the slice-3 disk-format
    //! loader + golden-corpus fixtures.

    use crate::lint::Warning;
    use crate::schema::{
        ControlBound, ControlDecl, ControlValue, NodeRef, PassSpec, Scene, UniformBinding,
    };

    use super::{ValidationError, ValidationMode, validate};

    fn valid_binding(symbol: &str) -> UniformBinding {
        UniformBinding {
            name: "u".to_string(),
            symbol: symbol.to_string(),
        }
    }

    fn pass_with(node: &str, bindings: Vec<UniformBinding>) -> PassSpec {
        PassSpec {
            node: NodeRef(node.to_string()),
            uniform_bindings: bindings,
            extensions: serde_json::Map::new(),
        }
    }

    #[test]
    fn default_scene_validates_under_both_modes() {
        let scene = Scene::default();
        assert_eq!(
            validate(&scene, ValidationMode::Compat),
            Ok(Vec::new()),
            "default scene must validate under compat mode with no warnings",
        );
        assert_eq!(
            validate(&scene, ValidationMode::Strict),
            Ok(Vec::new()),
            "default scene must validate under strict mode with no warnings",
        );
    }

    #[test]
    fn unknown_symbol_rejected_in_both_modes_for_slice2_subset() {
        // Slice-2 semantics: unknown symbols are rejected in BOTH
        // modes because the representative subset is authoritative
        // until the slice-3 full-catalog loader lands. The test
        // name reflects the slice-3 end-state — compat mode WILL
        // downgrade unknown-symbol to a warning once the full
        // catalog + alias table are loaded. Until then, the pin is
        // that both modes reject with the same variant and spelling.
        let mut scene = Scene::default();
        scene
            .passes
            .push(pass_with("n0", vec![valid_binding("chroma_DoesNotExist")]));

        match validate(&scene, ValidationMode::Strict) {
            Err(ValidationError::UnknownSymbol(name)) => {
                assert_eq!(name, "chroma_DoesNotExist");
            }
            other => panic!("expected UnknownSymbol error, got {other:?}"),
        }

        match validate(&scene, ValidationMode::Compat) {
            Err(ValidationError::UnknownSymbol(name)) => {
                assert_eq!(name, "chroma_DoesNotExist");
            }
            other => panic!(
                "expected UnknownSymbol error under compat mode (slice-2 \
                 representative-subset semantics), got {other:?}"
            ),
        }

        // Positive pin: a symbol from the representative subset
        // validates in both modes, guaranteeing the `HashSet` lookup
        // path reads what `docs/36` spells.
        let mut ok = Scene::default();
        ok.passes
            .push(pass_with("n0", vec![valid_binding("RENDERSIZE")]));
        assert!(validate(&ok, ValidationMode::Strict).is_ok());
        assert!(validate(&ok, ValidationMode::Compat).is_ok());
    }

    #[test]
    fn cp01_coercion_follows_canonical_slider_first_order() {
        // A control with numeric MIN/MAX and DEFAULT = 1.0 falls back
        // to slider per CP-01 step 1, NOT toggle per step 2, even
        // though DEFAULT ∈ {0, 1} in isolation. The MIN/MAX-present
        // guard on step 2 enforces the canonical slider-first order.
        let mut scene = Scene::default();
        scene.controls.push(ControlDecl {
            name: "brightness".to_string(),
            kind: None,
            default: ControlValue::Float(1.0),
            min: Some(0.0.into()),
            max: Some(2.0.into()),
            ..ControlDecl::default()
        });

        let warnings =
            validate(&scene, ValidationMode::Compat).expect("compat mode must coerce, not reject");
        assert_eq!(
            warnings.len(),
            1,
            "expected exactly one CoercionApplied warning"
        );
        match &warnings[0] {
            Warning::CoercionApplied {
                control_name,
                from,
                to,
            } => {
                assert_eq!(control_name, "brightness");
                assert_eq!(from, "missing-type");
                assert_eq!(
                    to, "slider",
                    "CP-01 step 1 (slider) must beat step 2 (toggle) when MIN/MAX are present",
                );
            }
            other => panic!("expected CoercionApplied warning, got {other:?}"),
        }

        // Strict mode rejects unconditionally.
        match validate(&scene, ValidationMode::Strict) {
            Err(ValidationError::MissingControlType(name)) => {
                assert_eq!(name, "brightness");
            }
            other => panic!("expected MissingControlType error under strict, got {other:?}"),
        }
    }

    #[test]
    fn cp01_preserves_author_ranges() {
        // CP-01 line 46: "The fallback must use MIN/MAX when present
        // even if they are not the canonical slider defaults ([0,
        // 1]); do not overwrite author-provided range hints."
        //
        // Slice-2 validator is non-mutating (the normalizer lives in
        // slice-3), so the pin takes the form: (a) the warning
        // carries the control name and the coerced target, and (b)
        // inspecting the input scene after validate shows the
        // author-supplied min / max unchanged. Slice-3 will extend
        // this pin into a positive assertion on the normalized
        // output.
        let mut scene = Scene::default();
        scene.controls.push(ControlDecl {
            name: "gain".to_string(),
            kind: None,
            default: ControlValue::Float(5.0),
            min: Some(2.5.into()),
            max: Some(7.5.into()),
            ..ControlDecl::default()
        });

        let warnings =
            validate(&scene, ValidationMode::Compat).expect("compat mode must coerce to slider");
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            Warning::CoercionApplied { to, .. } if to == "slider"
        ));

        // Input scene is untouched: author-supplied min / max
        // survive the validate pass unchanged.
        let decl = &scene.controls[0];
        assert_eq!(
            decl.min,
            Some(ControlBound::Scalar(2.5)),
            "min must not be overwritten"
        );
        assert_eq!(
            decl.max,
            Some(ControlBound::Scalar(7.5)),
            "max must not be overwritten"
        );
    }

    #[test]
    fn duplicate_node_ref_is_rejected() {
        // Node-ID uniqueness is a hard structural invariant in both
        // modes — not a coercion class.
        let mut scene = Scene::default();
        scene.passes.push(pass_with("n0", Vec::new()));
        scene.passes.push(pass_with("n0", Vec::new()));

        for mode in [ValidationMode::Strict, ValidationMode::Compat] {
            match validate(&scene, mode) {
                Err(ValidationError::DuplicateNode(name)) => {
                    assert_eq!(name, "n0");
                }
                other => {
                    panic!("expected DuplicateNode error under {mode:?}, got {other:?}")
                }
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod compat_fixture_coverage {
    //! Evidence: EV-COMPAT-FIXTURE-COVERAGE (T1) — CP-01
    //! coercion-fallback branch coverage per `docs/26` line 25.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: at least one
    //! fixture per branch of the CR-01 missing-TYPE fallback rule
    //! (slider, toggle, color, xy, strict-reject) per
    //! `docs/28-implementation-contract-pack.md` §CP-01, exercised
    //! against the slice-2 real validator under both
    //! [`ValidationMode::Compat`] and [`ValidationMode::Strict`].
    //! Covers:
    //!
    //! - Step 1 (slider): canonical `(min, max, Float(default))`
    //!   with `default ∈ [min, max]` — Compat coerces, Strict
    //!   rejects.
    //! - Step 2 (toggle): every DEFAULT spelling the validator
    //!   accepts — `Bool(true)`, `Bool(false)`, `Float(0.0)`,
    //!   `Float(1.0)` — when neither `min` nor `max` is set.
    //! - Step 3 (color): `DEF_COLOR`, `Vec3`, and `Vec4` shapes.
    //! - Step 4 (xy): `Vec2` default shape.
    //! - Step 5 (strict-reject): fallback exhaustion. Identical
    //!   `Err(ValidationError::MissingControlType)` return in BOTH
    //!   Compat and Strict modes — CP-01 step 5 applies in compat
    //!   mode too when no fallback matched (see
    //!   [`super::validate`] step-3-per-control and
    //!   [`super::coerce_missing_type`]). The "passes silently on
    //!   strict-reject-in-compat" shorthand refers to the absence
    //!   of an additional `Warning::CoercionApplied` layered on
    //!   top of the shared strict-reject error, NOT an
    //!   `Ok(vec![])` silent-pass path — there is no such path at
    //!   slice-2.
    //!
    //! Cumulative-release-gate scope. `docs/26` row 25
    //! (`EV-COMPAT-FIXTURE-COVERAGE`) declares
    //! `Gate Usage = "Phase 1 + release cumulative gate"`. The
    //! row's T2 conformance — "at least one fixture per branch
    //! using shipped reference scenes that trigger KM-01 or
    //! clean-room reconstructions" per `docs/26` line 25 and
    //! `docs/34 §Evidence Bindings` — requires the `.chroma`
    //! deserializer and the external-reader threshold that
    //! [`crate::schema`] module docs lines 14-26 defer to slice-3.
    //! This T1 module pins the CR-01 fallback behaviour under the
    //! slice-2 real validator; the slice-3+ T2 corpus at
    //! `engine/scene/tests/validator/main.rs` loads every shipped
    //! reference scene and re-validates the same invariants
    //! against author-written content, advancing row 25 past
    //! `pending:artifact` at that point. Per §15.5 T1 tests do
    //! not by themselves advance an `EV-*` row past
    //! `pending:artifact` to `Pass`; row 25 remains `Planned` /
    //! `pending:artifact` until T2 lands.

    use crate::lint::Warning;
    use crate::schema::{ControlDecl, ControlValue, Scene};

    use super::{ValidationError, ValidationMode, validate};

    /// Construct a minimal scene carrying a single [`ControlDecl`]
    /// with `kind = None` — the input shape that triggers the
    /// CP-01 missing-TYPE coercion fallback.
    fn scene_with_missing_type_control(
        name: &str,
        default: ControlValue,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Scene {
        let mut scene = Scene::default();
        scene.controls.push(ControlDecl {
            name: name.to_string(),
            kind: None,
            default,
            min: min.map(Into::into),
            max: max.map(Into::into),
            ..ControlDecl::default()
        });
        scene
    }

    /// Assert Compat validates and emits exactly one
    /// `Warning::CoercionApplied { control_name, from: "missing-type",
    /// to: <tag> }` for the named control.
    fn assert_compat_coerces(scene: &Scene, control_name: &str, to_tag: &str) {
        let warnings =
            validate(scene, ValidationMode::Compat).expect("compat mode must coerce, not reject");
        assert_eq!(
            warnings.len(),
            1,
            "expected exactly one CoercionApplied warning for {control_name}",
        );
        match &warnings[0] {
            Warning::CoercionApplied {
                control_name: cn,
                from,
                to,
            } => {
                assert_eq!(cn, control_name);
                assert_eq!(from, "missing-type");
                assert_eq!(
                    to, to_tag,
                    "CP-01 coercion target mismatch for {control_name}",
                );
            }
            other => panic!(
                "expected Warning::CoercionApplied {{ to: {to_tag:?} }} for {control_name}, got {other:?}",
            ),
        }
    }

    /// Assert Strict rejects with
    /// `Err(ValidationError::MissingControlType(control_name))`.
    fn assert_strict_rejects_missing_type(scene: &Scene, control_name: &str) {
        match validate(scene, ValidationMode::Strict) {
            Err(ValidationError::MissingControlType(name)) => {
                assert_eq!(name, control_name);
            }
            other => {
                panic!("expected MissingControlType({control_name:?}) under strict, got {other:?}",)
            }
        }
    }

    // ---- Step 1: slider ----

    #[test]
    fn cp01_step1_slider_coerces_in_compat_rejects_in_strict() {
        // Canonical slider: (min, max, Float(default)) with
        // default ∈ [min, max]. Distinct from the existing
        // drift_pins slider-first-priority test (which exercises
        // the step-1-beats-step-2 ordering with default ∈ {0, 1});
        // here the default is 0.5 so no step-2 ambiguity exists.
        let scene =
            scene_with_missing_type_control("gain", ControlValue::Float(0.5), Some(0.0), Some(1.0));
        assert_compat_coerces(&scene, "gain", "slider");
        assert_strict_rejects_missing_type(&scene, "gain");
    }

    // ---- Step 2: toggle (Bool default) ----

    #[test]
    fn cp01_step2_toggle_bool_default_coerces_in_compat_rejects_in_strict() {
        for (suffix, value) in [("true", true), ("false", false)] {
            let name = format!("flag_{suffix}");
            let scene =
                scene_with_missing_type_control(&name, ControlValue::Bool(value), None, None);
            assert_compat_coerces(&scene, &name, "toggle");
            assert_strict_rejects_missing_type(&scene, &name);
        }
    }

    // ---- Step 2: toggle (numeric 0 / 1 default) ----

    #[test]
    fn cp01_step2_toggle_numeric_default_coerces_in_compat_rejects_in_strict() {
        for (suffix, value) in [("zero", 0.0_f64), ("one", 1.0_f64)] {
            let name = format!("gate_{suffix}");
            let scene =
                scene_with_missing_type_control(&name, ControlValue::Float(value), None, None);
            assert_compat_coerces(&scene, &name, "toggle");
            assert_strict_rejects_missing_type(&scene, &name);
        }
    }

    // ---- Step 3: color (Vec3 / Vec4 default) ----

    #[test]
    fn cp01_step3_color_default_coerces_in_compat_rejects_in_strict() {
        let cases = [
            ("rgb", ControlValue::Vec3([0.1, 0.2, 0.3])),
            ("rgba", ControlValue::Vec4([0.1, 0.2, 0.3, 1.0])),
        ];
        for (suffix, default) in cases {
            let name = format!("tint_{suffix}");
            let scene = scene_with_missing_type_control(&name, default, None, None);
            assert_compat_coerces(&scene, &name, "color");
            assert_strict_rejects_missing_type(&scene, &name);
        }
    }

    #[test]
    fn cp01_step3_def_color_coerces_in_compat_rejects_in_strict() {
        let mut scene = Scene::default();
        scene.controls.push(ControlDecl {
            name: "palette".to_string(),
            kind: None,
            default: ControlValue::Float(0.5),
            def_color: Some([0.1, 0.2, 0.3, 1.0]),
            ..ControlDecl::default()
        });

        assert_compat_coerces(&scene, "palette", "color");
        assert_strict_rejects_missing_type(&scene, "palette");
    }

    // ---- Step 4: xy (Vec2 default) ----

    #[test]
    fn cp01_step4_xy_default_coerces_in_compat_rejects_in_strict() {
        let scene =
            scene_with_missing_type_control("cursor", ControlValue::Vec2([0.25, 0.75]), None, None);
        assert_compat_coerces(&scene, "cursor", "xy");
        assert_strict_rejects_missing_type(&scene, "cursor");
    }

    // ---- Step 5: strict-reject (fallback exhaustion) ----

    #[test]
    fn cp01_step5_strict_reject_returns_missing_type_in_both_modes() {
        // Construct a ControlDecl that fails every fallback step:
        //   - Step 1 slider: numeric DEFAULT 5.0 falls outside
        //     [MIN=0.0, MAX=1.0], so the range-check guard fails.
        //   - Step 2 toggle: rejected because MIN/MAX are present
        //     (the step-2 guard requires both `is_none`).
        //   - Step 3 color: DEFAULT is Float, not Vec3/Vec4.
        //   - Step 4 xy: DEFAULT is Float, not Vec2.
        //   - Step 5: `coerce_missing_type` returns None; the
        //     validator's compat-mode arm maps `None` to
        //     `Err(ValidationError::MissingControlType)` —
        //     IDENTICAL to strict mode's unconditional reject at
        //     the validate step-3-per-control Strict branch.
        let scene = scene_with_missing_type_control(
            "out_of_range",
            ControlValue::Float(5.0),
            Some(0.0),
            Some(1.0),
        );

        // Compat mode hits step 5 via coerce_missing_type -> None
        // and returns MissingControlType — there is NO silent-pass
        // Ok(vec![]) path at slice-2.
        match validate(&scene, ValidationMode::Compat) {
            Err(ValidationError::MissingControlType(name)) => {
                assert_eq!(name, "out_of_range");
            }
            other => panic!(
                "expected MissingControlType(\"out_of_range\") under compat \
                 (CP-01 step 5 applies in compat mode too when no fallback \
                 matches), got {other:?}",
            ),
        }

        // Strict mode hits the unconditional reject before
        // coerce_missing_type is even called. Same error variant
        // and name as the compat step-5 path.
        assert_strict_rejects_missing_type(&scene, "out_of_range");
    }
}

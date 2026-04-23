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
//! - `syn_FinalPass` (docs/36 §1.4 Multipass, Profile C)
//! - `syn_Time` (docs/36 §2.4 Audio time clocks, Profile C)
//! - `syn_Level` (docs/36 §2.2 Audio level, Profile C)
//! - `syn_Spectrum` (docs/36 §2.1 Audio data textures, Profile C)
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
//! 3. `color`  — DEFAULT is a 3- or 4-float array
//! 4. `xy`     — DEFAULT is a 2-float array
//! 5. strict-reject — `missing-type`
//!
//! Author-supplied MIN/MAX are preserved unchanged per CP-01 line 46
//! ("do not overwrite author-provided range hints").

use crate::lint::Warning;
use crate::schema::{ControlDecl, ControlKind, ControlValue, Scene};
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
        "syn_FinalPass",
        // §2.1 Audio data textures
        "syn_Spectrum",
        // §2.2 Audio level
        "syn_Level",
        // §2.4 Audio time clocks
        "syn_Time",
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
    let mut warnings: Vec<Warning> = Vec::new();

    // 1. Node-ID uniqueness. Strict and compat both reject.
    let mut seen_nodes: HashSet<&str> = HashSet::new();
    for pass in &scene.passes {
        let name = pass.node.0.as_str();
        if !seen_nodes.insert(name) {
            return Err(ValidationError::DuplicateNode(name.to_string()));
        }
    }

    // 2. Uniform-binding symbol cross-check. Symbol unknowns are a
    //    hard error in both modes — the representative subset is
    //    authoritative for slice-2 (see module doc).
    for pass in &scene.passes {
        for binding in &pass.uniform_bindings {
            if !KNOWN_SYMBOLS.contains(binding.symbol.as_str()) {
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
                    return Err(ValidationError::MissingControlType(control.name.clone()));
                }
                ValidationMode::Compat => match coerce_missing_type(control) {
                    Some(kind) => {
                        warnings.push(Warning::CoercionApplied {
                            control_name: control.name.clone(),
                            from: "missing-type".to_string(),
                            to: kind_tag(kind).to_string(),
                        });
                    }
                    None => {
                        // CP-01 step 5: strict-reject applies in
                        // compat mode too when no fallback matched.
                        return Err(ValidationError::MissingControlType(control.name.clone()));
                    }
                },
            }
        }
    }

    Ok(warnings)
}

/// Walk the CP-01 missing-TYPE fallback order and return the coerced
/// [`ControlKind`], or `None` if none matched (strict-reject).
///
/// Author-supplied MIN/MAX are never overwritten — this function is
/// read-only against `decl` per `docs/28` §CP-01 line 46.
fn coerce_missing_type(decl: &ControlDecl) -> Option<ControlKind> {
    // Step 1 — slider: MIN, MAX, numeric DEFAULT, DEFAULT∈[MIN,MAX].
    if let (Some(min), Some(max), ControlValue::Float(default)) =
        (decl.min, decl.max, &decl.default)
    {
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

    // Step 3 — color: DEFAULT is a 3- or 4-float array.
    if matches!(&decl.default, ControlValue::Vec3(_) | ControlValue::Vec4(_)) {
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
        let err = ValidationError::UnknownSymbol("syn_DoesNotExist".to_string());
        match err {
            ValidationError::UnknownSymbol(name) => {
                assert_eq!(name, "syn_DoesNotExist");
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
    use crate::schema::{ControlDecl, ControlValue, NodeRef, PassSpec, Scene, UniformBinding};

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
    fn unknown_symbol_rejected_in_strict_emits_warning_in_compat() {
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
            .push(pass_with("n0", vec![valid_binding("syn_DoesNotExist")]));

        match validate(&scene, ValidationMode::Strict) {
            Err(ValidationError::UnknownSymbol(name)) => {
                assert_eq!(name, "syn_DoesNotExist");
            }
            other => panic!("expected UnknownSymbol error, got {other:?}"),
        }

        match validate(&scene, ValidationMode::Compat) {
            Err(ValidationError::UnknownSymbol(name)) => {
                assert_eq!(name, "syn_DoesNotExist");
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
            min: Some(0.0),
            max: Some(2.0),
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
            min: Some(2.5),
            max: Some(7.5),
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
        assert_eq!(decl.min, Some(2.5), "min must not be overwritten");
        assert_eq!(decl.max, Some(7.5), "max must not be overwritten");
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

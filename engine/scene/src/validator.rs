// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-SCENE-IMPORT — baseline placeholder for the scene
//! import + canonical-model harness.
//!
//! Scene-validator first-slice stub.
//!
//! The real validator will cross-check every symbol reference in a
//! `Scene` against `docs/36-reactive-symbol-catalog.md`, which is
//! the authoritative source of truth for reactive symbol names
//! (standard uniforms, audio uniforms, meta controls, GLSL helper
//! shim catalog, script-host mirror rules). That catalog is
//! consumed, never edited, by this crate.
//!
//! For the first slice, `validate` always returns `Ok(())`; the
//! error shape exists so the `EV-SCENE-IMPORT` baseline test can
//! demonstrate the failure path compiles and is reachable once the
//! canonical-model normalizer begins populating `Scene`.

use crate::schema::Scene;

/// Validation error returned by [`validate`].
///
/// First-slice surface is deliberately narrow: a single variant
/// corresponding to the most common failure the real validator
/// will produce once it reads the reactive symbol catalog. More
/// variants (range errors, pass-wiring errors, transition-hook
/// errors tied to ADR-008) land alongside their enforcing code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// A scene referenced a symbol not present in
    /// `docs/36-reactive-symbol-catalog.md`.
    UnknownSymbol(String),
}

/// Validate a [`Scene`] against the reactive symbol catalog.
///
/// First-slice behavior: no-op; always returns `Ok(())`. The real
/// implementation reads `docs/36-reactive-symbol-catalog.md` and
/// rejects unknown symbol references with
/// [`ValidationError::UnknownSymbol`].
pub fn validate(_scene: &Scene) -> Result<(), ValidationError> {
    Ok(())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-SCENE-IMPORT — baseline placeholder for the
    //! scene import + canonical-model harness.

    use crate::schema::Scene;

    #[test]
    fn validate_default_scene_is_ok() {
        let scene = Scene::default();
        assert_eq!(super::validate(&scene), Ok(()));
    }

    #[test]
    fn unknown_symbol_error_is_constructible() {
        // Smoke-test the error shape so later work lands against a
        // known-good variant surface. The real validator will
        // return this variant from the symbol-catalog cross-check.
        let err = super::ValidationError::UnknownSymbol("syn_DoesNotExist".to_string());
        match err {
            super::ValidationError::UnknownSymbol(name) => {
                assert_eq!(name, "syn_DoesNotExist");
            }
        }
    }
}

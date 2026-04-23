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
//! producers (slice-3 scene-lint mode) will emit the same variants
//! outside the validate path.
//!
//! Slice-2 scope: the enum declaration plus variant-construction
//! smoke coverage. No classification logic yet; slice-3 adds the
//! scene-lint harness that walks unnormalized input and emits these
//! variants ahead of a validate call.

use serde::{Deserialize, Serialize};

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
    //! This is the first EV-SCHEMA-LINT claim anywhere in the tree;
    //! it reserves the EV row for the slice-3 lint harness, which
    //! will upgrade the tier to T1 (pinning the scene-lint
    //! classification transitions) and the phase-2 conformance
    //! suite, which will upgrade to T2.

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
    //! Row 24 stays `Planned`: T1 does not advance an EV-* row past
    //! `pending:artifact`. The slice-3 lint harness emission path
    //! (see lint.rs module doc) will upgrade this row to T2.

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

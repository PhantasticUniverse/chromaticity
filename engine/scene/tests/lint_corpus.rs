// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Evidence: EV-SCHEMA-LINT (T2) — scene-lint pipeline corpus
//! conformance over the shipped `.chroma` fixture set.
//!
//! Closes `docs/26-evidence-artifact-registry.md` row 26
//! (`EV-SCHEMA-LINT`): "validate schema and normalization warnings"
//! via the schema/lint pipeline. T2 per
//! `docs/43-development-standards.md` §15.5 conformance over a
//! fixture corpus is the tier that transitions
//! `EV-SCHEMA-LINT` past `pending:artifact` to Pass.
//!
//! The corpus is the union of every shipped scene fixture under
//! `engine/scene/tests/fixtures/scenes/`:
//!
//! - **3 W10 demo fixtures** (`demo-kaleido`, `demo-spectrum`,
//!   `demo-text`) — every control carries an explicit `TYPE` and no
//!   legacy root keys, so `lint_scene` emits zero warnings. Pins
//!   that the canonical-corpus baseline does not generate compat
//!   noise.
//! - **5 W11 cp01-fallback fixtures** (`cp01-slider`, `cp01-toggle`,
//!   `cp01-color`, `cp01-xy`, `cp01-reject`) — each carries one
//!   missing-`TYPE` control; the first four trigger one
//!   `CoercionApplied` warning per `docs/28-implementation-contract-pack.md`
//!   §CP-01 CR-01 step 1-4, and `cp01-reject` triggers zero
//!   warnings (CP-01 step 5 strict-reject is the validator's
//!   diagnostic, not a lint emission).
//! - **7 W12 PR1 reactive-symbol fixtures** (`multipass-feedback`,
//!   `multipass-chain`, `reactive-audio`, `meta-controls`,
//!   `traveler-controls`, `transition-hard`, `pass-sizing-sentinel`)
//!   — every control carries an explicit canonical first-token
//!   `TYPE` (including compound spellings like `slider traveler`,
//!   `xy traveler`, `bang counter` per `docs/02-synesthesia-contracts.md`
//!   §Observed control types lines 31-33), so `lint_scene` emits
//!   zero warnings. Pins that compound-`TYPE` spellings do not
//!   spuriously fire `unknown-control-type`.
//!
//! # Lint vs validator vantage points
//!
//! `lint_scene` walks the unnormalized `serde_json::Value`
//! pre-deserialization; the validator walks the typed
//! `chromaticity_scene::schema::Scene` post-deserialization. Both
//! emit the same `Warning` vocabulary, but lint detects classes that
//! disappear post-deserialize (legacy-key rewrites, raw script-source
//! deprecated-API access). This T2 suite pins lint's emissions; the
//! validator's emissions are pinned by
//! `engine/scene/src/validator.rs`'s `compat_fixture_coverage` T1
//! module and `engine/scene/tests/cp01_compat.rs`'s T2 module.
//!
//! Anchors:
//! - `docs/05-scene-system.md` §5 lines 79-83 — warning class
//!   taxonomy (`legacy-key`, `unknown-control-type`,
//!   `deprecated-script-api`, `coercion-applied`).
//! - `docs/26-evidence-artifact-registry.md` row 26 — EV-SCHEMA-LINT
//!   gate definition.
//! - `docs/28-implementation-contract-pack.md` §CP-01 — coercion
//!   precedence and CR-01 / CR-03 / CR-04 anchors used by
//!   `lint_scene`'s detection passes.
//! - `docs/43-development-standards.md` §15.5 — T2 conformance
//!   tier definition.
//!
//! Pairs with the W12 PR1 `EV-SCENE-IMPORT` T2 closure
//! (`engine/scene/tests/import.rs`); the present PR is the second
//! half of Phase-1 evidence-gate closure for the schema/lint family.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::path::PathBuf;

use chromaticity_scene::lint::{Warning, lint_scene};

const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/scenes");

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(FIXTURE_DIR);
    p.push(name);
    p
}

fn lint_fixture(package_name: &str) -> Vec<Warning> {
    let scene_json = fixture_path(package_name).join("scene.json");
    let raw = std::fs::read_to_string(&scene_json).unwrap_or_else(|e| {
        panic!(
            "fixture {package_name}: failed to read {}: {e}",
            scene_json.display()
        )
    });
    let value: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("fixture {package_name}: scene.json is not valid JSON: {e}"));
    lint_scene(&value)
}

/// Assert that the fixture produces exactly zero warnings.
fn assert_lint_clean(package_name: &str) {
    let warnings = lint_fixture(package_name);
    assert!(
        warnings.is_empty(),
        "fixture {package_name}: expected zero lint warnings, got {warnings:?}",
    );
}

/// Assert that the fixture produces exactly one
/// `Warning::CoercionApplied { control_name, from: "missing-type", to }`
/// for the named control.
fn assert_lint_coerces(package_name: &str, control_name: &str, to_tag: &str) {
    let warnings = lint_fixture(package_name);
    assert_eq!(
        warnings.len(),
        1,
        "fixture {package_name}: expected exactly one CoercionApplied warning, got {warnings:?}",
    );
    match &warnings[0] {
        Warning::CoercionApplied {
            control_name: cn,
            from,
            to,
        } => {
            assert_eq!(cn, control_name, "fixture {package_name}: control_name");
            assert_eq!(from, "missing-type", "fixture {package_name}: from");
            assert_eq!(to, to_tag, "fixture {package_name}: to");
        }
        other => panic!(
            "fixture {package_name}: expected CoercionApplied {{ to: {to_tag:?} }}, got {other:?}",
        ),
    }
}

// ---------------------------------------------------------------------------
// W10 demo fixtures — every control typed; zero warnings.
// ---------------------------------------------------------------------------

#[test]
fn lint_demo_kaleido_is_clean() {
    assert_lint_clean("library/demo-kaleido.chroma");
}

#[test]
fn lint_demo_spectrum_is_clean() {
    assert_lint_clean("library/demo-spectrum.chroma");
}

#[test]
fn lint_demo_text_is_clean() {
    assert_lint_clean("library/demo-text.chroma");
}

// ---------------------------------------------------------------------------
// W11 CP-01 fallback fixtures — each missing-TYPE control predicts
// the matching CR-01 fallback target. cp01-reject is step 5
// strict-reject and emits zero lint warnings (validator emits the
// MissingControlType error).
// ---------------------------------------------------------------------------

#[test]
fn lint_cp01_slider_emits_coercion_to_slider() {
    assert_lint_coerces("cp01-slider.chroma", "intensity", "slider");
}

#[test]
fn lint_cp01_toggle_emits_coercion_to_toggle() {
    assert_lint_coerces("cp01-toggle.chroma", "enabled", "toggle");
}

#[test]
fn lint_cp01_color_emits_coercion_to_toggle_via_step2_priority() {
    // The shipped `cp01-color.chroma` fixture carries
    // `DEFAULT: 0.0, DEF_COLOR: [...]` with no MIN / MAX. CP-01 step
    // 2 (toggle) takes priority over step 3 (color) when DEFAULT is
    // 0 or 1 and MIN / MAX are absent — so the canonical fallback
    // outcome is `toggle`, NOT `color`, despite the fixture's
    // intent. Lint mirrors the validator's `coerce_missing_type`
    // step ordering exactly; the validator-side
    // `cp01_color_compat_coerces` integration test passes because
    // it asserts only that compat mode emits a warning, not the
    // specific target. The fixture remains useful corpus coverage
    // (it pins step-2-beats-step-3 priority on a real fixture);
    // a `step3_color_with_nonzero_default` follow-up fixture would
    // exercise the pure step-3 branch — captured as W13+ corpus
    // gap, not gating EV-SCHEMA-LINT T2.
    assert_lint_coerces("cp01-color.chroma", "tint", "toggle");
}

#[test]
fn lint_cp01_xy_emits_coercion_to_xy() {
    assert_lint_coerces("cp01-xy.chroma", "anchor", "xy");
}

#[test]
fn lint_cp01_reject_emits_no_warning() {
    // CP-01 step 5: lint_scene returns no warning; the validator
    // owns the strict-reject diagnostic. Pairs with
    // `cp01_compat::cp01_reject_compat_rejects` which pins the
    // validator-side emission.
    assert_lint_clean("cp01-reject.chroma");
}

// ---------------------------------------------------------------------------
// W12 PR1 reactive-symbol fixtures — every control typed (including
// compound first-token spellings); zero lint warnings.
// ---------------------------------------------------------------------------

#[test]
fn lint_multipass_feedback_is_clean() {
    assert_lint_clean("multipass-feedback.chroma");
}

#[test]
fn lint_multipass_chain_is_clean() {
    assert_lint_clean("multipass-chain.chroma");
}

#[test]
fn lint_reactive_audio_is_clean() {
    assert_lint_clean("reactive-audio.chroma");
}

#[test]
fn lint_meta_controls_is_clean() {
    assert_lint_clean("meta-controls.chroma");
}

#[test]
fn lint_traveler_controls_is_clean() {
    // Compound TYPE spellings (`slider traveler`, `xy traveler`,
    // `bang counter`) keep their canonical first-token in the
    // taxonomy; lint_scene must not flag them as unknown.
    assert_lint_clean("traveler-controls.chroma");
}

#[test]
fn lint_transition_hard_is_clean() {
    assert_lint_clean("transition-hard.chroma");
}

#[test]
fn lint_pass_sizing_sentinel_is_clean() {
    assert_lint_clean("pass-sizing-sentinel.chroma");
}

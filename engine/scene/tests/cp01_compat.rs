// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-COMPAT-FIXTURE-COVERAGE (T2) — CP-01 fallback corpus
//! compat-vs-strict assertions over the shipped `cp01-*.chroma`
//! reference scenes.
//!
//! Closes the file-fixture wing of `docs/26-evidence-artifact-registry.md`
//! row 27 (`EV-COMPAT-FIXTURE-COVERAGE`): "at least one fixture per
//! branch of the CR-01 missing-`TYPE` fallback rule (slider, toggle,
//! color, xy, strict-reject) per
//! `docs/28-implementation-contract-pack.md` §CP-01, using shipped
//! reference scenes that trigger KM-01 or clean-room reconstructions."
//! Each fixture is a clean-room reconstruction authored under
//! `docs/29-cleanroom-legal-policy-baseline.md`.
//!
//! The five shipped reference scenes (`cp01-slider.chroma`,
//! `cp01-toggle.chroma`, `cp01-color.chroma`, `cp01-xy.chroma`,
//! `cp01-reject.chroma`) are loaded through
//! [`chromaticity_scene::loader::load_chroma_dir`] — the same loader
//! the runtime uses — and each is validated under both
//! [`ValidationMode::Compat`] and [`ValidationMode::Strict`]. This is
//! the file-fixture counterpart to the T1 `compat_fixture_coverage`
//! module in `engine/scene/src/validator.rs`, which exercises the same
//! CP-01 branch coverage against hand-built `Scene` structs.
//!
//! Per `docs/43-development-standards.md` §15.5, this T2 suite is what
//! advances `EV-COMPAT-FIXTURE-COVERAGE` past `pending:artifact` —
//! the T1 in-validator pins do not by themselves transition the row.
//! The CP-01 step-1-through-4 compat-mode warning shape (the exact
//! `Warning::CoercionApplied { .. }` payload) is `EV-SCHEMA-LINT`
//! scope and is intentionally not pinned here; this file asserts only
//! that compat-mode `validate(...)` returns `Ok(_)` for fallback
//! steps 1–4 and `Err(_)` for step 5. The validator's existing
//! `cp01_step5_strict_reject_returns_missing_type_in_both_modes` T1
//! pin establishes the step-5 in-both-modes-error invariant against
//! a hand-built scene; the `cp01_reject_*` tests here are its
//! file-fixture counterparts.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::path::PathBuf;

use chromaticity_scene::loader::load_chroma_dir;
use chromaticity_scene::schema::Scene;
use chromaticity_scene::validator::{ValidationError, ValidationMode, validate};

const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/scenes");

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(FIXTURE_DIR);
    p.push(name);
    p
}

fn load_fixture(name: &str) -> Scene {
    load_chroma_dir(fixture_path(name)).expect("cp-01 fixture must load")
}

/// Assert that compat-mode validation accepts the fixture (returns
/// `Ok(_)`). The exact warning shape is `EV-SCHEMA-LINT` scope; this
/// helper deliberately does not pin warning payloads. It does require
/// at least one warning to be present, which is the load-bearing
/// signal that CP-01 coercion fired rather than the scene being a
/// no-op.
fn assert_compat_coerces(fixture_name: &str) {
    let scene = load_fixture(fixture_name);
    let warnings = validate(&scene, ValidationMode::Compat)
        .expect("compat mode must coerce a missing-TYPE fallback fixture, not reject");
    assert!(
        !warnings.is_empty(),
        "{fixture_name}: compat mode must emit at least one CP-01 coercion warning",
    );
}

/// Assert that strict-mode validation rejects the fixture with
/// `Err(ValidationError::MissingControlType(_))`. The exact `NAME`
/// carried by the error is fixture-specific; the helper accepts any
/// non-empty spelling so the test stays decoupled from the per-fixture
/// `NAME` string.
fn assert_strict_rejects(fixture_name: &str) {
    let scene = load_fixture(fixture_name);
    match validate(&scene, ValidationMode::Strict) {
        Err(ValidationError::MissingControlType(name)) => {
            assert!(
                !name.is_empty(),
                "{fixture_name}: strict-mode MissingControlType must carry a non-empty NAME",
            );
        }
        other => panic!(
            "{fixture_name}: expected ValidationError::MissingControlType under strict, \
             got {other:?}",
        ),
    }
}

// ---------------------------------------------------------------------------
// CP-01 step 1 — slider fallback (`cp01-slider.chroma`).
// ---------------------------------------------------------------------------

#[test]
fn cp01_slider_compat_coerces() {
    assert_compat_coerces("cp01-slider.chroma");
}

#[test]
fn cp01_slider_strict_rejects() {
    assert_strict_rejects("cp01-slider.chroma");
}

// ---------------------------------------------------------------------------
// CP-01 step 2 — toggle fallback (`cp01-toggle.chroma`).
// ---------------------------------------------------------------------------

#[test]
fn cp01_toggle_compat_coerces() {
    assert_compat_coerces("cp01-toggle.chroma");
}

#[test]
fn cp01_toggle_strict_rejects() {
    assert_strict_rejects("cp01-toggle.chroma");
}

// ---------------------------------------------------------------------------
// CP-01 step 3 — color fallback (`cp01-color.chroma`).
// ---------------------------------------------------------------------------

#[test]
fn cp01_color_compat_coerces() {
    assert_compat_coerces("cp01-color.chroma");
}

#[test]
fn cp01_color_strict_rejects() {
    assert_strict_rejects("cp01-color.chroma");
}

// ---------------------------------------------------------------------------
// CP-01 step 4 — xy fallback (`cp01-xy.chroma`).
// ---------------------------------------------------------------------------

#[test]
fn cp01_xy_compat_coerces() {
    assert_compat_coerces("cp01-xy.chroma");
}

#[test]
fn cp01_xy_strict_rejects() {
    assert_strict_rejects("cp01-xy.chroma");
}

// ---------------------------------------------------------------------------
// CP-01 step 5 — strict-reject (`cp01-reject.chroma`). Both modes
// return `Err(ValidationError::MissingControlType)` — CP-01 step 5
// applies in compat mode too when no fallback predicate matches. This
// pairs with the in-validator
// `cp01_step5_strict_reject_returns_missing_type_in_both_modes` T1
// pin, which exercises the same invariant against a hand-built scene.
// ---------------------------------------------------------------------------

#[test]
fn cp01_reject_compat_rejects() {
    let scene = load_fixture("cp01-reject.chroma");
    match validate(&scene, ValidationMode::Compat) {
        Err(ValidationError::MissingControlType(name)) => {
            assert!(
                !name.is_empty(),
                "cp01-reject: compat-mode MissingControlType must carry a non-empty NAME",
            );
        }
        other => panic!(
            "cp01-reject: expected ValidationError::MissingControlType under compat \
             (CP-01 step 5 applies in compat mode too when no fallback matches), got {other:?}",
        ),
    }
}

#[test]
fn cp01_reject_strict_rejects() {
    assert_strict_rejects("cp01-reject.chroma");
}

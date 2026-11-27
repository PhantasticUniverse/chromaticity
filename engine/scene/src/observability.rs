// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Scene observability — slice-2 in-process counter emission.
//!
//! Slice-2 ships the first non-stub scene emission sites for two
//! `docs/31-supportability-privacy-ops-policy.md` §Scheme and Metric
//! Catalog rows:
//!
//! - `chromaticity_scene_validations_total` keyed by
//!   [`SceneValidationOutcome`]. Incremented at every exit of
//!   [`crate::validator::validate`]: `Ok(_)` maps to
//!   [`SceneValidationOutcome::Pass`], `Err(_)` maps to
//!   [`SceneValidationOutcome::Fail`].
//! - `chromaticity_scene_lint_warnings_total` keyed by
//!   [`SceneLintCategory`]. Incremented at the
//!   `warnings.push(Warning::CoercionApplied { .. })` site inside
//!   `validate`, mapping to [`SceneLintCategory::CoercionApplied`].
//!   The other three categories (`LegacyKey`, `UnknownControlType`,
//!   `DeprecatedScriptApi`) are reserved for the slice-3 scene-lint
//!   harness in [`crate::lint`], which will reuse the same
//!   [`SceneValidationCounters::record_lint_warning`] helper outside
//!   the validate path rather than open a parallel counter surface.
//!
//! Per-outcome / per-category counters are maintained in-process on a
//! module-level `LazyLock<SceneValidationCounters>` exposed via
//! [`validation_counters`]. This is the interim observable for the
//! two emission sites declared in `docs/31` §Scheme and Metric
//! Catalog; slice-3 will route each outcome / category through the
//! runtime bus via
//! `chromaticity_observability::catalog::MetricEvent::SceneValidations`
//! and `MetricEvent::SceneLintWarnings` respectively, and reduce
//! this scene-local table to a bus-side counter. The counter
//! vocabulary matches [`SceneValidationOutcome`] and
//! [`SceneLintCategory`] verbatim; the mapping from validator exit
//! points to counter increments is the same on both sides, so the
//! observed counts stay equivalent across the slice-3 transition.
//!
//! The validator is a free function ([`crate::validator::validate`]),
//! so counters live at module scope rather than as an adapter-instance
//! field (the OSC adapter precedent at
//! `integrations/osc-adapter/src/adapter.rs` uses per-instance `u64`
//! fields because the adapter *is* the object). Atomic counters are
//! used here rather than plain `u64` because `validate` takes `&Scene`
//! and may be called concurrently on the ADR-006 thread-model's
//! runtime bus hot path; `Ordering::Relaxed` is correct because
//! counters are read-only by observers and there is no happens-before
//! requirement between increments.
//!
//! Packet-minimum coverage for `EV-OBSERVABILITY` T1 is pinned by the
//! `#[cfg(test)] mod observability_emission` block at the bottom of
//! the module.

use core::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;

use chromaticity_observability::catalog::{SceneLintCategory, SceneValidationOutcome};

/// Per-process [`SceneValidationOutcome`] / [`SceneLintCategory`]
/// counters for `chromaticity-scene`.
///
/// Laid out as named [`AtomicU64`] fields (one per label value) rather
/// than a map so the compiler sees the full key set and external
/// callers cannot accidentally depend on the storage shape.
/// [`SceneValidationOutcome`] and [`SceneLintCategory`] are both
/// `#[non_exhaustive]` across the crate boundary, so
/// [`SceneValidationCounters::validation_count`] and
/// [`SceneValidationCounters::lint_warning_count`] keep wildcard arms
/// for future variants that slice-2 has not been taught to produce.
///
/// Instances are intended to be constructed once per process via
/// [`validation_counters`]. Callers can also construct a bare
/// [`SceneValidationCounters`] directly (e.g. for test harnesses that
/// want an isolated counter table), but the validator emission sites
/// always target the module-level static.
#[derive(Debug, Default)]
pub struct SceneValidationCounters {
    validations_pass: AtomicU64,
    validations_fail: AtomicU64,
    lint_legacy_key: AtomicU64,
    lint_unknown_control_type: AtomicU64,
    lint_deprecated_script_api: AtomicU64,
    lint_coercion_applied: AtomicU64,
}

impl SceneValidationCounters {
    /// Increment the validation-outcome counter for `outcome` by one.
    ///
    /// Unknown variants (future `#[non_exhaustive]` additions) are
    /// ignored rather than panicked on; slice-3 teaches the validator
    /// to produce new outcomes explicitly when they land.
    pub fn record_validation(&self, outcome: SceneValidationOutcome) {
        match outcome {
            SceneValidationOutcome::Pass => {
                self.validations_pass.fetch_add(1, Ordering::Relaxed);
            }
            SceneValidationOutcome::Fail => {
                self.validations_fail.fetch_add(1, Ordering::Relaxed);
            }
            _ => (),
        }
    }

    /// Increment the lint-category counter for `category` by one.
    ///
    /// Slice-2 only emits [`SceneLintCategory::CoercionApplied`] from
    /// [`crate::validator::validate`]. The helper accepts every
    /// variant so the slice-3 scene-lint harness in [`crate::lint`]
    /// can reuse it to emit `LegacyKey`, `UnknownControlType`, and
    /// `DeprecatedScriptApi` without opening a parallel counter
    /// surface. Unknown variants (future `#[non_exhaustive]`
    /// additions) are ignored rather than panicked on.
    pub fn record_lint_warning(&self, category: SceneLintCategory) {
        match category {
            SceneLintCategory::LegacyKey => {
                self.lint_legacy_key.fetch_add(1, Ordering::Relaxed);
            }
            SceneLintCategory::UnknownControlType => {
                self.lint_unknown_control_type
                    .fetch_add(1, Ordering::Relaxed);
            }
            SceneLintCategory::DeprecatedScriptApi => {
                self.lint_deprecated_script_api
                    .fetch_add(1, Ordering::Relaxed);
            }
            SceneLintCategory::CoercionApplied => {
                self.lint_coercion_applied.fetch_add(1, Ordering::Relaxed);
            }
            _ => (),
        }
    }

    /// Current count of validations that exited with the given
    /// [`SceneValidationOutcome`] since this counter table was
    /// constructed.
    ///
    /// The wildcard arm covers future `#[non_exhaustive]` additions
    /// to [`SceneValidationOutcome`] which slice-2 has not been
    /// taught to produce; any such variant reads as 0.
    #[must_use]
    pub fn validation_count(&self, outcome: SceneValidationOutcome) -> u64 {
        match outcome {
            SceneValidationOutcome::Pass => self.validations_pass.load(Ordering::Relaxed),
            SceneValidationOutcome::Fail => self.validations_fail.load(Ordering::Relaxed),
            _ => 0,
        }
    }

    /// Current count of lint warnings emitted under the given
    /// [`SceneLintCategory`] since this counter table was
    /// constructed.
    ///
    /// The wildcard arm covers future `#[non_exhaustive]` additions
    /// to [`SceneLintCategory`] which slice-2 has not been taught to
    /// produce; any such variant reads as 0.
    #[must_use]
    pub fn lint_warning_count(&self, category: SceneLintCategory) -> u64 {
        match category {
            SceneLintCategory::LegacyKey => self.lint_legacy_key.load(Ordering::Relaxed),
            SceneLintCategory::UnknownControlType => {
                self.lint_unknown_control_type.load(Ordering::Relaxed)
            }
            SceneLintCategory::DeprecatedScriptApi => {
                self.lint_deprecated_script_api.load(Ordering::Relaxed)
            }
            SceneLintCategory::CoercionApplied => {
                self.lint_coercion_applied.load(Ordering::Relaxed)
            }
            _ => 0,
        }
    }
}

static COUNTERS: LazyLock<SceneValidationCounters> =
    LazyLock::new(SceneValidationCounters::default);

/// Handle to the per-process [`SceneValidationCounters`] that the
/// scene emission sites write through.
///
/// Callers read the counters via
/// [`SceneValidationCounters::validation_count`] /
/// [`SceneValidationCounters::lint_warning_count`]. The validator
/// emission path at [`crate::validator::validate`] records through
/// the same handle via
/// [`SceneValidationCounters::record_validation`] /
/// [`SceneValidationCounters::record_lint_warning`].
#[must_use]
pub fn validation_counters() -> &'static SceneValidationCounters {
    &COUNTERS
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod observability_emission {
    //! Evidence: EV-OBSERVABILITY (T1) — scene emission site against docs/31 §Phase-1 baseline catalog (validations, lint_warnings).
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins the
    //! validator's per-exit `record_validation` call and its
    //! `CoercionApplied`-push `record_lint_warning` call against the
    //! module-level `LazyLock<SceneValidationCounters>` emission
    //! target. Any drift in the validator wiring (a return path that
    //! forgets to record, or a warnings.push site that forgets to
    //! emit) fails the corresponding pin.
    //!
    //! Counters are per-process; tests assert on deltas read before
    //! and after each `validate` call rather than on absolute values,
    //! so parallel execution of sibling `#[test]` functions in the
    //! same crate does not cause spurious failures on the shared
    //! counter state. Each pin asserts the delta is at least the
    //! call count invoked from the test — other in-flight tests can
    //! only increase the observed delta, never decrease it.

    use chromaticity_observability::catalog::{SceneLintCategory, SceneValidationOutcome};

    use super::validation_counters;
    use crate::schema::{ControlDecl, ControlValue, NodeRef, PassSpec, Scene};
    use crate::validator::{ValidationMode, validate};

    fn ok_scene() -> Scene {
        Scene::default()
    }

    fn fail_scene_via_duplicate_node() -> Scene {
        let mut scene = Scene::default();
        for _ in 0..2 {
            scene.passes.push(PassSpec {
                node: NodeRef("n0".to_string()),
                uniform_bindings: Vec::new(),
                extensions: serde_json::Map::new(),
            });
        }
        scene
    }

    fn coercion_scene() -> Scene {
        let mut scene = Scene::default();
        scene.controls.push(ControlDecl {
            name: "brightness".to_string(),
            kind: None,
            default: ControlValue::Float(0.5),
            min: Some(0.0.into()),
            max: Some(1.0.into()),
            ..ControlDecl::default()
        });
        scene
    }

    #[test]
    fn validate_ok_increments_pass_outcome() {
        let counters = validation_counters();
        let before = counters.validation_count(SceneValidationOutcome::Pass);
        let _ = validate(&ok_scene(), ValidationMode::Compat)
            .expect("default scene validates under compat");
        let after = counters.validation_count(SceneValidationOutcome::Pass);
        assert!(
            after.saturating_sub(before) >= 1,
            "Pass counter did not advance on Ok(_) exit: before={before} after={after}",
        );
    }

    #[test]
    fn validate_err_increments_fail_outcome() {
        let counters = validation_counters();
        let before = counters.validation_count(SceneValidationOutcome::Fail);
        let err = validate(&fail_scene_via_duplicate_node(), ValidationMode::Strict)
            .expect_err("duplicate NodeRef must reject in strict mode");
        drop(err);
        let after = counters.validation_count(SceneValidationOutcome::Fail);
        assert!(
            after.saturating_sub(before) >= 1,
            "Fail counter did not advance on Err(_) exit: before={before} after={after}",
        );
    }

    #[test]
    fn coercion_applied_warning_increments_category_counter() {
        let counters = validation_counters();
        let before = counters.lint_warning_count(SceneLintCategory::CoercionApplied);
        let warnings = validate(&coercion_scene(), ValidationMode::Compat)
            .expect("coercion scene validates under compat with warnings");
        assert!(
            !warnings.is_empty(),
            "coercion scene must emit at least one Warning::CoercionApplied",
        );
        let after = counters.lint_warning_count(SceneLintCategory::CoercionApplied);
        assert!(
            after.saturating_sub(before) >= 1,
            "CoercionApplied counter did not advance on warnings.push: before={before} after={after}",
        );
    }

    #[test]
    fn counters_survive_multiple_validate_calls() {
        let counters = validation_counters();
        let before = counters.validation_count(SceneValidationOutcome::Pass);
        for _ in 0..3 {
            let _ = validate(&ok_scene(), ValidationMode::Compat)
                .expect("default scene validates under compat");
        }
        let after = counters.validation_count(SceneValidationOutcome::Pass);
        assert!(
            after.saturating_sub(before) >= 3,
            "Pass counter did not accumulate across 3 validate calls: before={before} after={after}",
        );
    }
}

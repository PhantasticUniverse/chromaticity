// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Authoring + embedded-docs UX latency measurement harness.
//!
//! Co-developed surface satisfying both
//! [`EV-AUTHORING-UX-LATENCY`] (`docs/26-evidence-artifact-registry.md`
//! row 83) and [`EV-DOCS-UX-LATENCY`] (row 84) per AO direction
//! "co-develop the harness once and use it for both rows". The two
//! rows share the §3.3a budget table verbatim — same hover-docs (80
//! ms p95), symbol-search / doc-search (150 ms p95), and
//! first-paint-on-open (300 ms p95) targets — so a single harness
//! is the right shape; the row 83 surface is the authoring view
//! (hover-docs over GLSL identifiers + symbol-search across scene
//! controls + first-paint when opening a scene), and the row 84
//! surface is the embedded-docs view (hover-docs over linked
//! reference entries + full-text doc-search + first-paint when
//! navigating to a doc page). Both surfaces feed samples through
//! this harness; both surfaces assert against the same budgets.
//!
//! # Surface
//!
//! - [`LatencyMeasurement`] — a single observation: `(operation,
//!   observed_ms)`.
//! - [`LatencyBudget`] — a static p95 budget declaration:
//!   `(operation, p95_budget_ms)`.
//! - [`measure_p95`] — nearest-rank p95 over a sample slice.
//! - [`assert_p95_within_budget`] — comparator returning the
//!   computed p95 on success or [`BudgetExceeded`] on regression.
//! - [`AUTHORING_BUDGETS`] / [`DOCS_BUDGETS`] — the §3.3a target
//!   table, statically declared once per row. Both arrays carry
//!   the same numeric budgets per the co-development direction;
//!   the operation labels differ to keep the row 83 / row 84
//!   call-sites self-documenting.
//!
//! # Synthetic posture
//!
//! This crate is the producer-side harness — it does not drive a
//! real hover or search through the authoring or docs UI. Slice 6
//! (this PR) ships the measurement surface plus the per-operation
//! budget constants plus the T1 + T2 corpora that validate the
//! harness logic against synthetic timing inputs. The full
//! UI-driven validation (running the harness against real
//! authoring-view or embedded-docs-view rendering) is post-MVP
//! integration work and will land alongside the §3.3a paint-ack
//! plumbing called out at `docs/37-authoring-environment-spec.md`
//! lines 47-58.
//!
//! # p95 algorithm
//!
//! The harness uses the **nearest-rank** percentile: sort the
//! samples ascending, then index at
//! `((p / 100) * n).ceil() - 1`, clamped to `[0, n - 1]`. This
//! matches `docs/37` §3.3a's "p95 <= X ms" framing (the value at
//! the 95th percentile rank) and is the same convention SLO-02
//! uses in `docs/10-performance-latency-budgets.md`. A
//! distribution-free estimator was chosen over linear
//! interpolation between bracketing samples because the §3.3a
//! population (500 events per metric per platform per RC build)
//! is large enough that nearest-rank is well-defined and the lack
//! of interpolation makes the harness reproducible across
//! platforms with different floating-point rounding.
//!
//! # Empty-sample contract
//!
//! [`measure_p95`] returns `0.0` on an empty slice rather than
//! panicking. The §3.3a population is always non-empty in
//! production (the harness asserts after collecting the 500-event
//! batch), but downstream code paths that want to treat
//! "no observations yet" as "trivially within budget" benefit
//! from a total function. The T1 corpus pins this contract so a
//! later refactor doesn't silently flip it to a panic.
//!
//! # docs/37 §3.3a traceability
//!
//! Both [`AUTHORING_BUDGETS`] and [`DOCS_BUDGETS`] declare
//! exactly the three p95 targets from §3.3a:
//!
//! | Operation       | p95 budget |
//! |-----------------|------------|
//! | hover-docs      | 80 ms      |
//! | symbol-search   | 150 ms     |
//! | first-paint     | 300 ms     |
//!
//! The p99 tails-discipline targets (120 ms / 220 ms / 450 ms)
//! are not yet wired here — the row 83 / 84 acceptance bullets
//! cite p95 only, and SLO-02 tails discipline is a separate
//! pass-through gate. p99 plumbing slots in additively when the
//! tails-discipline harness lands.

use core::fmt;

/// A single latency observation produced by an instrumentation
/// site (e.g. authoring-view hover handler, docs-view search
/// keystroke handler).
///
/// The harness is intentionally minimal: a string label plus an
/// elapsed-millisecond floating-point value. Higher-level
/// metadata (platform, scene id, RC build) lives at the call-site
/// batch boundary, not on the per-observation struct.
#[derive(Debug, Clone, PartialEq)]
pub struct LatencyMeasurement {
    /// Operation label — matches a [`LatencyBudget::operation`]
    /// in [`AUTHORING_BUDGETS`] or [`DOCS_BUDGETS`] when the
    /// observation is being asserted against a budget.
    pub operation: String,
    /// Observed elapsed time in milliseconds (event timestamp ->
    /// paint-completion timestamp per `docs/37` §3.3a).
    pub observed_ms: f64,
}

/// A p95 budget declaration for a named operation.
///
/// Used to populate the static [`AUTHORING_BUDGETS`] /
/// [`DOCS_BUDGETS`] tables. Per `docs/37` §3.3a, the budgets are
/// numeric constants in the spec and not author-configurable;
/// `&'static str` for the label keeps the table allocation-free
/// at module load.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatencyBudget {
    /// Operation label (e.g. `"hover-docs"`, `"symbol-search"`,
    /// `"first-paint"`).
    pub operation: &'static str,
    /// p95 budget in milliseconds. A sample population whose p95
    /// exceeds this value fails the §3.3a acceptance bullet for
    /// the matching row.
    pub p95_budget_ms: f64,
}

/// Error returned by [`assert_p95_within_budget`] when the
/// observed p95 exceeds the declared budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetExceeded {
    /// The computed p95 in milliseconds.
    pub p95: f64,
    /// The budget the p95 was compared against.
    pub budget: f64,
}

impl fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "observed p95 {:.3} ms exceeds budget {:.3} ms",
            self.p95, self.budget
        )
    }
}

impl std::error::Error for BudgetExceeded {}

/// Compute the nearest-rank p95 of a sample slice in milliseconds.
///
/// Returns `0.0` for an empty slice (see module-level
/// "Empty-sample contract"). The input slice is not mutated; the
/// implementation clones into a local `Vec<f64>` and sorts that.
///
/// # Algorithm
///
/// Nearest-rank: sort ascending, take the value at index
/// `((0.95 * n).ceil() - 1).clamp(0, n - 1)`. NaN samples are
/// sorted to the high end via [`f64::total_cmp`]; the harness
/// caller is responsible for ensuring the sample population is
/// finite (the §3.3a measurement source is wall-clock event
/// arithmetic that cannot produce NaN under the documented
/// timestamp sources, but the comparator is total either way).
pub fn measure_p95(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<f64> = samples.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    // nearest-rank: ceil(0.95 * n) gives a 1-based rank in
    // [1, n]; subtract 1 for the 0-based index.
    let n = sorted.len();
    #[allow(clippy::cast_precision_loss)]
    let rank_1based = (0.95_f64 * n as f64).ceil() as usize;
    let rank_1based = rank_1based.max(1);
    let idx = (rank_1based - 1).min(n - 1);
    // Index is clamped to [0, n-1] above; direct index is safe.
    sorted[idx]
}

/// Assert that the p95 of `samples` is within `budget_ms`.
///
/// Returns `Ok(p95)` when the computed p95 is `<= budget_ms`,
/// otherwise `Err(BudgetExceeded { p95, budget })`. The
/// non-strict `<=` matches `docs/37` §3.3a's "p95 <= 80 ms"
/// framing — a sample exactly at the budget passes.
///
/// Use against [`AUTHORING_BUDGETS`] / [`DOCS_BUDGETS`] entries
/// at the call-site batch boundary; the per-observation struct
/// [`LatencyMeasurement`] is not consumed here directly so callers
/// can collect into a `Vec<f64>` of `observed_ms` values without
/// allocating intermediate `LatencyMeasurement` instances when
/// pushing through the assertion seam.
///
/// # Errors
///
/// Returns [`BudgetExceeded`] when the observed p95 exceeds
/// `budget_ms`. The error carries both the observed p95 and the
/// budget so a CI report can render the regression delta without
/// re-running the harness.
pub fn assert_p95_within_budget(samples: &[f64], budget_ms: f64) -> Result<f64, BudgetExceeded> {
    let p95 = measure_p95(samples);
    if p95 <= budget_ms {
        Ok(p95)
    } else {
        Err(BudgetExceeded {
            p95,
            budget: budget_ms,
        })
    }
}

/// Authoring-view (`docs/26` row 83) p95 budget table per
/// `docs/37` §3.3a.
///
/// The label vocabulary mirrors the spec's metric names:
/// `"hover-docs"` covers GLSL-identifier hover lookups in the
/// authoring scene editor; `"symbol-search"` covers
/// scene-control / uniform / function symbol search;
/// `"first-paint"` covers first-paint when opening a scene from
/// the library.
pub static AUTHORING_BUDGETS: &[LatencyBudget] = &[
    LatencyBudget {
        operation: "hover-docs",
        p95_budget_ms: 80.0,
    },
    LatencyBudget {
        operation: "symbol-search",
        p95_budget_ms: 150.0,
    },
    LatencyBudget {
        operation: "first-paint",
        p95_budget_ms: 300.0,
    },
];

/// Embedded-docs-view (`docs/26` row 84) p95 budget table per
/// `docs/37` §3.3a.
///
/// Same numeric budgets as [`AUTHORING_BUDGETS`] per the
/// co-development direction. Label vocabulary matches the
/// docs-view surface: `"hover-docs"` covers tooltip first-paint
/// over linked reference entries; `"symbol-search"` covers
/// full-text doc-search; `"first-paint"` covers doc-page
/// first-paint when clicking a sidebar entry.
pub static DOCS_BUDGETS: &[LatencyBudget] = &[
    LatencyBudget {
        operation: "hover-docs",
        p95_budget_ms: 80.0,
    },
    LatencyBudget {
        operation: "symbol-search",
        p95_budget_ms: 150.0,
    },
    LatencyBudget {
        operation: "first-paint",
        p95_budget_ms: 300.0,
    },
];

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-AUTHORING-UX-LATENCY (T1) + EV-DOCS-UX-LATENCY (T1) —
    //! co-developed §3.3a measurement harness in-source pins per AO
    //! direction. T1 per `docs/43-development-standards.md` §15.5:
    //! exercises the producer-side measurement surface against
    //! synthetic timing inputs without driving any real UI; the T2
    //! corpus in `engine/authoring/tests/ux_latency_corpus.rs`
    //! sweeps pass-and-fail boundary fixtures across both rows.

    use super::*;

    #[test]
    fn measure_p95_with_constant_samples_returns_constant() {
        // 100 identical samples — every percentile is the constant.
        let samples = vec![42.0_f64; 100];
        let p95 = measure_p95(&samples);
        assert!((p95 - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn measure_p95_with_ascending_samples_picks_correct_index() {
        // 100 samples 1..=100; nearest-rank p95 lands at index
        // ceil(0.95 * 100) - 1 = 94, value 95.
        let samples: Vec<f64> = (1..=100).map(|n| n as f64).collect();
        let p95 = measure_p95(&samples);
        assert!((p95 - 95.0).abs() < f64::EPSILON);
    }

    #[test]
    fn measure_p95_with_descending_samples_sorts_then_picks() {
        // Same population reversed must pick the same value.
        let samples: Vec<f64> = (1..=100).rev().map(|n| n as f64).collect();
        let p95 = measure_p95(&samples);
        assert!((p95 - 95.0).abs() < f64::EPSILON);
    }

    #[test]
    fn measure_p95_with_empty_samples_returns_zero() {
        // Empty-sample contract pinned: returns 0.0, not a panic.
        let samples: Vec<f64> = Vec::new();
        let p95 = measure_p95(&samples);
        assert!((p95 - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn measure_p95_with_single_sample_returns_that_sample() {
        // n = 1: ceil(0.95) = 1, idx 0 — the only sample.
        let samples = vec![17.5];
        let p95 = measure_p95(&samples);
        assert!((p95 - 17.5).abs() < f64::EPSILON);
    }

    #[test]
    fn assert_p95_within_budget_passes_when_under() {
        // Synthetic hover-docs run: 60..=78 ms, p95 = 78 < 80.
        let samples: Vec<f64> = (60..=78).map(|n| n as f64).collect();
        let result = assert_p95_within_budget(&samples, 80.0);
        let p95 = result.expect("under-budget samples must pass");
        assert!(p95 <= 80.0);
    }

    #[test]
    fn assert_p95_within_budget_passes_at_exact_boundary() {
        // p95 == budget_ms passes per the non-strict <= contract.
        let samples = vec![80.0_f64; 100];
        let result = assert_p95_within_budget(&samples, 80.0);
        assert!(result.is_ok());
    }

    #[test]
    fn assert_p95_within_budget_fails_when_over() {
        // Synthetic over-budget hover-docs run: p95 lands at 90.
        let samples: Vec<f64> = (1..=100).map(|n| n as f64).collect();
        let result = assert_p95_within_budget(&samples, 80.0);
        let err = result.expect_err("over-budget samples must fail");
        assert!(err.p95 > 80.0);
        assert!((err.budget - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn budget_exceeded_error_carries_observed_and_budget_values() {
        // Display + field round-trip; the CI surface needs both
        // values to render a regression delta.
        let samples: Vec<f64> = (200..=210).map(|n| n as f64).collect();
        let err = assert_p95_within_budget(&samples, 80.0)
            .expect_err("populations clearly over budget must fail");
        assert!((err.budget - 80.0).abs() < f64::EPSILON);
        assert!(err.p95 >= 200.0);
        let rendered = format!("{err}");
        assert!(rendered.contains("exceeds budget"));
    }

    #[test]
    fn authoring_budgets_match_docs_37_section_3_3a() {
        // §3.3a normative table: hover-docs 80, symbol-search
        // 150, first-paint 300. Pinning the constants prevents a
        // future drive-by edit from drifting away from the spec.
        assert_eq!(AUTHORING_BUDGETS.len(), 3);
        let hover = AUTHORING_BUDGETS
            .iter()
            .find(|b| b.operation == "hover-docs")
            .expect("authoring budgets must declare hover-docs");
        assert!((hover.p95_budget_ms - 80.0).abs() < f64::EPSILON);
        let search = AUTHORING_BUDGETS
            .iter()
            .find(|b| b.operation == "symbol-search")
            .expect("authoring budgets must declare symbol-search");
        assert!((search.p95_budget_ms - 150.0).abs() < f64::EPSILON);
        let paint = AUTHORING_BUDGETS
            .iter()
            .find(|b| b.operation == "first-paint")
            .expect("authoring budgets must declare first-paint");
        assert!((paint.p95_budget_ms - 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn docs_budgets_match_authoring_budgets() {
        // Co-development direction: same budgets across both rows.
        // Numeric equality is the load-bearing invariant; the
        // operation labels are the same vocabulary too.
        assert_eq!(AUTHORING_BUDGETS.len(), DOCS_BUDGETS.len());
        for (a, d) in AUTHORING_BUDGETS.iter().zip(DOCS_BUDGETS.iter()) {
            assert_eq!(a.operation, d.operation);
            assert!((a.p95_budget_ms - d.p95_budget_ms).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn latency_measurement_round_trips_label_and_value() {
        // The struct is intentionally minimal — pinning the
        // shape prevents an accidental field rename.
        let m = LatencyMeasurement {
            operation: "hover-docs".to_string(),
            observed_ms: 72.5,
        };
        assert_eq!(m.operation, "hover-docs");
        assert!((m.observed_ms - 72.5).abs() < f64::EPSILON);
    }

    #[test]
    fn measure_p95_handles_unsorted_floating_point_samples() {
        // Floating-point sample population, deliberately
        // unsorted, with a wide dynamic range. Sort + index must
        // still land on the 95th-percentile value.
        let samples = vec![
            12.3, 4.7, 78.1, 33.9, 2.0, 99.4, 50.0, 18.6, 65.2, 41.8, 7.7, 88.3, 22.1, 56.5, 91.0,
            8.4, 73.0, 27.7, 60.6, 35.5,
        ];
        // n = 20, ceil(0.95 * 20) = 19, idx 18 — the second
        // largest after sorting (91.0).
        let p95 = measure_p95(&samples);
        assert!((p95 - 91.0).abs() < f64::EPSILON);
    }
}

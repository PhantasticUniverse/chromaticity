// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4b Wave 4b.6 co-developed UX latency corpus. Tier
//! declaration and evidence tag live on the `fixtures` inner module
//! below, matching the pattern established by
//! `engine/authoring/tests/scaffolder_corpus.rs` so
//! `scripts/evidence-tag-lint.sh`'s tier-distribution summary
//! counts this file at T2.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-AUTHORING-UX-LATENCY (T2) + EV-DOCS-UX-LATENCY (T2) —
    //! Phase-4b Wave 4b.6 co-developed §3.3a UX latency boundary corpus.
    //!
    //! T2 per `docs/43-development-standards.md` §15.5: exercises
    //! the evidence-producing behaviour against a fixture corpus.
    //! Each fixture (a) constructs a synthetic per-operation
    //! sample population sized to land its p95 at a known
    //! pass-or-fail boundary against the §3.3a budget, (b) drives
    //! [`assert_p95_within_budget`] (and cross-checks
    //! [`measure_p95`]), and (c) asserts the comparator outcome
    //! matches the fixture's declared expectation.
    //!
    //! The harness is synthetic per the slice 6 module-level
    //! contract — the corpus does not drive any real authoring or
    //! docs UI. The full UI-driven validation (running the harness
    //! against real authoring-view or embedded-docs-view rendering)
    //! is post-MVP integration work and lands alongside the §3.3a
    //! paint-ack plumbing called out at
    //! `docs/37-authoring-environment-spec.md` lines 47-58.
    //!
    //! Fixture coverage matrix (3 per row × 2 rows + 1 cross-row
    //! consistency fixture):
    //!
    //! 1. authoring hover-docs at 75 ms p95 -> Pass against 80 ms.
    //! 2. authoring hover-docs at 85 ms p95 -> Fail against 80 ms.
    //! 3. authoring symbol-search at 140 ms p95 -> Pass against 150 ms.
    //! 4. authoring symbol-search at 160 ms p95 -> Fail against 150 ms.
    //! 5. authoring first-paint at 290 ms p95 -> Pass against 300 ms.
    //! 6. authoring first-paint at 310 ms p95 -> Fail against 300 ms.
    //! 7. docs hover-docs at 75 ms p95 -> Pass against 80 ms.
    //! 8. docs hover-docs at 85 ms p95 -> Fail against 80 ms.
    //! 9. docs symbol-search at 140 ms p95 -> Pass against 150 ms.
    //! 10. docs symbol-search at 160 ms p95 -> Fail against 150 ms.
    //! 11. docs first-paint at 290 ms p95 -> Pass against 300 ms.
    //! 12. docs first-paint at 310 ms p95 -> Fail against 300 ms.
    //! 13. cross-row budget identity — each authoring budget
    //!     numerically matches its docs-row peer per the AO
    //!     co-development direction.

    use chromaticity_authoring::ux_latency::{
        AUTHORING_BUDGETS, DOCS_BUDGETS, LatencyBudget, assert_p95_within_budget, measure_p95,
    };

    /// Build a synthetic 100-sample population whose nearest-rank
    /// p95 lands exactly on `target_p95_ms`. The 100-sample shape
    /// matches the §3.3a measurement-population description (n =
    /// 500 in production; 100 here is enough to exercise the
    /// harness with a well-defined boundary). With n = 100, the
    /// p95 index is `ceil(0.95 * 100) - 1 = 94`, so we set
    /// `samples[94] = target_p95_ms` and pack the surrounding
    /// indices below + above.
    fn synthetic_p95_at(target_p95_ms: f64) -> Vec<f64> {
        let mut samples = Vec::with_capacity(100);
        // Indices 0..94 (95 samples) at half the target — well
        // under the p95 rank.
        for _ in 0..95 {
            samples.push(target_p95_ms * 0.5);
        }
        // Index 94 — wait, the loop above is already 95 entries
        // (0..95 = 95 iterations). After sorting, the values at
        // indices 0..=94 will be the 95 lower samples. We want
        // index 94 (the 95th value) to be exactly target. Adjust:
        // emit 94 below-target samples plus 1 at-target plus 5
        // above-target so post-sort the 95th value is target.
        samples.clear();
        for _ in 0..94 {
            samples.push(target_p95_ms * 0.5);
        }
        samples.push(target_p95_ms);
        for _ in 0..5 {
            samples.push(target_p95_ms * 1.5);
        }
        debug_assert_eq!(samples.len(), 100);
        samples
    }

    fn lookup_budget(table: &'static [LatencyBudget], op: &str) -> LatencyBudget {
        table
            .iter()
            .copied()
            .find(|b| b.operation == op)
            .unwrap_or_else(|| panic!("budget table missing operation `{op}`"))
    }

    // -------------------------------------------------------------
    // Authoring (row 83) fixtures — hover-docs / symbol-search / first-paint
    // pass-and-fail boundary cases.
    // -------------------------------------------------------------

    #[test]
    fn authoring_hover_docs_at_75ms_passes_80ms_budget() {
        let budget = lookup_budget(AUTHORING_BUDGETS, "hover-docs");
        let samples = synthetic_p95_at(75.0);
        let p95 = measure_p95(&samples);
        assert!((p95 - 75.0).abs() < f64::EPSILON);
        let result = assert_p95_within_budget(&samples, budget.p95_budget_ms);
        assert!(
            result.is_ok(),
            "75 ms hover-docs p95 must pass the 80 ms authoring budget"
        );
    }

    #[test]
    fn authoring_hover_docs_at_85ms_fails_80ms_budget() {
        let budget = lookup_budget(AUTHORING_BUDGETS, "hover-docs");
        let samples = synthetic_p95_at(85.0);
        let p95 = measure_p95(&samples);
        assert!((p95 - 85.0).abs() < f64::EPSILON);
        let err = assert_p95_within_budget(&samples, budget.p95_budget_ms)
            .expect_err("85 ms hover-docs p95 must fail the 80 ms authoring budget");
        assert!((err.p95 - 85.0).abs() < f64::EPSILON);
        assert!((err.budget - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn authoring_symbol_search_at_140ms_passes_150ms_budget() {
        let budget = lookup_budget(AUTHORING_BUDGETS, "symbol-search");
        let samples = synthetic_p95_at(140.0);
        let result = assert_p95_within_budget(&samples, budget.p95_budget_ms);
        let p95 = result.expect("140 ms symbol-search p95 must pass the 150 ms authoring budget");
        assert!((p95 - 140.0).abs() < f64::EPSILON);
    }

    #[test]
    fn authoring_symbol_search_at_160ms_fails_150ms_budget() {
        let budget = lookup_budget(AUTHORING_BUDGETS, "symbol-search");
        let samples = synthetic_p95_at(160.0);
        let err = assert_p95_within_budget(&samples, budget.p95_budget_ms)
            .expect_err("160 ms symbol-search p95 must fail the 150 ms authoring budget");
        assert!((err.p95 - 160.0).abs() < f64::EPSILON);
        assert!((err.budget - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn authoring_first_paint_at_290ms_passes_300ms_budget() {
        let budget = lookup_budget(AUTHORING_BUDGETS, "first-paint");
        let samples = synthetic_p95_at(290.0);
        let result = assert_p95_within_budget(&samples, budget.p95_budget_ms);
        let p95 = result.expect("290 ms first-paint p95 must pass the 300 ms authoring budget");
        assert!((p95 - 290.0).abs() < f64::EPSILON);
    }

    #[test]
    fn authoring_first_paint_at_310ms_fails_300ms_budget() {
        let budget = lookup_budget(AUTHORING_BUDGETS, "first-paint");
        let samples = synthetic_p95_at(310.0);
        let err = assert_p95_within_budget(&samples, budget.p95_budget_ms)
            .expect_err("310 ms first-paint p95 must fail the 300 ms authoring budget");
        assert!((err.p95 - 310.0).abs() < f64::EPSILON);
        assert!((err.budget - 300.0).abs() < f64::EPSILON);
    }

    // -------------------------------------------------------------
    // Embedded-docs (row 84) fixtures — same operations, same
    // budgets, asserted through the `DOCS_BUDGETS` table to keep
    // the row 84 call-sites self-documenting.
    // -------------------------------------------------------------

    #[test]
    fn docs_hover_docs_at_75ms_passes_80ms_budget() {
        let budget = lookup_budget(DOCS_BUDGETS, "hover-docs");
        let samples = synthetic_p95_at(75.0);
        let result = assert_p95_within_budget(&samples, budget.p95_budget_ms);
        assert!(
            result.is_ok(),
            "75 ms hover-docs p95 must pass the 80 ms docs budget"
        );
    }

    #[test]
    fn docs_hover_docs_at_85ms_fails_80ms_budget() {
        let budget = lookup_budget(DOCS_BUDGETS, "hover-docs");
        let samples = synthetic_p95_at(85.0);
        let err = assert_p95_within_budget(&samples, budget.p95_budget_ms)
            .expect_err("85 ms hover-docs p95 must fail the 80 ms docs budget");
        assert!((err.p95 - 85.0).abs() < f64::EPSILON);
        assert!((err.budget - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn docs_symbol_search_at_140ms_passes_150ms_budget() {
        let budget = lookup_budget(DOCS_BUDGETS, "symbol-search");
        let samples = synthetic_p95_at(140.0);
        let result = assert_p95_within_budget(&samples, budget.p95_budget_ms);
        let p95 = result.expect("140 ms symbol-search p95 must pass the 150 ms docs budget");
        assert!((p95 - 140.0).abs() < f64::EPSILON);
    }

    #[test]
    fn docs_symbol_search_at_160ms_fails_150ms_budget() {
        let budget = lookup_budget(DOCS_BUDGETS, "symbol-search");
        let samples = synthetic_p95_at(160.0);
        let err = assert_p95_within_budget(&samples, budget.p95_budget_ms)
            .expect_err("160 ms symbol-search p95 must fail the 150 ms docs budget");
        assert!((err.p95 - 160.0).abs() < f64::EPSILON);
        assert!((err.budget - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn docs_first_paint_at_290ms_passes_300ms_budget() {
        let budget = lookup_budget(DOCS_BUDGETS, "first-paint");
        let samples = synthetic_p95_at(290.0);
        let result = assert_p95_within_budget(&samples, budget.p95_budget_ms);
        let p95 = result.expect("290 ms first-paint p95 must pass the 300 ms docs budget");
        assert!((p95 - 290.0).abs() < f64::EPSILON);
    }

    #[test]
    fn docs_first_paint_at_310ms_fails_300ms_budget() {
        let budget = lookup_budget(DOCS_BUDGETS, "first-paint");
        let samples = synthetic_p95_at(310.0);
        let err = assert_p95_within_budget(&samples, budget.p95_budget_ms)
            .expect_err("310 ms first-paint p95 must fail the 300 ms docs budget");
        assert!((err.p95 - 310.0).abs() < f64::EPSILON);
        assert!((err.budget - 300.0).abs() < f64::EPSILON);
    }

    // -------------------------------------------------------------
    // Cross-row consistency fixture — pin the AO-direction
    // co-development invariant at corpus level so a future drift
    // between the two budget tables is caught here, not just at
    // the T1 in-source pin.
    // -------------------------------------------------------------

    #[test]
    fn authoring_and_docs_budgets_share_numeric_table() {
        assert_eq!(AUTHORING_BUDGETS.len(), DOCS_BUDGETS.len());
        for (a, d) in AUTHORING_BUDGETS.iter().zip(DOCS_BUDGETS.iter()) {
            assert_eq!(
                a.operation, d.operation,
                "operation labels must match across rows"
            );
            assert!(
                (a.p95_budget_ms - d.p95_budget_ms).abs() < f64::EPSILON,
                "p95 budgets must match across rows for {}",
                a.operation
            );
        }
    }
}

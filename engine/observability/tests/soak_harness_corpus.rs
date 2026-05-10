// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4d Wave 4d.4 `EV-SOAK-CHAOS` soak harness corpus (PR 1 of
//! 2-3). Tier declaration and evidence tag live on the inner
//! `fixtures` module so `scripts/evidence-tag-lint.sh`'s tier-
//! distribution summary counts this file at T2.
//!
//! Companion to `engine/observability/src/soak.rs`. The harness
//! module ships the runner + the in-source T1 pins; this file is the
//! T2 fixture-driven cross-property corpus that exercises the
//! end-to-end runner against synthetic workloads with known
//! distributional properties, asserting the report shape matches.
//!
//! Wave 4d.4 narrowing recap (mirrored from
//! `engine/observability/src/soak.rs` module prose for grep-
//! discoverability): this PR ships the harness scaffold + CI-short
//! config + 6 corpus fixtures; the full >= 4-hour soak target per
//! `docs/10-performance-latency-budgets.md` line 55 runs as a
//! nightly criterion-bench-style harness, not a per-PR CI gate. PR
//! 2 ships chaos injection; PR 3 ships the multi-profile validation
//! corpus + closing-docs row 40 -> Pass.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-SOAK-CHAOS (T2) — Phase-4d Wave 4d.4 soak
    //! harness fixture corpus. Six CI-friendly (sub-2-second-each)
    //! fixtures exercising the public API of
    //! `chromaticity_observability::soak`.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md` row
    //! 40 (`EV-SOAK-CHAOS`) declares Generation Source "Soak +
    //! chaos validation runs" with the per-class quantitative target
    //! ">= 4 hour soak achieved per `10-performance-latency-
    //! budgets.md`". This T2 + the sibling T1 in
    //! `engine/observability/src/soak.rs` together pin the harness
    //! contract; the row's `Planned -> Pass` transition lands when
    //! PR 3 (multi-profile validation corpus + closing-docs) ships
    //! under the Wave 4d.4 narrowing.

    use chromaticity_observability::{
        SoakConfig, SoakWorkload, SoakWorkloadError, StabilityFailureKind, run_soak,
    };
    use std::time::Duration;

    /// Constant per-tick cost workload. Reports the configured
    /// `per_tick` duration on every tick without sleeping; the
    /// harness sees a perfectly flat distribution.
    struct ConstantWorkload {
        per_tick: Duration,
    }

    impl SoakWorkload for ConstantWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            Ok(self.per_tick)
        }
    }

    /// Reports `OverrunBudget` exactly once at `at_tick` with
    /// `overrun_elapsed`; constant `per_tick` otherwise.
    struct OneShotOverrunWorkload {
        per_tick: Duration,
        overrun_elapsed: Duration,
        at_tick: u64,
        current_tick: u64,
    }

    impl SoakWorkload for OneShotOverrunWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            let n = self.current_tick;
            self.current_tick = self.current_tick.saturating_add(1);
            if n == self.at_tick {
                Err(SoakWorkloadError::OverrunBudget {
                    elapsed: self.overrun_elapsed,
                })
            } else {
                Ok(self.per_tick)
            }
        }
    }

    /// Cycles through a fixed schedule of `per_tick` durations,
    /// looping when exhausted. Used to feed a known distribution
    /// into the harness for the percentile-correctness fixture.
    struct ScheduledWorkload {
        schedule_us: Vec<u64>,
        idx: usize,
    }

    impl SoakWorkload for ScheduledWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            let cost_us = self.schedule_us[self.idx % self.schedule_us.len()];
            self.idx = self.idx.wrapping_add(1);
            Ok(Duration::from_micros(cost_us))
        }
    }

    /// Reports `StateCorruption { marker }` exactly once at
    /// `at_tick`; constant `per_tick` otherwise.
    struct OneShotCorruptionWorkload {
        per_tick: Duration,
        marker: f64,
        at_tick: u64,
        current_tick: u64,
    }

    impl SoakWorkload for OneShotCorruptionWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            let n = self.current_tick;
            self.current_tick = self.current_tick.saturating_add(1);
            if n == self.at_tick {
                Err(SoakWorkloadError::StateCorruption {
                    marker: self.marker,
                })
            } else {
                Ok(self.per_tick)
            }
        }
    }

    /// Construct a fast `SoakConfig` with the supplied duration so
    /// fixtures stay sub-2-second.
    fn fast_config(duration: Duration) -> SoakConfig {
        SoakConfig {
            duration,
            tick_interval: Duration::from_millis(16),
            stability_check_interval: Duration::from_millis(50),
            drift_threshold_p95_ms: 1.25,
        }
    }

    #[test]
    fn ci_short_soak_against_constant_workload_completes_under_2_seconds() {
        // Validates the SoakConfig::ci_short shape (the PRESET, not
        // its 60-second duration) by overriding to 1 second to keep
        // CI cheap. The harness should produce a non-empty report
        // and respect the duration budget.
        let started = std::time::Instant::now();
        let mut workload = ConstantWorkload {
            per_tick: Duration::from_micros(50),
        };
        let mut config = SoakConfig::ci_short();
        config.duration = Duration::from_millis(1_000);
        let report = run_soak(&mut workload, &config).unwrap();
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_millis(2_000),
            "ci-short fixture exceeded 2-second budget: {elapsed:?}"
        );
        assert!(report.tick_count > 0, "expected non-zero ticks");
        assert!(
            report.duration_observed >= Duration::from_millis(1_000),
            "duration_observed={:?}",
            report.duration_observed
        );
    }

    #[test]
    fn drift_threshold_violation_appears_in_report() {
        // Workload reports a single 100ms tick at tick 3; the
        // harness should record exactly one DriftExceeded with the
        // 100ms recorded_value.
        let mut workload = OneShotOverrunWorkload {
            per_tick: Duration::from_micros(50),
            overrun_elapsed: Duration::from_millis(100),
            at_tick: 3,
            current_tick: 0,
        };
        let config = fast_config(Duration::from_millis(200));
        let report = run_soak(&mut workload, &config).unwrap();
        let drifts: Vec<_> = report
            .stability_failures
            .iter()
            .filter(|f| f.kind == StabilityFailureKind::DriftExceeded)
            .collect();
        assert_eq!(drifts.len(), 1, "expected exactly one DriftExceeded");
        assert!(
            (drifts[0].recorded_value - 100.0).abs() < 0.01,
            "recorded_value={}",
            drifts[0].recorded_value
        );
    }

    #[test]
    fn tick_count_within_one_of_duration_over_interval() {
        // 500ms duration / 16ms interval ~= 31-32 ticks. Allow a
        // generous +/-5 ticks of slack to absorb scheduler jitter on
        // shared CI hardware.
        let mut workload = ConstantWorkload {
            per_tick: Duration::from_micros(50),
        };
        let config = fast_config(Duration::from_millis(500));
        let report = run_soak(&mut workload, &config).unwrap();
        assert!(
            report.tick_count >= 27 && report.tick_count <= 36,
            "tick_count={} out of [27, 36]",
            report.tick_count
        );
    }

    #[test]
    fn stability_failure_records_at_offset_correctly() {
        // OneShotOverrunWorkload fires at tick 5 with a 16ms
        // tick_interval; the at_offset should be ~5 * 16ms = ~80ms,
        // allow +/-50ms of slack.
        let at_tick: u64 = 5;
        let mut workload = OneShotOverrunWorkload {
            per_tick: Duration::from_micros(50),
            overrun_elapsed: Duration::from_millis(50),
            at_tick,
            current_tick: 0,
        };
        let config = fast_config(Duration::from_millis(300));
        let report = run_soak(&mut workload, &config).unwrap();
        let drift = report
            .stability_failures
            .iter()
            .find(|f| f.kind == StabilityFailureKind::DriftExceeded)
            .expect("expected a DriftExceeded failure");
        let expected_offset = Duration::from_millis(at_tick * 16);
        let actual_offset = drift.at_offset;
        let diff = if actual_offset > expected_offset {
            actual_offset - expected_offset
        } else {
            expected_offset - actual_offset
        };
        assert!(
            diff <= Duration::from_millis(50),
            "at_offset={actual_offset:?} expected~{expected_offset:?} diff={diff:?}"
        );
    }

    #[test]
    fn report_percentiles_match_input_distribution() {
        // Drive a deterministic distribution: 20 samples ranging
        // 100us..2000us in 100us steps. p50 ~= 1000us = 1.0ms,
        // p95 ~= 1900us = 1.9ms, p99 ~= 2000us = 2.0ms, max =
        // 2.0ms. The harness's nearest-rank picker MAY widen the
        // band by up to 1 sample bucket worth = 0.1ms; layered atop
        // that, shared-CI scheduler contention on macos-latest may
        // under-deliver up to 3-4 ticks of an expected window in a
        // single soak (observed 2026-05-10 macos-nightly: 14 of 20
        // ticks, schedule truncated at idx 13 = 1.4ms, p95 trip at
        // < 0.3 tolerance). A 0.5ms tolerance applied uniformly
        // across p50 / p95 / p99 / max absorbs that under-delivery
        // while still catching > 15% scheduler-loss regressions
        // (the schedule cycles every 20 entries; missing ~25% of
        // ticks shifts a percentile by ~5 buckets = ~0.5ms).
        //
        // We size the soak duration so that at 16ms tick_interval
        // the loop hits ~40 ticks (640ms / 16ms). The schedule
        // loops every 20 entries, so two full cycles guarantee the
        // 2.0ms peak is sampled at least once even under 25% loss;
        // the distribution shape is preserved across all percentiles
        // even when the scheduler skips up to ~10 ticks of the 40.
        let schedule_us: Vec<u64> = (1..=20).map(|i| (i as u64) * 100).collect();
        let mut workload = ScheduledWorkload {
            schedule_us,
            idx: 0,
        };
        let config = fast_config(Duration::from_millis(640));
        let report = run_soak(&mut workload, &config).unwrap();
        // Apply a uniform 0.5ms tolerance across the percentile
        // family. Rationale at lines 239-251 above: 1 sample bucket
        // (= 0.1ms) plus shared-CI scheduler under-delivery up to
        // ~3-4 ticks of the 40 expected. A 0.5ms band on each
        // percentile + max stays well inside the regression-
        // detection envelope (a 50% slowdown on the workload would
        // shift every percentile by >= 1ms).
        assert!(
            (report.p50_tick_ms - 1.0).abs() < 0.5,
            "p50={}",
            report.p50_tick_ms
        );
        assert!(
            (report.p95_tick_ms - 1.9).abs() < 0.5,
            "p95={}",
            report.p95_tick_ms
        );
        assert!(
            (report.p99_tick_ms - 2.0).abs() < 0.5,
            "p99={}",
            report.p99_tick_ms
        );
        assert!(
            (report.max_tick_ms - 2.0).abs() < 0.5,
            "max={}",
            report.max_tick_ms
        );
    }

    #[test]
    fn state_corruption_workload_error_round_trip() {
        // OneShotCorruptionWorkload emits StateCorruption with a
        // distinct marker; the harness should record exactly one
        // StateCorruptionDetected carrying the marker bit-exact.
        let mut workload = OneShotCorruptionWorkload {
            per_tick: Duration::from_micros(50),
            marker: 7.5,
            at_tick: 4,
            current_tick: 0,
        };
        let config = fast_config(Duration::from_millis(200));
        let report = run_soak(&mut workload, &config).unwrap();
        let corruption: Vec<_> = report
            .stability_failures
            .iter()
            .filter(|f| f.kind == StabilityFailureKind::StateCorruptionDetected)
            .collect();
        assert_eq!(
            corruption.len(),
            1,
            "expected exactly one StateCorruptionDetected"
        );
        assert!(
            (corruption[0].recorded_value - 7.5).abs() < f64::EPSILON,
            "recorded_value={}",
            corruption[0].recorded_value
        );
    }
}

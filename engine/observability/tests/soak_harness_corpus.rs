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
    fn tick_count_tracks_duration_over_interval_under_scheduler_jitter() {
        // 800ms duration / 16ms interval = 50 nominal ticks.
        //
        // Upper bound 53: structural — the drift-corrected timer
        // cannot start tick N before `t0 + N * tick_interval`, so the
        // loop cannot deliver more than ceil(800/16) + 1 ticks on any
        // host. This side catches a busy-spin / broken-deadline
        // regression and never flakes.
        //
        // Lower bound 30 (60% of nominal): shared-CI macos runners
        // deschedule the soak thread for long stretches. The drift-
        // corrected timer catches up in bursts afterwards, EXCEPT
        // when the stall straddles the duration cutoff — those ticks
        // are permanently truncated. Observed on macos-nightly
        // 2026-05-30 through 2026-06-10 at the previous 500ms
        // duration: 24-26 ticks of 31 nominal (a ~115-180ms tail
        // loss) tripping the old [27, 36] band roughly every other
        // night. 800ms amortizes the same absolute tail loss to a
        // ~39-45-tick floor while 30 still catches the regression
        // classes this fixture exists for: a harness sleeping 2x the
        // interval delivers ~25 ticks, and a 40%+ scheduler-loss
        // regression lands below the bound. Precedent: the T1 pin in
        // `soak.rs` and the percentile fixture below took the same
        // widen-and-document path after the 2026-05-10 macos-nightly
        // incident.
        let mut workload = ConstantWorkload {
            per_tick: Duration::from_micros(50),
        };
        let config = fast_config(Duration::from_millis(800));
        let report = run_soak(&mut workload, &config).unwrap();
        assert!(
            report.tick_count >= 30 && report.tick_count <= 53,
            "tick_count={} out of [30, 53]",
            report.tick_count
        );
    }

    #[test]
    fn stability_failure_records_at_offset_correctly() {
        // OneShotOverrunWorkload fires at tick 5. The assertions pin
        // hard scheduling invariants rather than wall-clock proximity:
        //
        // - Lower bound: the drift-corrected timer sleeps tick N to
        //   at least `t0 + N * tick_interval`, so the recorded
        //   at_offset can never be earlier than 5 * 16ms = 80ms
        //   (catches an at_offset stamped at zero or from the wrong
        //   tick).
        // - Upper bound: the failure record exists only if tick 5
        //   started before the duration cutoff, so at_offset is below
        //   `config.duration` by construction whenever the lookup
        //   succeeds (catches an at_offset stamped at report
        //   generation).
        // - recorded_value carries the workload's self-reported
        //   elapsed — pure data, no clock — pinning the payload.
        //
        // The +/-50ms proximity band this replaces assumed nominal
        // scheduler delivery and flaked on loaded shared runners
        // (macos-nightly 2026-05-30 through 2026-06-12: seven red
        // nights, observed at_offset 130-154ms = 50-74ms of
        // cumulative sleep overshoot across five 16ms ticks).
        // Proximity to the nominal 80ms is a performance property of
        // the host, not a correctness property of the harness. Same
        // invariant style as the chaos corpus's dropout fixture
        // (`chaos_soak_corpus.rs` asserts `at_offset >= 80ms` plus
        // cross-record equality, no upper proximity).
        let at_tick: u32 = 5;
        let overrun = Duration::from_millis(50);
        let mut workload = OneShotOverrunWorkload {
            per_tick: Duration::from_micros(50),
            overrun_elapsed: overrun,
            at_tick: u64::from(at_tick),
            current_tick: 0,
        };
        let config = fast_config(Duration::from_millis(300));
        let report = run_soak(&mut workload, &config).unwrap();
        // per_tick (50us) never crosses the 1.25ms drift gate and the
        // overrun fires exactly once, so exactly one DriftExceeded is
        // deterministic regardless of host load.
        let drifts: Vec<_> = report
            .stability_failures
            .iter()
            .filter(|f| f.kind == StabilityFailureKind::DriftExceeded)
            .collect();
        assert_eq!(drifts.len(), 1, "expected exactly one DriftExceeded");
        let drift = drifts[0];
        let earliest = config.tick_interval * at_tick;
        assert!(
            drift.at_offset >= earliest,
            "at_offset={:?} earlier than the drift-corrected schedule allows \
             (tick {at_tick} cannot start before {earliest:?})",
            drift.at_offset
        );
        assert!(
            drift.at_offset < config.duration,
            "at_offset={:?} at or beyond the {:?} duration cutoff — stamped at \
             report generation instead of at the failing tick?",
            drift.at_offset,
            config.duration
        );
        assert!(
            (drift.recorded_value - 50.0).abs() < 0.01,
            "recorded_value={} expected the workload-reported 50ms",
            drift.recorded_value
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

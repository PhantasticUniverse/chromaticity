// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4d Wave 4d.4 `EV-SOAK-CHAOS` soak harness scaffold (PR 1
//! of 2-3).
//!
//! Row 40 of `docs/26-evidence-artifact-registry.md` declares the
//! invariant: validate session stability and recovery behaviour via
//! "soak + chaos validation runs", with the registry's evidence-
//! format-by-class table line 165 anchoring the per-class quantitative
//! target to ">= 4 hour soak achieved per `10-performance-latency-
//! budgets.md`". `docs/10-performance-latency-budgets.md` line 55
//! pins the operational target: "Continuous run soak target: >= 4
//! hours without crash".
//!
//! This module ships the harness scaffold + CI-short config. The
//! [`SoakWorkload`] trait abstracts the per-tick work the soak driver
//! exercises; [`run_soak`] is the runner that loops on a drift-
//! corrected timer for [`SoakConfig::duration`], samples per-tick
//! latencies into a percentile distribution, runs periodic stability
//! checks, and produces a [`SoakReport`].
//!
//! ## Wave 4d.4 narrowing
//!
//! This PR ships the harness scaffold + [`SoakConfig::ci_short`]
//! config. The full >= 4-hour soak target per
//! `docs/10-performance-latency-budgets.md` line 55 is **calendar-
//! gated and runs as a nightly criterion-bench-style harness**, not
//! as a per-PR CI gate. The strategy mirrors `EV-PERF-BUDGETS`
//! (Phase-3 Path B nightly-only criterion harness in
//! `engine/runtime/benches/budgets.rs`, ~$20/month-vs-~$70-80/month
//! AO-approved cost posture per CLAUDE.md).
//!
//! ### Sequencing
//!
//! - **PR 1 (this PR)**: harness scaffold + CI-short config + 7 T1
//!   pins + T2 corpus; row 40 stays at `Planned`.
//! - **PR 2**: chaos-injection extension (controlled fault injection
//!   across the soak window — adapter dropouts, audio-stream
//!   interruptions, GPU-reset signaling where the platform supports
//!   them). The [`StabilityFailureKind::StateCorruptionDetected`]
//!   arm + the [`SoakWorkloadError::StateCorruption`] error variant
//!   ship in this PR as the integration seam PR 2 will compose
//!   into.
//! - **PR 3**: multi-profile validation corpus + closing-docs row
//!   40 -> Pass under explicit narrowing.
//!
//! ## Design notes
//!
//! ### Drift-corrected timing
//!
//! [`run_soak`] computes each tick's deadline against
//! `t0 + tick_interval * tick_count` rather than incrementally
//! `sleep(tick_interval)`. This avoids accumulating per-tick rounding
//! errors over long runs and matches the established scheduler-tick
//! discipline in `app/shell/src/lib.rs::run` setup callback (the
//! production scheduler-tick thread spawned post-Wave-1c-α-2). A naive
//! `sleep`-based loop would drift by tens of seconds across a four-
//! hour run and silently invalidate the report's tick-count
//! arithmetic.
//!
//! ### Inline percentile computation
//!
//! Per-tick latency samples accumulate into a `Vec<f64>` and are
//! sorted at report-generation time to compute p50 / p95 / p99. A
//! streaming histogram (e.g. HDR-Histogram) was considered and
//! rejected for this PR's scaffold: at 60 Hz over 4 hours the sample
//! count is ~864K `f64`s (~6.6 MB), well within budget for a
//! diagnostic harness. The `Vec`-and-sort path is the simplest
//! correct implementation and avoids pulling a histogram dependency
//! through `workspace-hack`. PR 2 / PR 3 may revisit if profile data
//! shows the sort is the dominant cost.
//!
//! ### Stability checks
//!
//! Three failure modes are tracked in this PR:
//!
//! - [`StabilityFailureKind::DriftExceeded`] — the most recent tick
//!   took longer than [`SoakConfig::drift_threshold_p95_ms`].
//! - [`StabilityFailureKind::TickStalled`] — wallclock has advanced
//!   more than 2x [`SoakConfig::tick_interval`] since the last tick
//!   completed (the workload may have hung).
//! - [`StabilityFailureKind::StateCorruptionDetected`] — recorded
//!   when the workload returns
//!   [`SoakWorkloadError::StateCorruption`]; the arm exists in PR 1
//!   as the integration seam PR 2 / PR 3's chaos injectors will
//!   compose into.
//!
//! ## Governing documents
//!
//! - `docs/26-evidence-artifact-registry.md` row 40 — `EV-SOAK-CHAOS`.
//! - `docs/10-performance-latency-budgets.md` §6 line 55 — the >= 4-
//!   hour soak target.
//! - `docs/44-phase-4-exit-scoping.md` §4.3 row 40 + §5.3 sequencing.
//! - `docs/adr/ADR-016-observability-and-telemetry-posture.md` —
//!   parent observability posture.

use std::time::{Duration, Instant};

/// Drift-threshold default per `docs/10-performance-latency-budgets.md`
/// §6: 1.25ms is the in-process control-loop p95 budget rolled into
/// the harness as the per-tick drift gate. The scheduler-tick loop
/// itself runs at 60 Hz (~16.67ms / tick); the 1.25ms threshold
/// applies to the workload's reported per-tick wallclock cost, not
/// to the harness's tick-period overhead.
const CI_SHORT_DRIFT_THRESHOLD_MS: f64 = 1.25;

/// CI-short soak duration. Sub-minute so per-PR CI runs stay cheap;
/// the production target lives on [`SoakConfig::nightly_long`].
const CI_SHORT_DURATION_SECS: u64 = 60;

/// Production target per `docs/10-performance-latency-budgets.md`
/// line 55: >= 4 hours continuous run without crash.
const NIGHTLY_LONG_DURATION_SECS: u64 = 4 * 60 * 60;

/// Default stability-check cadence for the CI-short config. Every
/// second is fine-grained enough to localise a stall to within ~1
/// stability-window of its onset for a 60-second run.
const CI_SHORT_STABILITY_CHECK_INTERVAL_SECS: u64 = 1;

/// Default tick interval (~60 Hz). Mirrors
/// `app/shell/src/lib.rs::SCHEDULER_TICK_HZ` so the harness's tick
/// cadence matches the production scheduler-tick thread.
const DEFAULT_TICK_INTERVAL_NANOS: u128 = 16_666_667;

/// Soak-harness configuration.
///
/// Fields are public so test corpora can construct narrowed configs
/// (e.g. sub-second durations) without relying on the
/// [`SoakConfig::ci_short`] / [`SoakConfig::nightly_long`] presets.
#[derive(Debug, Clone)]
pub struct SoakConfig {
    /// Total wallclock duration the harness runs for. The harness
    /// stops at the first tick whose start `Instant` is at or beyond
    /// `t0 + duration`.
    pub duration: Duration,
    /// Nominal per-tick interval. The drift-corrected timer schedules
    /// each tick's deadline against `t0 + tick_interval * tick_count`.
    pub tick_interval: Duration,
    /// Minimum wallclock interval between stability-check sweeps. A
    /// sweep at most every `stability_check_interval`; the actual
    /// cadence is "the first tick after `last_stability_check +
    /// stability_check_interval`".
    pub stability_check_interval: Duration,
    /// Per-tick wallclock cost above which the harness records a
    /// [`StabilityFailureKind::DriftExceeded`]. Compared against the
    /// `Duration` returned by [`SoakWorkload::tick`].
    pub drift_threshold_p95_ms: f64,
}

impl SoakConfig {
    /// CI-short config: 60 second duration, ~60 Hz tick, 1 second
    /// stability-check cadence, 1.25ms drift threshold per
    /// `docs/10-performance-latency-budgets.md` §6.
    ///
    /// Used for PR-time T2 corpora that want the production-shape
    /// config without the production-shape duration. T2 fixtures
    /// override [`SoakConfig::duration`] to sub-second values to keep
    /// per-PR CI under budget.
    #[must_use]
    pub fn ci_short() -> Self {
        Self {
            duration: Duration::from_secs(CI_SHORT_DURATION_SECS),
            tick_interval: Duration::from_nanos(
                u64::try_from(DEFAULT_TICK_INTERVAL_NANOS).unwrap_or(u64::MAX),
            ),
            stability_check_interval: Duration::from_secs(CI_SHORT_STABILITY_CHECK_INTERVAL_SECS),
            drift_threshold_p95_ms: CI_SHORT_DRIFT_THRESHOLD_MS,
        }
    }

    /// Nightly-long config: 4-hour duration matching
    /// `docs/10-performance-latency-budgets.md` line 55. Calendar-
    /// gated; the nightly criterion-bench-style harness is the
    /// intended run site, not per-PR CI.
    #[must_use]
    pub fn nightly_long() -> Self {
        Self {
            duration: Duration::from_secs(NIGHTLY_LONG_DURATION_SECS),
            tick_interval: Duration::from_nanos(
                u64::try_from(DEFAULT_TICK_INTERVAL_NANOS).unwrap_or(u64::MAX),
            ),
            stability_check_interval: Duration::from_secs(60),
            drift_threshold_p95_ms: CI_SHORT_DRIFT_THRESHOLD_MS,
        }
    }
}

/// Per-soak summary report produced by [`run_soak`].
#[derive(Debug, Clone)]
pub struct SoakReport {
    /// Wallclock duration the harness actually ran for. Equal-or-
    /// greater than [`SoakConfig::duration`]; the loop terminates at
    /// the first tick whose deadline is past `t0 + duration`.
    pub duration_observed: Duration,
    /// Total ticks driven. Approximates
    /// `duration / tick_interval` to within +/-1 tick under nominal
    /// conditions.
    pub tick_count: u64,
    /// Stability checks where every gate passed.
    pub stability_checks_passed: u64,
    /// Stability checks where at least one gate failed.
    pub stability_checks_failed: u64,
    /// p50 per-tick wallclock cost (milliseconds).
    pub p50_tick_ms: f64,
    /// p95 per-tick wallclock cost (milliseconds).
    pub p95_tick_ms: f64,
    /// p99 per-tick wallclock cost (milliseconds).
    pub p99_tick_ms: f64,
    /// Maximum per-tick wallclock cost observed (milliseconds).
    pub max_tick_ms: f64,
    /// Per-failure record. Empty under nominal conditions.
    pub stability_failures: Vec<StabilityFailure>,
}

/// Single stability-failure record.
#[derive(Debug, Clone)]
pub struct StabilityFailure {
    /// Wallclock offset from `t0` at which the failure was detected.
    pub at_offset: Duration,
    /// Failure mode.
    pub kind: StabilityFailureKind,
    /// Recorded numeric value associated with the failure (e.g. the
    /// observed tick latency in milliseconds for
    /// [`StabilityFailureKind::DriftExceeded`]).
    pub recorded_value: f64,
}

/// Stability-failure mode discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityFailureKind {
    /// The most recent tick took longer than
    /// [`SoakConfig::drift_threshold_p95_ms`].
    DriftExceeded,
    /// Wallclock advanced more than 2x [`SoakConfig::tick_interval`]
    /// since the last tick completed; the workload may have hung.
    TickStalled,
    /// Workload reported [`SoakWorkloadError::StateCorruption`]. The
    /// arm exists in PR 1 as the integration seam for PR 2's chaos
    /// injectors.
    StateCorruptionDetected,
}

/// Workload abstraction. Implementors run one unit of work per
/// [`SoakWorkload::tick`] call and return the wallclock duration the
/// work took. The harness samples that duration into the percentile
/// distribution and runs it past the drift gate.
pub trait SoakWorkload {
    /// Drive one tick of work. Return the wallclock cost of the work
    /// (the harness uses this for the percentile distribution AND
    /// the per-tick drift gate). Return an error to surface a
    /// failure mode the harness should record as a
    /// [`StabilityFailure`].
    fn tick(&mut self) -> Result<Duration, SoakWorkloadError>;
}

/// Workload-side failure surfaced to the harness.
#[derive(Debug, Clone, PartialEq)]
pub enum SoakWorkloadError {
    /// Workload wallclock budget exceeded; the harness records a
    /// [`StabilityFailureKind::DriftExceeded`] with the elapsed
    /// time. PR 2's chaos injectors compose into this variant when
    /// they want to surface a controlled overrun.
    OverrunBudget {
        /// Elapsed wallclock time the workload reported.
        elapsed: Duration,
    },
    /// Workload detected internal state corruption; the harness
    /// records a [`StabilityFailureKind::StateCorruptionDetected`].
    /// The variant ships in PR 1 as the integration seam PR 2 / PR
    /// 3's chaos injectors will compose into.
    StateCorruption {
        /// Caller-supplied numeric tag. PR 2 / PR 3 will define the
        /// semantics (e.g. "bytes diverged from expected hash") when
        /// chaos injection lands.
        marker: f64,
    },
}

/// Harness-side failure (configuration error, etc.). Distinct from
/// [`SoakWorkloadError`] so the runner can surface harness misuse
/// separately from workload-reported failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoakHarnessError {
    /// `tick_interval` was zero. The drift-corrected timer would
    /// divide-by-zero or busy-spin; refuse to run.
    ZeroTickInterval,
    /// `duration` was zero. The runner would terminate before the
    /// first tick; refuse to run.
    ZeroDuration,
}

/// Run a soak against `workload` per `config`. Returns the per-soak
/// [`SoakReport`].
///
/// The runner is single-threaded and synchronous: it owns the calling
/// thread for the soak duration. Production callers spawn it on a
/// dedicated thread (see `app/shell/src/lib.rs::run` for the
/// established scheduler-tick spawn pattern).
///
/// # Errors
///
/// - [`SoakHarnessError::ZeroTickInterval`] if `config.tick_interval`
///   is zero.
/// - [`SoakHarnessError::ZeroDuration`] if `config.duration` is zero.
pub fn run_soak<W: SoakWorkload>(
    workload: &mut W,
    config: &SoakConfig,
) -> Result<SoakReport, SoakHarnessError> {
    if config.tick_interval.is_zero() {
        return Err(SoakHarnessError::ZeroTickInterval);
    }
    if config.duration.is_zero() {
        return Err(SoakHarnessError::ZeroDuration);
    }

    let t0 = Instant::now();
    let mut tick_count: u64 = 0;
    let mut samples_ms: Vec<f64> = Vec::new();
    let mut failures: Vec<StabilityFailure> = Vec::new();
    let mut stability_passed: u64 = 0;
    let mut stability_failed: u64 = 0;
    let mut last_tick_completed_at: Instant = t0;
    let mut last_stability_check_at: Instant = t0;

    loop {
        let now = Instant::now();
        if now.duration_since(t0) >= config.duration {
            break;
        }

        // Run the workload's tick + record its self-reported cost.
        let tick_started_at = Instant::now();
        match workload.tick() {
            Ok(elapsed) => {
                let elapsed_ms = duration_to_ms(elapsed);
                samples_ms.push(elapsed_ms);

                if elapsed_ms > config.drift_threshold_p95_ms {
                    failures.push(StabilityFailure {
                        at_offset: tick_started_at.duration_since(t0),
                        kind: StabilityFailureKind::DriftExceeded,
                        recorded_value: elapsed_ms,
                    });
                }
            }
            Err(SoakWorkloadError::OverrunBudget { elapsed }) => {
                let elapsed_ms = duration_to_ms(elapsed);
                samples_ms.push(elapsed_ms);
                failures.push(StabilityFailure {
                    at_offset: tick_started_at.duration_since(t0),
                    kind: StabilityFailureKind::DriftExceeded,
                    recorded_value: elapsed_ms,
                });
            }
            Err(SoakWorkloadError::StateCorruption { marker }) => {
                failures.push(StabilityFailure {
                    at_offset: tick_started_at.duration_since(t0),
                    kind: StabilityFailureKind::StateCorruptionDetected,
                    recorded_value: marker,
                });
            }
        }

        let tick_completed_at = Instant::now();
        // TickStalled: the gap from last completion to this
        // completion exceeded 2x the nominal tick interval.
        let since_last_completion = tick_completed_at.duration_since(last_tick_completed_at);
        let stall_threshold = config.tick_interval.saturating_mul(2);
        // Skip the stall check on the very first tick (last_tick_
        // completed_at is t0; the first tick's "since last" is
        // dominated by harness-startup overhead, not by workload
        // hang).
        if tick_count > 0 && since_last_completion > stall_threshold {
            failures.push(StabilityFailure {
                at_offset: tick_completed_at.duration_since(t0),
                kind: StabilityFailureKind::TickStalled,
                recorded_value: duration_to_ms(since_last_completion),
            });
        }
        last_tick_completed_at = tick_completed_at;

        // Periodic stability-check rollup. The "check" in the current
        // PR is "did any failure record fire since the last check"
        // (i.e. the per-tick gates above ARE the checks; the rollup
        // is just bookkeeping). PR 2 / PR 3 may extend with deeper
        // workload-side probes; the bookkeeping shape stays.
        if tick_completed_at.duration_since(last_stability_check_at)
            >= config.stability_check_interval
        {
            let failures_in_window = failures
                .iter()
                .filter(|f| f.at_offset >= last_stability_check_at.duration_since(t0))
                .count();
            if failures_in_window > 0 {
                stability_failed = stability_failed.saturating_add(1);
            } else {
                stability_passed = stability_passed.saturating_add(1);
            }
            last_stability_check_at = tick_completed_at;
        }

        tick_count = tick_count.saturating_add(1);

        // Drift-corrected sleep. Schedule the next tick against
        // `t0 + tick_interval * tick_count` rather than
        // `now + tick_interval`.
        let next_deadline = t0
            + config
                .tick_interval
                .saturating_mul(u32::try_from(tick_count).unwrap_or(u32::MAX));
        let after_bookkeeping = Instant::now();
        if next_deadline > after_bookkeeping {
            std::thread::sleep(next_deadline - after_bookkeeping);
        }
    }

    let duration_observed = Instant::now().duration_since(t0);
    let (p50, p95, p99, max) = compute_percentiles(&mut samples_ms);

    Ok(SoakReport {
        duration_observed,
        tick_count,
        stability_checks_passed: stability_passed,
        stability_checks_failed: stability_failed,
        p50_tick_ms: p50,
        p95_tick_ms: p95,
        p99_tick_ms: p99,
        max_tick_ms: max,
        stability_failures: failures,
    })
}

#[inline]
fn duration_to_ms(d: Duration) -> f64 {
    // `as_secs_f64()` returns seconds; multiply for ms.
    d.as_secs_f64() * 1_000.0
}

/// Compute (p50, p95, p99, max) over `samples`. Sorts in place.
/// Returns zeros when `samples` is empty.
fn compute_percentiles(samples: &mut [f64]) -> (f64, f64, f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let last_idx = samples.len() - 1;
    let pick = |q: f64| -> f64 {
        // Nearest-rank percentile. For p95 over 20 samples, idx =
        // ceil(0.95 * 20) - 1 = 18 (the 19th element zero-indexed).
        // Bound by [0, len-1] so q=1.0 maps to the last index.
        let mut idx = (q * samples.len() as f64).ceil() as isize - 1;
        if idx < 0 {
            idx = 0;
        }
        let idx_usize = idx as usize;
        let bounded = if idx_usize > last_idx {
            last_idx
        } else {
            idx_usize
        };
        samples[bounded]
    };
    let p50 = pick(0.50);
    let p95 = pick(0.95);
    let p99 = pick(0.99);
    let max = samples[last_idx];
    (p50, p95, p99, max)
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-SOAK-CHAOS (T1) — in-source drift pins for the
    //! soak harness scaffold.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: exercises a
    //! discrete behavioural property of the unit under test against a
    //! small in-source fixture. The T2 corpus fixture-driven cross-
    //! property round-trip lives at
    //! `engine/observability/tests/soak_harness_corpus.rs`.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md` row
    //! 40 (`EV-SOAK-CHAOS`) declares Generation Source "Soak + chaos
    //! validation runs" with the per-class quantitative target ">= 4
    //! hour soak achieved per `10-performance-latency-budgets.md`".
    //! This T1 + the sibling T2 corpus together pin the harness-
    //! scaffold contract; the row's `Planned -> Pass` transition lands
    //! when PR 3 (multi-profile validation corpus + closing-docs)
    //! ships under the Wave 4d.4 narrowing.

    use super::{
        NIGHTLY_LONG_DURATION_SECS, SoakConfig, SoakWorkload, SoakWorkloadError,
        StabilityFailureKind, run_soak,
    };
    use std::time::Duration;

    /// Constant-cost workload — every tick claims `per_tick`
    /// wallclock cost without actually sleeping. Used by the pins to
    /// drive the harness without burning real time.
    struct ConstantWorkload {
        per_tick: Duration,
    }

    impl SoakWorkload for ConstantWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            Ok(self.per_tick)
        }
    }

    /// Workload that reports `OverrunBudget` on tick `overrun_at_tick`
    /// with the configured `overrun_elapsed`, and the constant cost
    /// otherwise.
    struct OneShotOverrunWorkload {
        per_tick: Duration,
        overrun_elapsed: Duration,
        overrun_at_tick: u64,
        current_tick: u64,
    }

    impl SoakWorkload for OneShotOverrunWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            let n = self.current_tick;
            self.current_tick = self.current_tick.saturating_add(1);
            if n == self.overrun_at_tick {
                Err(SoakWorkloadError::OverrunBudget {
                    elapsed: self.overrun_elapsed,
                })
            } else {
                Ok(self.per_tick)
            }
        }
    }

    /// Workload that sleeps for `stall_for` on tick `stall_at_tick`,
    /// to provoke the `TickStalled` gate.
    struct OneShotStallWorkload {
        per_tick: Duration,
        stall_for: Duration,
        stall_at_tick: u64,
        current_tick: u64,
    }

    impl SoakWorkload for OneShotStallWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            let n = self.current_tick;
            self.current_tick = self.current_tick.saturating_add(1);
            if n == self.stall_at_tick {
                std::thread::sleep(self.stall_for);
            }
            Ok(self.per_tick)
        }
    }

    fn fast_config(duration: Duration) -> SoakConfig {
        SoakConfig {
            duration,
            tick_interval: Duration::from_millis(5),
            stability_check_interval: Duration::from_millis(10),
            drift_threshold_p95_ms: 1.25,
        }
    }

    #[test]
    fn ci_short_soak_completes_within_budget() {
        // Run a 100ms duration against a near-zero workload. The
        // harness should return promptly (well under 2 seconds even
        // on a slow CI runner) and produce a non-empty report.
        let mut workload = ConstantWorkload {
            per_tick: Duration::from_micros(50),
        };
        let config = fast_config(Duration::from_millis(100));
        let report = run_soak(&mut workload, &config).unwrap();
        assert!(report.tick_count > 0);
        assert!(
            report.duration_observed >= Duration::from_millis(100),
            "duration_observed={:?}",
            report.duration_observed
        );
    }

    #[test]
    fn report_p50_p95_p99_match_observed_distribution() {
        // Constant 0.5ms-per-tick workload over a short duration:
        // every percentile bucket should land at ~0.5ms.
        let mut workload = ConstantWorkload {
            per_tick: Duration::from_micros(500),
        };
        let config = fast_config(Duration::from_millis(60));
        let report = run_soak(&mut workload, &config).unwrap();
        assert!((report.p50_tick_ms - 0.5).abs() < 0.05);
        assert!((report.p95_tick_ms - 0.5).abs() < 0.05);
        assert!((report.p99_tick_ms - 0.5).abs() < 0.05);
        assert!((report.max_tick_ms - 0.5).abs() < 0.05);
    }

    #[test]
    fn drift_exceeded_recorded_when_tick_overruns() {
        // OneShotOverrunWorkload reports a 100ms overrun on tick 2;
        // the harness should record exactly one DriftExceeded.
        let mut workload = OneShotOverrunWorkload {
            per_tick: Duration::from_micros(50),
            overrun_elapsed: Duration::from_millis(100),
            overrun_at_tick: 2,
            current_tick: 0,
        };
        let config = fast_config(Duration::from_millis(60));
        let report = run_soak(&mut workload, &config).unwrap();
        let drift_count = report
            .stability_failures
            .iter()
            .filter(|f| f.kind == StabilityFailureKind::DriftExceeded)
            .count();
        assert_eq!(drift_count, 1);
    }

    #[test]
    fn stalled_tick_recorded_when_workload_hangs() {
        // OneShotStallWorkload sleeps for 50ms on tick 2 — well
        // beyond 2x the 5ms tick_interval. The harness should record
        // a TickStalled. (The same overrun also fires DriftExceeded
        // since 50ms > 1.25ms threshold; we assert TickStalled
        // specifically.)
        let mut workload = OneShotStallWorkload {
            per_tick: Duration::from_micros(50),
            stall_for: Duration::from_millis(50),
            stall_at_tick: 2,
            current_tick: 0,
        };
        let config = fast_config(Duration::from_millis(150));
        let report = run_soak(&mut workload, &config).unwrap();
        let stall_count = report
            .stability_failures
            .iter()
            .filter(|f| f.kind == StabilityFailureKind::TickStalled)
            .count();
        assert!(
            stall_count >= 1,
            "expected >= 1 TickStalled, got {stall_count}; failures={:?}",
            report.stability_failures
        );
    }

    #[test]
    fn tick_count_matches_duration_div_interval_within_one_tick() {
        // 100ms duration / 5ms tick_interval = ~20 ticks. Real
        // wallclock will be slightly over (the loop terminates at
        // the first tick whose start is past `t0 + duration`); allow
        // +/-3 ticks of slack to absorb scheduler jitter on shared
        // CI hardware.
        let mut workload = ConstantWorkload {
            per_tick: Duration::from_micros(50),
        };
        let config = fast_config(Duration::from_millis(100));
        let report = run_soak(&mut workload, &config).unwrap();
        assert!(
            report.tick_count >= 17 && report.tick_count <= 25,
            "tick_count={}",
            report.tick_count
        );
    }

    #[test]
    fn nightly_long_config_is_4_hours() {
        let cfg = SoakConfig::nightly_long();
        assert_eq!(
            cfg.duration,
            Duration::from_secs(NIGHTLY_LONG_DURATION_SECS)
        );
        assert_eq!(cfg.duration, Duration::from_secs(4 * 60 * 60));
    }

    #[test]
    fn state_corruption_failure_kind_round_trip() {
        // A workload that emits StateCorruption on its first tick
        // produces a report whose first stability failure is
        // `StateCorruptionDetected` carrying the marker value.
        struct CorruptionWorkload {
            done: bool,
        }
        impl SoakWorkload for CorruptionWorkload {
            fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
                if self.done {
                    Ok(Duration::from_micros(50))
                } else {
                    self.done = true;
                    Err(SoakWorkloadError::StateCorruption { marker: 42.0 })
                }
            }
        }
        let mut workload = CorruptionWorkload { done: false };
        let config = fast_config(Duration::from_millis(60));
        let report = run_soak(&mut workload, &config).unwrap();
        let corruption = report
            .stability_failures
            .iter()
            .find(|f| f.kind == StabilityFailureKind::StateCorruptionDetected)
            .expect("expected a StateCorruptionDetected failure");
        assert!((corruption.recorded_value - 42.0).abs() < f64::EPSILON);
    }
}

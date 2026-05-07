// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Chaos-injection layer for soak runs per `docs/26-evidence-artifact-
//! registry.md` row 40 `EV-SOAK-CHAOS` + `docs/44-phase-4-exit-scoping.md`
//! §4.3 row 40. Composes with the [`crate::soak::SoakHarness`]-shaped
//! runner from [`super::soak`].
//!
//! ## Design
//!
//! A [`ChaosPlan`] is a deterministic schedule of [`FaultEvent`]
//! variants applied to a [`ChaosWorkload`] at prescribed offsets within
//! the soak window. The workload's [`crate::soak::SoakWorkload::tick`]
//! is responsible for either recovering from the fault (returning `Ok`
//! within bounded latency) or surfacing it as a
//! [`crate::soak::SoakWorkloadError`].
//!
//! Chaos events are NOT automatically translated into stability
//! failures — the harness records both the chaos schedule (the
//! [`FiredFaultEvent`] log) AND the workload's response (the embedded
//! [`crate::soak::SoakReport`] continues to record
//! [`crate::soak::StabilityFailure`] entries from the underlying drift
//! / stall / corruption gates) so closing-docs PR 3 can validate the
//! recovery contract per row 40.
//!
//! ## Recovery-window tagging
//!
//! Each [`ChaosScheduleEntry`] carries a `recovery_after` window. Any
//! [`crate::soak::StabilityFailure`] firing within that window of the
//! event's `at_offset` is annotated in the [`ChaosReport`] as a
//! `chaos-recovery-window` failure (vs. an `independent` failure
//! firing outside any recovery window). This separation lets PR 3's
//! multi-profile validation corpus distinguish "the chaos injection
//! produced an expected transient stability dip the workload then
//! recovered from" from "an unrelated stability failure fired and the
//! soak should fail closed".
//!
//! ## Sequencing
//!
//! - **PR 1 (#374, merge `da34db6`)**: harness scaffold + CI-short
//!   config + 7 T1 pins + T2 corpus; row 40 stays at `Planned`.
//! - **PR 2 (this PR)**: chaos-injection layer per this module.
//! - **PR 3**: multi-profile validation corpus + closing-docs row
//!   40 -> Pass under explicit narrowing.
//!
//! Produces evidence for `EV-SOAK-CHAOS` via the `chaos_pins` T1
//! module + the sibling T2 corpus at
//! `engine/observability/tests/chaos_soak_corpus.rs` (Wave 4d.4 PR 2
//! of 2-3).
//!
//! ## Governing documents
//!
//! - `docs/26-evidence-artifact-registry.md` row 40 — `EV-SOAK-CHAOS`.
//! - `docs/10-performance-latency-budgets.md` §6 line 55 — the >= 4-
//!   hour soak target.
//! - `docs/44-phase-4-exit-scoping.md` §4.3 row 40 + §5.3 sequencing.
//! - `docs/adr/ADR-016-observability-and-telemetry-posture.md` —
//!   parent observability posture.
//! - `docs/adr/ADR-019-audio-uniform-ipc.md` §6 — stale-but-last-good
//!   semantics referenced by [`FaultEvent::AudioStreamInterruption`].

use std::time::{Duration, Instant};

use crate::soak::{
    SoakConfig, SoakHarnessError, SoakReport, SoakWorkload, SoakWorkloadError, StabilityFailure,
};

/// Controlled fault variants the harness can inject at a scheduled
/// offset. New variants extend the matrix the closing-docs PR 3
/// validation corpus exercises.
#[derive(Debug, Clone, PartialEq)]
pub enum FaultEvent {
    /// Adapter dropout (OSC / MIDI / camera). Workload should observe
    /// the adapter health-state machine transition and recover when
    /// the adapter reconnects.
    AdapterDropout {
        /// Adapter identifier (e.g. `"osc-in-7000"`, `"midi-akai-mpk"`).
        /// Opaque to the harness; the workload interprets it.
        adapter_id: String,
        /// Wallclock budget after `at_offset` within which any
        /// stability failure is tagged as `chaos-recovery-window`.
        recovery_after: Duration,
    },
    /// Audio stream interruption (`cpal` callback halts). Per ADR-019
    /// §6 stale-but-last-good, visualisers freeze at last-good
    /// snapshot; the audio runtime is expected to re-establish on or
    /// before `recovery_after`.
    AudioStreamInterruption {
        /// Wallclock budget after `at_offset` within which any
        /// stability failure is tagged as `chaos-recovery-window`.
        recovery_after: Duration,
    },
    /// GPU device-lost signaling. On platforms supporting it
    /// (`wgpu` `DeviceLost` callback), the workload should re-create
    /// the surface; on platforms without device-lost, the workload
    /// returns [`ChaosInjectionError::UnsupportedFault`] and the
    /// harness records the rejection in the [`ChaosReport`].
    GpuDeviceLost {
        /// Wallclock budget after `at_offset` within which any
        /// stability failure is tagged as `chaos-recovery-window`.
        recovery_after: Duration,
    },
    /// Catch-all extensibility variant for future fault classes that
    /// the closing-docs PR 3 validation corpus may want to cover
    /// without bumping the enum surface in a separate ADR cycle.
    Custom {
        /// Caller-supplied tag describing the fault class.
        name: String,
        /// Wallclock budget after `at_offset` within which any
        /// stability failure is tagged as `chaos-recovery-window`.
        recovery_after: Duration,
    },
}

impl FaultEvent {
    /// Stable string tag for the variant (used in [`FiredFaultEvent`]
    /// and the test corpus's match-on-kind assertions).
    #[must_use]
    pub fn kind_tag(&self) -> &'static str {
        match self {
            FaultEvent::AdapterDropout { .. } => "adapter-dropout",
            FaultEvent::AudioStreamInterruption { .. } => "audio-stream-interruption",
            FaultEvent::GpuDeviceLost { .. } => "gpu-device-lost",
            FaultEvent::Custom { .. } => "custom",
        }
    }

    /// Recovery-window for this event variant.
    #[must_use]
    pub fn recovery_after(&self) -> Duration {
        match self {
            FaultEvent::AdapterDropout { recovery_after, .. }
            | FaultEvent::AudioStreamInterruption { recovery_after }
            | FaultEvent::GpuDeviceLost { recovery_after }
            | FaultEvent::Custom { recovery_after, .. } => *recovery_after,
        }
    }
}

/// One scheduled chaos event: the wallclock offset within the soak
/// window at which the harness should call
/// [`ChaosWorkload::inject_fault`] with the carried event.
#[derive(Debug, Clone, PartialEq)]
pub struct ChaosScheduleEntry {
    /// Wallclock offset from `t0` at which to fire the event. The
    /// runner fires events whose offset has been crossed since the
    /// last tick; an event whose offset is past
    /// [`SoakConfig::duration`] is skipped (recorded nowhere — the
    /// soak terminates first).
    pub at_offset: Duration,
    /// The fault to inject.
    pub event: FaultEvent,
}

/// Deterministic chaos plan. The runner sorts the entries by
/// `at_offset` at start-up so callers may construct the plan in any
/// order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChaosPlan {
    /// Events to fire over the soak window. May be empty (in which
    /// case [`run_chaos_soak`] degenerates to a plain [`super::soak::run_soak`]).
    pub events: Vec<ChaosScheduleEntry>,
}

impl ChaosPlan {
    /// Construct an empty chaos plan. Equivalent to running the soak
    /// without chaos injection.
    #[must_use]
    pub fn empty() -> Self {
        Self { events: Vec::new() }
    }
}

/// Workload extension. A [`ChaosWorkload`] extends [`SoakWorkload`]
/// with a callback the harness invokes when a scheduled chaos event
/// fires.
pub trait ChaosWorkload: SoakWorkload {
    /// Called when the harness fires a chaos event. The workload
    /// should transition to its faulted state OR begin its recovery
    /// protocol.
    ///
    /// # Errors
    ///
    /// Return `Err` if the workload cannot accept the fault — e.g. the
    /// adapter doesn't exist, or the fault class is unsupported on
    /// this platform. The harness records the rejection in the final
    /// [`ChaosReport`] but continues running (a workload that rejects
    /// every fault still produces a useful soak signal).
    fn inject_fault(&mut self, event: &FaultEvent) -> Result<(), ChaosInjectionError>;
}

/// Why a workload rejected a chaos injection. Returned to the runner
/// from [`ChaosWorkload::inject_fault`]; recorded into
/// [`ChaosInjectionOutcome::Rejected`] in the final [`ChaosReport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChaosInjectionError {
    /// The named adapter does not exist on this workload (e.g. the
    /// chaos plan referenced an OSC port the workload never opened).
    AdapterNotFound(String),
    /// The fault class is not supported on this platform / build (e.g.
    /// [`FaultEvent::GpuDeviceLost`] on a backend without device-lost
    /// signalling).
    UnsupportedFault(String),
    /// The workload was already mid-recovery from a previous chaos
    /// event whose `recovery_after` window has not yet elapsed.
    RecoveryDeadlineExceeded,
}

/// Per-event outcome after the workload's `inject_fault` call returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChaosInjectionOutcome {
    /// Workload accepted the fault; recovery is the workload's
    /// responsibility.
    Accepted,
    /// Workload rejected the fault with the given reason.
    Rejected(ChaosInjectionError),
}

/// Single fired-fault record. One entry per [`ChaosScheduleEntry`] the
/// runner crossed within the soak window.
#[derive(Debug, Clone)]
pub struct FiredFaultEvent {
    /// Wallclock offset from `t0` at which the harness invoked
    /// `inject_fault`. Equal-or-greater than the entry's `at_offset`
    /// (the runner fires at the first tick whose loop-iteration time
    /// is at or beyond the scheduled offset).
    pub at_offset: Duration,
    /// Stable tag for the fault variant per [`FaultEvent::kind_tag`].
    pub event_kind: String,
    /// Workload's response.
    pub injection_outcome: ChaosInjectionOutcome,
    /// Recovery-window for the event (carried through from
    /// [`FaultEvent::recovery_after`]). Used by
    /// [`stability_failure_chaos_classification`] to tag failures as
    /// `chaos-recovery-window` vs. `independent`.
    pub recovery_after: Duration,
}

/// Classification of a [`StabilityFailure`] relative to the chaos
/// schedule. Computed post-hoc by
/// [`stability_failure_chaos_classification`] — not stored on the
/// failure itself (the soak harness has no chaos awareness).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityFailureChaosClass {
    /// The failure fired within the recovery window of at least one
    /// chaos event (i.e. `event.at_offset <= failure.at_offset <=
    /// event.at_offset + event.recovery_after`).
    ChaosRecoveryWindow,
    /// The failure fired outside every chaos event's recovery window.
    Independent,
}

/// Composite report from [`run_chaos_soak`]. Embeds the underlying
/// [`SoakReport`] and the per-event chaos log.
#[derive(Debug, Clone)]
pub struct ChaosReport {
    /// Stability metrics from the underlying soak run.
    pub soak: SoakReport,
    /// Per-event chaos firing log, in firing order (which equals
    /// schedule order after sort-by-offset).
    pub events_fired: Vec<FiredFaultEvent>,
}

impl ChaosReport {
    /// Classify each [`StabilityFailure`] in [`ChaosReport::soak`] as
    /// either inside a chaos recovery window or independent. Returns a
    /// vector aligned 1:1 with `self.soak.stability_failures`.
    #[must_use]
    pub fn stability_failure_chaos_classification(&self) -> Vec<StabilityFailureChaosClass> {
        self.soak
            .stability_failures
            .iter()
            .map(|failure| classify_failure_against_chaos(failure, &self.events_fired))
            .collect()
    }
}

fn classify_failure_against_chaos(
    failure: &StabilityFailure,
    events: &[FiredFaultEvent],
) -> StabilityFailureChaosClass {
    for fired in events {
        let window_end = fired.at_offset.saturating_add(fired.recovery_after);
        if failure.at_offset >= fired.at_offset && failure.at_offset <= window_end {
            return StabilityFailureChaosClass::ChaosRecoveryWindow;
        }
    }
    StabilityFailureChaosClass::Independent
}

/// Run a chaos-augmented soak against `workload` per `config`, firing
/// events from `plan` at-or-after their declared offsets.
///
/// The runner mirrors [`crate::soak::run_soak`]'s drift-corrected
/// timing (each tick's deadline is computed against
/// `t0 + tick_interval * tick_count`, not incrementally) plus the
/// same per-tick stability gates: drift, stall, and state-corruption
/// failures continue to land in [`SoakReport::stability_failures`].
/// The chaos layer adds:
///
/// 1. A pre-tick event-firing pass that crosses any scheduled events
///    whose `at_offset` is at-or-before the current loop-iteration
///    time.
/// 2. Per-event recording of [`FiredFaultEvent`] with the workload's
///    `inject_fault` outcome.
///
/// # Errors
///
/// Same as [`crate::soak::run_soak`]: zero `tick_interval` or zero
/// `duration` rejects.
pub fn run_chaos_soak<W: ChaosWorkload>(
    workload: &mut W,
    config: &SoakConfig,
    plan: &ChaosPlan,
) -> Result<ChaosReport, SoakHarnessError> {
    if config.tick_interval.is_zero() {
        return Err(SoakHarnessError::ZeroTickInterval);
    }
    if config.duration.is_zero() {
        return Err(SoakHarnessError::ZeroDuration);
    }

    // Sort events by offset so the harness can fire them in order
    // with a single forward-walking cursor.
    let mut sorted_events: Vec<ChaosScheduleEntry> = plan.events.clone();
    sorted_events.sort_by_key(|e| e.at_offset);
    let mut next_event_idx: usize = 0;

    let t0 = Instant::now();
    let mut tick_count: u64 = 0;
    let mut samples_ms: Vec<f64> = Vec::new();
    let mut failures: Vec<StabilityFailure> = Vec::new();
    let mut events_fired: Vec<FiredFaultEvent> = Vec::new();
    let mut stability_passed: u64 = 0;
    let mut stability_failed: u64 = 0;
    let mut last_tick_completed_at: Instant = t0;
    let mut last_stability_check_at: Instant = t0;

    loop {
        let now = Instant::now();
        let now_offset = now.duration_since(t0);
        if now_offset >= config.duration {
            break;
        }

        // Fire any chaos events whose offset has been crossed. Events
        // past the soak window are skipped (the loop exits first).
        while next_event_idx < sorted_events.len() {
            let entry = &sorted_events[next_event_idx];
            if entry.at_offset > now_offset {
                break;
            }
            if entry.at_offset >= config.duration {
                break;
            }
            let outcome = match workload.inject_fault(&entry.event) {
                Ok(()) => ChaosInjectionOutcome::Accepted,
                Err(err) => ChaosInjectionOutcome::Rejected(err),
            };
            events_fired.push(FiredFaultEvent {
                at_offset: now_offset,
                event_kind: entry.event.kind_tag().to_string(),
                injection_outcome: outcome,
                recovery_after: entry.event.recovery_after(),
            });
            next_event_idx = next_event_idx.saturating_add(1);
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
                        kind: crate::soak::StabilityFailureKind::DriftExceeded,
                        recorded_value: elapsed_ms,
                    });
                }
            }
            Err(SoakWorkloadError::OverrunBudget { elapsed }) => {
                let elapsed_ms = duration_to_ms(elapsed);
                samples_ms.push(elapsed_ms);
                failures.push(StabilityFailure {
                    at_offset: tick_started_at.duration_since(t0),
                    kind: crate::soak::StabilityFailureKind::DriftExceeded,
                    recorded_value: elapsed_ms,
                });
            }
            Err(SoakWorkloadError::StateCorruption { marker }) => {
                failures.push(StabilityFailure {
                    at_offset: tick_started_at.duration_since(t0),
                    kind: crate::soak::StabilityFailureKind::StateCorruptionDetected,
                    recorded_value: marker,
                });
            }
        }

        let tick_completed_at = Instant::now();
        let since_last_completion = tick_completed_at.duration_since(last_tick_completed_at);
        let stall_threshold = config.tick_interval.saturating_mul(2);
        if tick_count > 0 && since_last_completion > stall_threshold {
            failures.push(StabilityFailure {
                at_offset: tick_completed_at.duration_since(t0),
                kind: crate::soak::StabilityFailureKind::TickStalled,
                recorded_value: duration_to_ms(since_last_completion),
            });
        }
        last_tick_completed_at = tick_completed_at;

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

    let soak = SoakReport {
        duration_observed,
        tick_count,
        stability_checks_passed: stability_passed,
        stability_checks_failed: stability_failed,
        p50_tick_ms: p50,
        p95_tick_ms: p95,
        p99_tick_ms: p99,
        max_tick_ms: max,
        stability_failures: failures,
    };

    Ok(ChaosReport { soak, events_fired })
}

#[inline]
fn duration_to_ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000.0
}

fn compute_percentiles(samples: &mut [f64]) -> (f64, f64, f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let last_idx = samples.len() - 1;
    let pick = |q: f64| -> f64 {
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
mod chaos_pins {
    //! Evidence: EV-SOAK-CHAOS (T1) — in-source chaos-injection
    //! drift pins for the soak chaos extension (Wave 4d.4 PR 2 of
    //! 2-3).
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: exercises a
    //! discrete behavioural property of the unit under test against a
    //! small in-source fixture. The T2 corpus fixture-driven cross-
    //! property round-trip lives at
    //! `engine/observability/tests/chaos_soak_corpus.rs`.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md` row
    //! 40 (`EV-SOAK-CHAOS`) declares Generation Source "Soak + chaos
    //! validation runs". This T1 + the sibling T2 corpus extend the
    //! PR 1 harness scaffold pins with chaos-injection coverage; the
    //! row's `Planned -> Pass` transition lands when PR 3 (multi-
    //! profile validation corpus + closing-docs) ships under the Wave
    //! 4d.4 narrowing.

    use super::{
        ChaosInjectionError, ChaosInjectionOutcome, ChaosPlan, ChaosScheduleEntry, ChaosWorkload,
        FaultEvent, run_chaos_soak,
    };
    use crate::soak::{
        SoakConfig, SoakWorkload, SoakWorkloadError, StabilityFailureKind, run_soak,
    };
    use std::time::Duration;

    /// Constant-cost workload that accepts every fault. Used as the
    /// happy-path baseline.
    struct AcceptingWorkload {
        per_tick: Duration,
        injections_seen: usize,
    }

    impl SoakWorkload for AcceptingWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            Ok(self.per_tick)
        }
    }

    impl ChaosWorkload for AcceptingWorkload {
        fn inject_fault(&mut self, _event: &FaultEvent) -> Result<(), ChaosInjectionError> {
            self.injections_seen = self.injections_seen.saturating_add(1);
            Ok(())
        }
    }

    /// Workload that always rejects faults with a fixed reason.
    struct RejectingWorkload {
        per_tick: Duration,
        reason: ChaosInjectionError,
    }

    impl SoakWorkload for RejectingWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            Ok(self.per_tick)
        }
    }

    impl ChaosWorkload for RejectingWorkload {
        fn inject_fault(&mut self, _event: &FaultEvent) -> Result<(), ChaosInjectionError> {
            Err(self.reason.clone())
        }
    }

    /// Workload that, after `inject_fault` is called, surfaces a
    /// single `OverrunBudget` on its very next tick — used to drive
    /// the recovery-window classification pin.
    struct OneShotPostInjectionOverrunWorkload {
        per_tick: Duration,
        overrun_elapsed: Duration,
        injection_seen: bool,
        overrun_pending: bool,
    }

    impl SoakWorkload for OneShotPostInjectionOverrunWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            if self.overrun_pending {
                self.overrun_pending = false;
                Err(SoakWorkloadError::OverrunBudget {
                    elapsed: self.overrun_elapsed,
                })
            } else {
                Ok(self.per_tick)
            }
        }
    }

    impl ChaosWorkload for OneShotPostInjectionOverrunWorkload {
        fn inject_fault(&mut self, _event: &FaultEvent) -> Result<(), ChaosInjectionError> {
            self.injection_seen = true;
            self.overrun_pending = true;
            Ok(())
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
    fn chaos_plan_with_zero_events_matches_soak_run() {
        // An empty chaos plan reduces to a plain soak run: same tick
        // count band, same baseline stability metrics, no fired
        // events recorded.
        let mut chaos_workload = AcceptingWorkload {
            per_tick: Duration::from_micros(50),
            injections_seen: 0,
        };
        let mut soak_workload = AcceptingWorkload {
            per_tick: Duration::from_micros(50),
            injections_seen: 0,
        };
        let config = fast_config(Duration::from_millis(100));

        let chaos_report =
            run_chaos_soak(&mut chaos_workload, &config, &ChaosPlan::empty()).unwrap();
        let soak_report = run_soak(&mut soak_workload, &config).unwrap();

        assert!(
            chaos_report.events_fired.is_empty(),
            "empty plan should fire zero events; got {:?}",
            chaos_report.events_fired
        );
        // Both runs should land in the same tick-count band.
        let tick_diff = chaos_report
            .soak
            .tick_count
            .abs_diff(soak_report.tick_count);
        assert!(
            tick_diff <= 5,
            "tick_count diverged: chaos={} soak={}",
            chaos_report.soak.tick_count,
            soak_report.tick_count
        );
        assert_eq!(chaos_workload.injections_seen, 0);
    }

    #[test]
    fn single_event_fires_at_or_after_offset() {
        // Schedule a single event at 20ms; the harness should fire it
        // exactly once and the recorded at_offset should be >= 20ms.
        let mut workload = AcceptingWorkload {
            per_tick: Duration::from_micros(50),
            injections_seen: 0,
        };
        let plan = ChaosPlan {
            events: vec![ChaosScheduleEntry {
                at_offset: Duration::from_millis(20),
                event: FaultEvent::AdapterDropout {
                    adapter_id: "osc-in-7000".to_string(),
                    recovery_after: Duration::from_millis(50),
                },
            }],
        };
        let config = fast_config(Duration::from_millis(100));

        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        assert_eq!(report.events_fired.len(), 1);
        assert!(
            report.events_fired[0].at_offset >= Duration::from_millis(20),
            "fired at {:?}, expected >= 20ms",
            report.events_fired[0].at_offset
        );
        assert_eq!(report.events_fired[0].event_kind, "adapter-dropout");
        assert_eq!(
            report.events_fired[0].injection_outcome,
            ChaosInjectionOutcome::Accepted
        );
        assert_eq!(workload.injections_seen, 1);
    }

    #[test]
    fn multi_event_plan_fires_in_offset_order() {
        // Construct events out of order; the runner should sort and
        // fire them in ascending offset order.
        let mut workload = AcceptingWorkload {
            per_tick: Duration::from_micros(50),
            injections_seen: 0,
        };
        let plan = ChaosPlan {
            events: vec![
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(60),
                    event: FaultEvent::Custom {
                        name: "third".to_string(),
                        recovery_after: Duration::from_millis(10),
                    },
                },
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(20),
                    event: FaultEvent::Custom {
                        name: "first".to_string(),
                        recovery_after: Duration::from_millis(10),
                    },
                },
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(40),
                    event: FaultEvent::Custom {
                        name: "second".to_string(),
                        recovery_after: Duration::from_millis(10),
                    },
                },
            ],
        };
        let config = fast_config(Duration::from_millis(120));

        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        assert_eq!(report.events_fired.len(), 3);
        // Each subsequent firing's at_offset must be >= previous.
        let offsets: Vec<Duration> = report.events_fired.iter().map(|e| e.at_offset).collect();
        assert!(offsets[0] <= offsets[1], "{:?}", offsets);
        assert!(offsets[1] <= offsets[2], "{:?}", offsets);
        // And the firing at_offsets must be at-or-after the schedule
        // offsets in sorted order.
        assert!(offsets[0] >= Duration::from_millis(20));
        assert!(offsets[1] >= Duration::from_millis(40));
        assert!(offsets[2] >= Duration::from_millis(60));
    }

    #[test]
    fn injection_rejection_recorded_with_error_kind() {
        // Workload rejects every fault with AdapterNotFound; the
        // FiredFaultEvent should record the rejection variant
        // bit-exact.
        let mut workload = RejectingWorkload {
            per_tick: Duration::from_micros(50),
            reason: ChaosInjectionError::AdapterNotFound("missing-osc".to_string()),
        };
        let plan = ChaosPlan {
            events: vec![ChaosScheduleEntry {
                at_offset: Duration::from_millis(15),
                event: FaultEvent::AdapterDropout {
                    adapter_id: "missing-osc".to_string(),
                    recovery_after: Duration::from_millis(20),
                },
            }],
        };
        let config = fast_config(Duration::from_millis(80));

        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        assert_eq!(report.events_fired.len(), 1);
        match &report.events_fired[0].injection_outcome {
            ChaosInjectionOutcome::Rejected(ChaosInjectionError::AdapterNotFound(name)) => {
                assert_eq!(name, "missing-osc");
            }
            other => panic!("expected Rejected(AdapterNotFound), got {other:?}"),
        }
    }

    #[test]
    fn event_after_soak_window_does_not_fire() {
        // Schedule an event past the soak duration; it should NOT
        // fire.
        let mut workload = AcceptingWorkload {
            per_tick: Duration::from_micros(50),
            injections_seen: 0,
        };
        let plan = ChaosPlan {
            events: vec![ChaosScheduleEntry {
                at_offset: Duration::from_millis(500),
                event: FaultEvent::GpuDeviceLost {
                    recovery_after: Duration::from_millis(100),
                },
            }],
        };
        let config = fast_config(Duration::from_millis(60));

        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        assert!(
            report.events_fired.is_empty(),
            "expected no fired events, got {:?}",
            report.events_fired
        );
        assert_eq!(workload.injections_seen, 0);
    }

    #[test]
    fn recovery_window_failures_tagged_distinctly_from_independent_failures() {
        // Workload accepts the chaos event at ~15ms and surfaces an
        // OverrunBudget on the very next tick — that overrun's
        // at_offset should land within the 50ms recovery window of
        // the chaos event, so classification should be
        // ChaosRecoveryWindow.
        let mut workload = OneShotPostInjectionOverrunWorkload {
            per_tick: Duration::from_micros(50),
            overrun_elapsed: Duration::from_millis(80),
            injection_seen: false,
            overrun_pending: false,
        };
        let plan = ChaosPlan {
            events: vec![ChaosScheduleEntry {
                at_offset: Duration::from_millis(15),
                event: FaultEvent::AudioStreamInterruption {
                    recovery_after: Duration::from_millis(50),
                },
            }],
        };
        let config = fast_config(Duration::from_millis(100));

        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        assert!(
            workload.injection_seen,
            "workload should have seen injection"
        );
        assert_eq!(report.events_fired.len(), 1);
        // There should be exactly one DriftExceeded (the post-
        // injection overrun).
        let drifts: Vec<_> = report
            .soak
            .stability_failures
            .iter()
            .filter(|f| f.kind == StabilityFailureKind::DriftExceeded)
            .collect();
        assert_eq!(drifts.len(), 1, "expected exactly one DriftExceeded");

        let classifications = report.stability_failure_chaos_classification();
        assert_eq!(classifications.len(), report.soak.stability_failures.len());
        // The DriftExceeded should be tagged ChaosRecoveryWindow
        // (its at_offset is within 50ms of the chaos event firing).
        let drift_idx = report
            .soak
            .stability_failures
            .iter()
            .position(|f| f.kind == StabilityFailureKind::DriftExceeded)
            .unwrap();
        assert_eq!(
            classifications[drift_idx],
            super::StabilityFailureChaosClass::ChaosRecoveryWindow,
            "DriftExceeded at {:?} should be classified ChaosRecoveryWindow given chaos at {:?}",
            report.soak.stability_failures[drift_idx].at_offset,
            report.events_fired[0].at_offset
        );
    }
}

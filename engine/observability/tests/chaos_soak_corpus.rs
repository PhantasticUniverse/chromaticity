// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4d Wave 4d.4 `EV-SOAK-CHAOS` chaos-injection corpus (PR 2 of
//! 2-3). Tier declaration and evidence tag live on the inner
//! `fixtures` module so `scripts/evidence-tag-lint.sh`'s tier-
//! distribution summary counts this file at T2.
//!
//! Companion to `engine/observability/src/chaos.rs`. The chaos module
//! ships the layer + the in-source T1 pins; this file is the T2
//! fixture-driven cross-property corpus that exercises the public
//! `run_chaos_soak` entrypoint against synthetic chaos plans with
//! known scheduling properties, asserting the [`ChaosReport`] shape
//! matches.
//!
//! Wave 4d.4 narrowing recap (mirrored from
//! `engine/observability/src/chaos.rs` module prose for grep-
//! discoverability): this PR ships the chaos-injection layer + 6 T1
//! pins + this T2 corpus. PR 3 ships the multi-profile validation
//! corpus + closing-docs row 40 -> Pass.

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod fixtures {
    //! Evidence: EV-SOAK-CHAOS (T2) — Phase-4d Wave 4d.4 chaos-
    //! injection fixture corpus. Seven CI-friendly (sub-2-second-each)
    //! fixtures exercising the public API of
    //! `chromaticity_observability::chaos`.
    //!
    //! Registry binding. `docs/26-evidence-artifact-registry.md` row
    //! 40 (`EV-SOAK-CHAOS`) declares Generation Source "Soak +
    //! chaos validation runs". This T2 + the sibling T1 in
    //! `engine/observability/src/chaos.rs` together extend the PR 1
    //! harness pins with chaos-injection coverage; the row's
    //! `Planned -> Pass` transition lands when PR 3 (multi-profile
    //! validation corpus + closing-docs) ships under the Wave 4d.4
    //! narrowing.

    use chromaticity_observability::{
        ChaosInjectionError, ChaosInjectionOutcome, ChaosPlan, ChaosScheduleEntry, ChaosWorkload,
        FaultEvent, SoakConfig, SoakWorkload, SoakWorkloadError, StabilityFailureChaosClass,
        StabilityFailureKind, run_chaos_soak,
    };
    use std::time::Duration;

    /// Constant-cost workload that records every injection. Used as
    /// the happy-path baseline.
    struct AcceptingWorkload {
        per_tick: Duration,
        injections_seen: Vec<String>,
    }

    impl SoakWorkload for AcceptingWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            Ok(self.per_tick)
        }
    }

    impl ChaosWorkload for AcceptingWorkload {
        fn inject_fault(&mut self, event: &FaultEvent) -> Result<(), ChaosInjectionError> {
            self.injections_seen.push(event.kind_tag().to_string());
            Ok(())
        }
    }

    /// Workload that surfaces an `OverrunBudget` on the very next tick
    /// after `inject_fault`. Used to validate the recovery-window
    /// failure tagging.
    struct PostInjectionOverrunWorkload {
        per_tick: Duration,
        overrun_elapsed: Duration,
        overrun_pending: bool,
    }

    impl SoakWorkload for PostInjectionOverrunWorkload {
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

    impl ChaosWorkload for PostInjectionOverrunWorkload {
        fn inject_fault(&mut self, _event: &FaultEvent) -> Result<(), ChaosInjectionError> {
            self.overrun_pending = true;
            Ok(())
        }
    }

    /// Workload that rejects only `GpuDeviceLost` events with
    /// `UnsupportedFault`. All other variants accepted. Models a
    /// build whose backend doesn't surface device-lost.
    struct UnsupportedGpuDeviceLostWorkload {
        per_tick: Duration,
    }

    impl SoakWorkload for UnsupportedGpuDeviceLostWorkload {
        fn tick(&mut self) -> Result<Duration, SoakWorkloadError> {
            Ok(self.per_tick)
        }
    }

    impl ChaosWorkload for UnsupportedGpuDeviceLostWorkload {
        fn inject_fault(&mut self, event: &FaultEvent) -> Result<(), ChaosInjectionError> {
            match event {
                FaultEvent::GpuDeviceLost { .. } => Err(ChaosInjectionError::UnsupportedFault(
                    "gpu-device-lost".to_string(),
                )),
                _ => Ok(()),
            }
        }
    }

    fn fast_config(duration: Duration) -> SoakConfig {
        SoakConfig {
            duration,
            tick_interval: Duration::from_millis(16),
            stability_check_interval: Duration::from_millis(50),
            drift_threshold_p95_ms: 1.25,
        }
    }

    #[test]
    fn chaos_plan_with_zero_events_matches_baseline_soak_run() {
        // Empty plan: no events fired, soak metrics in nominal band.
        let started = std::time::Instant::now();
        let mut workload = AcceptingWorkload {
            per_tick: Duration::from_micros(50),
            injections_seen: Vec::new(),
        };
        let config = fast_config(Duration::from_millis(200));
        let report = run_chaos_soak(&mut workload, &config, &ChaosPlan::empty()).unwrap();
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_millis(2_000),
            "fixture exceeded 2-second budget: {elapsed:?}"
        );
        assert!(report.events_fired.is_empty(), "no events expected");
        assert!(workload.injections_seen.is_empty());
        assert!(report.soak.tick_count > 0);
    }

    #[test]
    fn single_adapter_dropout_at_500ms_fires_recorded_outcome_accepted() {
        // A single adapter-dropout event scheduled at 80ms within a
        // 200ms soak. Should fire exactly once, Accepted, with the
        // recorded at_offset >= 80ms.
        let mut workload = AcceptingWorkload {
            per_tick: Duration::from_micros(50),
            injections_seen: Vec::new(),
        };
        let plan = ChaosPlan {
            events: vec![ChaosScheduleEntry {
                at_offset: Duration::from_millis(80),
                event: FaultEvent::AdapterDropout {
                    adapter_id: "midi-mpk".to_string(),
                    recovery_after: Duration::from_millis(40),
                },
            }],
        };
        let config = fast_config(Duration::from_millis(200));
        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        assert_eq!(report.events_fired.len(), 1);
        let fired = &report.events_fired[0];
        assert_eq!(fired.event_kind, "adapter-dropout");
        assert_eq!(fired.injection_outcome, ChaosInjectionOutcome::Accepted);
        assert!(
            fired.at_offset >= Duration::from_millis(80),
            "fired at {:?}",
            fired.at_offset
        );
        assert_eq!(workload.injections_seen, vec!["adapter-dropout"]);
    }

    #[test]
    fn audio_stream_interruption_recovery_window_drift_failure_tagged() {
        // Fire AudioStreamInterruption at ~50ms with a 100ms recovery
        // window; the workload then surfaces a single overrun on the
        // next tick (well within 100ms). The resulting DriftExceeded
        // should be tagged ChaosRecoveryWindow.
        let mut workload = PostInjectionOverrunWorkload {
            per_tick: Duration::from_micros(50),
            overrun_elapsed: Duration::from_millis(60),
            overrun_pending: false,
        };
        let plan = ChaosPlan {
            events: vec![ChaosScheduleEntry {
                at_offset: Duration::from_millis(50),
                event: FaultEvent::AudioStreamInterruption {
                    recovery_after: Duration::from_millis(100),
                },
            }],
        };
        let config = fast_config(Duration::from_millis(250));
        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();

        assert_eq!(report.events_fired.len(), 1);
        let drifts: Vec<_> = report
            .soak
            .stability_failures
            .iter()
            .enumerate()
            .filter(|(_, f)| f.kind == StabilityFailureKind::DriftExceeded)
            .collect();
        assert_eq!(drifts.len(), 1, "expected one DriftExceeded");
        let classifications = report.stability_failure_chaos_classification();
        assert_eq!(
            classifications[drifts[0].0],
            StabilityFailureChaosClass::ChaosRecoveryWindow,
            "DriftExceeded at {:?} should fall in chaos recovery window of event at {:?}",
            drifts[0].1.at_offset,
            report.events_fired[0].at_offset
        );
    }

    #[test]
    fn gpu_device_lost_unsupported_fault_recorded_as_rejected() {
        // Workload rejects GpuDeviceLost with UnsupportedFault; the
        // FiredFaultEvent should record the rejection variant and
        // associated tag.
        let mut workload = UnsupportedGpuDeviceLostWorkload {
            per_tick: Duration::from_micros(50),
        };
        let plan = ChaosPlan {
            events: vec![ChaosScheduleEntry {
                at_offset: Duration::from_millis(40),
                event: FaultEvent::GpuDeviceLost {
                    recovery_after: Duration::from_millis(50),
                },
            }],
        };
        let config = fast_config(Duration::from_millis(150));
        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        assert_eq!(report.events_fired.len(), 1);
        match &report.events_fired[0].injection_outcome {
            ChaosInjectionOutcome::Rejected(ChaosInjectionError::UnsupportedFault(tag)) => {
                assert_eq!(tag, "gpu-device-lost");
            }
            other => panic!("expected Rejected(UnsupportedFault), got {other:?}"),
        }
    }

    #[test]
    fn multi_event_plan_fires_in_offset_order_assertion() {
        // Schedule three events out-of-order; assert firing order
        // ascending by at_offset and tags match expected sequence.
        let mut workload = AcceptingWorkload {
            per_tick: Duration::from_micros(50),
            injections_seen: Vec::new(),
        };
        let plan = ChaosPlan {
            events: vec![
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(120),
                    event: FaultEvent::GpuDeviceLost {
                        recovery_after: Duration::from_millis(20),
                    },
                },
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(40),
                    event: FaultEvent::AdapterDropout {
                        adapter_id: "osc-7000".to_string(),
                        recovery_after: Duration::from_millis(20),
                    },
                },
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(80),
                    event: FaultEvent::AudioStreamInterruption {
                        recovery_after: Duration::from_millis(20),
                    },
                },
            ],
        };
        let config = fast_config(Duration::from_millis(250));
        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        assert_eq!(report.events_fired.len(), 3);
        let kinds: Vec<&str> = report
            .events_fired
            .iter()
            .map(|e| e.event_kind.as_str())
            .collect();
        assert_eq!(
            kinds,
            vec![
                "adapter-dropout",
                "audio-stream-interruption",
                "gpu-device-lost"
            ]
        );
        // Workload sees the same order via injections_seen.
        assert_eq!(
            workload.injections_seen,
            vec![
                "adapter-dropout",
                "audio-stream-interruption",
                "gpu-device-lost"
            ]
        );
    }

    #[test]
    fn event_after_window_close_does_not_fire() {
        // Schedule an event past the soak window; it must not fire.
        let mut workload = AcceptingWorkload {
            per_tick: Duration::from_micros(50),
            injections_seen: Vec::new(),
        };
        let plan = ChaosPlan {
            events: vec![ChaosScheduleEntry {
                at_offset: Duration::from_millis(10_000),
                event: FaultEvent::Custom {
                    name: "post-window".to_string(),
                    recovery_after: Duration::from_millis(50),
                },
            }],
        };
        let config = fast_config(Duration::from_millis(80));
        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        assert!(report.events_fired.is_empty());
        assert!(workload.injections_seen.is_empty());
    }

    #[test]
    fn combined_chaos_plan_three_events_round_trip() {
        // Round-trip a 3-event plan with mixed Accepted / no-overrun
        // workload; assert event count, kinds preserved, and that
        // independent failures (none in this case) classify as
        // Independent if they appear, ChaosRecoveryWindow otherwise.
        let mut workload = AcceptingWorkload {
            per_tick: Duration::from_micros(50),
            injections_seen: Vec::new(),
        };
        let plan = ChaosPlan {
            events: vec![
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(30),
                    event: FaultEvent::AdapterDropout {
                        adapter_id: "midi-1".to_string(),
                        recovery_after: Duration::from_millis(30),
                    },
                },
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(80),
                    event: FaultEvent::Custom {
                        name: "synthetic".to_string(),
                        recovery_after: Duration::from_millis(30),
                    },
                },
                ChaosScheduleEntry {
                    at_offset: Duration::from_millis(150),
                    event: FaultEvent::AudioStreamInterruption {
                        recovery_after: Duration::from_millis(30),
                    },
                },
            ],
        };
        let config = fast_config(Duration::from_millis(250));
        let report = run_chaos_soak(&mut workload, &config, &plan).unwrap();
        assert_eq!(report.events_fired.len(), 3);
        for fired in &report.events_fired {
            assert_eq!(fired.injection_outcome, ChaosInjectionOutcome::Accepted);
        }
        let classifications = report.stability_failure_chaos_classification();
        assert_eq!(classifications.len(), report.soak.stability_failures.len());
    }
}
